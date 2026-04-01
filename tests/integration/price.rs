use crate::common::{
    body_to_value, create_test_app, create_test_app_with_token, register_and_login,
    setup_redis_pool, test_config,
};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use sqlx::PgPool;
use tower_service::Service;

#[sqlx::test]
async fn create_and_list_prices(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    // Create
    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/prices")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"consumption_type": "Diesel", "price": 1850.0}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["consumption_type"], "Diesel");
    assert_eq!(body["price"], 1850.0);

    // List
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/prices")
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
async fn get_price_by_type(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    sqlx::query("INSERT INTO prices (id, consumption_type, price) VALUES ($1, 'Gasoline', 2100.0)")
        .bind(uuid::Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap();

    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/prices/by-type/Gasoline")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["consumption_type"], "Gasoline");
    assert_eq!(body["price"], 2100.0);
}

#[sqlx::test]
async fn get_price_by_type_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/prices/by-type/NonExistent")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn price_served_from_cache_on_second_call(pool: PgPool) {
    sqlx::query(
        "INSERT INTO prices (id, consumption_type, price) VALUES ($1, 'CacheTest', 1500.0)",
    )
    .bind(uuid::Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();

    let (redis, _container) = setup_redis_pool().await;

    let first = storm_api::services::price_service::get_by_type(&pool, "CacheTest", &redis)
        .await
        .unwrap();
    assert_eq!(first.consumption_type, "CacheTest");

    let second = storm_api::services::price_service::get_by_type(&pool, "CacheTest", &redis)
        .await
        .unwrap();
    assert_eq!(second.consumption_type, "CacheTest");
    assert_eq!(first.price, second.price);
}
