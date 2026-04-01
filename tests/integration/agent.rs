use crate::common::{
    body_to_value, create_test_app, create_test_app_with_token, register_and_login, seed_agent,
    seed_card, seed_house_account, test_config,
};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use sqlx::PgPool;
use tower_service::Service;
use uuid::Uuid;

// Agent login
// ===========

#[sqlx::test]
async fn agent_login_success(pool: PgPool) {
    seed_agent(&pool, "AGENT-001", "Test Agent", "agent.pass", 1000.0).await;
    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "AGENT-001", "password": "agent.pass"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert!(body["token"].is_string());
    assert_eq!(body["agent"]["agent_ref"], "AGENT-001");
}

#[sqlx::test]
async fn agent_login_wrong_password(pool: PgPool) {
    seed_agent(&pool, "AGENT-002", "Test Agent", "agent.pass", 0.0).await;
    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "AGENT-002", "password": "wrong"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn agent_login_nonexistent(pool: PgPool) {
    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "GHOST", "password": "x"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
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

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "AGENT-NO.PASS", "password": "x"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// Agent CRUD
// ==========

#[sqlx::test]
async fn list_create_get_delete_agent(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    // List (empty)
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/agents")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert!(body.as_array().unwrap().is_empty());

    // Create
    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "agent_ref": "NEW-AGENT-001",
                        "name": "New Agent",
                        "password": "new.pass"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let agent = body_to_value(resp.into_body()).await;
    let agent_id = agent["id"].as_str().unwrap();

    // Get
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/agents/{agent_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Delete
    let resp = app
        .call(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/agents/{agent_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn get_agent_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/agents/{}", Uuid::new_v4()))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn delete_agent_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/agents/{}", Uuid::new_v4()))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn cannot_delete_house_account(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    seed_house_account(&pool).await;

    let row: (Uuid,) = sqlx::query_as(&format!(
        "SELECT id FROM agent_accounts WHERE agent_ref = '{}'",
        storm_api::models::agent::HOUSE_ACCOUNT_REF
    ))
    .fetch_one(&pool)
    .await
    .unwrap();

    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/agents/{}", row.0))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// Agent balance / history / customer registration
// ===============================================

#[sqlx::test]
async fn agent_check_balance_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/agents/cards/NONEXISTENT/balance")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn agent_check_balance_success(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

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

    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/agents/cards/{nfc}/balance"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["nfc_ref"], nfc);
}

#[sqlx::test]
async fn agent_history_empty(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/agents/AGENT-X/history")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert!(body.as_array().unwrap().is_empty());
}

#[sqlx::test(fixtures(path = "../../fixtures", scripts("seed_categories")))]
async fn agent_register_customer(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    seed_card(&pool, "NFC-ARES-001").await;
    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/customers")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "Agent Customer",
                        "last_name": "Customer",
                        "first_name": "Agent",
                        "phone": "0899999",
                        "card_ref": "NFC-ARES-001"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[sqlx::test]
async fn agent_register_customer_card_conflict(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

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

    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/customers")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "Dup",
                        "last_name": "Dup",
                        "first_name": "Dup",
                        "phone": "000",
                        "card_ref": nfc
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

// Agent password
// ==============

#[sqlx::test]
async fn agent_update_password(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    seed_agent(&pool, "AGENT-PWD", "Test Agent", "agent.pass", 0.0).await;
    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/agents/password")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "agent_ref": "AGENT-PWD",
                        "last_password": "agent.pass",
                        "new_password": "new.agent.pass"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test]
async fn agent_update_password_wrong_old(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    seed_agent(&pool, "AGENT-PWD.BAD", "Test Agent", "agent.pass", 0.0).await;
    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/agents/password")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "agent_ref": "AGENT-PWD.BAD",
                        "last_password": "wrong",
                        "new_password": "new"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn agent_update_password_nonexistent(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/agents/password")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "agent_ref": "NONEXISTENT",
                        "last_password": "x",
                        "new_password": "y"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn agent_update_password_null_stored(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    sqlx::query(
        "INSERT INTO agent_accounts (id, agent_ref, name, password, balance, currency_code)
         VALUES ($1, 'AGENT-NULL.PW', 'Null Pw', NULL, 0, 'CDF')",
    )
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();

    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/agents/password")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "agent_ref": "AGENT-NULL.PW",
                        "last_password": "x",
                        "new_password": "y"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn create_agent_with_custom_currency(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "agent_ref": "AG-USD",
                        "name": "USD Agent",
                        "password": "pass",
                        "currency_code": "USD"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["currency_code"], "USD");
}
