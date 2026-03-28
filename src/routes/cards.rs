//! NFC card routes (JWT-protected).
//!
//! | Method | Path | Handler |
//! |--------|------|---------|
//! | `GET` | `/` | [`card_handler::list_cards`](card_handler::list_cards) |
//! | `POST` | `/` | [`card_handler::create_card`](card_handler::create_card) |
//! | `GET` | `/{id}` | [`card_handler::get_card`](card_handler::get_card) |
//! | `POST` | `/{nfc_ref}/balance` | [`card_handler::check_balance`](card_handler::check_balance) |

use axum::{
    Router,
    routing::{get, post},
};

use crate::{handlers::card_handler, state::app_state::AppState};

/// Returns the cards router.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(card_handler::list_cards).post(card_handler::create_card),
        )
        .route("/{id}", get(card_handler::get_card))
        .route("/{nfc_ref}/balance", post(card_handler::check_balance))
}
