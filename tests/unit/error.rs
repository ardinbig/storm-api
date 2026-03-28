use axum::{http::StatusCode, response::IntoResponse};
use http_body_util::BodyExt;
use storm_api::errors::AppError;

#[test]
fn error_display_messages() {
    assert_eq!(AppError::NotFound("x".into()).to_string(), "Not found: x");
    assert_eq!(
        AppError::BadRequest("y".into()).to_string(),
        "Bad request: y"
    );
    assert_eq!(AppError::Unauthorized.to_string(), "Unauthorized");
    assert_eq!(AppError::Conflict("z".into()).to_string(), "Conflict: z");
    assert_eq!(AppError::Internal.to_string(), "Internal server error");
}

#[tokio::test]
async fn error_into_response_status_codes() {
    let cases: Vec<(AppError, StatusCode)> = vec![
        (AppError::NotFound("x".into()), StatusCode::NOT_FOUND),
        (AppError::BadRequest("y".into()), StatusCode::BAD_REQUEST),
        (AppError::Unauthorized, StatusCode::UNAUTHORIZED),
        (AppError::Conflict("z".into()), StatusCode::CONFLICT),
        (AppError::Internal, StatusCode::INTERNAL_SERVER_ERROR),
    ];

    for (err, expected) in cases {
        let resp = err.into_response();
        assert_eq!(resp.status(), expected);
    }
}

#[tokio::test]
async fn error_json_response_body_contains_error_and_code() {
    let cases: Vec<(AppError, u16, &str)> = vec![
        (
            AppError::NotFound("widget".into()),
            404,
            "Not found: widget",
        ),
        (
            AppError::BadRequest("bad input".into()),
            400,
            "Bad request: bad input",
        ),
        (AppError::Unauthorized, 401, "Unauthorized"),
        (
            AppError::Conflict("duplicate".into()),
            409,
            "Conflict: duplicate",
        ),
        (AppError::Internal, 500, "Internal server error"),
    ];

    for (err, expected_code, expected_msg) in cases {
        let resp = err.into_response();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["code"], expected_code);
        assert_eq!(body["error"], expected_msg);
    }
}

#[tokio::test]
async fn database_error_hides_internal_details() {
    let pool_result = sqlx::PgPool::connect("postgres://invalid:invalid@localhost:1/none").await;
    if let Err(e) = pool_result {
        let app_err: AppError = e.into();
        assert_eq!(app_err.to_string(), "Database error");
        let resp = app_err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"], "Database error");
        assert_eq!(body["code"], 500);
    }
}

#[test]
fn from_jsonwebtoken_error() {
    let err = jsonwebtoken::decode::<serde_json::Value>(
        "bad",
        &jsonwebtoken::DecodingKey::from_secret(b"s"),
        &jsonwebtoken::Validation::default(),
    )
    .unwrap_err();
    let app_err: AppError = err.into();
    assert!(matches!(app_err, AppError::Unauthorized));
}

#[tokio::test]
async fn cache_error_maps_to_internal_server_error() {
    let redis_err = redis::RedisError::from((redis::ErrorKind::Io, "connection lost"));
    let app_err: AppError = redis_err.into();
    assert_eq!(app_err.to_string(), "Cache error");
    let resp = app_err.into_response();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"], "Cache error");
    assert_eq!(body["code"], 500);
}
