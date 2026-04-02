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
- Dual JWT auth flows for system users and field agents, each issuing role-scoped tokens.
- Argon2id password hashing for all credentials; never stored plaintext.
- JWT revocation via Redis blocklist - logout stores the token for its remaining TTL; middleware rejects it immediately.

### Cards & Customers
- NFC card registry with lifecycle status tracking and PIN-protected balance checks.
- Customer profiles with full CRUD, NFC card linkage, category assignment, and lookup by card.

### Agents & Transactions
- Agent accounts with balance and currency tracking, transaction history, and self-service password change.
- Agent-led customer registration - assign an NFC card and create a customer profile in one request.
- Cash withdrawal flow - verifies card PIN, deducts amount + commission, credits the agent, and splits the fee to the house account.
- Append-only commission rate history - most recently created row is the active rate.
- House account protected from deletion; receives all withdrawal commissions.

### Fuel & Loyalty
- Fuel consumption logging with client reference, fuel type, quantity, unit price, operator, and timestamp.
- 2-level MLM loyalty bonus via PostgreSQL trigger (`fn_consumption_bonus_tree`) fired on every consumption insert.
- Commission tiers with configurable L-1 and L-2 bonus percentages, optionally scoped per vehicle category.
- Fuel pricing per type with full history - most recent row per type is the current price.

### Observability & Operations
- Middleware stack: atomic request counter, structured tracing, gzip compression, 30s timeout, CORS, JWT auth.
- Request counter exposed at `GET /metrics` as `{ "requests": N }`.
- Liveness probe at `GET /health`; readiness probe at `GET /ready` - returns `503` after a shutdown signal.
- Graceful shutdown on `SIGTERM` / `Ctrl+C` with a 5 seconds in-flight drain window.
- Unified error responses - all `AppError` variants serialized as `{ "error": "…", "code": N }`.
- Structured JSON logging via `tracing` with `RUST_LOG` environment-driven filtering.

### Developer Experience
- OpenAPI 3.0 spec generated at compile time with `utoipa`; served as JSON at `/api-doc/openapi.json`.
- Swagger UI at `/api/v1/docs` pre-configured with Bearer JWT security scheme.
- Three-tier test suite - unit (no I/O), integration (real PostgreSQL + Redis via `sqlx::test`), and end-to-end.
- ≥ 80 % line coverage enforced in CI via `cargo-llvm-cov`; report uploaded to Codecov.
- GitHub Actions pipeline: lint → unit → integration → coverage → release build.

---

## Quickstart

**Prerequisites:** [Rust toolchain](https://www.rust-lang.org/tools/install), [Docker](https://docs.docker.com/get-docker/), `sqlx-cli`.

```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres

git clone https://github.com/ardinbig/storm-api.git && cd storm-api
cp .env.example .env
docker compose up -d database redis
sqlx migrate run --source migrations
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
  4. Timeout          — 408 after 30 s        
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
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
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
| Language              | Rust 1.94+ (edition 2024)                              |
| Web framework         | Axum 0.8 + Tower + tower-http                          |
| Async runtime         | Tokio 1.50                                             |
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

