//! Transaction routes (JWT-protected).
//!
//! | Method | Path | Handler |
//! |--------|------|---------|
//! | `GET` | `/` | [`transaction_handler::list_transactions`](transaction_handler::list_transactions) |
//! | `POST` | `/withdrawal` | [`transaction_handler::withdrawal`](transaction_handler::withdrawal) |
//! | `GET` | `/by-agent/{agent_ref}` | [`transaction_handler::list_by_agent`](transaction_handler::list_by_agent) |

use axum::{
    Router,
    routing::{get, post},
};

use crate::{handlers::transaction_handler, state::app_state::AppState};

/// Returns the transactions router.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(transaction_handler::list_transactions))
        .route("/withdrawal", post(transaction_handler::withdrawal))
        .route(
            "/by-agent/{agent_ref}",
            get(transaction_handler::list_by_agent),
        )
}
