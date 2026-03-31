use crate::common::{body_to_value, register_and_login, test_config, test_state};
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

async fn seed_customer_for_consumption(pool: &PgPool) -> String {
    let nfc = format!("NFC-CON-{}", &Uuid::new_v4().to_string()[..8]);
    let client_code = format!("CC-CON-{}", &Uuid::new_v4().to_string()[..8]);
    sqlx::query("INSERT INTO cards (id, card_id) VALUES ($1, $2)")
        .bind(Uuid::new_v4())
        .bind(&nfc)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO customers (id, client_code, first_name, last_name, card_id)
         VALUES ($1, $2, 'Consumption', 'Customer', $3)",
    )
    .bind(Uuid::new_v4())
    .bind(&client_code)
    .bind(&nfc)
    .execute(pool)
    .await
    .unwrap();
    client_code
}

// Tests
// =====

#[sqlx::test]
async fn create_and_list_consumptions(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let client_code = seed_customer_for_consumption(&pool).await;

    let state = test_state(pool);
    let mut app = create_app(state);

    // Create
    let resp = app
        .call(
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
                        "quantity": 50.0,
                        "price": 2500.0,
                        "username": "test.user",
                        "is_online": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // List all
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/consumptions")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert!(!body.as_array().unwrap().is_empty());

    // List by client
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/consumptions/by-client/{client_code}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn list_by_client_empty(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/consumptions/by-client/NOBODY")
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
