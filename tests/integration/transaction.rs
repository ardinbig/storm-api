use crate::common::{
    body_to_value, register_and_login, seed_house_account, test_config, test_state,
};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use sqlx::PgPool;
use storm_api::app::create_app;
use tower_service::Service;
use uuid::Uuid;

// Local helper
// ============

async fn seed_withdrawal_data(pool: &PgPool) -> (String, String) {
    let nfc = &Uuid::new_v4().to_string();
    let client_code = &Uuid::new_v4().to_string();
    let agent_ref = "AGENT-WD-001";

    sqlx::query("INSERT INTO cards (id, card_id) VALUES ($1, $2)")
        .bind(Uuid::new_v4())
        .bind(nfc)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO customers (id, client_code, first_name, middle_name, last_name, card_id)
         VALUES ($1, $2, 'WDC Firstname', NULL, 'WDC Lastname', $3)",
    )
    .bind(Uuid::new_v4())
    .bind(client_code)
    .bind(nfc)
    .execute(pool)
    .await
    .unwrap();

    // The card_details row will be inserted by the trigger on customers insert.
    // Optionally, update the password and amount if needed:
    let hash = storm_api::services::auth_service::hash_password("wd.pass").unwrap();
    sqlx::query("UPDATE card_details SET password = $1, amount = 10000 WHERE nfc_ref = $2")
        .bind(&hash)
        .bind(nfc)
        .execute(pool)
        .await
        .unwrap();

    let agent_hash = storm_api::services::auth_service::hash_password("agent.pw").unwrap();
    sqlx::query(
        "INSERT INTO agent_accounts (id, agent_ref, name, password, balance, currency_code)
         VALUES ($1, $2, 'WD Agent', $3, 500, 'CDF')",
    )
    .bind(Uuid::new_v4())
    .bind(agent_ref)
    .bind(&agent_hash)
    .execute(pool)
    .await
    .unwrap();

    seed_house_account(pool).await;

    sqlx::query("INSERT INTO commissions (id, percentage) VALUES ($1, 5.0)")
        .bind(Uuid::new_v4())
        .execute(pool)
        .await
        .unwrap();

    (nfc.to_string(), agent_ref.to_string())
}

// Tests
// =====

#[sqlx::test]
async fn withdrawal_success(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let (nfc, agent_ref) = seed_withdrawal_data(&pool).await;

    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/transactions/withdrawal")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "client_code": nfc,
                        "withdrawal_amount": 100.0,
                        "client_password": "wd.pass",
                        "agent_code": agent_ref,
                        "currency_type": "CDF"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["message"], "Withdrawal successful");
    assert_eq!(body["client_balance"], 10000.0 - 105.0);
    assert_eq!(body["agent_balance"], 500.0 + 100.0);
}

#[sqlx::test]
async fn withdrawal_invalid_card(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/transactions/withdrawal")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "client_code": "FAKE",
                        "withdrawal_amount": 10.0,
                        "client_password": "x",
                        "agent_code": "x",
                        "currency_type": "CDF"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn withdrawal_wrong_password(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let (nfc, agent_ref) = seed_withdrawal_data(&pool).await;

    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/transactions/withdrawal")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "client_code": nfc,
                        "withdrawal_amount": 10.0,
                        "client_password": "wrongpw",
                        "agent_code": agent_ref,
                        "currency_type": "CDF"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn withdrawal_agent_not_found(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let (nfc, _) = seed_withdrawal_data(&pool).await;

    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/transactions/withdrawal")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "client_code": nfc,
                        "withdrawal_amount": 10.0,
                        "client_password": "wd.pass",
                        "agent_code": "GHOST-AGENT",
                        "currency_type": "CDF"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn withdrawal_insufficient_balance(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let (nfc, agent_ref) = seed_withdrawal_data(&pool).await;

    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/transactions/withdrawal")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "client_code": nfc,
                        "withdrawal_amount": 999999.0,
                        "client_password": "wd.pass",
                        "agent_code": agent_ref,
                        "currency_type": "CDF"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn withdrawal_no_commission_rate(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    let nfc = &Uuid::new_v4().to_string();
    let client_code = &Uuid::new_v4().to_string();
    let agent_ref = &Uuid::new_v4().to_string();

    sqlx::query("INSERT INTO cards (id, card_id) VALUES ($1, $2)")
        .bind(Uuid::new_v4())
        .bind(nfc)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO customers (id, client_code, first_name, middle_name, last_name, card_id) VALUES ($1, $2, 'NC', NULL, NULL, $3)")
        .bind(Uuid::new_v4())
        .bind(client_code)
        .bind(nfc)
        .execute(&pool)
        .await
        .unwrap();
    let hash = storm_api::services::auth_service::hash_password("pw").unwrap();
    sqlx::query("UPDATE card_details SET password = $1, amount = 10000 WHERE nfc_ref = $2")
        .bind(&hash)
        .bind(nfc)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO agent_accounts (id, agent_ref, name, password, balance, currency_code)
         VALUES ($1, $2, 'NC Agent', $3, 100, 'CDF')",
    )
    .bind(Uuid::new_v4())
    .bind(agent_ref)
    .bind(&hash)
    .execute(&pool)
    .await
    .unwrap();

    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/transactions/withdrawal")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "client_code": nfc,
                        "withdrawal_amount": 10.0,
                        "client_password": "pw",
                        "agent_code": agent_ref,
                        "currency_type": "CDF"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[sqlx::test]
async fn withdrawal_null_card_password(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    let nfc = &Uuid::new_v4().to_string();
    let client_code = &Uuid::new_v4().to_string();

    sqlx::query("INSERT INTO cards (id, card_id) VALUES ($1, $2)")
        .bind(Uuid::new_v4())
        .bind(nfc)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO customers (id, client_code, first_name, middle_name, last_name, card_id) VALUES ($1, $2, 'NP', NULL, NULL, $3)")
        .bind(Uuid::new_v4())
        .bind(client_code)
        .bind(nfc)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE card_details SET password = NULL, amount = 10000 WHERE nfc_ref = $1")
        .bind(nfc)
        .execute(&pool)
        .await
        .unwrap();

    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/transactions/withdrawal")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "client_code": nfc,
                        "withdrawal_amount": 10.0,
                        "client_password": "x",
                        "agent_code": "x",
                        "currency_type": "CDF"
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
async fn list_transactions(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/transactions")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test]
async fn list_transactions_by_agent(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/transactions/by-agent/NOBODY")
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

#[sqlx::test]
async fn withdrawal_then_balance_reflects_deduction(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let (nfc, agent_ref) = seed_withdrawal_data(&pool).await;

    let state = test_state(pool.clone());
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/transactions/withdrawal")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "client_code": nfc,
                        "withdrawal_amount": 200.0,
                        "client_password": "wd.pass",
                        "agent_code": agent_ref,
                        "currency_type": "CDF"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let card = storm_api::services::card_service::check_balance(&pool, &nfc, "wd.pass")
        .await
        .unwrap();
    let expected = 10000.0 - 200.0 - (200.0 * 5.0 / 100.0);
    assert!((card.amount - expected).abs() < 1e-6);
}

#[sqlx::test]
async fn list_by_agent_after_withdrawal(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let (nfc, agent_ref) = seed_withdrawal_data(&pool).await;

    let state = test_state(pool.clone());
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/transactions/withdrawal")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "client_code": nfc,
                        "withdrawal_amount": 50.0,
                        "client_password": "wd.pass",
                        "agent_code": agent_ref,
                        "currency_type": "CDF"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/transactions/by-agent/{agent_ref}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["agent_account"], agent_ref);
}
