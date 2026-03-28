use crate::common::{body_to_string, body_to_value, test_state};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use storm_api::app::create_app;
use tower_service::Service;

#[sqlx::test]
async fn health_returns_ok(pool: PgPool) {
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(body_to_string(resp.into_body()).await, "OK");
}

#[sqlx::test]
async fn ready_returns_ok(pool: PgPool) {
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(body_to_string(resp.into_body()).await, "ready");
}

#[sqlx::test]
async fn ready_returns_unavailable_when_not_ready(pool: PgPool) {
    let state = test_state(pool);
    state
        .ready
        .store(false, std::sync::atomic::Ordering::SeqCst);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[sqlx::test]
async fn metrics_returns_json(pool: PgPool) {
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert!(body["requests"].as_u64().unwrap() >= 1);
}

#[sqlx::test]
async fn not_found_returns_404(pool: PgPool) {
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .uri("/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
