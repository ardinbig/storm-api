//! Commission tier routes (JWT-protected).
//!
//! | Method | Path | Handler |
//! |--------|------|---------|
//! | `GET` | `/` | [`commission_tier_handler::list_tiers`](commission_tier_handler::list_tiers) |
//! | `POST` | `/` | [`commission_tier_handler::create_tier`](commission_tier_handler::create_tier) |
//! | `GET` | `/by-category/{category}` | [`commission_tier_handler::get_by_category`](commission_tier_handler::get_by_category) |

use axum::{Router, routing::get};

use crate::{handlers::commission_tier_handler, state::app_state::AppState};

/// Returns the commission-tiers router.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(commission_tier_handler::list_tiers).post(commission_tier_handler::create_tier),
        )
        .route(
            "/by-category/{category}",
            get(commission_tier_handler::get_by_category),
        )
}
