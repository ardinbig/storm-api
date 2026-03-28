use crate::common::{
    body_to_value, register_and_login,  test_config, test_state
};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use sqlx::PgPool;
use storm_api::app::create_app;
use tower_service::Service;

#[sqlx::test]
async fn protected_route_rejects_without_token(pool: PgPool) {
    let state = test_state(pool);
    let mut app = create_app(state);

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
    let state = test_state(pool);
    let mut app = create_app(state);

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
async fn protected_route_accepts_valid_token(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
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

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["role"], "user");
}
