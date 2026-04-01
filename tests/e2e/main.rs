//! End-to-end tests using testcontainers (Docker) + reqwest.
//!
//! Each test function spins up its own disposable PostgreSQL container and a
//! real Axum HTTP server. Completely isolated: no shared mutable state.

#[path = "../common/mod.rs"]
mod common;

mod cards_categories;
mod customers_agents;
mod infra_auth;
mod pricing;
mod transactions;
