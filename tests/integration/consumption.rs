use crate::common::{
    body_to_value, create_test_app, create_test_app_with_token, register_and_login,
    seed_card_with_customer, test_config,
};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use sqlx::PgPool;
use tower_service::Service;

#[sqlx::test]
async fn create_and_list_consumptions(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let client_code = seed_card_with_customer(&pool).await;
    let mut app = create_test_app(pool);

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
    let (mut app, token) = create_test_app_with_token(pool).await;

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
