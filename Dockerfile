# Build stage
FROM rust:slim-bookworm AS builder

WORKDIR /app

# Cache dependencies: copy manifests first, build a dummy project
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release && rm -rf src

# Build the real application
COPY src ./src
# Touch main.rs so cargo detects source change
RUN touch src/main.rs
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash appuser

COPY --from=builder /app/target/release/storm-api /usr/local/bin/storm-api
COPY migrations /app/migrations

WORKDIR /app
USER appuser

ENV APP_ADDR=0.0.0.0:3000
EXPOSE 3000

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

ENTRYPOINT ["/usr/local/bin/storm-api"]
