use crate::common::{
    body_to_value, create_test_app, create_test_app_with_token, register_and_login,
    setup_redis_pool, test_config, test_state_with_redis,
};
use axum::{
    body::Body,
    http::{HeaderValue, Request, StatusCode, header},
};
use sqlx::PgPool;
use storm_api::{app::create_app, utils::cache};
use tower_service::Service;

#[sqlx::test]
async fn protected_route_rejects_without_token(pool: PgPool) {
    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/users/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn protected_route_rejects_with_bad_token(pool: PgPool) {
    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/users/me")
                .header(header::AUTHORIZATION, "Bearer invalid.token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn protected_route_rejects_without_bearer_prefix(pool: PgPool) {
    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/users/me")
                .header(header::AUTHORIZATION, "Token abc.def.ghi")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn protected_route_rejects_non_utf8_authorization_header(pool: PgPool) {
    let mut app = create_test_app(pool);

    // Forces the `to_str()` failure branch in auth middleware.
    let invalid_auth = HeaderValue::from_bytes(b"Bearer \xFF").unwrap();

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/users/me")
                .header(header::AUTHORIZATION, invalid_auth)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn protected_route_accepts_valid_token(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/users/me")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["role"], "user");
}

#[sqlx::test]
async fn protected_route_rejects_blocklisted_token(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    let (redis, _redis_container) = setup_redis_pool().await;
    cache::blocklist_token(&redis, &token, 60).await;

    let state = test_state_with_redis(pool, redis);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/users/me")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
