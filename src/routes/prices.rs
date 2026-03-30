//! Fuel price routes (JWT-protected).
//!
//! | Method | Path | Handler |
//! |--------|------|---------|
//! | `GET` | `/` | [`price_handler::list_prices`](price_handler::list_prices) |
//! | `POST` | `/` | [`price_handler::create_price`](price_handler::create_price) |
//! | `GET` | `/by-type/{consumption_type}` | [`price_handler::get_by_type`](price_handler::get_by_type) |

use axum::{Router, routing::get};

use crate::{handlers::price_handler, state::app_state::AppState};

/// Returns the prices router.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(price_handler::list_prices).post(price_handler::create_price),
        )
        .route(
            "/by-type/{consumption_type}",
            get(price_handler::get_by_type),
        )
}
