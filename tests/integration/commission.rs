use crate::common::{
    body_to_value, create_test_app, create_test_app_with_token, register_and_login, test_config,
};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use sqlx::PgPool;
use tower_service::Service;

#[sqlx::test]
async fn create_and_list_commissions(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    // Create
    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/commissions")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"percentage": 3.5}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["percentage"], 3.5);

    // List
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/commissions")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert!(!body.as_array().unwrap().is_empty());
}

#[sqlx::test]
async fn get_current_commission(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    sqlx::query("INSERT INTO commissions (id, percentage) VALUES ($1, 7.0)")
        .bind(uuid::Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap();

    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/commissions/current")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["percentage"], 7.0);
}

#[sqlx::test]
async fn get_current_commission_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/commissions/current")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn multiple_commissions_get_current_returns_latest(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    sqlx::query("INSERT INTO commissions (id, percentage, created_at) VALUES ($1, 3.0, NOW() - INTERVAL '2 hours')")
        .bind(uuid::Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO commissions (id, percentage, created_at) VALUES ($1, 5.0, NOW() - INTERVAL '1 hour')")
        .bind(uuid::Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO commissions (id, percentage, created_at) VALUES ($1, 7.0, NOW())")
        .bind(uuid::Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap();

    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/commissions/current")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["percentage"], 7.0);
}
