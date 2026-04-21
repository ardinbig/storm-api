use crate::common::{
    body_to_value, register_and_login, seed_agent, seed_agent_with_station, seed_house_account,
    test_config, test_state,
};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use sqlx::PgPool;
use storm_api::app::create_app;
use storm_api::{
    models::agent::HOUSE_ACCOUNT_REF, models::transaction::WithdrawalRequest,
    services::transaction_service, state::app_state::RedisPool,
};
use tower_service::Service;
use uuid::Uuid;

// Local seed helper
// =================

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

// Paginated list_transactions
// ===========================

/// GET /api/v1/transactions returns paginated metadata on an empty DB.
#[sqlx::test]
async fn list_transactions_empty_returns_paginated_shape(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let mut app = create_app(test_state(pool));

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
    let body = body_to_value(resp.into_body()).await;
    assert!(body["data"].as_array().unwrap().is_empty());
    assert_eq!(body["page"], 1);
    assert_eq!(body["page_size"], 10);
    assert_eq!(body["total_items"], 0);
    assert_eq!(body["total_pages"], 1);
    assert_eq!(body["has_next_page"], false);
    assert_eq!(body["has_prev_page"], false);
    assert_eq!(body["remaining_items"], 0);
}

/// After a withdrawal the item appears in the paginated list.
#[sqlx::test]
async fn list_transactions_after_withdrawal(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let (nfc, agent_ref) = seed_withdrawal_data(&pool).await;

    let state = test_state(pool);
    let mut app = create_app(state);

    // Create a withdrawal
    app.call(
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
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 1);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"][0]["agent_account"], agent_ref);
}

/// `?agent=` filter only returns that agent's transactions.
#[sqlx::test]
async fn list_transactions_filter_by_agent(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let (nfc, agent_ref) = seed_withdrawal_data(&pool).await;

    let state = test_state(pool);
    let mut app = create_app(state);

    app.call(
        Request::builder()
            .method("POST")
            .uri("/api/v1/transactions/withdrawal")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "client_code": nfc, "withdrawal_amount": 10.0,
                    "client_password": "wd.pass", "agent_code": agent_ref,
                    "currency_type": "CDF"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();

    // Filter by that agent
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/transactions?agent={agent_ref}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 1);

    // Filter by an unknown agent — expect 0
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/transactions?agent=NOBODY")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 0);
    assert!(body["data"].as_array().unwrap().is_empty());
}

/// `?station=` filter scopes to agents linked to that station.
#[sqlx::test]
async fn list_transactions_filter_by_station(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    // Create a station (user)
    let station_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, name, username, password) VALUES ($1, 'Station A', 'sta', 'x')",
    )
    .bind(station_id)
    .execute(&pool)
    .await
    .unwrap();

    let nfc = Uuid::new_v4().to_string();
    let client_code = Uuid::new_v4().to_string();
    let agent_ref = "AGENT-STA-001";

    // Seed card + customer
    sqlx::query("INSERT INTO cards (id, card_id) VALUES ($1, $2)")
        .bind(Uuid::new_v4())
        .bind(&nfc)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO customers (id, client_code, first_name, last_name, card_id)
         VALUES ($1, $2, 'Sta', 'Customer', $3)",
    )
    .bind(Uuid::new_v4())
    .bind(&client_code)
    .bind(&nfc)
    .execute(&pool)
    .await
    .unwrap();
    let hash = storm_api::services::auth_service::hash_password("pw").unwrap();
    sqlx::query("UPDATE card_details SET password = $1, amount = 5000 WHERE nfc_ref = $2")
        .bind(&hash)
        .bind(&nfc)
        .execute(&pool)
        .await
        .unwrap();

    // Seed station-linked agent
    seed_agent_with_station(&pool, agent_ref, "Sta Agent", "pw", 100.0, station_id).await;
    seed_house_account(&pool).await;
    sqlx::query("INSERT INTO commissions (id, percentage) VALUES ($1, 5.0)")
        .bind(Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap();

    let state = test_state(pool);
    let mut app = create_app(state);

    // Perform a withdrawal
    app.call(
        Request::builder()
            .method("POST")
            .uri("/api/v1/transactions/withdrawal")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "client_code": nfc, "withdrawal_amount": 10.0,
                    "client_password": "pw", "agent_code": agent_ref,
                    "currency_type": "CDF"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();

    // Filter by correct station_id
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/transactions?station={station_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 1);

    // Filter by a different station — 0 results
    let other = Uuid::new_v4();
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/transactions?station={other}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 0);
}

