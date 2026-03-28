//! Integration tests — require a PostgreSQL database via `#[sqlx::test]`.

mod auth_endpoint;
mod auth_middleware;
mod cache;
mod category;
#[path = "../common/mod.rs"]
mod common;
mod db_connection;
mod health;
mod state;

mod commission;
mod commission_tier;
