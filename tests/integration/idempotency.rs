use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    middleware,
    response::IntoResponse,
    routing::{get, post},
};
use http_body_util::BodyExt;
use redis::AsyncCommands;
use sqlx::PgPool;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use storm_api::{errors::AppError, middleware::idempotency};
use tower_service::Service;

use crate::common::{
    assert_canonical_idempotency_log_layout, body_to_value, fallback_marker, register_and_login,
    setup_redis_pool, test_config, test_state, test_state_with_redis,
};

#[test]
fn canonical_log_layout_helper_accepts_both_authenticated_and_unauthenticated_shapes() {
    let with_user = idempotency::IdempotencyTelemetry::new(
        idempotency::IdempotencyEvent::LockConflict,
        "processing lock conflict",
        "POST::NO_MATCHED_PATH",
        Some("user-42"),
    );
    let without_user = idempotency::IdempotencyTelemetry::new(
        idempotency::IdempotencyEvent::CacheHit,
        "cache hit",
        fallback_marker(),
        None::<&str>,
    );

    assert_canonical_idempotency_log_layout(&with_user);
    assert_canonical_idempotency_log_layout(&without_user);
}

#[test]
fn path_label_prefers_matched_path_and_falls_back_to_the_static_method_marker() {
    assert_eq!(
        idempotency::path_label("POST", Some("/api/v1/widgets")),
        "/api/v1/widgets"
    );
    assert_eq!(
        idempotency::path_label("POST", None),
        "POST::NO_MATCHED_PATH"
    );
}

#[tokio::test]
async fn processing_conflict_response_has_the_standard_json_contract_and_retry_after_header() {
    let response = idempotency::processing_conflict_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let retry_after = response.headers().get(header::RETRY_AFTER);
    assert_eq!(retry_after.and_then(|v| v.to_str().ok()), Some("1"));

    let body = body_to_value(response.into_body()).await;
    assert_eq!(body["error"], "processing");
    assert_eq!(body["code"], 409);
}

#[tokio::test]
async fn app_error_conflict_keeps_existing_domain_contract_without_retry_after_header() {
    let response = AppError::Conflict("duplicate widget".into()).into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert!(response.headers().get(header::RETRY_AFTER).is_none());

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["error"], "Conflict: duplicate widget");
    assert_eq!(body["code"], 409);
}

#[sqlx::test]
async fn malformed_comma_joined_idempotency_header_returns_bad_request(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let (redis, _container) = setup_redis_pool().await;

    let state = test_state_with_redis(pool, redis);
    let mut app = storm_api::app::create_app(state);

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(idempotency::IDEMPOTENCY_HEADER_NAME, "abc,def")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response.headers().get(header::RETRY_AFTER).is_none());

    let body = body_to_value(response.into_body()).await;
    assert_eq!(body["code"], 400);
    assert_eq!(
        body["error"],
        "Bad request: malformed x-idempotency-key header"
    );
}

#[sqlx::test]
async fn repeated_idempotency_header_returns_bad_request(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let (redis, _container) = setup_redis_pool().await;

    let state = test_state_with_redis(pool, redis);
    let mut app = storm_api::app::create_app(state);

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(idempotency::IDEMPOTENCY_HEADER_NAME, "key-a")
                .header(idempotency::IDEMPOTENCY_HEADER_NAME, "key-b")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response.headers().get(header::RETRY_AFTER).is_none());

    let body = body_to_value(response.into_body()).await;
    assert_eq!(body["code"], 400);
    assert_eq!(
        body["error"],
        "Bad request: malformed x-idempotency-key header"
    );
}

#[sqlx::test]
async fn processing_lock_conflict_returns_retry_after_and_processing_body(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let claims = storm_api::services::auth_service::verify_token(&config, &token).unwrap();
    let (redis, _container) = setup_redis_pool().await;

    let scope = format!("idempotency:user:{}:key:{}", claims.sub, "dup-key");
    let lock_key = format!("{scope}:lock");
    {
        let mut conn = redis.as_ref().unwrap().clone();
        conn.set_ex::<_, _, ()>(lock_key, "processing", 30)
            .await
            .unwrap();
    }

    let state = test_state_with_redis(pool, redis);
    let mut app = storm_api::app::create_app(state);

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(idempotency::IDEMPOTENCY_HEADER_NAME, "dup-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok()),
        Some("1")
    );

    let body = body_to_value(response.into_body()).await;
    assert_eq!(body["code"], 409);
    assert_eq!(body["error"], "processing");
}

async fn inject_current_user(
    mut request: axum::extract::Request,
    next: middleware::Next,
) -> axum::response::Response {
    request
        .extensions_mut()
        .insert(storm_api::models::user::CurrentUser {
            id: "user-1".into(),
            role: "user".into(),
        });
    next.run(request).await
}

