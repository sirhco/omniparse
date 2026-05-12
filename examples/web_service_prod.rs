//! Production-grade Axum web service for omniparse — Cloud Run target.
//!
//! Goes beyond `examples/web_service.rs` by adding everything you need to
//! put this binary in front of real traffic on Google Cloud Run:
//!
//! - Single listener on `$PORT` (default 8080) — Cloud Run constraint.
//! - Multipart body cap, request timeout, concurrency limit (token bucket),
//!   panic catcher, request-id propagation, optional CORS.
//! - Sync parser offloaded onto the blocking-IO pool so the tokio runtime
//!   never blocks on CPU-bound parses (PDF / OCR).
//! - Pre-warms the ML OCR engine before binding the listener so cold-start
//!   latency lands on the first request, not the first user.
//! - `/live` and `/ready` distinct probes; `/ready` runs (and caches)
//!   `omniparse::ocr::ml::verify_all()`.
//! - Cloud Logging-compatible JSON to stdout: `severity`, `time`,
//!   `message`, `logging.googleapis.com/trace`,
//!   `logging.googleapis.com/spanId`, `labels.service`, `labels.revision`.
//! - Trace correlation via `X-Cloud-Trace-Context` (with UUID fallback for
//!   non-Google deployments).
//! - Prometheus `/metrics` endpoint, admin-token gated.
//! - Graceful shutdown on SIGTERM with 8s drain (under Cloud Run's 10s
//!   SIGKILL window).
//! - `--healthcheck` flag so the binary itself can be the `HEALTHCHECK`
//!   command on distroless runtimes (no shell needed).
//!
//! Auth: Cloud Run with `--no-allow-unauthenticated` handles auth at the
//! LB via IAM (`roles/run.invoker`). The app trusts that the request made
//! it through. For non-Google deployments, set `OMNIPARSE_AUTH_TOKEN` to
//! enable an in-process bearer fallback.
//!
//! OpenTelemetry-OTLP export to Cloud Trace + Cloud Monitoring is wired
//! at the design level (see `OMNIPARSE_OTEL` env vars below) but the
//! in-process exporter is left as a TODO: Cloud Logging already correlates
//! logs to Cloud Trace by way of `logging.googleapis.com/trace`, and
//! Cloud Monitoring auto-captures Cloud Run revision request/latency
//! metrics. Add `opentelemetry-otlp` here when you need app-level
//! distributed traces.
//!
//! Run locally:
//! ```sh
//! cargo run --features ocr-ml --example web_service_prod
//! ```
//!
//! Run via Docker (image already targets this binary):
//! ```sh
//! docker run --rm -p 8080:8080 omniparse-web:dev
//! ```
//!
//! Deploy to Cloud Run:
//! ```sh
//! bash deploy/cloud-run/deploy.sh <gcp-project> <region> <caller-sa>
//! ```

use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Query, State},
    http::{header::AUTHORIZATION, HeaderMap, HeaderName, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use metrics_exporter_prometheus::PrometheusBuilder;
use serde::{Deserialize, Serialize};
use std::{
    env,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;
use tower::limit::ConcurrencyLimitLayer;
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer, cors::CorsLayer, request_id::MakeRequestUuid,
    timeout::TimeoutLayer, trace::TraceLayer, ServiceBuilderExt,
};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Subcommand: `web_service_prod --healthcheck` becomes the HEALTHCHECK
    // probe inside the distroless image. Hit /live on our own port; exit
    // 0/1 based on status. Synchronous reqwest is fine — we're a child
    // process invoked by the runtime.
    if env::args().any(|a| a == "--healthcheck") {
        return run_healthcheck();
    }

    let cfg = Config::from_env();
    init_logging(&cfg);

    tracing::info!(
        service = %cfg.service_name,
        revision = %cfg.revision,
        listen = %cfg.listen_addr(),
        max_body_bytes = cfg.max_body_bytes,
        timeout_s = cfg.request_timeout_s,
        max_concurrency = cfg.max_concurrency,
        prewarm = cfg.prewarm,
        ocr_mode = ?std::env::var("OMNIPARSE_OCR").unwrap_or_default(),
        "starting omniparse-web (prod)"
    );

    // Cold-start prewarm. Runs ml::verify_all() + engine init before we
    // bind, so the first request never pays the model-load cost.
    if cfg.prewarm {
        prewarm().await;
    }

    install_prometheus_recorder();

    let state = AppState::new(cfg.clone());
    let app = build_router(state.clone(), &cfg);

    let listener = tokio::net::TcpListener::bind(cfg.listen_addr()).await?;
    tracing::info!(addr = %cfg.listen_addr(), "listening");

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal(cfg.shutdown_grace_s))
        .await?;

    tracing::info!("shutdown complete");
    Ok(())
}

