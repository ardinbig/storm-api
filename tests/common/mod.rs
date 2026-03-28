// False positives warnings
#![allow(dead_code)]

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64},
};

use axum::body::Body;
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use storm_api::state::app_state::{AppState, AuthConfig};

// Constants
// =========

pub const JWT_SECRET: &str = "test-secret-for-unit-tests-only";

// State helpers (pool comes from #[sqlx::test])
// =============================================

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

// Body helpers
// ============

pub async fn body_to_value(body: Body) -> Value {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

pub async fn body_to_string(body: Body) -> String {
    let bytes = body.collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

// Auth helpers
// ============

/// Register a user with a unique username and return the JWT token.
pub async fn register_and_login(pool: &PgPool, config: &AuthConfig) -> String {
    let unique = &Uuid::new_v4().to_string()[..8];
    let username = format!("test.user-{unique}");

    storm_api::services::user_service::register(
        pool,
        &storm_api::models::user::RegisterRequest {
            name: "Test User".into(),
            email: Some(format!("{username}@example.com")),
            username: username.clone(),
            password: "password123".into(),
        },
    )
    .await
    .unwrap();

    storm_api::services::user_service::authenticate(pool, config, &username, "password123")
        .await
        .unwrap()
        .token
}

// Redis helpers (testcontainers)
// ==============================

/// Spin up a disposable Redis container and return a live `RedisPool` plus the
/// container guard (must be kept alive for the pool to remain connected).
pub async fn setup_redis_pool() -> (
    storm_api::state::app_state::RedisPool,
    testcontainers::ContainerAsync<testcontainers_modules::redis::Redis>,
) {
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::redis::Redis;

    let container = Redis::default()
        .start()
        .await
        .expect("Failed to start Redis container");
    let port = container
        .get_host_port_ipv4(6379)
        .await
        .expect("Failed to get Redis port");
    let url = format!("redis://127.0.0.1:{port}");
    let client = redis::Client::open(url).expect("Failed to create Redis client");
    let conn = redis::aio::ConnectionManager::new(client)
        .await
        .expect("Failed to connect to Redis container");
    (Some(conn), container)
}

/// Build an `AppState` with a **real** Redis pool (for cache-hit tests).
pub fn test_state_with_redis(
    pool: PgPool,
    redis: storm_api::state::app_state::RedisPool,
) -> AppState {
    AppState {
        pool,
        redis,
        auth_config: Arc::new(test_config()),
        ready: Arc::new(AtomicBool::new(true)),
        request_count: Arc::new(AtomicU64::new(0)),
    }
}

// Seed helpers
// ============

/// Insert a card into `cards`.
pub async fn seed_card(pool: &PgPool, nfc: &str) {
    sqlx::query("INSERT INTO cards (id, card_id) VALUES ($1, $2)")
        .bind(Uuid::new_v4())
        .bind(nfc)
        .execute(pool)
        .await
        .unwrap();
}

/// Insert a commission percentage into `commissions`
async fn seed_commission(pool: &PgPool, percentage: f64) {
    sqlx::query("INSERT INTO commissions (id, percentage) VALUES ($1, $2)")
        .bind(Uuid::new_v4())
        .bind(percentage)
        .execute(pool)
        .await
        .unwrap();
}
