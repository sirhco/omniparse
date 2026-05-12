# Omniparse Web Service Guide

This guide shows you how to build a web service using Omniparse and Axum.
Two ready-to-run examples ship in `examples/`:

| Example                           | Audience                             | Use when                                  |
| --------------------------------- | ------------------------------------ | ----------------------------------------- |
| `examples/web_service.rs`         | Tutorial / minimal demo              | Reading the code; smallest learnable surface |
| `examples/web_service_prod.rs`    | Production — Cloud Run target        | You want to deploy this for real          |

The production example adds: structured Cloud Logging JSON, Prometheus
`/metrics`, distinct `/live` vs `/ready` probes (with model SHA-256
verification), body-size limit, request timeout, concurrency cap,
panic catcher, request-id propagation, `X-Cloud-Trace-Context` →
log/trace correlation, graceful shutdown sized for Cloud Run's 10 s
SIGKILL window, model prewarm, optional bearer-token auth, and a
`--healthcheck` mode so the binary itself serves as the Docker
`HEALTHCHECK` command on distroless. The published Docker image uses
**the production binary** as its `ENTRYPOINT`.

## Quick Start (minimal demo)

> **Port note:** the minimal demo (`examples/web_service.rs`) listens on
> **port 3000** by default. The production example
> (`examples/web_service_prod.rs`) — and the published Docker image — listen
> on **port 8080** (Cloud Run convention). When using the production binary,
> swap `:3000` for `:8080` in every `curl` below. See
> [Production example](#production-example-cloud-run).

### 1. Run the Example Server

```bash
cargo run --example web_service
```

The server starts on `http://localhost:3000`

### 2. Test with cURL

```bash
# Parse a JSON file
curl -X POST -F "file=@test_data/text/sample.json" http://localhost:3000/parse

# Detect file type only
curl -X POST -F "file=@test_data/document/sample.docx" http://localhost:3000/detect

# Get metadata only (no content)
curl -X POST -F "file=@test_data/text/sample.csv" http://localhost:3000/parse?metadata_only=true
```

### OCR-specific fixtures

The repo ships pre-generated OCR test files at `test_data/ocr/`. They're
rendered from a system font into clean printed Latin — easy targets for
both backends and a quick way to confirm the container's ML OCR pipeline
is wired correctly.

```bash
# Single-line image
curl -s -X POST -F "file=@test_data/ocr/hello_world.png" http://localhost:3000/parse | jq .
# expected: "content": "HELLO WORLD", "ocr_status": "recognized"

# Multi-line image (mixed case + digits)
curl -s -X POST -F "file=@test_data/ocr/multi_line.png" http://localhost:3000/parse | jq .content

# JPEG variant (same text, lossy compression)
curl -s -X POST -F "file=@test_data/ocr/hello_world.jpg" http://localhost:3000/parse | jq .content

# Scanned PDF (single page, image-only, no text layer)
curl -s -X POST -F "file=@test_data/ocr/scanned.pdf" http://localhost:3000/parse | jq .
# expected: "ocr_images_total": 1, "ocr_images_recognized": 1
```

Regenerate or extend with custom strings:

```sh
cargo run --features ocr-train --example create_ocr_fixtures
# or specify font + output dir
cargo run --features ocr-train --example create_ocr_fixtures -- \
    /System/Library/Fonts/Supplemental/Arial.ttf  test_data/ocr
```

### 3. Run the Test Script

```bash
bash examples/test_web_service.sh
```

### 4. Run the Client Example

```bash
# In one terminal
cargo run --example web_service

# In another terminal
cargo run --example web_client
```

## API Reference

### Endpoints

#### `GET /`
Root endpoint with service information.

**Response:**
```
Omniparse Web Service

Endpoints:
  POST /parse - Parse file
  POST /detect - Detect file type
  GET /health - Health check
```

#### `GET /health`
Health check endpoint.

**Response:**
```json
{
  "status": "healthy",
  "service": "omniparse-web"
}
```

#### `POST /parse`
Parse a file and extract content and metadata.

**Request:**
- Content-Type: `multipart/form-data`
- Field: `file` (the file to parse)
- Query Parameters:
  - `metadata_only` (optional, boolean): If true, only metadata is returned

**Response:**
```json
{
  "filename": "sample.json",
  "mime_type": "application/json",
  "detection_confidence": 0.95,
  "metadata": {
    "valid": true,
    "schema_info": "object{author, data, name, version}"
  },
  "content": "author: Omniparse Test\n..."
}
```

**Content Types:**
- Text content: Returns as string
- Binary content: Returns size and hex preview
- No content: Returns null

#### `POST /detect`
Detect file type without parsing content.

**Request:**
- Content-Type: `multipart/form-data`
- Field: `file` (the file to detect)

**Response:**
```json
{
  "filename": "sample.docx",
  "mime_type": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "confidence": 0.95,
  "detected_by": "MagicBytes"
}
```

### Error Responses

All errors return JSON with this structure:

```json
{
  "error": "error_type",
  "message": "Detailed error message"
}
```

**Error Types:**

| Status Code | Error Type | Description |
|-------------|------------|-------------|
| 400 | `multipart_error` | Invalid multipart data |
| 400 | `missing_file` | No file in request |
| 422 | `parse_error` | File parsing failed |

## Integration Examples

### JavaScript/TypeScript (Node.js)

```javascript
const FormData = require('form-data');
const fs = require('fs');
const axios = require('axios');

async function parseFile(filePath) {
  const form = new FormData();
  form.append('file', fs.createReadStream(filePath));
  
  const response = await axios.post('http://localhost:3000/parse', form, {
    headers: form.getHeaders()
  });
  
  return response.data;
}

// Usage
parseFile('document.pdf').then(result => {
  console.log('MIME Type:', result.mime_type);
  console.log('Metadata:', result.metadata);
});
```

### Python

```python
import requests

def parse_file(file_path):
    with open(file_path, 'rb') as f:
        files = {'file': f}
        response = requests.post('http://localhost:3000/parse', files=files)
        return response.json()

# Usage
result = parse_file('document.pdf')
print(f"MIME Type: {result['mime_type']}")
print(f"Metadata: {result['metadata']}")
```

### Rust (using reqwest)

See `examples/web_client.rs` for a complete example.

```rust
use reqwest::multipart;

async fn parse_file(file_path: &str) -> Result<ParseResponse, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let file_data = std::fs::read(file_path)?;
    
    let form = multipart::Form::new()
        .part("file", multipart::Part::bytes(file_data)
            .file_name("document.pdf"));
    
    let response = client
        .post("http://localhost:3000/parse")
        .multipart(form)
        .send()
        .await?
        .json()
        .await?;
    
    Ok(response)
}
```

## Production Deployment

### Configuration

For production, you'll want to add:

1. **Environment Variables**
```rust
let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
let addr = SocketAddr::from(([0, 0, 0, 0], port.parse().unwrap()));
```

2. **Request Size Limits**
```rust
use axum::extract::DefaultBodyLimit;

let app = Router::new()
    .route("/parse", post(parse_file))
    .layer(DefaultBodyLimit::max(10 * 1024 * 1024)); // 10MB limit
```

3. **CORS Support**
```rust
use tower_http::cors::{CorsLayer, Any};

let app = Router::new()
    .route("/parse", post(parse_file))
    .layer(CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any));
```

4. **Logging**
```rust
use tower_http::trace::TraceLayer;

let app = Router::new()
    .route("/parse", post(parse_file))
    .layer(TraceLayer::new_for_http());
```

5. **Rate Limiting**
```rust
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

let governor_conf = Box::new(
    GovernorConfigBuilder::default()
        .per_second(10)
        .burst_size(20)
        .finish()
        .unwrap()
);

let app = Router::new()
    .route("/parse", post(parse_file))
    .layer(GovernorLayer { config: Box::leak(governor_conf) });
```

### Docker Deployment

A production-ready multi-stage `Dockerfile` lives at the project root. It
builds the production example (`examples/web_service_prod.rs`) as its
`ENTRYPOINT`, bakes the ML OCR models into the image at
`/opt/omniparse/models` (SHA-256 verified at build time), and ships a
distroless runtime under a non-root UID.

```sh
# Build locally
docker build -t omniparse-web:dev .

# Or use the pre-published image (multi-arch: linux/amd64 + linux/arm64)
docker pull ghcr.io/sirhco/omniparse-web:latest
docker run --rm -p 8080:8080 ghcr.io/sirhco/omniparse-web:latest
```

For local development, a `docker-compose.yml` is also provided:

```sh
docker compose up --build
curl -s http://localhost:8080/ready
```

Container baseline runtime config (override via `-e VAR=value`):

| Env var                | Default                  | Purpose                                  |
| ---------------------- | ------------------------ | ---------------------------------------- |
| `PORT`                 | `8080`                   | Listener port (Cloud Run injects this)   |
| `OMNIPARSE_OCR`        | `ml`                     | OCR backend (`off` / `classical` / `ml`) |
| `OMNIPARSE_OCR_MODELS` | `/opt/omniparse/models`  | Where to read the rten models from       |

Full env var reference for the production binary is in the
[Production example](#production-example-cloud-run) section below.

To use a host-side model cache instead of the baked-in one, mount a volume
over `/opt/omniparse/models` (or override `OMNIPARSE_OCR_MODELS` and mount
elsewhere):

```sh
docker run --rm -p 8080:8080 \
  -e OMNIPARSE_OCR_MODELS=/models \
  -v "$PWD/my-models:/models:ro" \
  ghcr.io/sirhco/omniparse-web:latest
```

Want the minimal demo binary inside the same image? Override the
entrypoint:

```sh
docker run --rm -p 3000:3000 \
  --entrypoint /usr/local/bin/web_service \
  -e OMNIPARSE_BIND=0.0.0.0:3000 \
  ghcr.io/sirhco/omniparse-web:latest
```

### Performance Tips

1. **Use Connection Pooling**: Reuse the `Extractor` instance
2. **Enable Parallel Processing**: Use the `parallel` feature for batch operations
3. **Stream Large Files**: Use streaming for files > 10MB
4. **Cache Detection Results**: Cache MIME type detection for known file signatures
5. **Set Timeouts**: Add request timeouts to prevent hanging

## Security Considerations

1. **File Size Limits**: Always set maximum file size limits
2. **File Type Validation**: Validate MIME types before processing
3. **Sanitize Filenames**: Clean user-provided filenames
4. **Rate Limiting**: Implement rate limiting per IP
5. **Input Validation**: Validate all query parameters
6. **Error Messages**: Don't expose internal paths or system info in errors

## Monitoring

Add health checks and metrics:

```rust
#[derive(Serialize)]
struct Metrics {
    requests_total: u64,
    requests_success: u64,
    requests_failed: u64,
    avg_processing_time_ms: f64,
}

async fn metrics() -> Json<Metrics> {
    // Implement metrics collection
    Json(Metrics { /* ... */ })
}
```

## Testing

Run integration tests:

```bash
# Start the server
cargo run --example web_service &
SERVER_PID=$!

# Run tests
bash examples/test_web_service.sh

# Stop the server
kill $SERVER_PID
```

## Troubleshooting

### "Connection refused"
- Ensure the server is running
- Check the port is not already in use: `lsof -i :3000`

### "File too large"
- Increase the body size limit in Axum configuration

### "Parse error"
- Check the file format is supported: `omniparse --help`
- Verify the file is not corrupted

### "Out of memory"
- Enable streaming for large files
- Reduce concurrent request limits

## Production example (Cloud Run)

`examples/web_service_prod.rs` is built around Google Cloud Run's runtime
contract: single listener on `$PORT`, structured Cloud Logging JSON to
stdout, SIGTERM-driven 8 s graceful shutdown, IAM-based auth at the LB
(`--no-allow-unauthenticated`).

### Endpoints

| Method | Path        | Auth                   | Purpose                                       |
| ------ | ----------- | ---------------------- | --------------------------------------------- |
| GET    | /           | none                   | Service banner                                |
| GET    | /live       | none                   | Liveness — always 200                         |
| GET    | /ready      | none                   | Readiness — 200 only when models verify       |
| POST   | /parse      | IAM (or bearer)        | Multipart parse                               |
| POST   | /detect     | IAM (or bearer)        | Multipart detect                              |
| GET    | /metrics    | `X-Admin-Token` header | Prometheus exposition                         |
| GET    | /debug/info | `X-Admin-Token` header | Version + model status                        |

### Local development

```sh
cargo run --features ocr-ml --example web_service_prod
# in another terminal:
curl -sf http://localhost:8080/live
curl -s  http://localhost:8080/ready
curl -s -X POST -F file=@test_data/ocr/hello_world.png http://localhost:8080/parse | jq .content
# admin endpoints
OMNIPARSE_ADMIN_TOKEN=adm cargo run --features ocr-ml --example web_service_prod &
curl -sH "X-Admin-Token: adm" http://localhost:8080/metrics | head -10
curl -sH "X-Admin-Token: adm" http://localhost:8080/debug/info | jq .
```

### Configuration

All settings come from environment variables. Defaults are tuned for
Cloud Run; override per environment.

| Var                          | Default              | Purpose                                      |
| ---------------------------- | -------------------- | -------------------------------------------- |
| `PORT`                       | `8080`               | Cloud Run injects this                       |
| `OMNIPARSE_BIND_ADDR`        | `0.0.0.0`            | Interface to bind                            |
| `OMNIPARSE_MAX_BODY_BYTES`   | `26214400` (25 MB)   | Multipart body cap                           |
| `OMNIPARSE_REQUEST_TIMEOUT_S`| `60`                 | Per-request timeout                          |
| `OMNIPARSE_MAX_CONCURRENCY`  | `num_cpus * 2`       | Concurrent parses (above → 429)              |
| `OMNIPARSE_AUTH_TOKEN`       | unset (use IAM)      | Bearer fallback for non-Google deploys       |
| `OMNIPARSE_ADMIN_TOKEN`      | unset (admin off)    | Header gate for `/metrics` and `/debug/*`    |
| `OMNIPARSE_CORS_ORIGINS`     | unset (no CORS)      | Comma-separated allow list or `*`            |
| `OMNIPARSE_LOG`              | `info`               | `RUST_LOG`-style filter                      |
| `OMNIPARSE_LOG_FORMAT`       | `cloud` on Cloud Run, else `pretty` | `cloud`, `json`, `pretty`     |
| `OMNIPARSE_SHUTDOWN_GRACE_S` | `8`                  | Drain window (Cloud Run kills at 10 s)       |
| `OMNIPARSE_READY_CACHE_S`    | `60`                 | TTL on cached `verify_all()` result          |
| `OMNIPARSE_PREWARM`          | `1`                  | Load ML engine before listening              |

### Deploy to Cloud Run

```sh
bash deploy/cloud-run/deploy.sh <gcp-project> <region> [caller-sa-email]
```

This script:

1. Creates a runtime service account (`omniparse-web@PROJECT.iam.gserviceaccount.com`) and grants it Cloud Observability roles:
   - `roles/logging.logWriter`
   - `roles/monitoring.metricWriter`
   - `roles/cloudtrace.agent`
2. Deploys `ghcr.io/sirhco/omniparse-web:latest` with
   `--no-allow-unauthenticated`. Cloud Run validates each caller's identity
   token against the service URL before forwarding.
3. Optionally grants `roles/run.invoker` to a caller service account so a
   workload identity can call `/parse` and `/detect`.

### Calling the deployed service

```sh
URL=$(gcloud run services describe omniparse-web --region <REGION> --format='value(status.url)')
TOKEN=$(gcloud auth print-identity-token \
    --impersonate-service-account=client-app@<PROJECT>.iam.gserviceaccount.com \
    --audiences="$URL")
curl -sf -H "Authorization: Bearer $TOKEN" \
    -X POST -F file=@test_data/ocr/hello_world.png "$URL/parse" | jq .content
```

### Inspecting telemetry

```sh
# Logs (Cloud Logging structured JSON, trace IDs auto-correlated)
gcloud logging read \
    "resource.type=cloud_run_revision AND resource.labels.service_name=omniparse-web" \
    --project <PROJECT> --limit 5 --format json | jq '.[].jsonPayload'

# Trace timeline (populated by X-Cloud-Trace-Context propagation)
gcloud trace list-traces --project <PROJECT> --limit 5

# Cloud Run revision metrics (request count, p50/p95/p99 latency) come for
# free via Cloud Monitoring without app instrumentation.
```

Application-level Prometheus metrics live at `GET /metrics` (admin-token
gated). Counters: `omniparse_parse_total`, `omniparse_detect_total`,
`omniparse_error_total{code="..."}`. Histograms: `omniparse_parse_seconds`.

### Manifest path (advanced)

If you prefer a declarative deploy, `deploy/cloud-run/service.yaml` is a
Knative-style manifest you can apply with
`gcloud run services replace`. Edit `serviceAccountName` and `image`
first, or generate it from the `gcloud run deploy` output.

### What's still your responsibility

- TLS: Cloud Run terminates HTTPS at the LB; the app sees HTTP. Good.
- Public access: leave `--no-allow-unauthenticated` on. Grant
  `roles/run.invoker` only to specific SAs.
- Secrets: store `OMNIPARSE_AUTH_TOKEN` (if used) and `OMNIPARSE_ADMIN_TOKEN`
  in Secret Manager and reference them via `--set-secrets`.
- Quotas / rate limits: Cloud Run has per-service concurrency; for
  per-caller quotas, front the service with Cloud Armor or API Gateway.
- File-content scanning: omniparse is pure-Rust (no shellouts) and far
  safer than Tika, but untrusted PDFs / Office docs are still untrusted
  input. Treat parse output accordingly.

## Further Reading

- [Axum Documentation](https://docs.rs/axum/)
- [Omniparse Documentation](../README.md)
- [Tokio Runtime Guide](https://tokio.rs/)
- [Cloud Run docs](https://cloud.google.com/run/docs)
- [Cloud Logging structured logs](https://cloud.google.com/logging/docs/structured-logging)
