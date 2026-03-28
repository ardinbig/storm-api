//! Axum [`Router`](axum::Router) definitions for each domain.
//!
//! Each submodule exposes a `routes() -> Router<AppState>` function that
//! is mounted in [`crate::app::create_app`].

pub mod auth;
pub mod categories;
pub mod health;
pub mod users;
