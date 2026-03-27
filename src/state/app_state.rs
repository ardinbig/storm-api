//! Shared application state.

use axum::extract::FromRef;
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64},
};

/// Type alias for an optional Redis connection manager.
/// `None` when Redis is unavailable — all cache operations no-op gracefully.
pub type RedisPool = Option<ConnectionManager>;

/// JWT authentication configuration.
#[derive(Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
}

/// Central application state shared across all Axum handlers and middleware.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub redis: RedisPool,
    pub auth_config: Arc<AuthConfig>,
    pub ready: Arc<AtomicBool>,
    pub request_count: Arc<AtomicU64>,
}

/// Allows handlers to extract `State<PgPool>` directly.
impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

/// Allows handlers to extract `State<Arc<AuthConfig>>` directly.
impl FromRef<AppState> for Arc<AuthConfig> {
    fn from_ref(state: &AppState) -> Self {
        state.auth_config.clone()
    }
}

/// Allows handlers to extract `State<RedisPool>` directly.
impl FromRef<AppState> for RedisPool {
    fn from_ref(state: &AppState) -> Self {
        state.redis.clone()
    }
}
