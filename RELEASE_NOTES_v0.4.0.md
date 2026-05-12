# Omniparse v0.4.0 — Release Notes

Released: 2026-05-12

This release focuses on first-run UX. v0.3 shipped two working OCR backends
but the "try it out" path was rough: no way to pre-fetch models, fragmented
docs, no published container image, and a two-variable env-var dance to
enable ML OCR. v0.4 fixes all four.

## Highlights

### `omniparse models` CLI

New subcommand for the ML OCR model cache:

```sh
omniparse models path           # where do the models live?
omniparse models download       # pull all required models (~12 MB)
omniparse models download --force
omniparse models list           # name, size, sha256, ok/missing
omniparse models verify         # re-hash every cached model
```

Each model is now pinned to a known SHA-256; downloads stream through a
`Sha256` hasher and a mismatch is a hard error (`OcrError::ChecksumMismatch`).
The previously-orphaned `sha2` dependency is finally doing work.

### One env var to enable OCR

```sh
OMNIPARSE_OCR=ml          omniparse photo.jpg
OMNIPARSE_OCR=classical   omniparse scan.png
OMNIPARSE_OCR=off         omniparse photo.jpg   # also: unset
```

Old form (`OMNIPARSE_OCR=1 OMNIPARSE_OCR_ML=1`) still works but emits a
one-shot deprecation warning. Removal scheduled for 0.5.

### Container image with ML models baked in

A real `Dockerfile` lives at the project root. Multi-stage build:

1. cargo-chef planner + cook so dependency layers cache between code edits
2. Builder compiles the `omniparse` CLI and the `web_service` example
3. Model stage runs `omniparse models download && omniparse models verify`
   against `OMNIPARSE_OCR_MODELS=/opt/omniparse/models`
4. Distroless runtime layer copies just the binary + the model directory

Pull the published image (after the first tagged release):

```sh
docker run --rm -p 3000:3000 ghcr.io/sirhco/omniparse-web:v0.4.0
curl -s -X POST -F file=@photo.jpg http://localhost:3000/parse | jq .content
```

`docker-compose.yml` is provided for local development
(`docker compose up --build`).

### Single canonical OCR doc

`OCR_GUIDE.md` is now the single reference. README points there with a
6-line quickstart. The backend chooser, model-cache CLI section, tuning
table, library API, FAQ — all live in one place.

## Breaking changes

None at the lib API level. The legacy env-var form still works.

## Migration tips

If you have scripts using the old form, update them when convenient:

```diff
-OMNIPARSE_OCR=1 OMNIPARSE_OCR_ML=1 omniparse photo.jpg
+OMNIPARSE_OCR=ml omniparse photo.jpg
```

If you previously relied on the silent first-run model download, consider
pre-fetching in CI / Dockerfile builds:

```sh
omniparse models download
```

If you maintain a `Dockerfile` cribbed from `examples/WEB_SERVICE_GUIDE.md`,
the new project-root `Dockerfile` is a drop-in replacement — distroless,
multi-arch ready, models baked in.

## Internals

- `src/ocr/ml.rs`: `ModelSpec` + pub `prefetch_all` / `verify_all` /
  `list_models`. `ensure_model` / `download_to` are `pub(crate)` and take
  a `ModelSpec` instead of three positional args.
- `src/ocr/mod.rs`: `OcrMode` enum + `ocr_mode()` reader; `runtime_enabled()`
  / `ml_enabled()` are now thin wrappers.
- `src/cli/args.rs`: subcommand-aware `Cli` with `args_conflicts_with_subcommands`
  + `subcommand_negates_reqs`; bare-args extraction still works.
- `src/cli/models.rs` (new): subcommand handlers, `#[cfg(feature = "ocr-ml")]`
  gated.

### Production web service example (Cloud Run)

`examples/web_service_prod.rs` is a separate, production-grade Axum
example built around Google Cloud Run's runtime contract. Sits alongside
the minimal `web_service.rs` demo — that one stays unchanged.

Features baked in:

