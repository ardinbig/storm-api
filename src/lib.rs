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
//! **routes → handlers → services → database (raw SQL via SQLx)**
//!
//! - [`routes`] — Axum [`Router`](axum::Router) definitions that map HTTP
//!   verbs and paths to handler functions.
//! - [`handlers`] — Thin request/response wrappers that extract state and
//!   JSON payloads, delegate to services, and return typed JSON responses.
//! - [`services`] — Free functions containing all business logic and SQL
//!   queries; each takes `&PgPool` as its first argument.
//! - [`models`] — Database row structs (`FromRow`), request DTOs
//!   (`Deserialize`), response DTOs (`Serialize`), and `From` conversions.
//! - [`errors`] — Unified [`AppError`](errors::AppError) enum implementing
//!   `IntoResponse` for consistent JSON error bodies.
//! - [`state`] — [`AppState`](state::app_state::AppState) holding the
//!   connection pool, Redis connection, JWT config, readiness flag, and
//!   request counter.
//! - [`db`] — PostgreSQL connection-pool factory.
//! - [`utils`] — Shared helpers: password hashing and Redis caching.
//!
//! ## Getting Started
//!
//! ```bash
//! # Start PostgreSQL (via Docker Compose or locally)
//! docker compose up database redis -d
//!
//! # Apply the database schema
//! psql stormdb < migrations/001_init.sql
//!
//! # Run the server (reads .env via dotenvy)
//! cargo run
//! ```
//!
//! Environment variables: `DATABASE_URL`, `REDIS_URL` (optional),
//! `JWT_SECRET`, `APP_ADDR` (default `127.0.0.1:3000`), `RUST_LOG`,
//! `MAX_DB_CONNECTIONS`.

pub mod app;
pub mod db;
pub mod errors;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod services;
pub mod state;
pub mod utils;
