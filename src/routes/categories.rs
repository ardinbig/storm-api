//! Category routes (JWT-protected).
//!
//! | Method | Path | Handler |
//! |--------|------|---------|
//! | `GET` | `/` | [`category_handler::list_categories`] |
//! | `POST` | `/` | [`category_handler::create_category`] |
//! | `GET` | `/{id}` | [`category_handler::get_category`] |

use axum::{Router, routing::get};

use crate::{handlers::category_handler, state::app_state::AppState};

/// Returns the categories router.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(category_handler::list_categories).post(category_handler::create_category),
        )
        .route("/{id}", get(category_handler::get_category))
}
