//! Health check and observability routes (public — no JWT required).
//!
//! | Method | Path | Handler |
//! |--------|------|---------|
//! | `GET` | `/health` | [`health_handler::health`](health_handler::health) |
//! | `GET` | `/ready` | [`health_handler::ready`](health_handler::ready) |
//! | `GET` | `/metrics` | [`health_handler::metrics`](health_handler::metrics) |

use axum::{Router, routing::get};

use crate::{handlers::health_handler, state::app_state::AppState};

/// Returns the health check router.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_handler::health))
        .route("/ready", get(health_handler::ready))
        .route("/metrics", get(health_handler::metrics))
}
