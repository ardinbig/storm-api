//! Integration tests — require a PostgreSQL database via `#[sqlx::test]`.

mod agent;
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
mod customer;
mod db_connection;
mod health;
mod price;
mod state;
mod transaction;