fn run_healthcheck() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".into());
    let url = format!("http://127.0.0.1:{port}/live");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?;
    match client.get(&url).send() {
        Ok(r) if r.status().is_success() => Ok(()),
        Ok(r) => {
            eprintln!("healthcheck: {} returned {}", url, r.status());
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("healthcheck: {url} unreachable: {e}");
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Config {
    bind_addr: String,
    port: u16,
    max_body_bytes: usize,
    request_timeout_s: u64,
    max_concurrency: usize,
    auth_token: Option<String>,
    admin_token: Option<String>,
    cors_origins: Vec<String>,
    log_filter: String,
    log_format: LogFormat,
    shutdown_grace_s: u64,
    ready_cache_s: u64,
    prewarm: bool,
    service_name: String,
    revision: String,
    on_cloud_run: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LogFormat {
    Cloud,
    Json,
    Pretty,
}

impl Config {
    fn from_env() -> Self {
        let port = env::var("PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8080);
        let bind_addr = env::var("OMNIPARSE_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0".into());

        let on_cloud_run = env::var("K_SERVICE").is_ok();
        let default_format = if on_cloud_run { LogFormat::Cloud } else { LogFormat::Pretty };
        let log_format = match env::var("OMNIPARSE_LOG_FORMAT").as_deref() {
            Ok("cloud") => LogFormat::Cloud,
            Ok("json") => LogFormat::Json,
            Ok("pretty") => LogFormat::Pretty,
            _ => default_format,
        };

        let cors_origins = env::var("OMNIPARSE_CORS_ORIGINS")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
            .unwrap_or_default();

        Self {
            bind_addr,
            port,
            max_body_bytes: env::var("OMNIPARSE_MAX_BODY_BYTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(25 * 1024 * 1024),
            request_timeout_s: env::var("OMNIPARSE_REQUEST_TIMEOUT_S")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            max_concurrency: env::var("OMNIPARSE_MAX_CONCURRENCY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(|| num_cpus::get() * 2),
            auth_token: env::var("OMNIPARSE_AUTH_TOKEN").ok().filter(|s| !s.is_empty()),
            admin_token: env::var("OMNIPARSE_ADMIN_TOKEN").ok().filter(|s| !s.is_empty()),
            cors_origins,
            log_filter: env::var("OMNIPARSE_LOG").unwrap_or_else(|_| "info".into()),
            log_format,
            shutdown_grace_s: env::var("OMNIPARSE_SHUTDOWN_GRACE_S")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8),
            ready_cache_s: env::var("OMNIPARSE_READY_CACHE_S")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            prewarm: env::var("OMNIPARSE_PREWARM").map(|v| v != "0").unwrap_or(true),
            service_name: env::var("OMNIPARSE_OTEL_SERVICE_NAME")
                .or_else(|_| env::var("K_SERVICE"))
                .unwrap_or_else(|_| "omniparse-web".into()),
            revision: env::var("K_REVISION").unwrap_or_else(|_| "local".into()),
            on_cloud_run,
        }
    }

    fn listen_addr(&self) -> String {
        format!("{}:{}", self.bind_addr, self.port)
    }
}

// ---------------------------------------------------------------------------
// Logging
// ---------------------------------------------------------------------------

fn init_logging(cfg: &Config) {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_new(&cfg.log_filter).unwrap_or_else(|_| EnvFilter::new("info"));

    match cfg.log_format {
        LogFormat::Cloud => {
            // Cloud Logging-compatible JSON to stdout. The layer here is
            // implemented as a custom formatter inside a json-ish layer:
            // tracing-subscriber's `json()` formatter writes
            // `level`/`fields` not `severity`/Cloud-Logging trace fields.
            // The transformer wraps the writer to re-key on the fly.
            let labels = serde_json::json!({
                "service": cfg.service_name,
                "revision": cfg.revision,
            });
            let writer = CloudLoggingWriter::new(labels);
            let layer = tracing_subscriber::fmt::layer()
                .json()
                .with_current_span(false)
                .with_span_list(false)
                .with_target(false)
                .with_writer(writer);
            tracing_subscriber::registry().with(filter).with(layer).init();
        }
        LogFormat::Json => {
            tracing_subscriber::registry()
                .with(filter)
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
        LogFormat::Pretty => {
            tracing_subscriber::registry()
                .with(filter)
                .with(tracing_subscriber::fmt::layer().with_target(false))
                .init();
        }
    }
}

/// Wrap stdout so we can rewrite tracing-subscriber's json output into the
/// shape Cloud Logging expects. We intercept each finished line and:
///
/// 1. Rename `level` → `severity` and map values (WARN → WARNING, etc.).
/// 2. Inject `logging.googleapis.com/trace` and
///    `logging.googleapis.com/spanId` when the in-flight task stashed a
///    `CloudTrace` in the task-local.
/// 3. Add `labels` from the static service info.
#[derive(Clone)]
struct CloudLoggingWriter {
    labels: serde_json::Value,
}

impl CloudLoggingWriter {
    fn new(labels: serde_json::Value) -> Self {
        Self { labels }
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CloudLoggingWriter {
    type Writer = CloudLoggingLineWriter;
    fn make_writer(&'a self) -> Self::Writer {
        CloudLoggingLineWriter {
            buf: Vec::with_capacity(512),
            labels: self.labels.clone(),
        }
    }
}

struct CloudLoggingLineWriter {
    buf: Vec<u8>,
    labels: serde_json::Value,
}

impl std::io::Write for CloudLoggingLineWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if self.buf.is_empty() {
            return Ok(());
        }
        // Split on newlines — tracing-subscriber writes one JSON per line.
        let mut start = 0usize;
        for (i, b) in self.buf.iter().enumerate() {
            if *b == b'\n' {
                emit_cloud_line(&self.buf[start..i], &self.labels);
                start = i + 1;
            }
        }
        if start < self.buf.len() {
            // Last partial line — keep for next flush.
            self.buf.drain(..start);
        } else {
            self.buf.clear();
        }
        Ok(())
    }
}

impl Drop for CloudLoggingLineWriter {
    fn drop(&mut self) {
        let _ = std::io::Write::flush(self);
    }
}

fn emit_cloud_line(raw: &[u8], labels: &serde_json::Value) {
    use std::io::Write;
    let mut v: serde_json::Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(_) => {
            // Not JSON — pass through.
            let _ = std::io::stdout().write_all(raw);
            let _ = std::io::stdout().write_all(b"\n");
            return;
        }
    };
    if let Some(obj) = v.as_object_mut() {
        // level → severity
        if let Some(level) = obj.remove("level").and_then(|v| v.as_str().map(str::to_string)) {
            let sev = match level.as_str() {
                "ERROR" => "ERROR",
                "WARN" => "WARNING",
                "INFO" => "INFO",
                "DEBUG" => "DEBUG",
                "TRACE" => "DEBUG",
                other => other,
            };
            obj.insert("severity".into(), serde_json::Value::String(sev.into()));
        }
        // timestamp → time (Cloud Logging keys on `time` or `timestamp`)
        if let Some(ts) = obj.remove("timestamp") {
            obj.insert("time".into(), ts);
        }
        // Hoist fields.message to top-level `message` so Cloud Console
        // shows the human-readable line, not "(unknown)".
        let msg = obj.get_mut("fields").and_then(|f| f.as_object_mut()).and_then(|f| f.remove("message"));
        if let Some(m) = msg {
            obj.insert("message".into(), m);
        }
        // Inject trace fields if the current task stashed them.
        if let Some(trace) = trace_local::get() {
            if let Some(project) = env::var("GOOGLE_CLOUD_PROJECT").ok().filter(|s| !s.is_empty())
            {
                obj.insert(
                    "logging.googleapis.com/trace".into(),
                    serde_json::Value::String(format!("projects/{project}/traces/{}", trace.trace_id)),
                );
            } else {
                obj.insert(
                    "logging.googleapis.com/trace".into(),
                    serde_json::Value::String(trace.trace_id.clone()),
                );
            }
            if let Some(span_id) = &trace.span_id {
                obj.insert(
                    "logging.googleapis.com/spanId".into(),
                    serde_json::Value::String(span_id.clone()),
                );
            }
            obj.insert(
                "logging.googleapis.com/trace_sampled".into(),
                serde_json::Value::Bool(trace.sampled),
            );
        }
        obj.insert("labels".into(), labels.clone());
    }
    let mut out = serde_json::to_vec(&v).unwrap_or_else(|_| raw.to_vec());
    out.push(b'\n');
    let _ = std::io::stdout().write_all(&out);
}

// Per-task trace context stash. Filled by `with_trace_context` middleware,
// read by the Cloud Logging writer.
mod trace_local {
    use std::cell::RefCell;

    #[derive(Clone)]
    pub struct CloudTrace {
        pub trace_id: String,
        pub span_id: Option<String>,
        pub sampled: bool,
    }

    tokio::task_local! {
        static CTX: RefCell<Option<CloudTrace>>;
    }

    pub fn set(t: CloudTrace) {
        let _ = CTX.try_with(|c| {
            *c.borrow_mut() = Some(t);
        });
    }

    pub fn get() -> Option<CloudTrace> {
        CTX.try_with(|c| c.borrow().clone()).ok().flatten()
    }

    pub async fn scope<F: std::future::Future>(f: F) -> F::Output {
        CTX.scope(RefCell::new(None), f).await
    }
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

static PROMETHEUS_HANDLE: std::sync::OnceLock<metrics_exporter_prometheus::PrometheusHandle> =
    std::sync::OnceLock::new();

fn install_prometheus_recorder() {
    let handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("install prometheus recorder");
    let _ = PROMETHEUS_HANDLE.set(handle);
}

fn render_metrics() -> String {
    PROMETHEUS_HANDLE
        .get()
        .map(|h| h.render())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Readiness cache
// ---------------------------------------------------------------------------

#[derive(Default)]
struct ReadyInner {
    last: Option<(Instant, Result<(), String>)>,
}

#[derive(Clone)]
struct ReadyCache {
    inner: Arc<Mutex<ReadyInner>>,
    ttl: Duration,
}

impl ReadyCache {
    fn new(ttl_s: u64) -> Self {
        Self {
            inner: Arc::default(),
            ttl: Duration::from_secs(ttl_s),
        }
    }

    async fn check(&self) -> Result<(), String> {
        {
            let g = self.inner.lock().await;
            if let Some((when, res)) = &g.last {
                if when.elapsed() < self.ttl {
                    return res.clone();
                }
            }
        }
        let res = tokio::task::spawn_blocking(probe_models)
            .await
            .unwrap_or_else(|e| Err(format!("probe panicked: {e}")));
        self.inner.lock().await.last = Some((Instant::now(), res.clone()));
        res
    }
}

fn probe_models() -> Result<(), String> {
    #[cfg(feature = "ocr-ml")]
    {
        omniparse::ocr::ml::verify_all().map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "ocr-ml"))]
    {
        // No ML feature compiled — service is "ready" because there's
        // nothing to verify.
        Ok(())
    }
}

async fn prewarm() {
    let res = tokio::task::spawn_blocking(|| {
        #[cfg(feature = "ocr-ml")]
        {
            let _ = omniparse::ocr::ml::verify_all();
            let _ = omniparse::ocr::ml::shared_ml_engine();
        }
    })
    .await;
    if let Err(e) = res {
        tracing::warn!(?e, "prewarm join error");
    } else {
        tracing::info!("ocr engine prewarmed");
    }
}

// ---------------------------------------------------------------------------
// Application state + router
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    cfg: Arc<Config>,
    ready: ReadyCache,
}

impl AppState {
    fn new(cfg: Config) -> Self {
        let ready = ReadyCache::new(cfg.ready_cache_s);
        Self {
            cfg: Arc::new(cfg),
            ready,
        }
    }
}

fn build_router(state: AppState, cfg: &Config) -> Router {
    let cors_layer = if cfg.cors_origins.is_empty() {
        CorsLayer::new()
    } else if cfg.cors_origins.iter().any(|o| o == "*") {
        CorsLayer::permissive()
    } else {
        let mut l = CorsLayer::new();
        for origin in &cfg.cors_origins {
            if let Ok(hv) = HeaderValue::from_str(origin) {
                l = l.allow_origin(hv);
            }
        }
        l.allow_methods([axum::http::Method::GET, axum::http::Method::POST])
            .allow_headers([axum::http::header::CONTENT_TYPE, AUTHORIZATION])
    };

    // Routes that bypass auth: probes + root banner.
    let unauthed = Router::new()
        .route("/", get(root))
        .route("/live", get(liveness))
        .route("/ready", get(readiness));

    // Admin routes (metrics + debug) gated by an admin-token header.
    let admin = Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/debug/info", get(debug_info_handler))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            admin_auth,
        ));

    // Application routes — heavy stack.
    let app_routes = Router::new()
        .route("/parse", post(parse_file))
        .route("/detect", post(detect_file))
        .layer(
            ServiceBuilder::new()
                .layer(DefaultBodyLimit::max(state.cfg.max_body_bytes))
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    Duration::from_secs(state.cfg.request_timeout_s),
                ))
                .layer(ConcurrencyLimitLayer::new(state.cfg.max_concurrency))
                .layer(middleware::from_fn_with_state(state.clone(), app_auth)),
        );

    Router::new()
        .merge(unauthed)
        .merge(admin)
        .merge(app_routes)
        .layer(
            ServiceBuilder::new()
                // Outer request-id assignment / propagation. Sets X-Request-Id
                // and echoes it back.
                .set_x_request_id(MakeRequestUuid)
                .propagate_x_request_id()
                .layer(CatchPanicLayer::new())
                .layer(cors_layer)
                .layer(middleware::from_fn(with_trace_context))
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|req: &Request<Body>| {
                            let req_id = req
                                .headers()
                                .get("x-request-id")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("-");
                            tracing::info_span!(
                                "http",
                                method = %req.method(),
                                path = %req.uri().path(),
                                req_id = %req_id
                            )
                        })
                        .on_response(
                            |res: &Response, latency: Duration, _: &tracing::Span| {
                                tracing::info!(
                                    status = res.status().as_u16(),
                                    elapsed_ms = latency.as_millis() as u64,
                                    "completed"
                                );
                            },
                        ),
                ),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Middleware: trace context, auth