// Deprecated by-agent endpoint (backward compat)
// ==============================================

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
async fn list_by_agent_after_withdrawal(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let (nfc, agent_ref) = seed_withdrawal_data(&pool).await;

    let state = test_state(pool.clone());
    let mut app = create_app(state);

    app.call(
        Request::builder()
            .method("POST")
            .uri("/api/v1/transactions/withdrawal")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "client_code": nfc, "withdrawal_amount": 50.0,
                    "client_password": "wd.pass", "agent_code": agent_ref,
                    "currency_type": "CDF"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();

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

// Unified activity feed
// =====================

/// Empty DB → activity feed returns paginated shape with zero items.
#[sqlx::test]
async fn list_activity_empty(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let mut app = create_app(test_state(pool));

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/activity")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert!(body["data"].as_array().unwrap().is_empty());
    assert_eq!(body["page"], 1);
    assert_eq!(body["total_items"], 0);
}

/// A withdrawal appears in the activity feed with kind = "WITHDRAWAL".
#[sqlx::test]
async fn list_activity_contains_withdrawal(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let (nfc, agent_ref) = seed_withdrawal_data(&pool).await;

    let state = test_state(pool);
    let mut app = create_app(state);

    app.call(
        Request::builder()
            .method("POST")
            .uri("/api/v1/transactions/withdrawal")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "client_code": nfc, "withdrawal_amount": 50.0,
                    "client_password": "wd.pass", "agent_code": agent_ref,
                    "currency_type": "CDF"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/activity")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 1);
    assert_eq!(body["data"][0]["kind"], "WITHDRAWAL");
    assert_eq!(body["data"][0]["agent_ref"], agent_ref);
}

/// A consumption appears in the activity feed with kind = "CONSUMPTION".
#[sqlx::test]
async fn list_activity_contains_consumption(pool: PgPool) {
    use crate::common::seed_card_with_customer;

    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let client_code = seed_card_with_customer(&pool).await;
    let agent_ref = "CONS-AGENT-001";
    seed_agent(&pool, agent_ref, "Cons Agent", "pw", 0.0).await;

    let state = test_state(pool);
    let mut app = create_app(state);

    app.call(
        Request::builder()
            .method("POST")
            .uri("/api/v1/consumptions")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "date": "2025-01-15T10:00:00Z",
                    "client_ref": client_code,
                    "consumption_type": "Diesel",
                    "quantity": 20.0,
                    "price": 500.0,
                    "username": agent_ref,
                    "is_online": true
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/activity")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 1);
    assert_eq!(body["data"][0]["kind"], "CONSUMPTION");
    assert_eq!(body["data"][0]["agent_ref"], agent_ref);
    // amount = quantity × price = 20 × 500 = 10000
    assert_eq!(body["data"][0]["amount"], 10000.0);
}

/// `?kind=WITHDRAWAL` returns only withdrawals; CONSUMPTION items are excluded.
#[sqlx::test]
async fn list_activity_filter_by_kind_withdrawal(pool: PgPool) {
    use crate::common::seed_card_with_customer;

    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let (nfc, agent_ref) = seed_withdrawal_data(&pool).await;
    let client_code = seed_card_with_customer(&pool).await;
    let cons_agent = "KIND-CONS-AGENT";
    seed_agent(&pool, cons_agent, "Kind Agent", "pw", 0.0).await;

    let state = test_state(pool);
    let mut app = create_app(state);

    // Insert a withdrawal
    app.call(
        Request::builder()
            .method("POST")
            .uri("/api/v1/transactions/withdrawal")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "client_code": nfc, "withdrawal_amount": 10.0,
                    "client_password": "wd.pass", "agent_code": agent_ref,
                    "currency_type": "CDF"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();

    // Insert a consumption
    app.call(
        Request::builder()
            .method("POST")
            .uri("/api/v1/consumptions")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "date": "2025-01-15T10:00:00Z",
                    "client_ref": client_code, "consumption_type": "Diesel",
                    "quantity": 5.0, "price": 100.0,
                    "username": cons_agent, "is_online": true
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();

    // Filter: only WITHDRAWAL
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/activity?kind=WITHDRAWAL")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 1);
    assert_eq!(body["data"][0]["kind"], "WITHDRAWAL");

    // Filter: only CONSUMPTION
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/activity?kind=CONSUMPTION")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 1);
    assert_eq!(body["data"][0]["kind"], "CONSUMPTION");
}

/// `?agent=` on activity filters both withdrawals and consumptions.
#[sqlx::test]
async fn list_activity_filter_by_agent(pool: PgPool) {
    use crate::common::seed_card_with_customer;

    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let client_code = seed_card_with_customer(&pool).await;
    let agent_ref = "ACT-AGENT-001";
    seed_agent(&pool, agent_ref, "Act Agent", "pw", 0.0).await;

    let state = test_state(pool);
    let mut app = create_app(state);

    app.call(
        Request::builder()
            .method("POST")
            .uri("/api/v1/consumptions")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "date": "2025-01-15T10:00:00Z",
                    "client_ref": client_code, "consumption_type": "Diesel",
                    "quantity": 5.0, "price": 100.0,
                    "username": agent_ref, "is_online": true
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();

    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/activity?agent={agent_ref}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 1);

    // unknown agent → 0 results
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/activity?agent=NOBODY")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 0);
}

/// `?station=` on activity scopes to agents belonging to that station.
#[sqlx::test]
async fn list_activity_filter_by_station(pool: PgPool) {
    use crate::common::seed_card_with_customer;

    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    let station_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, name, username, password) VALUES ($1, 'Sta Act', 'sta_act', 'x')",
    )
    .bind(station_id)
    .execute(&pool)
    .await
    .unwrap();

    let client_code = seed_card_with_customer(&pool).await;
    let agent_ref = "STA-ACT-AGENT";
    seed_agent_with_station(&pool, agent_ref, "Sta Act Agent", "pw", 0.0, station_id).await;

    let state = test_state(pool);
    let mut app = create_app(state);

    app.call(
        Request::builder()
            .method("POST")
            .uri("/api/v1/consumptions")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "date": "2025-01-15T10:00:00Z",
                    "client_ref": client_code, "consumption_type": "Diesel",
                    "quantity": 5.0, "price": 100.0,
                    "username": agent_ref, "is_online": true
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();

    // Correct station
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/activity?station={station_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 1);

    // Different station
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/activity?station={}", Uuid::new_v4()))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 0);
}

/// Activity feed paginates correctly: page 2 contains the older item.
#[sqlx::test]
async fn list_activity_pagination_page2(pool: PgPool) {
    use crate::common::seed_card_with_customer;

    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    // Seed 11 consumptions so they span 2 pages
    let agent_ref = "PAGE2-AGENT";
    seed_agent(&pool, agent_ref, "P2 Agent", "pw", 0.0).await;

    for i in 0..11u32 {
        let client_code = seed_card_with_customer(&pool).await;
        sqlx::query(
            "INSERT INTO consumptions (id, client_ref, consumption_type,
             quantity, price, username, consumption_date, status)
             VALUES ($1, $2, 'Diesel', 1.0, 100.0, $3,
                     NOW() - ($4 || ' minutes')::INTERVAL, 1)",
        )
        .bind(Uuid::new_v4())
        .bind(&client_code)
        .bind(agent_ref)
        .bind(i.to_string())
        .execute(&pool)
        .await
        .unwrap();
    }

    let state = test_state(pool);
    let mut app = create_app(state);

    // Page 1
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/activity?page=1")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 11);
    assert_eq!(body["total_pages"], 2);
    assert_eq!(body["data"].as_array().unwrap().len(), 10);
    assert_eq!(body["has_next_page"], true);
    assert_eq!(body["remaining_items"], 1);

    // Page 2
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/activity?page=2")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["has_next_page"], false);
    assert_eq!(body["has_prev_page"], true);
    assert_eq!(body["remaining_items"], 0);
}

/// Activity endpoint requires authentication.
#[sqlx::test]
async fn list_activity_requires_auth(pool: PgPool) {
    let mut app = create_app(test_state(pool));

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/activity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn service_withdrawal_updates_balances_and_records_transaction(pool: PgPool) {
    let (nfc, agent_ref) = seed_withdrawal_data(&pool).await;
    let redis: RedisPool = None;

    let input = WithdrawalRequest {
        client_code: nfc.clone(),
        withdrawal_amount: 100.0,
        client_password: "wd.pass".into(),
        agent_code: agent_ref.clone(),
        currency_type: "CDF".into(),
    };

    let response = transaction_service::withdrawal(&pool, &input, &redis)
        .await
        .unwrap();

    assert_eq!(response.message, "Withdrawal successful");
    assert_eq!(response.client_balance, 9895.0);
    assert_eq!(response.agent_balance, 600.0);

    let card_balance: f64 =
        sqlx::query_scalar("SELECT amount::FLOAT8 FROM card_details WHERE nfc_ref = $1")
            .bind(&nfc)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!((card_balance - 9895.0).abs() < 1e-6);

    let house_balance: f64 =
        sqlx::query_scalar("SELECT balance::FLOAT8 FROM agent_accounts WHERE agent_ref = $1")
            .bind(HOUSE_ACCOUNT_REF)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!((house_balance - 5.0).abs() < 1e-6);
}

#[sqlx::test]
#[allow(deprecated)]
async fn service_list_returns_transactions(pool: PgPool) {
    let (nfc, agent_ref) = seed_withdrawal_data(&pool).await;
    let redis: RedisPool = None;

    let input = WithdrawalRequest {
        client_code: nfc,
        withdrawal_amount: 50.0,
        client_password: "wd.pass".into(),
        agent_code: agent_ref.clone(),
        currency_type: "CDF".into(),
    };
    transaction_service::withdrawal(&pool, &input, &redis)
        .await
        .unwrap();

    let rows = transaction_service::list(&pool).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].agent_account.as_deref(), Some(agent_ref.as_str()));
}

#[sqlx::test]
#[allow(deprecated)]
async fn service_list_by_agent_filters_rows(pool: PgPool) {
    seed_agent(&pool, "AGENT-A", "Svc Agent A", "pw", 0.0).await;
    seed_agent(&pool, "AGENT-B", "Svc Agent B", "pw", 0.0).await;

    sqlx::query(
        "INSERT INTO transactions
         (id, transaction_type, client_account, agent_account, amount, currency_code, commission)
         VALUES ($1, 'WITHDRAWAL', 'C-A', 'AGENT-A', 10.0, 'CDF', 1.0)",
    )
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO transactions
         (id, transaction_type, client_account, agent_account, amount, currency_code, commission)
         VALUES ($1, 'WITHDRAWAL', 'C-B', 'AGENT-B', 20.0, 'CDF', 2.0)",
    )
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();

    let agent_a_rows = transaction_service::list_by_agent(&pool, "AGENT-A")
        .await
        .unwrap();
    assert_eq!(agent_a_rows.len(), 1);
    assert_eq!(agent_a_rows[0].agent_account.as_deref(), Some("AGENT-A"));

    let missing_rows = transaction_service::list_by_agent(&pool, "NOPE")
        .await
        .unwrap();
    assert!(missing_rows.is_empty());
}
