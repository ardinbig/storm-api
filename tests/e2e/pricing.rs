use crate::common::TestApp;
use serde_json::json;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn e2e_commission_and_prices() {
    let app = TestApp::spawn().await;
    let token = app.token().await;

    let resp = app
        .post_json_auth("/api/v1/commissions", &json!({"percentage": 4.5}), &token)
        .await;
    assert_eq!(resp.status(), 201);

    let resp = app.get_auth("/api/v1/commissions/current", &token).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["percentage"], 4.5);

    let resp = app
        .post_json_auth(
            "/api/v1/prices",
            &json!({"consumption_type": "Diesel", "price": 1850.0}),
            &token,
        )
        .await;
    assert_eq!(resp.status(), 201);

    let resp = app.get_auth("/api/v1/prices/by-type/Diesel", &token).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["price"], 1850.0);
}

#[tokio::test]
#[serial]
async fn e2e_commission_tier_crud_flow() {
    let app = TestApp::spawn().await;
    let token = app.token().await;

    let resp = app
        .post_json_auth(
            "/api/v1/commission-tiers",
            &json!({"level1": 2.5, "level2": 1.25, "category": "Bus"}),
            &token,
        )
        .await;
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["level1"], 2.5);
    assert_eq!(body["level2"], 1.25);
    assert_eq!(body["category"], "Bus");

    let resp = app.get_auth("/api/v1/commission-tiers", &token).await;
    assert_eq!(resp.status(), 200);
    let list: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(!list.is_empty());

    let resp = app
        .get_auth("/api/v1/commission-tiers/by-category/Bus", &token)
        .await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["category"], "Bus");
}
