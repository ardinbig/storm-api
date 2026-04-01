use crate::common::{
    body_to_value, create_test_app, create_test_app_with_token, register_and_login, seed_card,
    test_config,
};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use sqlx::PgPool;
use tower_service::Service;
use uuid::Uuid;

#[sqlx::test]
async fn register_customer_via_endpoint(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    seed_card(&pool, "NFC-REG-001").await;
    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/customers")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "card_id": "NFC-REG-001",
                        "name": "Jane Doe",
                        "last_name": "Doe",
                        "first_name": "Jane",
                        "phone": "08111111",
                        "password": "pass123",
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
async fn list_customers_empty(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/customers")
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
async fn get_customer_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let id = Uuid::new_v4();
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/customers/{id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn get_customer_by_card_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/customers/by-card/NONEXISTENT")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn update_and_delete_customer(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    let customer_id = Uuid::new_v4();
    let card_id = "CARD-UPDATE-001";
    seed_card(&pool, card_id).await;
    sqlx::query(
        "INSERT INTO customers (id, client_code, first_name, last_name, phone, card_id)
         VALUES ($1, 'CUST-001', 'Old Firstname', 'Old Lastname', '000', $2)",
    )
    .bind(customer_id)
    .bind(card_id)
    .execute(&pool)
    .await
    .unwrap();

    let mut app = create_test_app(pool);

    // Get
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/customers/{customer_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Update
    let resp = app
        .call(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/customers/{customer_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "first_name": "New Firstname",
                        "last_name": "New Lastname"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["first_name"], "New Firstname");
    assert_eq!(body["last_name"], "New Lastname");

    // Delete
    let resp = app
        .call(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/customers/{customer_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn update_customer_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let id = Uuid::new_v4();
    let resp = app
        .call(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/customers/{id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "first_name": "X", "last_name": "Y" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn delete_customer_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let id = Uuid::new_v4();
    let resp = app
        .call(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/customers/{id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn get_customer_by_card_success(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    let card_nfc = "NFC-BYCARD-001";
    seed_card(&pool, card_nfc).await;
    sqlx::query(
        "INSERT INTO customers (id, client_code, first_name, last_name, card_id)
         VALUES ($1, 'CC-001', 'ByCard', 'Customer', $2)",
    )
    .bind(Uuid::new_v4())
    .bind(card_nfc)
    .execute(&pool)
    .await
    .unwrap();

    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/customers/by-card/{card_nfc}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["client_code"], "CC-001");
}

#[sqlx::test]
async fn partial_update_preserves_unchanged_fields(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    let customer_id = Uuid::new_v4();
    let card_id = "CARD-PARTIAL-001";
    seed_card(&pool, card_id).await;
    sqlx::query(
        "INSERT INTO customers (id, client_code, phone, first_name, last_name, card_id)
         VALUES ($1, 'PARTIAL-001', '0800000', 'OrigFirst', 'OrigLast', $2)",
    )
    .bind(customer_id)
    .bind(card_id)
    .execute(&pool)
    .await
    .unwrap();

    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/customers/{customer_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"first_name": "OrigFirst", "last_name": "OrigLast"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["phone"], "0800000");
    assert_eq!(body["first_name"], "OrigFirst");
    assert_eq!(body["last_name"], "OrigLast");
}
