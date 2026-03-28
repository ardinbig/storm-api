use storm_api::utils::cache;

#[tokio::test]
async fn cache_get_returns_none_when_redis_missing() {
    let value = cache::get::<serde_json::Value>(&None, "missing:key").await;
    assert!(value.is_none());
}

#[tokio::test]
async fn cache_is_blocklisted_returns_false_when_redis_missing() {
    let blocked = cache::is_blocklisted(&None, "token-123").await;
    assert!(!blocked);
}

#[tokio::test]
async fn cache_set_del_and_blocklist_noop_when_redis_missing() {
    cache::set(&None, "k", &serde_json::json!({"a": 1}), 60).await;
    cache::del(&None, "k").await;
    cache::blocklist_token(&None, "token-abc", 60).await;

    let blocked = cache::is_blocklisted(&None, "token-abc").await;
    assert!(!blocked);
}

#[test]
fn cache_key_helpers_format_expected_prefixes() {
    assert_eq!(cache::card_detail_key("NFC-001"), "card_detail:NFC-001");
    assert_eq!(cache::price_key("Diesel"), "price:Diesel");
}