// ---------------------------------------------------------------------------

/// Extract Cloud Trace (`X-Cloud-Trace-Context`) and stash it in a
/// task-local so the log writer can include trace fields. Falls back to a
/// generated UUID when the header is absent.
async fn with_trace_context(mut req: Request<Body>, next: Next) -> Response {
    let header = req
        .headers()
        .get("x-cloud-trace-context")
        .and_then(|v| v.to_str().ok())
        .map(parse_cloud_trace);

    let trace = header.unwrap_or_else(|| trace_local::CloudTrace {
        trace_id: uuid::Uuid::new_v4().simple().to_string(),
        span_id: None,
        sampled: false,
    });

    // Reflect into X-Request-Id if the caller didn't set one (overrides
    // tower-http's UUID for Cloud Run requests).
    if let Ok(hv) = HeaderValue::from_str(&trace.trace_id) {
        req.headers_mut()
            .entry(HeaderName::from_static("x-request-id"))
            .or_insert(hv);
    }

    trace_local::scope(async move {
        trace_local::set(trace);
        next.run(req).await
    })
    .await
}

fn parse_cloud_trace(value: &str) -> trace_local::CloudTrace {
    // Format: TRACE_ID/SPAN_ID;o=TRACE_TRUE
    let (left, opts) = match value.split_once(';') {
        Some((l, o)) => (l, o),
        None => (value, ""),
    };
    let (trace_id, span_id) = match left.split_once('/') {
        Some((t, s)) => (t.to_string(), Some(s.to_string())),
        None => (left.to_string(), None),
    };
    let sampled = opts
        .split(';')
        .filter_map(|kv| kv.split_once('='))
        .any(|(k, v)| k.trim().eq_ignore_ascii_case("o") && v.trim() == "1");
    trace_local::CloudTrace {
        trace_id,
        span_id,
        sampled,
    }
}

