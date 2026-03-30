//! Consumption routes (JWT-protected).
//!
//! | Method | Path | Handler |
//! |--------|------|---------|
//! | `GET` | `/` | [`consumption_handler::list_consumptions`](consumption_handler::list_consumptions) |
//! | `POST` | `/` | [`consumption_handler::create`](consumption_handler::create) |
//! | `GET` | `/by-client/{client_ref}` | [`consumption_handler::list_by_client`](consumption_handler::list_by_client) |

use axum::{Router, routing::get};

use crate::{handlers::consumption_handler, state::app_state::AppState};

/// Returns the consumptions router.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(consumption_handler::list_consumptions).post(consumption_handler::create),
        )
        .route(
            "/by-client/{client_ref}",
            get(consumption_handler::list_by_client),
        )
}
