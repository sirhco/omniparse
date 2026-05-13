# syntax=docker/dockerfile:1.7
#
# Multi-stage build for the omniparse Axum web service. Built with every
# capability flag enabled so the published image is a single drop-in for
# all real-world inputs:
#
#   Feature        Effect
#   -----------    ---------------------------------------------------------
#   pdf            Strict-tier PDF parser (lopdf) + lenient raw_scan
#                  fallback (FlateDecode / LZWDecode / ASCII85Decode).
#                  Default-on; bundled via the implicit default set.
#   pdf-extract    Fourth-tier PDF parser (pdf-extract crate) — handles
#                  linearized PDFs and Identity-H + /ToUnicode CMaps that
#                  lopdf rejects (Lucidchart exports, Word/PowerPoint
#                  print-to-PDF, browser print-to-PDF).
#   ocr            Classical pure-Rust OCR pipeline (default off; pulled
#                  in transitively by ocr-ml and ocr-parallel).
#   ocr-ml         ML OCR backend via ocrs + rten with auto-downloaded,
#                  SHA-256-verified models at /opt/omniparse/models.
#   ocr-train      Train custom glyph prototypes from a TTF/OTF font for
#                  the classical OCR pipeline.
#   ocr-parallel   Per-region OCR recognition parallelized via rayon.
#                  Implies `ocr` + `parallel`.
#   async          tokio-backed async extraction (extract_from_path_async).
#   parallel       Rayon-backed batch processing (process_files_parallel).
#   markdown, svg, webp, epub, mp3
#                  Format parsers bundled by default.
#
# Security posture: omniparse is pure-Rust end-to-end (rustls for HTTPS,
# pure-Rust image / PDF / OCR stacks), so we statically link against musl
# and ship the final image on `gcr.io/distroless/static-debian12` —
# **zero glibc** in the runtime layer. No libssl, no libgcc, no glibc
# CVE surface. Image runs as non-root UID 65532.
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

ARG FEATURES="ocr-ml ocr-train ocr-parallel pdf-extract async parallel"

# ---------- cargo-chef base ----------
# `cargo-chef` separates dependency compilation from source compilation so
# code-only changes don't re-pull/recompile the dependency graph. Musl
# toolchain enables fully-static binaries that run on a glibc-free
# distroless runtime.
FROM rust:1-slim-bookworm AS chef
ARG TARGETARCH
WORKDIR /src
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
        pkg-config \
        ca-certificates \
        musl-tools \
        musl-dev \
 && rm -rf /var/lib/apt/lists/* \
 && cargo install cargo-chef --locked --version ^0.1 \
 && case "${TARGETARCH}" in \
        amd64) echo "x86_64-unknown-linux-musl" > /tmp/rust-target ;; \
        arm64) echo "aarch64-unknown-linux-musl" > /tmp/rust-target ;; \
        "")    echo "x86_64-unknown-linux-musl" > /tmp/rust-target ;; \
        *)     echo "unsupported TARGETARCH=${TARGETARCH}" >&2 && exit 1 ;; \
    esac \
 && rustup target add "$(cat /tmp/rust-target)"

# ---------- recipe planner ----------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---------- dependency build + binary build ----------
FROM chef AS builder
ARG FEATURES
COPY --from=planner /src/recipe.json recipe.json
RUN TARGET="$(cat /tmp/rust-target)" \
 && cargo chef cook --release --target "$TARGET" \
        --features "$FEATURES" --recipe-path recipe.json
COPY . .
RUN TARGET="$(cat /tmp/rust-target)" \
 && cargo build --release --target "$TARGET" --features "$FEATURES" --bin omniparse \
 && cargo build --release --target "$TARGET" --features "$FEATURES" --example web_service \
 && cargo build --release --target "$TARGET" --features "$FEATURES" --example web_service_prod \
 && mkdir -p /out \
 && cp "target/${TARGET}/release/omniparse" /out/omniparse \
 && cp "target/${TARGET}/release/examples/web_service" /out/web_service \
 && cp "target/${TARGET}/release/examples/web_service_prod" /out/web_service_prod

# ---------- model fetch + verify ----------
# Isolated stage so the model layer only invalidates when the model URLs or
# pinned sha256 change — not on every code edit. Runs on debian-slim only
# because we need a working shell to invoke `omniparse models download`;
# the actual binary is the same musl-static one we ship at runtime.
FROM debian:bookworm-slim AS models
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /out/omniparse /usr/local/bin/omniparse
ENV OMNIPARSE_OCR_MODELS=/opt/omniparse/models
RUN omniparse models download && omniparse models verify

# ---------- runtime ----------
# `distroless/static-debian12` carries only ca-certificates + tzdata +
# /etc/passwd — no glibc, no shell, no package manager. Anything dynamic
# (libssl, libstdc++, libgcc) is absent. A musl-static Rust binary is all
# we ship.
FROM gcr.io/distroless/static-debian12 AS runtime
ENV OMNIPARSE_OCR_MODELS=/opt/omniparse/models \
    OMNIPARSE_OCR=ml \
    PORT=8080
COPY --from=models  /opt/omniparse/models         /opt/omniparse/models
# Ship all three binaries. ENTRYPOINT picks the prod web service; ops can
# override with `--entrypoint /usr/local/bin/web_service` for the minimal
# demo, or `--entrypoint /usr/local/bin/omniparse` for the CLI.
COPY --from=builder /out/web_service              /usr/local/bin/web_service
COPY --from=builder /out/web_service_prod         /usr/local/bin/web_service_prod
COPY --from=builder /out/omniparse                /usr/local/bin/omniparse
EXPOSE 8080
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/web_service_prod"]
