use axum::Router;
use sqlx::PgPool;
use storm_api::app::create_app;

use crate::common::{register_and_login, test_config, test_state};

/// Build an `axum::Router` wired to a test pool (no auth).
pub fn create_test_app(pool: PgPool) -> Router {
    create_app(test_state(pool))
}

/// Register a user, build the app, and return both the router and the JWT.
pub async fn create_test_app_with_token(pool: PgPool) -> (Router, String) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let app = create_app(test_state(pool));
    (app, token)
}
