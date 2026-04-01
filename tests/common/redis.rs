use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64},
};

use sqlx::PgPool;
use storm_api::state::app_state::{AppState, RedisPool};
use testcontainers::ImageExt;

use crate::common::test_config;

/// Spin up a disposable Redis container and return a live `RedisPool` plus the
/// container guard (must be kept alive for the pool to remain connected).
pub async fn setup_redis_pool() -> (
    RedisPool,
    testcontainers::ContainerAsync<testcontainers_modules::redis::Redis>,
) {
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::redis::Redis;

    let container = Redis::default()
        .with_tag("8-bookworm")
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
pub fn test_state_with_redis(pool: PgPool, redis: RedisPool) -> AppState {
    AppState {
        pool,
        redis,
        auth_config: Arc::new(test_config()),
        ready: Arc::new(AtomicBool::new(true)),
        request_count: Arc::new(AtomicU64::new(0)),
    }
}
