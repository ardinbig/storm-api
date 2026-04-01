use crate::common::TestApp;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn e2e_card_crud() {
    let app = TestApp::spawn().await;
    let token = app.token().await;

    let resp = app
        .post_json_auth("/api/v1/cards", &json!({"card_id": "NFC-E2E-001"}), &token)
        .await;
    assert_eq!(resp.status(), 201);
    let card: serde_json::Value = resp.json().await.unwrap();
    let card_id = card["id"].as_str().unwrap();
    assert_eq!(card["card_id"], "NFC-E2E-001");

    let resp = app.get_auth("/api/v1/cards", &token).await;
    assert_eq!(resp.status(), 200);
    let list: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(list.len(), 1);

    let resp = app
        .get_auth(&format!("/api/v1/cards/{card_id}"), &token)
        .await;
    assert_eq!(resp.status(), 200);

    let resp = app
        .get_auth(&format!("/api/v1/cards/{}", Uuid::new_v4()), &token)
        .await;
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
#[serial]
async fn e2e_category_crud() {
    let app = TestApp::spawn().await;
    let token = app.token().await;

    let resp = app.get_auth("/api/v1/categories", &token).await;
    assert_eq!(resp.status(), 200);
    let list: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(list.len(), 2);

    let resp = app
        .post_json_auth("/api/v1/categories", &json!({"name": "Truck"}), &token)
        .await;
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Truck");
}