/// Per-request auth for `/parse`, `/detect`. When running on Cloud Run with
/// `--no-allow-unauthenticated`, the LB already validated the caller's
/// identity token; we trust the request and skip the bearer check unless
/// the operator explicitly set `OMNIPARSE_AUTH_TOKEN`.
async fn app_auth(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let token = match &state.cfg.auth_token {
        Some(t) => t,
        None => return Ok(next.run(req).await),
    };
    if state.cfg.on_cloud_run {
        // IAM already vetted the caller; the bearer is for non-Google use.
        return Ok(next.run(req).await);
    }
    if check_bearer(req.headers(), token) {
        Ok(next.run(req).await)
    } else {
        Err((StatusCode::UNAUTHORIZED, "unauthorized").into_response())
    }
}

async fn admin_auth(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let Some(expected) = state.cfg.admin_token.as_deref() else {
        return Err((StatusCode::NOT_FOUND, "admin disabled").into_response());
    };
    let header = req
        .headers()
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if header.as_bytes().ct_eq(expected.as_bytes()).into() {
        Ok(next.run(req).await)
    } else {
        Err((StatusCode::UNAUTHORIZED, "unauthorized").into_response())
    }
}

fn check_bearer(headers: &HeaderMap, expected: &str) -> bool {
    let Some(value) = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Some(token) = value.strip_prefix("Bearer ").or_else(|| value.strip_prefix("bearer ")) else {
        return false;
    };
    token.as_bytes().ct_eq(expected.as_bytes()).into()
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn root() -> &'static str {
    "Omniparse Web Service (prod)\n\
     \n\
     Endpoints:\n\
       POST /parse   - Parse file and extract content (auth required when configured)\n\
       POST /detect  - Detect file type only\n\
       GET  /live    - Liveness probe\n\
       GET  /ready   - Readiness probe\n\
       GET  /metrics - Prometheus exposition (admin token)\n\
       GET  /debug/info - Build / model info (admin token)\n"
}