- **Cloud Logging-compatible JSON** stdout (severity, time, message,
  `logging.googleapis.com/trace`, `logging.googleapis.com/spanId`, labels)
  with one-shot `tracing::Layer` that rewrites the line shape.
- **Trace propagation** from `X-Cloud-Trace-Context` → log/trace
  correlation in Cloud Console; UUID fallback for non-Google use.
- **Prometheus** `/metrics` (admin-token gated) — counters
  `omniparse_parse_total`, `omniparse_detect_total`,
  `omniparse_error_total{code="..."}`; histogram
  `omniparse_parse_seconds`. Cloud Monitoring picks up Cloud Run
  revision-level metrics (latency, request count) automatically.
- **Distinct `/live` and `/ready`** probes. Readiness lazily runs
  `omniparse::ocr::ml::verify_all()` with a 60 s TTL cache.
- **Request body limit** (25 MB default), **timeout** (60 s default),
  **concurrency cap** (`2 × CPU` default → 429 with `Retry-After`),
  **panic catcher**, **request-id** propagation, optional **CORS**.
- **CPU-bound parsing offloaded** to `tokio::task::spawn_blocking` so
  the tokio runtime never blocks on PDF/OCR work.
- **Model prewarm** before the listener binds; cold-start cost lands on
  startup, not on the first user request.
- **Graceful shutdown** on SIGTERM with 8 s drain (under Cloud Run's
  10 s SIGKILL window).
- **`--healthcheck` mode** on the binary itself — distroless-compatible
  HEALTHCHECK command without needing a shell.
- **Auth strategy**: IAM-first (Cloud Run `--no-allow-unauthenticated`
  validates identity tokens at the LB; the app trusts the request).
  In-app `OMNIPARSE_AUTH_TOKEN` bearer fallback for non-Google deploys
  using constant-time compare via `subtle`.

The published Docker image ships **this binary** as its `ENTRYPOINT`.

Deploy to Cloud Run with the included one-shot script:

```sh
bash deploy/cloud-run/deploy.sh <gcp-project> <region> <caller-sa-email>
```

The script creates a runtime service account, grants Cloud Observability
write roles (`logging.logWriter`, `monitoring.metricWriter`,
`cloudtrace.agent`), deploys with `--no-allow-unauthenticated`, and
optionally grants `roles/run.invoker` to a caller service account.

See `examples/WEB_SERVICE_GUIDE.md` → "Production example (Cloud Run)"
for the full env-var reference, manifest path, and telemetry inspection
commands.

### OCR test fixtures

`test_data/ocr/` ships four small fixtures (~48 KB) you can hit the
service with out of the box:

| File                | What it tests                                       |
| ------------------- | --------------------------------------------------- |
| `hello_world.png`   | Single-line clean Latin OCR                         |
| `hello_world.jpg`   | Same content via lossy JPEG                         |
| `multi_line.png`    | 3 lines, mixed case + digits + punctuation          |
| `scanned.pdf`       | Image-only PDF — exercises PDF OCR (DCTDecode) path |

Generated by `examples/create_ocr_fixtures.rs` (`--features ocr-train`,
pure Rust via `ab_glyph` + `lopdf`). All four are recognized at 0.9
confidence by the ML backend.

## What's not in this release

- The legacy `OMNIPARSE_OCR_ML` will be **removed** (not just deprecated) in
  0.5. Switch when convenient.
- `omniparse models rm` was intentionally left out of this release. If you
  need it, `rm -rf "$(omniparse models path)"` works today.
- No CI-published binary release for the CLI itself yet — only the Docker
  image. A `cargo install omniparse --features ocr-ml` still works from
  crates.io as before.
- `web_service_prod` does not bundle an in-process OpenTelemetry OTLP
  exporter. Cloud Logging already correlates logs ↔ Cloud Trace via
  `logging.googleapis.com/trace`, and Cloud Run revision metrics ship
  free via Cloud Monitoring. Add `opentelemetry-otlp` if you need
  app-level distributed traces from a non-GCP environment.
