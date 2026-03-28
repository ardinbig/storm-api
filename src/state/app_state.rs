//! Shared application state passed to every Axum handler via [`State`](axum::extract::State).
//!
//! [`AppState`] implements [`Clone`] so it can be shared across tasks, and
//! provides [`FromRef`] implementations for [`PgPool`] and
//! [`Arc<AuthConfig>`] so handlers can extract either the full state or
//! individual components directly.

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
    /// The HMAC secret used to sign and verify JSON Web Tokens.
    pub jwt_secret: String,
    /// Token validity period in hours from the time of issuance.
    pub jwt_expiry_hours: i64,
}

/// Central application state shared across all Axum handlers and middleware.
///
/// # Extracting subcomponents
///
/// Thanks to the [`FromRef`] implementations below, handlers can extract
/// `State<PgPool>` or `State<Arc<AuthConfig>>` directly without destructuring
/// the full `AppState`.
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL connection pool managed by SQLx.
    pub pool: PgPool,
    /// Optional Redis connection for caching and JWT blocklist.
    /// `None` disables all cache operations (graceful degradation).
    pub redis: RedisPool,
    /// JWT signing/verification settings.
    pub auth_config: Arc<AuthConfig>,
    /// Readiness flag — set to `false` during graceful shutdown so the
    /// `/ready` health check endpoint starts returning `503`.
    pub ready: Arc<AtomicBool>,
    /// Monotonically increasing request counter exposed at `/metrics`.
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
