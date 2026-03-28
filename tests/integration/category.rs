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

#[sqlx::test(fixtures(path = "../../fixtures", scripts("seed_categories")))]
async fn list_categories(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/categories")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[sqlx::test(fixtures(path = "../../fixtures", scripts("seed_categories")))]
async fn get_category_by_id(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/categories/a0000000-0000-0000-0000-000000000001")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["name"], "Motorbike");
}

#[sqlx::test]
async fn get_category_not_found(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
    let mut app = create_app(state);

    let id = Uuid::new_v4();
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/categories/{id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn create_category(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/categories")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"name": "Truck"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["name"], "Truck");
}
