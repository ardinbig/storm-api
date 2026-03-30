//! Integration tests — require a PostgreSQL database via `#[sqlx::test]`.

mod auth_endpoint;
mod auth_middleware;
mod cache;
mod card;
mod category;
mod commission;
mod commission_tier;
#[path = "../common/mod.rs"]
mod common;
mod consumption;
mod db_connection;
mod health;
mod price;
mod state;
