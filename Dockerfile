# syntax=docker/dockerfile:1.7

# Build stage
FROM rust:slim-bookworm AS builder

WORKDIR /app

ARG CARGO_PROFILE_RELEASE_LTO
ARG CARGO_PROFILE_RELEASE_CODEGEN_UNITS
ARG CARGO_PROFILE_RELEASE_PANIC
ARG CARGO_PROFILE_RELEASE_STRIP
ARG CARGO_PROFILE_RELEASE_DEBUG

ENV CARGO_PROFILE_RELEASE_LTO=${CARGO_PROFILE_RELEASE_LTO} \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=${CARGO_PROFILE_RELEASE_CODEGEN_UNITS} \
    CARGO_PROFILE_RELEASE_PANIC=${CARGO_PROFILE_RELEASE_PANIC} \
    CARGO_PROFILE_RELEASE_STRIP=${CARGO_PROFILE_RELEASE_STRIP} \
    CARGO_PROFILE_RELEASE_DEBUG=${CARGO_PROFILE_RELEASE_DEBUG}

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    && rm -rf /var/lib/apt/lists/* /var/cache/apt/*

# Cache dependencies: copy manifests first, build a dummy project
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    cargo build --release --locked && rm -rf src

# Build the real application
COPY src ./src
# Touch main.rs so cargo detects source change
RUN touch src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    cargo build --release --locked && strip target/release/storm-api

# Runtime stage
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* /var/cache/apt/* \
    && rm -rf /usr/share/doc/* /usr/share/man/* /usr/share/locale/*

RUN useradd --create-home --shell /bin/bash appuser

COPY --from=builder --chown=appuser:appuser /app/target/release/storm-api /usr/local/bin/storm-api
COPY --chown=appuser:appuser migrations /app/migrations

WORKDIR /app
USER appuser

ENV APP_ADDR=0.0.0.0:3000
EXPOSE 3000

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

ENTRYPOINT ["/usr/local/bin/storm-api"]
