//! Agent routes (JWT-protected).
//!
//! | Method | Path | Handler |
//! |--------|------|---------|
//! | `GET` | `/` | [`agent_handler::list_agents`] |
//! | `POST` | `/` | [`agent_handler::create_agent`] |
//! | `GET` | `/{id}` | [`agent_handler::get_agent`] |
//! | `DELETE` | `/{id}` | [`agent_handler::delete_agent`] |
//! | `GET` | `/cards/{card_id}/balance` | [`agent_handler::check_balance`] |
//! | `GET` | `/{agent_id}/history` | [`agent_handler::history`] |
//! | `POST` | `/customers` | [`agent_handler::register_customer`] |
//! | `PUT` | `/password` | [`agent_handler::update_password`] |

use axum::{
    Router,
    routing::{get, post, put},
};

use crate::{handlers::agent_handler, state::app_state::AppState};

/// Returns the agents router.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(agent_handler::list_agents).post(agent_handler::create_agent),
        )
        .route(
            "/{id}",
            get(agent_handler::get_agent).delete(agent_handler::delete_agent),
        )
        .route(
            "/cards/{card_id}/balance",
            get(agent_handler::check_balance),
        )
        .route("/{agent_id}/history", get(agent_handler::history))
        .route("/customers", post(agent_handler::register_customer))
        .route("/password", put(agent_handler::update_password))
}
