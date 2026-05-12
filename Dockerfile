# syntax=docker/dockerfile:1.7
#
# Multi-stage build for the omniparse Axum web service. Built with every
# capability flag enabled so the published image is a single drop-in for
# all real-world inputs:
#
#   Feature       Effect
#   ----------    ----------------------------------------------------------
#   ocr-ml        Classical + ML OCR for images and scanned PDFs.
#   pdf           Strict-tier PDF parser (lopdf) + lenient raw_scan fallback
#                 (FlateDecode / LZWDecode / ASCII85Decode). On by default.
#   pdf-extract   Fourth-tier PDF parser (pdf-extract crate) — handles
#                 linearized PDFs and Identity-H + /ToUnicode CMaps that
#                 lopdf rejects (Lucidchart exports, Word print-to-PDF, etc.).
#
# Ship the production `web_service_prod` example binary plus the
# pre-downloaded, SHA-256-verified rten models at `/opt/omniparse/models`.
# Override the model directory at runtime with
# `-e OMNIPARSE_OCR_MODELS=/path -v host/models:/path`.
#
# Build: docker build -t omniparse-web:dev .
# Run:   docker run --rm -p 8080:8080 omniparse-web:dev
#
# Cloud Run: the image listens on $PORT (default 8080) and emits structured
# Cloud Logging JSON automatically when K_SERVICE is set. See
# deploy/cloud-run/deploy.sh for a one-shot deploy script.

# ---------- cargo-chef base ----------
# `cargo-chef` separates dependency compilation from source compilation so
# code-only changes don't re-pull/recompile the dependency graph.
FROM rust:1-slim-bookworm AS chef
WORKDIR /src
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
        pkg-config \
        ca-certificates \
        libssl-dev \
 && rm -rf /var/lib/apt/lists/* \
 && cargo install cargo-chef --locked --version ^0.1

# ---------- recipe planner ----------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---------- dependency build + binary build ----------
FROM chef AS builder
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release --features "ocr-ml pdf-extract" --recipe-path recipe.json
COPY . .
RUN cargo build --release --features "ocr-ml pdf-extract" --bin omniparse \
 && cargo build --release --features "ocr-ml pdf-extract" --example web_service \
 && cargo build --release --features "ocr-ml pdf-extract" --example web_service_prod

# ---------- model fetch + verify ----------
# Isolated stage so the model layer only invalidates when the model URLs or
# pinned sha256 change — not on every code edit.
FROM debian:bookworm-slim AS models
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /src/target/release/omniparse /usr/local/bin/omniparse
ENV OMNIPARSE_OCR_MODELS=/opt/omniparse/models
RUN omniparse models download && omniparse models verify

# ---------- runtime ----------
FROM gcr.io/distroless/cc-debian12 AS runtime
ENV OMNIPARSE_OCR_MODELS=/opt/omniparse/models \
    OMNIPARSE_OCR=ml \
    PORT=8080
COPY --from=models  /opt/omniparse/models                          /opt/omniparse/models
# Ship both binaries. ENTRYPOINT picks the prod one; ops can override with
# `--entrypoint /usr/local/bin/web_service` for the minimal demo.
COPY --from=builder /src/target/release/examples/web_service       /usr/local/bin/web_service
COPY --from=builder /src/target/release/examples/web_service_prod  /usr/local/bin/web_service_prod
COPY --from=builder /src/target/release/omniparse                  /usr/local/bin/omniparse
EXPOSE 8080
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/web_service_prod"]
