use crate::common::{body_to_value, register_and_login, setup_redis_pool, test_config, test_state};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use sqlx::PgPool;
use storm_api::app::create_app;
use tower_service::Service;
use uuid::Uuid;

// CRUD
// ====

#[sqlx::test]
async fn create_list_get_card(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
    let mut app = create_app(state);

    // Create
    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cards")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"card_id": "NFC-001"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let card = body_to_value(resp.into_body()).await;
    let card_uuid = card["id"].as_str().unwrap();

    // List
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/cards")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body.as_array().unwrap().len(), 1);

    // Get by id
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/cards/{card_uuid}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["card_id"], "NFC-001");
}

#[sqlx::test]
async fn get_card_not_found(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
    let mut app = create_app(state);

    let id = Uuid::new_v4();
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/cards/{id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn create_duplicate_card_id_returns_error(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
    let mut app = create_app(state);

    let body = json!({"card_id": "DUP-CARD-001"}).to_string();

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cards")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cards")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status() == StatusCode::CONFLICT || resp.status() == StatusCode::INTERNAL_SERVER_ERROR
    );
}

// Balance checks
// ==============

#[sqlx::test]
async fn balance_check_card_not_found(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cards/NONEXISTENT/balance")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"password": "x"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn balance_check_success(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    let nfc = "NFC-BALANCE-001";
    sqlx::query("INSERT INTO cards (id, card_id) VALUES ($1, $2)")
        .bind(Uuid::new_v4())
        .bind(nfc)
        .execute(&pool)
        .await
        .unwrap();
    let hash = storm_api::services::auth_service::hash_password("card.pw").unwrap();
    sqlx::query(
        "INSERT INTO card_details (nfc_ref, registration_code, password, amount)
         VALUES ($1, 'REG-BALANCE-001', $2, 250.5)",
    )
    .bind(nfc)
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
                .uri(format!("/api/v1/cards/{nfc}/balance"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"password": "card.pw"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["amount"], 250.5);
}

#[sqlx::test]
async fn balance_check_wrong_password(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    let nfc = "NFC-BAL-PW-001";
    sqlx::query("INSERT INTO cards (id, card_id) VALUES ($1, $2)")
        .bind(Uuid::new_v4())
        .bind(nfc)
        .execute(&pool)
        .await
        .unwrap();
    let hash = storm_api::services::auth_service::hash_password("correct").unwrap();
    sqlx::query(
        "INSERT INTO card_details (nfc_ref, registration_code, password)
         VALUES ($1, 'REG-BAL-PW-001', $2)",
    )
    .bind(nfc)
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
                .uri(format!("/api/v1/cards/{nfc}/balance"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"password": "wrong"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn balance_null_password_in_db(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    let nfc = "NFC-NULL-PW-001";
    sqlx::query("INSERT INTO cards (id, card_id) VALUES ($1, $2)")
        .bind(Uuid::new_v4())
        .bind(nfc)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO card_details (nfc_ref, registration_code, password)
         VALUES ($1, 'REG-NULL-PW-001', NULL)",
    )
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
                .uri(format!("/api/v1/cards/{nfc}/balance"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"password": "x"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// Cache-hit path
// ==============

#[sqlx::test]
async fn card_detail_served_from_cache_on_second_call(pool: PgPool) {
    let nfc = "NFC-CACHE-HIT-001";
    sqlx::query("INSERT INTO cards (id, card_id) VALUES ($1, $2)")
        .bind(Uuid::new_v4())
        .bind(nfc)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO card_details (nfc_ref, registration_code, amount)
         VALUES ($1, 'REG-CACHE-001', 100.0)",
    )
    .bind(nfc)
    .execute(&pool)
    .await
    .unwrap();

    let (redis, _container) = setup_redis_pool().await;

    let first = storm_api::services::card_service::get_detail_by_nfc(&pool, nfc, &redis)
        .await
        .unwrap();
    assert!(first.is_some());

    let second = storm_api::services::card_service::get_detail_by_nfc(&pool, nfc, &redis)
        .await
        .unwrap();
    assert!(second.is_some());
    assert_eq!(first.unwrap().nfc_ref, second.unwrap().nfc_ref);
}
