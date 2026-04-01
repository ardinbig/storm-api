//! Customer routes (JWT-protected).
//!
//! | Method | Path | Handler |
//! |--------|------|---------|
//! | `GET` | `/` | [`customer_handler::list_customers`] |
//! | `POST` | `/` | [`customer_handler::register`] |
//! | `GET` | `/{id}` | [`customer_handler::get_customer`] |
//! | `PUT` | `/{id}` | [`customer_handler::update_customer`] |
//! | `DELETE` | `/{id}` | [`customer_handler::delete_customer`] |
//! | `GET` | `/by-card/{card_id}` | [`customer_handler::get_by_card`] |

use axum::{Router, routing::get};

use crate::{handlers::customer_handler, state::app_state::AppState};

/// Returns the customers router.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(customer_handler::list_customers).post(customer_handler::register),
        )
        .route(
            "/{id}",
            get(customer_handler::get_customer)
                .put(customer_handler::update_customer)
                .delete(customer_handler::delete_customer),
        )
        .route("/by-card/{card_id}", get(customer_handler::get_by_card))
}
