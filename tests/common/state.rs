use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64},
};

use sqlx::PgPool;
use storm_api::state::app_state::{AppState, AuthConfig};

pub const JWT_SECRET: &str = "test-secret-for-unit-tests-only";

pub fn test_config() -> AuthConfig {
    AuthConfig {
        jwt_secret: JWT_SECRET.into(),
        jwt_expiry_hours: 24,
    }
}

pub fn test_state(pool: PgPool) -> AppState {
    AppState {
        pool,
        redis: None,
        auth_config: Arc::new(test_config()),
        ready: Arc::new(AtomicBool::new(true)),
        request_count: Arc::new(AtomicU64::new(0)),
    }
}
