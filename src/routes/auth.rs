//! Authentication routes (public — no JWT required).
//!
//! | Method | Path | Handler |
//! |--------|------|---------|
//! | `POST` | `/login` | [`auth_handler::login`](auth_handler::login) |
//! | `POST` | `/register` | [`auth_handler::register`](auth_handler::register) |

use axum::{Router, routing::post};

use crate::{handlers::auth_handler, state::app_state::AppState};

/// Returns the authentication router.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", post(auth_handler::login))
        .route("/register", post(auth_handler::register))
}