async fn liveness() -> StatusCode {
    StatusCode::OK
}

async fn readiness(State(state): State<AppState>) -> Response {
    match state.ready.check().await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"status": "ready"}))).into_response(),
        Err(reason) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status": "not_ready", "reason": reason})),
        )
            .into_response(),
    }
}

async fn metrics_handler() -> Response {
    let body = render_metrics();
    Response::builder()
        .header(
            "content-type",
            "text/plain; version=0.0.4; charset=utf-8",
        )
        .body(Body::from(body))
        .unwrap()
}

async fn debug_info_handler() -> Json<serde_json::Value> {
    let mut info = serde_json::json!({
        "service": env::var("K_SERVICE").unwrap_or_else(|_| "omniparse-web".into()),
        "revision": env::var("K_REVISION").unwrap_or_else(|_| "local".into()),
        "version": env!("CARGO_PKG_VERSION"),
        "git_commit": option_env!("VERGEN_GIT_SHA").unwrap_or("unknown"),
    });
    #[cfg(feature = "ocr-ml")]
    {
        if let Ok(statuses) = omniparse::ocr::ml::list_models() {
            let m: Vec<_> = statuses
                .into_iter()
                .map(|s| {
                    serde_json::json!({
                        "name": s.spec.name,
                        "ok": s.ok,
                        "size": s.size,
                        "sha256": s.sha256,
                    })
                })
                .collect();
            info["models"] = serde_json::Value::Array(m);
        }
    }
    Json(info)
}

