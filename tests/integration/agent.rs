use crate::common::{
    body_to_value, create_test_app, create_test_app_with_token, seed_agent, seed_card,
    seed_house_account,
};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower_service::Service;
use uuid::Uuid;

// Request helpers
// ===============

async fn send(
    app: &mut Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    let req_body = body.map_or_else(Body::empty, |v| Body::from(v.to_string()));
    let resp = app.call(builder.body(req_body).unwrap()).await.unwrap();
    let status = resp.status();
    (status, body_to_value(resp.into_body()).await)
}

async fn auth_get(app: &mut Router, uri: &str, t: &str) -> (StatusCode, Value) {
    send(app, "GET", uri, Some(t), None).await
}

async fn auth_post(app: &mut Router, uri: &str, b: Value, t: &str) -> (StatusCode, Value) {
    send(app, "POST", uri, Some(t), Some(b)).await
}

async fn auth_patch(app: &mut Router, uri: &str, b: Value, t: &str) -> (StatusCode, Value) {
    send(app, "PATCH", uri, Some(t), Some(b)).await
}

async fn auth_put(app: &mut Router, uri: &str, b: Value, t: &str) -> (StatusCode, Value) {
    send(app, "PUT", uri, Some(t), Some(b)).await
}

async fn auth_delete(app: &mut Router, uri: &str, t: &str) -> (StatusCode, Value) {
    send(app, "DELETE", uri, Some(t), None).await
}

async fn public_post(app: &mut Router, uri: &str, b: Value) -> (StatusCode, Value) {
    send(app, "POST", uri, None, Some(b)).await
}

// Agent login
// ===========

#[sqlx::test]
async fn agent_login_success(pool: PgPool) {
    seed_agent(&pool, "AGENT-001", "Test Agent", "agent.pass", 1000.0).await;
    let mut app = create_test_app(pool);

    let (status, body) = public_post(
        &mut app,
        "/api/v1/agents/login",
        json!({"username": "AGENT-001", "password": "agent.pass"}),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["token"].is_string());
    assert_eq!(body["agent"]["agent_ref"], "AGENT-001");
}

