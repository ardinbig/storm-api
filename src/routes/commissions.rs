//! Commission rate routes (JWT-protected).
//!
//! | Method | Path | Handler |
//! |--------|------|---------|
//! | `GET` | `/` | [`commission_handler::list_commissions`] |
//! | `POST` | `/` | [`commission_handler::create_commission`] |
//! | `GET` | `/current` | [`commission_handler::get_current`] |
//! | `DELETE` | `/{id}` | [`commission_handler::delete_commission`] |

use axum::{Router, routing::get};

use crate::{handlers::commission_handler, state::app_state::AppState};

/// Returns the commissions router.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(commission_handler::list_commissions).post(commission_handler::create_commission),
        )
        .route("/current", get(commission_handler::get_current))
        .route(
            "/{id}",
            axum::routing::delete(commission_handler::delete_commission),
        )
}
