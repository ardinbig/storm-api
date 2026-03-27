//! Redis caching helpers and JWT blocklist operations.
//!
//! Every function accepts an [`RedisPool`] (i.e. `Option<ConnectionManager>`).
//! When `None`, all operations are no-ops, providing transparent graceful
//! degradation when Redis is unavailable (e.g. in tests).

use redis::AsyncCommands;
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
    let conn = redis.as_ref()?;
    let mut conn = conn.clone();
    let json: String = match conn.get(key).await {
        Ok(v) => v,
        Err(e) => {
            warn!("cache GET {key}: {e}");
            return None;
        }
    };
    match serde_json::from_str(&json) {
        Ok(v) => Some(v),
        Err(e) => {
            warn!("cache deserialize {key}: {e}");
            None
        }
    }
}

/// Stores a JSON-serialized value with a TTL. No-ops when Redis is unavailable.
pub async fn set<T: Serialize>(redis: &RedisPool, key: &str, value: &T, ttl_secs: u64) {
    let Some(conn) = redis.as_ref() else { return };
    let mut conn = conn.clone();
    let json = match serde_json::to_string(value) {
        Ok(j) => j,
        Err(e) => {
            warn!("cache serialize {key}: {e}");
            return;
        }
    };
    if let Err(e) = conn.set_ex::<_, _, ()>(key, json, ttl_secs).await {
        warn!("cache SET {key}: {e}");
    }
}

/// Deletes a cached key. No-ops when Redis is unavailable.
pub async fn del(redis: &RedisPool, key: &str) {
    let Some(conn) = redis.as_ref() else { return };
    let mut conn = conn.clone();
    if let Err(e) = conn.del::<_, ()>(key).await {
        warn!("cache DEL {key}: {e}");
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
    let key = format!("blocklist:{token}");
    if let Err(e) = conn.set_ex::<_, _, ()>(&key, "1", ttl_secs).await {
        warn!("cache blocklist SET: {e}");
    }
}

/// Returns `true` if the token has been blocklisted (i.e. the user logged out).
/// Returns `false` when Redis is unavailable (fail-open).
pub async fn is_blocklisted(redis: &RedisPool, token: &str) -> bool {
    let Some(conn) = redis.as_ref() else {
        return false;
    };
    let mut conn = conn.clone();
    let key = format!("blocklist:{token}");
    conn.exists::<_, bool>(&key).await.unwrap_or_else(|e| {
        warn!("cache blocklist check: {e}");
        false
    })
}

// Cache-key helpers
// =================

/// Cache key for a card detail looked up by NFC reference.
pub fn card_detail_key(nfc_ref: &str) -> String {
    format!("card_detail:{nfc_ref}")
}

/// Cache key for the current fuel price by consumption type.
pub fn price_key(consumption_type: &str) -> String {
    format!("price:{consumption_type}")
}
