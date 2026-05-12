# syntax=docker/dockerfile:1.7
#
# Multi-stage build for the omniparse Axum web service with the ML OCR
# backend baked in. The final image ships a static-ish `web_service` binary
# plus the pre-downloaded, SHA-256-verified rten models at
# `/opt/omniparse/models`. Override the model directory at runtime with
# `-e OMNIPARSE_OCR_MODELS=/path -v host/models:/path`.
#
# Build: docker build -t omniparse-web:dev .
# Run:   docker run --rm -p 3000:3000 omniparse-web:dev

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
RUN cargo chef cook --release --features ocr-ml --recipe-path recipe.json
COPY . .
RUN cargo build --release --features ocr-ml --bin omniparse \
 && cargo build --release --features ocr-ml --example web_service

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
    OMNIPARSE_BIND=0.0.0.0:3000
COPY --from=models  /opt/omniparse/models                       /opt/omniparse/models
COPY --from=builder /src/target/release/examples/web_service    /usr/local/bin/web_service
EXPOSE 3000
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/web_service"]
