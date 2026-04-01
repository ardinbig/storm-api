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
async fn create_and_list_tiers(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    // Create
    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/commission-tiers")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"level1": 2.0, "level2": 1.0, "category": "Motorbike"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["level1"], 2.0);
    assert_eq!(body["level2"], 1.0);

    // List
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/commission-tiers")
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
async fn get_tier_by_category(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    sqlx::query(
        "INSERT INTO commission_tiers (id, level1, level2, category) VALUES ($1, 3.0, 1.5, 'Bus')",
    )
    .bind(uuid::Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();

    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/commission-tiers/by-category/Bus")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["category"], "Bus");
}

#[sqlx::test]
async fn get_tier_by_category_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/commission-tiers/by-category/NonExistent")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
