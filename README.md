# Storm API

> Production-grade Axum + SQLx backend for fuel-station operations, wallet-style card balances, commission workflows, and loyalty-aware transaction processing.

[![API workflow](https://github.com/ardinbig/storm-api/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ardinbig/storm-api/actions/workflows/ci.yml)
[![codecov](https://codecov.io/github/ardinbig/storm-api/graph/badge.svg?token=WcHmafLVMx)](https://codecov.io/github/ardinbig/storm-api)
[![Rust](https://img.shields.io/badge/Rust-1.94.1%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

`storm-api` is a modular Rust REST API built on [Axum](https://docs.rs/axum), [SQLx](https://docs.rs/sqlx), PostgreSQL, and Redis. It is designed for operational reliability: structured logs, explicit health checks, graceful shutdown, OpenAPI-driven documentation, and a test layout that spans unit, integration, and end-to-end scenarios.

---

## Features

### Auth & Security
- Dual JWT auth flows with Argon2id password hashing.
- JWT revocation via Redis blocklist.

### Cards & Customers
- NFC card registry with PIN-protected balance checks.
- Customer profiles with card linkage and category assignment.

### Agents & Transactions
- Agent account management with transaction history.
- Agent-led customer registration with NFC card assignment.
- Cash withdrawal with PIN verification, commission split, and house account tracking.
- Configurable commission tier history with L-1 and L-2 MLM bonuses.

### Fuel & Loyalty
- Fuel consumption logging with pricing per type.
- 2-level MLM bonus via PostgreSQL trigger on consumption.

### Observability & Operations
- Request counter, tracing, gzip, 30s timeout, CORS, JWT auth.
- Health probes: `/health` (liveness), `/ready` (readiness), `/metrics` (counter).
- Graceful shutdown on `SIGTERM` with 5s drain window.
- Structured JSON logging via `tracing`.

### Developer Experience
- OpenAPI 3.0 with Swagger UI at `/api/v1/docs`.
- Three-tier tests (unit, integration, e2e) with ≥80% coverage enforced.
- GitHub Actions CI pipeline.

---

## Quickstart

**Prerequisites:** [Rust toolchain](https://www.rust-lang.org/tools/install), [Docker](https://docs.docker.com/get-docker/), `sqlx-cli`.

```bash
# Install sqlx-cli for database migrations
cargo install sqlx-cli --no-default-features --features rustls,postgres

# Clone the repository and set up environment
git clone https://github.com/ardinbig/storm-api.git && cd storm-api
cp .env.example .env

# Start PostgreSQL and Redis services
docker compose up -d database redis

# Run database migrations
sqlx migrate run --source migrations

# Build and run the API server
cargo run
```

- Local API: `http://127.0.0.1:3000`
- Swagger UI: `http://127.0.0.1:3000/api/v1/docs`
- OpenAPI JSON: `http://127.0.0.1:3000/api-doc/openapi.json`

---

## Architecture

```text
storm-api/
├── src/
│   ├── app.rs          # Router + middleware assembly
│   ├── main.rs         # Entry point, graceful shutdown
│   ├── routes/         # HTTP topology per domain
│   ├── handlers/       # Request extraction → service call → response
│   ├── services/       # Business logic + SQLx queries
│   ├── models/         # DTOs and database row types
│   ├── state/          # AppState (pool, Redis, JWT config, counters)
│   ├── errors/         # Unified AppError → JSON response
│   └── utils/          # Password hashing, Redis cache helpers
├── migrations/
├── tests/              # unit / integration / e2e
├── compose.yml
└── Dockerfile
```

### Request Flow

```text
┌────────────────────────────────────────────┐
                      CLIENT                         
└────────────────────────────────────────────┘
                        │
                        ▼
┌────────────────────────────────────────────┐
                   AXUM ROUTER                      
└────────────────────────────────────────────┘
                        │
                        ▼
┌────────────────────────────────────────────┐
                MIDDLEWARE STACK                     
                                                   
  1. Request Counter  — feeds /metrics             
  2. Tracing          — structured JSON logs       
  3. Compression      — gzip                       
  4. Timeout          — 408 after 30s        
  5. CORS             — cross-origin policy   
  6. Auth             — JWT → CurrentUser     
                       (protected routes)     
└────────────────────────────────────────────┘
                        │
                        ▼
┌────────────────────────────────────────────┐
                     HANDLER                    
        extract → delegate → shape response        
└────────────────────────────────────────────┘
                        │
                        ▼
┌────────────────────────────────────────────┐
                     SERVICE                    
     Business logic · SQLx queries · &PgPool    
└────────────────────────────────────────────┘
           │                       │
           ▼                       ▼
┌─────────────────┐     ┌────────────────────┐
     PostgreSQL                    Redis         
   (SQLx + PgPool)           JWT blocklist/cache 
└─────────────────┘     └────────────────────┘
```
---
## Environment Variables

| Variable             | Description                                              | Default                                          |
|----------------------|----------------------------------------------------------|--------------------------------------------------|
| `DATABASE_URL`       | PostgreSQL connection string.                            | `postgres://postgres:postgres@localhost/stormdb` |
| `REDIS_URL`          | Redis URL. Optional - app degrades gracefully if absent. | `redis://127.0.0.1:6379`                         |
| `JWT_SECRET`         | HMAC secret. **Change in production.**                   | `dev-secret-change-in-production`                |
| `APP_ADDR`           | Bind address.                                            | `127.0.0.1:3000`                                 |
| `RUST_LOG`           | Log filter.                                              | `storm_api=debug,tower_http=debug`               |
| `MAX_DB_CONNECTIONS` | SQLx pool size.                                          | `10`                                             |

---

## Development

```bash
# Format code according to Rust conventions
cargo fmt --all

# Lint check for code quality and best practices
cargo clippy --all-targets --all-features -- -D warnings

# Run all tests (unit, integration, e2e)
cargo test --locked

# Build and start all services with Docker
docker compose up --build 
```

### Health probes

| Endpoint       | Description                             | Response                      |
|----------------|-----------------------------------------|-------------------------------|
| `GET /health`  | Liveness                                | `200 OK`                      |
| `GET /ready`   | Readiness - `503` after shutdown signal | `200 ready` / `503 not ready` |
| `GET /metrics` | Request counter                         | `{ "requests": N }`           |

Graceful shutdown listens for `SIGTERM` / `Ctrl+C`, flips the readiness flag to `false`, then waits 5s for in-flight requests to drain.

## Tech Stack

| Layer                 | Technology (crates)                                    |
|-----------------------|--------------------------------------------------------|
| Language              | Rust 1.94+                                             |
| Web framework         | Axum 0.8 + Tower + tower-http                          |
| Async runtime         | Tokio                                                  |
| Database              | PostgreSQL 18 via SQLx                                 |
| Cache / JWT blocklist | Redis 8 via redis-rs (optional - graceful degradation) |
| Authentication        | JWT via jsonwebtoken, Argon2id via argon2              |
| Serialization         | serde + serde_json + uuid + chrono                     |
| Error handling        | thiserror                                              |
| Observability         | tracing + tracing-subscriber (JSON, env-filter)        |
| API docs              | utoipa + utoipa-swagger-ui                             |
| Testing               | sqlx::test, mockall, testcontainers, reqwest           |
| Containerization      | Docker (multi-stage), Docker Compose                   |
| CI                    | GitHub Actions                                         |

