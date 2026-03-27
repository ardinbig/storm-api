//! Redis caching helpers and JWT blocklist operations.

use redis::AsyncCommands;
use serde::{Serialize, de::DeserializeOwned};
use tracing::warn;

use crate::state::app_state::RedisPool;

// Generic cache helpers

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

pub async fn del(redis: &RedisPool, key: &str) {
    let Some(conn) = redis.as_ref() else { return };
    let mut conn = conn.clone();
    if let Err(e) = conn.del::<_, ()>(key).await {
        warn!("cache DEL {key}: {e}");
    }
}

// JWT blocklist

pub async fn blocklist_token(redis: &RedisPool, token: &str, ttl_secs: u64) {
    let Some(conn) = redis.as_ref() else { return };
    let mut conn = conn.clone();
    let key = format!("blocklist:{token}");
    if let Err(e) = conn.set_ex::<_, _, ()>(&key, "1", ttl_secs).await {
        warn!("cache blocklist SET: {e}");
    }
}

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

pub fn card_detail_key(nfc_ref: &str) -> String {
    format!("card_detail:{nfc_ref}")
}

pub fn price_key(consumption_type: &str) -> String {
    format!("price:{consumption_type}")
}
