//! Current-user routes (JWT-protected).
//!
//! | Method | Path | Handler |
//! |--------|------|---------|
//! | `GET` | `/me` | [`user_handler::me`](user_handler::me) |

use axum::{Router, routing::get};

use crate::{handlers::user_handler, state::app_state::AppState};

/// Returns the users router.
pub fn routes() -> Router<AppState> {
    Router::new().route("/me", get(user_handler::me))
}
