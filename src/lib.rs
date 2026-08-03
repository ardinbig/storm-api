//! # Storm API
//!
//! A Rust REST API built with [Axum](https://docs.rs/axum) and
//! [SQLx](https://docs.rs/sqlx) (PostgreSQL) for fuel-station management.
//!
//! The API manages NFC card balances, agent withdrawals with commission,
//! fuel consumption logging, and a 2-level MLM loyalty bonus system.
//!
//! ## Architecture
//!
//! The request flow follows a layered pattern:
//!
//! **routes → middleware → handlers → services → database (raw SQL via SQLx)**
//!
//! - [`routes`] — Axum [`Router`](axum::Router) definitions mapping HTTP
//!   methods and paths to handlers.
//! - [`middleware`] — Cross-cutting request processing.
//! - [`handlers`] — Thin request/response wrappers that extract state and
//!   payloads, delegate to services, and return typed responses.
//! - [`services`] — Business logic and SQL queries; functions generally take
//!   `&PgPool` first.
//! - [`models`] — Database row structs (`FromRow`), request DTOs
//!   (`Deserialize`), response DTOs (`Serialize`), and conversions.
//! - [`errors`] — Unified [`AppError`](errors::AppError) implementing
//!   `IntoResponse` for consistent JSON error bodies.
//! - [`state`] — [`AppState`](state::app_state::AppState) holding the
//!   PostgreSQL pool, optional Redis connection, JWT config, readiness flag,
//!   and request counter.
//! - [`db`] — PostgreSQL connection-pool factory.
//! - [`utils`] — Shared helper utilities (password hashing, cache helpers,
//!   and related support utilities).
//!
//! ## Getting Started
//!
//! ```bash
//! # Start PostgreSQL and Redis
//! docker compose up database redis -d
//!
//! # Apply database schema
//! psql stormdb < migrations/001_init.sql
//!
//! # Run the server (reads .env via dotenvy)
//! cargo run
//! ```
//!
//! ## Environment
//!
//! Key variables: `DATABASE_URL`, `REDIS_URL` (optional), `JWT_SECRET`,
//! `APP_ADDR` (default `127.0.0.1:3000`), `RUST_LOG`, `MAX_DB_CONNECTIONS`.

pub mod app;
pub mod db;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod services;
pub mod state;
pub mod utils;
