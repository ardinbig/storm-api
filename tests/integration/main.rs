//! Integration tests — require a PostgreSQL database via `#[sqlx::test]`.

#[path = "../common/mod.rs"]
mod common;
mod auth_endpoint;
mod auth_middleware;
mod cache;
mod db_connection;
mod health;
mod state;