#[sqlx::test]
async fn non_mutating_request_bypasses_idempotency_validation(pool: PgPool) {
    let state = test_state(pool);
    let mut app = Router::new()
        .route("/demo", get(|| async { StatusCode::NO_CONTENT }))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            idempotency::idempotency_middleware,
        ))
        .with_state(state);

    let response = app
        .call(
            Request::builder()
                .method("GET")
                .uri("/demo")
                .header(idempotency::IDEMPOTENCY_HEADER_NAME, "abc,def")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn mutating_request_without_idempotency_header_bypasses_middleware(pool: PgPool) {
    let state = test_state(pool);
    let mut app = Router::new()
        .route("/demo", post(|| async { StatusCode::CREATED }))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            idempotency::idempotency_middleware,
        ))
        .with_state(state);

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/demo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[sqlx::test]
async fn idempotency_header_without_authenticated_user_returns_unauthorized(pool: PgPool) {
    let state = test_state(pool);
    let mut app = Router::new()
        .route("/demo", post(|| async { StatusCode::OK }))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            idempotency::idempotency_middleware,
        ))
        .with_state(state);

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/demo")
                .header(idempotency::IDEMPOTENCY_HEADER_NAME, "k1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn authenticated_request_with_missing_redis_returns_service_unavailable(pool: PgPool) {
    let state = test_state(pool);
    let mut app = Router::new()
        .route("/demo", post(|| async { StatusCode::OK }))
        .route_layer(
            tower::ServiceBuilder::new()
                .layer(middleware::from_fn(inject_current_user))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    idempotency::idempotency_middleware,
                )),
        )
        .with_state(state);

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/demo")
                .header(idempotency::IDEMPOTENCY_HEADER_NAME, "k1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = body_to_value(response.into_body()).await;
    assert_eq!(body["code"], 503);
}

#[sqlx::test]
async fn successful_response_is_cached_and_replayed(pool: PgPool) {
    let (redis, _container) = setup_redis_pool().await;
    let state = test_state_with_redis(pool, redis);
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_handler = calls.clone();
    let mut app = Router::new()
        .route(
            "/demo",
            post(move || {
                let calls = calls_for_handler.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    axum::response::Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::CONTENT_LENGTH, "11")
                        .body(Body::from("{\"ok\":true}"))
                        .unwrap()
                }
            }),
        )
        .route_layer(
            tower::ServiceBuilder::new()
                .layer(middleware::from_fn(inject_current_user))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    idempotency::idempotency_middleware,
                )),
        )
        .with_state(state);

    for _ in 0..2 {
        let response = app
            .call(
                Request::builder()
                    .method("POST")
                    .uri("/demo")
                    .header(idempotency::IDEMPOTENCY_HEADER_NAME, "same-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_to_value(response.into_body()).await;
        assert_eq!(body["ok"], true);
    }

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[sqlx::test]
async fn invalid_cached_json_falls_back_to_handler(pool: PgPool) {
    let (redis, _container) = setup_redis_pool().await;
    let mut conn = redis.as_ref().unwrap().clone();
    conn.set::<_, _, ()>("idempotency:user:user-1:key:k1:response", "not-json")
        .await
        .unwrap();

    let state = test_state_with_redis(pool, redis);
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_handler = calls.clone();
    let mut app = Router::new()
        .route(
            "/demo",
            post(move || {
                let calls = calls_for_handler.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    (StatusCode::OK, [(header::CONTENT_LENGTH, "2")], "ok").into_response()
                }
            }),
        )
        .route_layer(
            tower::ServiceBuilder::new()
                .layer(middleware::from_fn(inject_current_user))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    idempotency::idempotency_middleware,
                )),
        )
        .with_state(state);

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/demo")
                .header(idempotency::IDEMPOTENCY_HEADER_NAME, "k1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[sqlx::test]
async fn failed_downstream_response_releases_lock_and_does_not_cache(pool: PgPool) {
    let (redis, _container) = setup_redis_pool().await;
    let mut inspect = redis.as_ref().unwrap().clone();
    let state = test_state_with_redis(pool, redis);
    let mut app = Router::new()
        .route(
            "/demo",
            post(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "boom").into_response() }),
        )
        .route_layer(
            tower::ServiceBuilder::new()
                .layer(middleware::from_fn(inject_current_user))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    idempotency::idempotency_middleware,
                )),
        )
        .with_state(state);

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/demo")
                .header(idempotency::IDEMPOTENCY_HEADER_NAME, "err-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let lock_exists: bool = inspect
        .exists("idempotency:user:user-1:key:err-key:lock")
        .await
        .unwrap();
    let response_exists: bool = inspect
        .exists("idempotency:user:user-1:key:err-key:response")
        .await
        .unwrap();
    assert!(!lock_exists);
    assert!(!response_exists);
}

