//! Redis caching helpers and JWT blocklist operations.
//!
//! Every function accepts an [`RedisPool`] (i.e. `Option<ConnectionManager>`).
//! When `None`, all operations are no-ops, providing transparent graceful
//! degradation when Redis is unavailable (e.g. in tests).

use redis::{AsyncCommands, aio::ConnectionManager};
use serde::{Serialize, de::DeserializeOwned};
use tracing::warn;

use crate::state::app_state::RedisPool;

// Generic cache helpers
// =====================

/// Attempts to read a cached JSON value.
///
/// Returns `None` if Redis is unavailable, the key does not exist, or
/// deserialization fails.
pub async fn get<T: DeserializeOwned>(redis: &RedisPool, key: &str) -> Option<T> {
    let json = read_json(redis, key).await?;
    serde_json::from_str(&json)
        .inspect_err(|error| log_json_error("deserialize", error))
        .ok()
}

async fn read_json(redis: &RedisPool, key: &str) -> Option<String> {
    let mut conn = redis.as_ref()?.clone();
    match conn.get(key).await {
        Ok(value) => value,
        Err(error) => {
            log_redis_error("get", &error);
            None
        }
    }
}

/// Stores a JSON-serialized value with a TTL. No-ops when Redis is unavailable.
pub async fn set<T: Serialize>(redis: &RedisPool, key: &str, value: &T, ttl_secs: u64) {
    if let Some(conn) = redis.as_ref() {
        store(conn, key, serde_json::to_string(value), ttl_secs).await;
    }
}

async fn store(conn: &ConnectionManager, key: &str, encoded: serde_json::Result<String>, ttl: u64) {
    let Ok(json) = encoded.inspect_err(|error| log_json_error("serialize", error)) else {
        return;
    };
    let mut conn = conn.clone();
    if let Err(error) = conn.set_ex::<_, _, ()>(key, json, ttl).await {
        log_redis_error("set", &error);
    }
}

/// Deletes a cached key. No-ops when Redis is unavailable.
pub async fn del(redis: &RedisPool, key: &str) {
    let Some(conn) = redis.as_ref() else { return };
    let mut conn = conn.clone();
    if let Err(error) = conn.del::<_, ()>(key).await {
        log_redis_error("del", &error);
    }
}

// JWT blocklist
// =============

/// Adds a token to the blocklist with the given TTL (seconds).
///
/// Call this on logout. The TTL should match the token's remaining lifetime
/// so the entry auto-expires when the token would have expired anyway.
pub async fn blocklist_token(redis: &RedisPool, token: &str, ttl_secs: u64) {
    let Some(conn) = redis.as_ref() else { return };
    let mut conn = conn.clone();
    let key = ["blocklist:", token].concat();
    if let Err(error) = conn.set_ex::<_, _, ()>(&key, "1", ttl_secs).await {
        log_redis_error("blocklist_set", &error);
    }
}

/// Returns `true` if the token has been blocklisted (i.e. the user logged out).
/// Returns `false` when Redis is unavailable (fail-open).
pub async fn is_blocklisted(redis: &RedisPool, token: &str) -> bool {
    let Some(conn) = redis.as_ref() else {
        return false;
    };
    let mut conn = conn.clone();
    let key = ["blocklist:", token].concat();
    conn.exists::<_, bool>(&key).await.unwrap_or_else(|error| {
        log_redis_error("blocklist_check", &error);
        false
    })
}

// Keys can contain NFC references or tokens; log the operation and error class only.
fn log_redis_error(operation: &'static str, error: &redis::RedisError) {
    let kind = tracing::field::debug(error.kind());
    warn!(operation, kind, "Redis cache operation failed");
}

fn log_json_error(operation: &'static str, error: &serde_json::Error) {
    let category = tracing::field::debug(error.classify());
    warn!(operation, category, "Redis cache JSON conversion failed");
}

// Cache-key helpers
// =================

/// Cache key for a card detail looked up by NFC reference.
pub fn card_detail_key(nfc_ref: &str) -> String {
    ["card_detail:", nfc_ref].concat()
}

/// Cache key for the current fuel price by consumption type.
pub fn price_key(consumption_type: &str) -> String {
    ["price:", consumption_type].concat()
}