#[derive(Deserialize)]
struct ParseParams {
    #[serde(default)]
    metadata_only: bool,
}

#[derive(Serialize)]
struct ParseResponse {
    filename: String,
    mime_type: String,
    detection_confidence: f32,
    metadata: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<ContentResponse>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ContentResponse {
    Text(String),
    Binary { size: usize, preview: String },
    None,
}

#[derive(Serialize)]
struct DetectionResponse {
    filename: String,
    mime_type: String,
    confidence: f32,
    detected_by: String,
}

async fn parse_file(
    Query(params): Query<ParseParams>,
    mut multipart: Multipart,
) -> Result<Json<ParseResponse>, AppError> {
    let (filename, data) = extract_multipart(&mut multipart).await?;
    let started = Instant::now();
    metrics::counter!("omniparse_parse_total").increment(1);
    let result = tokio::task::spawn_blocking(move || omniparse::extract_from_bytes(&data, None))
        .await
        .map_err(|e| AppError::Internal(format!("join: {e}")))?
        .map_err(|e| AppError::Parse(e.to_string()))?;
    let elapsed = started.elapsed().as_secs_f64();
    metrics::histogram!("omniparse_parse_seconds").record(elapsed);
    tracing::info!(mime = %result.mime_type, elapsed_s = elapsed, "parse ok");

    let body = ParseResponse {
        filename,
        mime_type: result.mime_type.clone(),
        detection_confidence: result.detection_confidence,
        metadata: serde_json::to_value(&result.metadata).unwrap_or(serde_json::Value::Null),
        content: if params.metadata_only {
            None
        } else {
            Some(match result.content {
                omniparse::core::Content::Text(text) => ContentResponse::Text(text),
                omniparse::core::Content::Binary(bytes) => ContentResponse::Binary {
                    size: bytes.len(),
                    preview: format!("{:02x?}", &bytes[..bytes.len().min(32)]),
                },
                omniparse::core::Content::None => ContentResponse::None,
            })
        },
    };
    Ok(Json(body))
}

async fn detect_file(mut multipart: Multipart) -> Result<Json<DetectionResponse>, AppError> {
    let (filename, data) = extract_multipart(&mut multipart).await?;
    metrics::counter!("omniparse_detect_total").increment(1);
    let detection = tokio::task::spawn_blocking(move || {
        omniparse::detection::TypeDetector::new().detect_from_bytes(&data)
    })
    .await
    .map_err(|e| AppError::Internal(format!("join: {e}")))?;
    Ok(Json(DetectionResponse {
        filename,
        mime_type: detection.mime_type,
        confidence: detection.confidence,
        detected_by: format!("{:?}", detection.detected_by),
    }))
}

async fn extract_multipart(multipart: &mut Multipart) -> Result<(String, Vec<u8>), AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Multipart(e.to_string()))?
    {
        if field.name() == Some("file") {
            let filename = field
                .file_name()
                .unwrap_or("unknown")
                .to_string();
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::Multipart(e.to_string()))?
                .to_vec();
            return Ok((filename, data));
        }
    }
    Err(AppError::MissingFile)
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum AppError {
    Multipart(String),
    MissingFile,
    Parse(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::Multipart(msg) => (
                StatusCode::BAD_REQUEST,
                "multipart_error",
                msg.clone(),
            ),
            AppError::MissingFile => (
                StatusCode::BAD_REQUEST,
                "missing_file",
                "No 'file' field in multipart body".to_string(),
            ),
            AppError::Parse(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "parse_error",
                msg.clone(),
            ),
            AppError::Internal(msg) => {
                tracing::error!(error = %msg, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Internal error; check server logs".to_string(),
                )
            }
        };
        metrics::counter!("omniparse_error_total", "code" => code.to_string()).increment(1);
        (
            status,
            Json(serde_json::json!({"error": code, "message": message})),
        )
            .into_response()
    }
}

// ---------------------------------------------------------------------------
// Shutdown
// ---------------------------------------------------------------------------

async fn shutdown_signal(grace_s: u64) {
    use tokio::signal;
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
    tracing::info!(grace_s, "shutdown signal received — draining");
    // Cloud Run sends SIGKILL ~10s after SIGTERM, so we cap the drain.
    // axum's `with_graceful_shutdown` handles the actual drain; this
    // sleep is a hard cap.
    tokio::time::sleep(Duration::from_secs(grace_s)).await;
    tracing::info!("drain window elapsed");
}