#[sqlx::test]
async fn non_cacheable_streamed_response_releases_lock_without_storing_cache(pool: PgPool) {
    let (redis, _container) = setup_redis_pool().await;
    let mut inspect = redis.as_ref().unwrap().clone();
    let state = test_state_with_redis(pool, redis);
    let mut app = Router::new()
        .route(
            "/demo",
            post(|| async {
                axum::response::Response::builder()
                    .status(StatusCode::OK)
                    .header(header::TRANSFER_ENCODING, "chunked")
                    .body(Body::from("ok"))
                    .unwrap()
            }),
        )
        .route_layer(
            tower::ServiceBuilder::new()
                .layer(middleware::from_fn(inject_current_user))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    idempotency::idempotency_middleware,
                )),
        )
        .with_state(state);

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/demo")
                .header(idempotency::IDEMPOTENCY_HEADER_NAME, "stream-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let lock_exists: bool = inspect
        .exists("idempotency:user:user-1:key:stream-key:lock")
        .await
        .unwrap();
    let response_exists: bool = inspect
        .exists("idempotency:user:user-1:key:stream-key:response")
        .await
        .unwrap();
    assert!(!lock_exists);
    assert!(!response_exists);
}

#[sqlx::test]
async fn oversized_response_body_skips_replay_cache(pool: PgPool) {
    let (redis, _container) = setup_redis_pool().await;
    let mut inspect = redis.as_ref().unwrap().clone();
    let state = test_state_with_redis(pool, redis);
    let large_body = "a".repeat(idempotency::IDEMPOTENCY_BODY_CAP_BYTES + 1);
    let mut app = Router::new()
        .route(
            "/demo",
            post(move || {
                let body = large_body.clone();
                async move {
                    axum::response::Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_LENGTH, "1")
                        .body(Body::from(body))
                        .unwrap()
                }
            }),
        )
        .route_layer(
            tower::ServiceBuilder::new()
                .layer(middleware::from_fn(inject_current_user))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    idempotency::idempotency_middleware,
                )),
        )
        .with_state(state);

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/demo")
                .header(idempotency::IDEMPOTENCY_HEADER_NAME, "large-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.len(), idempotency::IDEMPOTENCY_BODY_CAP_BYTES + 1);

    let response_exists: bool = inspect
        .exists("idempotency:user:user-1:key:large-key:response")
        .await
        .unwrap();
    assert!(!response_exists);
}

#[sqlx::test]
async fn body_read_over_limit_returns_service_unavailable_and_releases_lock(pool: PgPool) {
    let (redis, _container) = setup_redis_pool().await;
    let mut inspect = redis.as_ref().unwrap().clone();
    let state = test_state_with_redis(pool, redis);
    let huge_body = "a".repeat(idempotency::IDEMPOTENCY_BODY_CAP_BYTES + 2);
    let mut app = Router::new()
        .route(
            "/demo",
            post(move || {
                let body = huge_body.clone();
                async move {
                    axum::response::Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_LENGTH, "1")
                        .body(Body::from(body))
                        .unwrap()
                }
            }),
        )
        .route_layer(
            tower::ServiceBuilder::new()
                .layer(middleware::from_fn(inject_current_user))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    idempotency::idempotency_middleware,
                )),
        )
        .with_state(state);

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/demo")
                .header(idempotency::IDEMPOTENCY_HEADER_NAME, "too-large-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let lock_exists: bool = inspect
        .exists("idempotency:user:user-1:key:too-large-key:lock")
        .await
        .unwrap();
    let response_exists: bool = inspect
        .exists("idempotency:user:user-1:key:too-large-key:response")
        .await
        .unwrap();
    assert!(!lock_exists);
    assert!(!response_exists);
}

#[sqlx::test]
async fn redis_get_error_returns_service_unavailable(pool: PgPool) {
    let (redis, container) = setup_redis_pool().await;
    let state = test_state_with_redis(pool, redis);
    drop(container);

    let mut app = Router::new()
        .route("/demo", post(|| async { StatusCode::OK }))
        .route_layer(
            tower::ServiceBuilder::new()
                .layer(middleware::from_fn(inject_current_user))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    idempotency::idempotency_middleware,
                )),
        )
        .with_state(state);

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/demo")
                .header(idempotency::IDEMPOTENCY_HEADER_NAME, "dead-redis")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[sqlx::test]
async fn redis_set_error_returns_service_unavailable(pool: PgPool) {
    let (redis, _container) = setup_redis_pool().await;
    let mut conn = redis.as_ref().unwrap().clone();
    redis::cmd("REPLICAOF")
        .arg("127.0.0.1")
        .arg("1")
        .query_async::<redis::Value>(&mut conn)
        .await
        .unwrap();

    let state = test_state_with_redis(pool, redis);
    let mut app = Router::new()
        .route("/demo", post(|| async { StatusCode::OK }))
        .route_layer(
            tower::ServiceBuilder::new()
                .layer(middleware::from_fn(inject_current_user))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    idempotency::idempotency_middleware,
                )),
        )
        .with_state(state);

    let response = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/demo")
                .header(idempotency::IDEMPOTENCY_HEADER_NAME, "readonly-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
