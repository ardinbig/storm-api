use crate::common::setup_redis_pool;
use redis::AsyncCommands;
use storm_api::utils::cache;

fn init_warn_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::WARN)
        .try_init();
}

async fn wait_until_redis_unreachable(redis: &storm_api::state::app_state::RedisPool) {
    // Container teardown is async; poll until a simple command fails so
    // downstream tests reliably hit cache error branches.
    for _ in 0..40 {
        let Some(conn) = redis.as_ref() else {
            return;
        };
        let mut conn = conn.clone();
        let ping: redis::RedisResult<String> = conn.ping().await;
        if ping.is_err() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("redis container is still reachable after shutdown wait");
}

// get / set / del
// ===============

#[tokio::test]
async fn cache_set_then_get_returns_stored_value() {
    init_warn_tracing();
    let (redis, _c) = setup_redis_pool().await;

    let value = serde_json::json!({"name": "test", "amount": 42.5});
    cache::set(&redis, "test:set-get", &value, 60).await;

    let result: Option<serde_json::Value> = cache::get(&redis, "test:set-get").await;
    assert_eq!(result, Some(value));
}

#[tokio::test]
async fn cache_get_nonexistent_key_returns_none_with_redis() {
    init_warn_tracing();
    let (redis, _c) = setup_redis_pool().await;

    let result: Option<serde_json::Value> = cache::get(&redis, "nonexistent:key").await;
    assert!(result.is_none());
}

#[tokio::test]
async fn cache_del_removes_stored_key() {
    init_warn_tracing();
    let (redis, _c) = setup_redis_pool().await;

    cache::set(&redis, "test:del", &"hello", 60).await;
    cache::del(&redis, "test:del").await;

    let result: Option<String> = cache::get(&redis, "test:del").await;
    assert!(result.is_none());
}

// Serialization / deserialization edge cases
// ==========================================

#[tokio::test]
async fn cache_set_unserializable_value_no_ops() {
    init_warn_tracing();
    let (redis, _c) = setup_redis_pool().await;

    cache::_test_set_bad_serialize(&redis, "test:bad").await;

    let result: Option<serde_json::Value> = cache::get(&redis, "test:bad").await;
    assert!(result.is_none());
}

#[tokio::test]
async fn cache_get_invalid_json_returns_none() {
    init_warn_tracing();
    let (redis, _c) = setup_redis_pool().await;

    let mut conn = redis.as_ref().unwrap().clone();
    conn.set::<_, _, ()>("test:bad-json", "not-valid-json")
        .await
        .unwrap();

    let result: Option<serde_json::Value> = cache::get(&redis, "test:bad-json").await;
    assert!(result.is_none());
}

// JWT blocklist
// =============

#[tokio::test]
async fn cache_blocklist_then_is_blocklisted_returns_true() {
    init_warn_tracing();
    let (redis, _c) = setup_redis_pool().await;

    assert!(!cache::is_blocklisted(&redis, "tok-bl-001").await);
    cache::blocklist_token(&redis, "tok-bl-001", 60).await;
    assert!(cache::is_blocklisted(&redis, "tok-bl-001").await);
}

#[tokio::test]
async fn cache_is_blocklisted_unknown_token_returns_false() {
    init_warn_tracing();
    let (redis, _c) = setup_redis_pool().await;

    assert!(!cache::is_blocklisted(&redis, "unknown-tok").await);
}

// Error paths (Redis connection gone)
// ===================================

#[tokio::test]
async fn cache_set_handles_redis_connection_error() {
    init_warn_tracing();
    let (redis, container) = setup_redis_pool().await;
    drop(container);
    wait_until_redis_unreachable(&redis).await;

    cache::set(&redis, "fail:key", &"value", 60).await;
}

#[tokio::test]
async fn cache_del_handles_redis_connection_error() {
    init_warn_tracing();
    let (redis, container) = setup_redis_pool().await;
    drop(container);
    wait_until_redis_unreachable(&redis).await;

    cache::del(&redis, "fail:key").await;
}

#[tokio::test]
async fn cache_blocklist_handles_redis_connection_error() {
    init_warn_tracing();
    let (redis, container) = setup_redis_pool().await;
    drop(container);
    wait_until_redis_unreachable(&redis).await;

    cache::blocklist_token(&redis, "fail-tok", 60).await;
}

#[tokio::test]
async fn cache_is_blocklisted_returns_false_on_redis_error() {
    init_warn_tracing();
    let (redis, container) = setup_redis_pool().await;
    drop(container);
    wait_until_redis_unreachable(&redis).await;

    assert!(!cache::is_blocklisted(&redis, "fail-tok").await);
}

#[tokio::test]
async fn cache_get_returns_none_on_redis_error() {
    init_warn_tracing();
    let (redis, container) = setup_redis_pool().await;
    drop(container);
    wait_until_redis_unreachable(&redis).await;

    let result: Option<String> = cache::get(&redis, "fail:key").await;
    assert!(result.is_none());
}