#[sqlx::test]
async fn agent_login_wrong_password(pool: PgPool) {
    seed_agent(&pool, "AGENT-002", "Test Agent", "agent.pass", 0.0).await;
    let mut app = create_test_app(pool);

    let (status, _) = public_post(
        &mut app,
        "/api/v1/agents/login",
        json!({"username": "AGENT-002", "password": "wrong"}),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn agent_login_nonexistent(pool: PgPool) {
    let mut app = create_test_app(pool);

    let (status, _) = public_post(
        &mut app,
        "/api/v1/agents/login",
        json!({"username": "GHOST", "password": "x"}),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn agent_login_no_password_in_db(pool: PgPool) {
    sqlx::query(
        "INSERT INTO agent_accounts (id, agent_ref, name, password, balance, currency_code)
         VALUES ($1, 'AGENT-NO.PASS', 'No Pass Agent', NULL, 0, 'CDF')",
    )
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();

    let mut app = create_test_app(pool);

    let (status, _) = public_post(
        &mut app,
        "/api/v1/agents/login",
        json!({"username": "AGENT-NO.PASS", "password": "x"}),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// Agent CRUD
// ==========

#[sqlx::test]
async fn list_create_get_delete_agent(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    // List (empty)
    let (status, body) = auth_get(&mut app, "/api/v1/agents", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.as_array().unwrap().is_empty());

    // Create
    let (status, agent) = auth_post(
        &mut app,
        "/api/v1/agents",
        json!({"agent_ref": "NEW-AGENT-001", "name": "New Agent", "password": "new.pass"}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let agent_id = agent["id"].as_str().unwrap();

    // Get
    let (status, _) = auth_get(&mut app, &format!("/api/v1/agents/{agent_id}"), &token).await;
    assert_eq!(status, StatusCode::OK);

    // Delete
    let (status, _) = auth_delete(&mut app, &format!("/api/v1/agents/{agent_id}"), &token).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn create_agent_with_custom_currency(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, body) = auth_post(
        &mut app,
        "/api/v1/agents",
        json!({"agent_ref": "AG-USD", "name": "USD Agent", "password": "pass", "currency_code": "USD"}),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["currency_code"], "USD");
}

#[sqlx::test]
async fn get_agent_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, _) = auth_get(
        &mut app,
        &format!("/api/v1/agents/{}", Uuid::new_v4()),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn delete_agent_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, _) = auth_delete(
        &mut app,
        &format!("/api/v1/agents/{}", Uuid::new_v4()),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn cannot_delete_house_account(pool: PgPool) {
    seed_house_account(&pool).await;

    let row: (Uuid,) = sqlx::query_as(&format!(
        "SELECT id FROM agent_accounts WHERE agent_ref = '{}'",
        storm_api::models::agent::HOUSE_ACCOUNT_REF
    ))
    .fetch_one(&pool)
    .await
    .unwrap();

    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, _) = auth_delete(&mut app, &format!("/api/v1/agents/{}", row.0), &token).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// Agent update
// ============

#[sqlx::test]
async fn update_agent_partial_fields(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, agent) = auth_post(
        &mut app,
        "/api/v1/agents",
        json!({"agent_ref": "AG-UPD-001", "name": "Original", "password": "pass"}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let uri = format!("/api/v1/agents/{}", agent["id"].as_str().unwrap());

    // Patch name only → currency unchanged
    let (status, body) = auth_patch(&mut app, &uri, json!({"name": "Updated"}), &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Updated");
    assert_eq!(body["currency_code"], "CDF");

    // Patch currency only → name preserved
    let (status, body) = auth_patch(&mut app, &uri, json!({"currency_code": "USD"}), &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["currency_code"], "USD");
    assert_eq!(body["name"], "Updated");

    // Empty patch → no-op
    let (status, body) = auth_patch(&mut app, &uri, json!({}), &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Updated");
    assert_eq!(body["currency_code"], "USD");
}

#[sqlx::test]
async fn update_agent_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, _) = auth_patch(
        &mut app,
        &format!("/api/v1/agents/{}", Uuid::new_v4()),
        json!({"name": "Ghost"}),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// Agent balance / history / customer registration
// ===============================================

#[sqlx::test]
async fn agent_check_balance_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, _) = auth_get(&mut app, "/api/v1/agents/cards/NONEXISTENT/balance", &token).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn agent_check_balance_success(pool: PgPool) {
    let nfc = "NFC-BAL-001";
    seed_card(&pool, nfc).await;
    let hash = storm_api::services::auth_service::hash_password("card.pass").unwrap();
    sqlx::query(
        "INSERT INTO card_details (nfc_ref, client_code, password, amount)
         VALUES ($1, 'REG-BAL-001', $2, 500)",
    )
    .bind(nfc)
    .bind(&hash)
    .execute(&pool)
    .await
    .unwrap();

    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, body) = auth_get(
        &mut app,
        &format!("/api/v1/agents/cards/{nfc}/balance"),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["nfc_ref"], nfc);
}

#[sqlx::test]
async fn agent_history_empty(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, body) = auth_get(&mut app, "/api/v1/agents/AGENT-X/history", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.as_array().unwrap().is_empty());
}

#[sqlx::test(fixtures(path = "../../fixtures", scripts("seed_categories")))]
async fn agent_register_customer(pool: PgPool) {
    seed_card(&pool, "NFC-ARES-001").await;
    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, _) = auth_post(
        &mut app,
        "/api/v1/agents/customers",
        json!({
            "name": "Agent Customer",
            "last_name": "Customer",
            "first_name": "Agent",
            "phone": "0899999",
            "card_ref": "NFC-ARES-001"
        }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
}

#[sqlx::test]
async fn agent_register_customer_card_conflict(pool: PgPool) {
    let nfc = "NFC-CONFLICT-001";
    seed_card(&pool, nfc).await;
    sqlx::query(
        "INSERT INTO card_details (nfc_ref, client_code, password)
         VALUES ($1, 'REG-CONF', 'hash')",
    )
    .bind(nfc)
    .execute(&pool)
    .await
    .unwrap();

    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, _) = auth_post(
        &mut app,
        "/api/v1/agents/customers",
        json!({
            "name": "Dup",
            "last_name": "Dup",
            "first_name": "Dup",
            "phone": "000",
            "card_ref": nfc
        }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
}

// Agent password
// ==============

#[sqlx::test]
async fn agent_update_password(pool: PgPool) {
    seed_agent(&pool, "AGENT-PWD", "Test Agent", "agent.pass", 0.0).await;
    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, _) = auth_put(
        &mut app,
        "/api/v1/agents/password",
        json!({"agent_ref": "AGENT-PWD", "last_password": "agent.pass", "new_password": "new.agent.pass"}),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn agent_update_password_wrong_old(pool: PgPool) {
    seed_agent(&pool, "AGENT-PWD.BAD", "Test Agent", "agent.pass", 0.0).await;
    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, _) = auth_put(
        &mut app,
        "/api/v1/agents/password",
        json!({"agent_ref": "AGENT-PWD.BAD", "last_password": "wrong", "new_password": "new"}),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn agent_update_password_nonexistent(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, _) = auth_put(
        &mut app,
        "/api/v1/agents/password",
        json!({"agent_ref": "NONEXISTENT", "last_password": "x", "new_password": "y"}),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn agent_update_password_null_stored(pool: PgPool) {
    sqlx::query(
        "INSERT INTO agent_accounts (id, agent_ref, name, password, balance, currency_code)
         VALUES ($1, 'AGENT-NULL.PW', 'Null Pw', NULL, 0, 'CDF')",
    )
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();

    let (mut app, token) = create_test_app_with_token(pool).await;

    let (status, _) = auth_put(
        &mut app,
        "/api/v1/agents/password",
        json!({"agent_ref": "AGENT-NULL.PW", "last_password": "x", "new_password": "y"}),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
