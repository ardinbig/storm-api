use crate::common::{self, TestApp};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn e2e_customer_crud() {
    let app = TestApp::spawn().await;
    let token = app.token().await;

    let resp = app
        .post_json_auth("/api/v1/cards", &json!({"card_id": "NFC-CUST-E2E"}), &token)
        .await;
    assert_eq!(resp.status(), 201);

    let resp = app
        .post_json_auth(
            "/api/v1/customers",
            &json!({
                "card_id": "NFC-CUST-E2E",
                "name": "E2E Customer",
                "last_name": "Customer",
                "first_name": "E2E",
                "phone": "0800000",
                "password": "customer.pass"
            }),
            &token,
        )
        .await;
    assert_eq!(resp.status(), 201);

    let custom_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO customers (id, client_code, first_name, last_name, phone, card_id)
         VALUES ($1, 'CC-CUST-E2E', 'E2E', 'Customer', '0800000', 'NFC-CUST-E2E')",
    )
    .bind(custom_id)
    .execute(&app.pool)
    .await
    .unwrap();

    let resp = app.get_auth("/api/v1/customers", &token).await;
    assert_eq!(resp.status(), 200);
    let list: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(!list.is_empty());

    let resp = app
        .get_auth(&format!("/api/v1/customers/{custom_id}"), &token)
        .await;
    assert_eq!(resp.status(), 200);

    let resp = app
        .put_json_auth(
            &format!("/api/v1/customers/{custom_id}"),
            &json!({"first_name": "Updated"}),
            &token,
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["first_name"], "Updated");

    let resp = app
        .delete_auth(&format!("/api/v1/customers/{custom_id}"), &token)
        .await;
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
#[serial]
async fn e2e_agent_crud_and_login() {
    let app = TestApp::spawn().await;
    let token = app.token().await;

    let resp = app
        .post_json_auth(
            "/api/v1/agents",
            &json!({
                "agent_ref": "E2E-AGENT-001",
                "name": "E2E Agent",
                "password": "agent.pw"
            }),
            &token,
        )
        .await;
    assert_eq!(resp.status(), 201);
    let agent: serde_json::Value = resp.json().await.unwrap();
    let agent_id = agent["id"].as_str().unwrap();

    let resp = app
        .post_json(
            "/api/v1/agents/login",
            &json!({"username": "E2E-AGENT-001", "password": "agent.pw"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["token"].is_string());

    let resp = app.get_auth("/api/v1/agents", &token).await;
    assert_eq!(resp.status(), 200);
    let list: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(!list.is_empty());

    let resp = app
        .get_auth(&format!("/api/v1/agents/{agent_id}"), &token)
        .await;
    assert_eq!(resp.status(), 200);

    // Patch (update name)
    let resp = app
        .patch_json_auth(
            &format!("/api/v1/agents/{agent_id}"),
            &json!({"name": "Updated E2E Agent"}),
            &token,
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Updated E2E Agent");

    let resp = app
        .delete_auth(&format!("/api/v1/agents/{agent_id}"), &token)
        .await;
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
#[serial]
async fn e2e_agent_password_update_flow() {
    let app = TestApp::spawn().await;
    let token = app.token().await;

    app.post_json_auth(
        "/api/v1/agents",
        &json!({
            "agent_ref": "AGENT-PWD-E2E",
            "name": "Pwd Agent",
            "password": "old.pass"
        }),
        &token,
    )
    .await;

    let resp = app
        .post_json(
            "/api/v1/agents/login",
            &json!({"username": "AGENT-PWD-E2E", "password": "old.pass"}),
        )
        .await;
    assert_eq!(resp.status(), 200);

    let resp = app
        .put_json_auth(
            "/api/v1/agents/password",
            &json!({
                "agent_ref": "AGENT-PWD-E2E",
                "last_password": "old.pass",
                "new_password": "newpass"
            }),
            &token,
        )
        .await;
    assert_eq!(resp.status(), 200);

    let resp = app
        .post_json(
            "/api/v1/agents/login",
            &json!({"username": "AGENT-PWD-E2E", "password": "old.pass"}),
        )
        .await;
    assert_eq!(resp.status(), 401);

    let resp = app
        .post_json(
            "/api/v1/agents/login",
            &json!({"username": "AGENT-PWD-E2E", "password": "newpass"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
#[serial]
async fn e2e_agent_token_accesses_user_me() {
    let app = TestApp::spawn().await;

    common::seed_agent(&app.pool, "AGENT-ROLE-TEST", "Role Agent", "agent.pw", 0.0).await;

    let resp = app
        .post_json(
            "/api/v1/agents/login",
            &json!({"username": "AGENT-ROLE-TEST", "password": "agent.pw"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let agent_token = body["token"].as_str().unwrap();

    let resp = app.get_auth("/api/v1/users/me", agent_token).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["role"], "agent");
}

#[tokio::test]
#[serial]
async fn e2e_duplicate_agent_ref_returns_error() {
    let app = TestApp::spawn().await;
    let token = app.token().await;

    let body = json!({
        "agent_ref": "DUP-AGENT-E2E",
        "name": "Dup Agent",
        "password": "pass"
    });

    let resp = app.post_json_auth("/api/v1/agents", &body, &token).await;
    assert_eq!(resp.status(), 201);

    let resp = app.post_json_auth("/api/v1/agents", &body, &token).await;
    assert!(resp.status() == 409 || resp.status() == 500);
}
