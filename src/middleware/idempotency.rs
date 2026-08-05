//! Idempotency middleware primitives.

use std::borrow::Cow;

use axum::{
    Json,
    body::{Body, to_bytes},
    extract::{MatchedPath, Request, State},
    http::{HeaderName, HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::{
    errors::{AppError, ErrorResponse},
    models::user::CurrentUser,
    state::app_state::RedisPool,
};

/// Static fallback marker used when [`MatchedPath`] is unavailable.
///
/// The `{METHOD}` token is replaced at runtime with the request method.
pub const NO_MATCHED_PATH_FALLBACK_MARKER: &str = "{METHOD}::NO_MATCHED_PATH";

/// Number of seconds a processing conflict should ask clients to wait.
pub const PROCESSING_RETRY_AFTER_SECONDS: &str = "1";

/// Maximum successful response body size cached for idempotent replay.
pub const IDEMPOTENCY_BODY_CAP_BYTES: usize = 1_048_576;

/// TTL for cached successful responses.
pub const IDEMPOTENCY_CACHE_TTL_SECS: u64 = 24 * 60 * 60;

/// TTL for in-flight processing lock keys.
pub const IDEMPOTENCY_LOCK_TTL_SECS: u64 = 30;

/// Canonical request header name for idempotency keys.
pub const IDEMPOTENCY_HEADER_NAME: &str = "x-idempotency-key";

const MALFORMED_IDEMPOTENCY_HEADER_MESSAGE: &str = "malformed x-idempotency-key header";

/// Stable idempotency telemetry event names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdempotencyEvent {
    /// Request header was malformed.
    MalformedHeader,
    /// A cached response was used.
    CacheHit,
    /// No cached response existed.
    CacheMiss,
    /// A processing lock was acquired.
    LockAcquired,
    /// Another request is currently processing the same idempotency key.
    LockConflict,
    /// A successful response was cached for replay.
    ReplayStored,
    /// Caching was skipped for the response.
    ReplaySkipped,
    /// Redis was unavailable for an idempotency operation.
    StorageUnavailable,
}

impl IdempotencyEvent {
    /// Returns the canonical string representation used in structured logs.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MalformedHeader => "malformed_header",
            Self::CacheHit => "cache_hit",
            Self::CacheMiss => "cache_miss",
            Self::LockAcquired => "lock_acquired",
            Self::LockConflict => "lock_conflict",
            Self::ReplayStored => "replay_stored",
            Self::ReplaySkipped => "replay_skipped",
            Self::StorageUnavailable => "storage_unavailable",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedHeader {
    name: String,
    value: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedResponse {
    status: u16,
    headers: Vec<CachedHeader>,
    body: Vec<u8>,
}

/// Canonical idempotency telemetry payload.
///
/// The structured log layout is intentionally stable:
/// `event`, `reason`, `path`, and optional `user_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyTelemetry<'a> {
    /// Canonical event name.
    pub event: IdempotencyEvent,
    /// Human-readable reason string.
    pub reason: Cow<'a, str>,
    /// Matched route pattern or fallback marker.
    pub path: Cow<'a, str>,
    /// Optional authenticated user id.
    pub user_id: Option<Cow<'a, str>>,
}

impl<'a> IdempotencyTelemetry<'a> {
    /// Creates a new telemetry payload.
    pub fn new<U>(
        event: IdempotencyEvent,
        reason: impl Into<Cow<'a, str>>,
        path: impl Into<Cow<'a, str>>,
        user_id: Option<U>,
    ) -> Self
    where
        U: Into<Cow<'a, str>>,
    {
        Self {
            event,
            reason: reason.into(),
            path: path.into(),
            user_id: user_id.map(Into::into),
        }
    }

    /// Returns the canonical field names used when emitting structured logs.
    pub fn canonical_field_names(&self) -> Vec<&'static str> {
        if self.user_id.is_some() {
            vec!["event", "reason", "path", "user_id"]
        } else {
            vec!["event", "reason", "path"]
        }
    }
}

/// Returns the matched route path when available, otherwise formats the
/// fallback marker using the HTTP method.
pub fn path_label(method: &str, matched_path: Option<&str>) -> String {
    match matched_path {
        Some(path) => path.to_owned(),
        None => NO_MATCHED_PATH_FALLBACK_MARKER.replace("{METHOD}", method),
    }
}

/// Emits a single idempotency telemetry event using a stable structured layout.
///
/// The `user_id` field is omitted entirely when unavailable.
pub fn emit_idempotency_telemetry(telemetry: &IdempotencyTelemetry<'_>) {
    match telemetry.event {
        IdempotencyEvent::MalformedHeader => {
            if let Some(user_id) = telemetry.user_id.as_ref() {
                emit_warn_with_user(telemetry, user_id);
            } else {
                emit_warn_without_user(telemetry);
            }
        }
        _ => {
            if let Some(user_id) = telemetry.user_id.as_ref() {
                emit_debug_with_user(telemetry, user_id);
            } else {
                emit_debug_without_user(telemetry);
            }
        }
    }
}

fn emit_warn_with_user(telemetry: &IdempotencyTelemetry<'_>, user_id: &str) {
    let event = telemetry.event.as_str();
    tracing::warn!(
        event = event,
        reason = %telemetry.reason,
        path = %telemetry.path,
        user_id = %user_id,
        "idempotency telemetry"
    );
}

fn emit_warn_without_user(telemetry: &IdempotencyTelemetry<'_>) {
    let event = telemetry.event.as_str();
    tracing::warn!(
        event = event,
        reason = %telemetry.reason,
        path = %telemetry.path,
        "idempotency telemetry"
    );
}

fn emit_debug_with_user(telemetry: &IdempotencyTelemetry<'_>, user_id: &str) {
    let event = telemetry.event.as_str();
    tracing::debug!(
        event = event,
        reason = %telemetry.reason,
        path = %telemetry.path,
        user_id = %user_id,
        "idempotency telemetry"
    );
}

fn emit_debug_without_user(telemetry: &IdempotencyTelemetry<'_>) {
    let event = telemetry.event.as_str();
    tracing::debug!(
        event = event,
        reason = %telemetry.reason,
        path = %telemetry.path,
        "idempotency telemetry"
    );
}

fn idempotency_header_name() -> HeaderName {
    HeaderName::from_static(IDEMPOTENCY_HEADER_NAME)
}

fn malformed_idempotency_header_error() -> AppError {
    AppError::BadRequest(MALFORMED_IDEMPOTENCY_HEADER_MESSAGE.into())
}

fn parse_idempotency_key(headers: &axum::http::HeaderMap) -> Result<Option<String>, AppError> {
    let values = headers.get_all(idempotency_header_name());
    let mut iter = values.iter();
    let Some(first) = iter.next() else {
        return Ok(None);
    };

    if iter.next().is_some() {
        return Err(malformed_idempotency_header_error());
    }

    let value = first
        .to_str()
        .map_err(|_| malformed_idempotency_header_error())?;

    if value.contains(',') {
        return Err(malformed_idempotency_header_error());
    }

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(malformed_idempotency_header_error());
    }

    Ok(Some(trimmed.to_owned()))
}

fn scope_key(user_id: &str, idempotency_key: &str) -> String {
    format!("idempotency:user:{user_id}:key:{idempotency_key}")
}

fn lock_key(scope: &str) -> String {
    format!("{scope}:lock")
}

fn response_key(scope: &str) -> String {
    format!("{scope}:response")
}

fn cacheable_success_response(response: &Response) -> bool {
    if response.headers().contains_key(header::TRANSFER_ENCODING) {
        return false;
    }

    let Some(content_length) = response.headers().get(header::CONTENT_LENGTH) else {
        return false;
    };

    let Ok(length) = content_length.to_str() else {
        return false;
    };
    let Ok(length) = length.parse::<usize>() else {
        return false;
    };

    length <= IDEMPOTENCY_BODY_CAP_BYTES
}

fn cached_response_to_http(cached: CachedResponse) -> Response {
    let status = StatusCode::from_u16(cached.status).unwrap_or(StatusCode::OK);
    let mut response = Response::new(Body::from(cached.body));
    *response.status_mut() = status;

    for h in cached.headers {
        let Ok(name) = HeaderName::from_bytes(h.name.as_bytes()) else {
            continue;
        };
        let Ok(value) = HeaderValue::from_bytes(&h.value) else {
            continue;
        };
        response.headers_mut().append(name, value);
    }

    response
}

fn unavailable_idempotency_response() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "service unavailable: idempotency storage unavailable".to_owned(),
            code: StatusCode::SERVICE_UNAVAILABLE.as_u16(),
        }),
    )
        .into_response()
}

fn emit_user_telemetry(event: IdempotencyEvent, reason: &'static str, path: &str, user_id: &str) {
    emit_idempotency_telemetry(&IdempotencyTelemetry::new(
        event,
        reason,
        path,
        Some(user_id),
    ));
}

fn storage_unavailable_response(reason: &'static str, path: &str, user_id: &str) -> Response {
    emit_user_telemetry(IdempotencyEvent::StorageUnavailable, reason, path, user_id);
    unavailable_idempotency_response()
}

/// Idempotency middleware for authenticated mutating endpoints.
///
/// Behavior:
/// - non-mutating methods bypass idempotency logic
/// - missing idempotency header bypasses idempotency logic
/// - malformed idempotency header returns `400`
/// - duplicate in-flight request returns `409` + `Retry-After: 1` + `processing`
/// - successful cacheable response (`2xx`) is replay-cached for 24h
/// - client/server errors (`4xx`/`5xx`) unlock immediately for safe retries
pub async fn idempotency_middleware(
    State(redis): State<RedisPool>,
    request: Request,
    next: Next,
) -> Response {
    if !matches!(
        *request.method(),
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    ) {
        return next.run(request).await;
    }

    let path = path_label(
        request.method().as_str(),
        request
            .extensions()
            .get::<MatchedPath>()
            .map(MatchedPath::as_str),
    );
    let user_id = request
        .extensions()
        .get::<CurrentUser>()
        .map(|u| u.id.clone());

    let idempotency_key = match parse_idempotency_key(request.headers()) {
        Ok(Some(key)) => key,
        Ok(None) => return next.run(request).await,
        Err(err) => {
            emit_idempotency_telemetry(&IdempotencyTelemetry::new(
                IdempotencyEvent::MalformedHeader,
                "invalid idempotency key header format",
                path.as_str(),
                user_id.as_deref(),
            ));
            return err.into_response();
        }
    };

    let Some(user_id) = user_id else {
        return AppError::Unauthorized.into_response();
    };

    let Some(redis) = redis.as_ref() else {
        return storage_unavailable_response("redis pool unavailable", path.as_str(), &user_id);
    };

    let scope = scope_key(&user_id, &idempotency_key);
    let lock = lock_key(&scope);
    let response_cache = response_key(&scope);

    let mut conn = redis.clone();
    let cached_json: Option<String> = match conn.get(&response_cache).await {
        Ok(cached) => cached,
        Err(_) => {
            return storage_unavailable_response(
                "failed reading idempotency cache",
                path.as_str(),
                &user_id,
            );
        }
    };

    if let Some(cached_json) = cached_json
        && let Ok(cached) = serde_json::from_str::<CachedResponse>(&cached_json)
    {
        emit_user_telemetry(
            IdempotencyEvent::CacheHit,
            "served idempotency cache hit",
            path.as_str(),
            &user_id,
        );
        return cached_response_to_http(cached);
    }
    emit_user_telemetry(
        IdempotencyEvent::CacheMiss,
        "idempotency cache miss",
        path.as_str(),
        &user_id,
    );

    let acquired: Option<String> = match redis::cmd("SET")
        .arg(&lock)
        .arg("processing")
        .arg("NX")
        .arg("EX")
        .arg(IDEMPOTENCY_LOCK_TTL_SECS)
        .query_async(&mut conn)
        .await
    {
        Ok(res) => res,
        Err(_) => {
            return storage_unavailable_response(
                "failed acquiring idempotency lock",
                path.as_str(),
                &user_id,
            );
        }
    };

    if acquired.is_none() {
        emit_user_telemetry(
            IdempotencyEvent::LockConflict,
            "processing",
            path.as_str(),
            &user_id,
        );
        return processing_conflict_response();
    }

    emit_user_telemetry(
        IdempotencyEvent::LockAcquired,
        "acquired idempotency lock",
        path.as_str(),
        &user_id,
    );

    let response = next.run(request).await;
    if !response.status().is_success() {
        let _ = conn.del::<_, ()>(&lock).await;
        return response;
    }

    if !cacheable_success_response(&response) {
        emit_user_telemetry(
            IdempotencyEvent::ReplaySkipped,
            "response is streamed or exceeds cache body cap",
            path.as_str(),
            &user_id,
        );
        let _ = conn.del::<_, ()>(&lock).await;
        return response;
    }

    let status = response.status();
    let headers = response.headers().clone();
    let (parts, body) = response.into_parts();
    let body_bytes = match to_bytes(body, IDEMPOTENCY_BODY_CAP_BYTES + 1).await {
        Ok(bytes) => bytes,
        Err(_) => {
            let _ = conn.del::<_, ()>(&lock).await;
            return unavailable_idempotency_response();
        }
    };

    if body_bytes.len() > IDEMPOTENCY_BODY_CAP_BYTES {
        emit_user_telemetry(
            IdempotencyEvent::ReplaySkipped,
            "response body exceeded idempotency cache cap",
            path.as_str(),
            &user_id,
        );
        let _ = conn.del::<_, ()>(&lock).await;
        return Response::from_parts(parts, Body::from(body_bytes));
    }

    let cached = CachedResponse {
        status: status.as_u16(),
        headers: headers
            .iter()
            .map(|(name, value)| CachedHeader {
                name: name.to_string(),
                value: value.as_bytes().to_vec(),
            })
            .collect(),
        body: body_bytes.clone().to_vec(),
    };

    if let Ok(payload) = serde_json::to_string(&cached)
        && conn
            .set_ex::<_, _, ()>(&response_cache, payload, IDEMPOTENCY_CACHE_TTL_SECS)
            .await
            .is_ok()
    {
        emit_user_telemetry(
            IdempotencyEvent::ReplayStored,
            "stored idempotency replay response",
            path.as_str(),
            &user_id,
        );
    }

    let _ = conn.del::<_, ()>(&lock).await;
    Response::from_parts(parts, Body::from(body_bytes))
}

/// Builds the standard idempotency processing-conflict response.
///
/// The body follows the app's JSON error contract and the response includes
/// `Retry-After: 1`.
pub fn processing_conflict_response() -> Response {
    (
        StatusCode::CONFLICT,
        [(
            header::RETRY_AFTER,
            HeaderValue::from_static(PROCESSING_RETRY_AFTER_SECONDS),
        )],
        Json(ErrorResponse {
            error: "processing".to_owned(),
            code: StatusCode::CONFLICT.as_u16(),
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    fn with_debug_tracing() -> tracing::subscriber::DefaultGuard {
        let subscriber = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        tracing::subscriber::set_default(subscriber)
    }

    #[test]
    fn idempotency_event_as_str_covers_all_variants() {
        assert_eq!(
            IdempotencyEvent::MalformedHeader.as_str(),
            "malformed_header"
        );
        assert_eq!(IdempotencyEvent::CacheHit.as_str(), "cache_hit");
        assert_eq!(IdempotencyEvent::CacheMiss.as_str(), "cache_miss");
        assert_eq!(IdempotencyEvent::LockAcquired.as_str(), "lock_acquired");
        assert_eq!(IdempotencyEvent::LockConflict.as_str(), "lock_conflict");
        assert_eq!(IdempotencyEvent::ReplayStored.as_str(), "replay_stored");
        assert_eq!(IdempotencyEvent::ReplaySkipped.as_str(), "replay_skipped");
        assert_eq!(
            IdempotencyEvent::StorageUnavailable.as_str(),
            "storage_unavailable"
        );
    }

    #[test]
    fn parse_idempotency_key_accepts_valid_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            idempotency_header_name(),
            HeaderValue::from_static("  abc-123  "),
        );

        let key = parse_idempotency_key(&headers).unwrap();
        assert_eq!(key.as_deref(), Some("abc-123"));
    }

    #[test]
    fn parse_idempotency_key_missing_header_returns_none() {
        let headers = HeaderMap::new();
        assert!(parse_idempotency_key(&headers).unwrap().is_none());
    }

    #[test]
    fn parse_idempotency_key_rejects_repeated_header() {
        let mut headers = HeaderMap::new();
        headers.append(idempotency_header_name(), HeaderValue::from_static("a"));
        headers.append(idempotency_header_name(), HeaderValue::from_static("b"));

        assert!(parse_idempotency_key(&headers).is_err());
    }

    #[test]
    fn parse_idempotency_key_rejects_invalid_utf8() {
        let mut headers = HeaderMap::new();
        headers.insert(
            idempotency_header_name(),
            HeaderValue::from_bytes(&[0xff]).unwrap(),
        );

        assert!(parse_idempotency_key(&headers).is_err());
    }

    #[test]
    fn parse_idempotency_key_rejects_comma_joined_value() {
        let mut headers = HeaderMap::new();
        headers.insert(
            idempotency_header_name(),
            HeaderValue::from_static("abc,def"),
        );

        assert!(parse_idempotency_key(&headers).is_err());
    }

    #[test]
    fn parse_idempotency_key_rejects_empty_value() {
        let mut headers = HeaderMap::new();
        headers.insert(idempotency_header_name(), HeaderValue::from_static("   "));

        assert!(parse_idempotency_key(&headers).is_err());
    }

    #[test]
    fn scope_related_keys_are_stable() {
        let scope = scope_key("u1", "k1");
        assert_eq!(scope, "idempotency:user:u1:key:k1");
        assert_eq!(lock_key(&scope), "idempotency:user:u1:key:k1:lock");
        assert_eq!(response_key(&scope), "idempotency:user:u1:key:k1:response");
    }

    #[test]
    fn cacheable_success_response_requires_content_length_and_no_transfer_encoding() {
        let mut streamed = Response::new(Body::from("ok"));
        streamed.headers_mut().insert(
            header::TRANSFER_ENCODING,
            HeaderValue::from_static("chunked"),
        );
        assert!(!cacheable_success_response(&streamed));

        let no_len = Response::new(Body::from("ok"));
        assert!(!cacheable_success_response(&no_len));

        let mut invalid_len = Response::new(Body::from("ok"));
        invalid_len
            .headers_mut()
            .insert(header::CONTENT_LENGTH, HeaderValue::from_static("bad"));
        assert!(!cacheable_success_response(&invalid_len));

        let mut non_utf8_len = Response::new(Body::from("ok"));
        non_utf8_len.headers_mut().insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_bytes(&[0xff]).unwrap(),
        );
        assert!(!cacheable_success_response(&non_utf8_len));

        let mut too_large = Response::new(Body::from("ok"));
        too_large.headers_mut().insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&(IDEMPOTENCY_BODY_CAP_BYTES + 1).to_string()).unwrap(),
        );
        assert!(!cacheable_success_response(&too_large));

        let mut ok = Response::new(Body::from("ok"));
        ok.headers_mut()
            .insert(header::CONTENT_LENGTH, HeaderValue::from_static("2"));
        assert!(cacheable_success_response(&ok));
    }

    #[tokio::test]
    async fn cached_response_to_http_ignores_invalid_headers() {
        let cached = CachedResponse {
            status: StatusCode::CREATED.as_u16(),
            headers: vec![
                CachedHeader {
                    name: "content-type".into(),
                    value: b"text/plain".to_vec(),
                },
                CachedHeader {
                    name: "bad header".into(),
                    value: b"ignored".to_vec(),
                },
                CachedHeader {
                    name: "x-bad-value".into(),
                    value: b"bad\nvalue".to_vec(),
                },
            ],
            body: b"cached".to_vec(),
        };

        let response = cached_response_to_http(cached);
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("text/plain")
        );
        assert!(response.headers().get("bad header").is_none());
        assert!(response.headers().get("x-bad-value").is_none());

        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], b"cached");
    }

    #[tokio::test]
    async fn unavailable_idempotency_response_has_service_unavailable_contract() {
        let response = unavailable_idempotency_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["code"], 503);
        assert_eq!(
            body["error"],
            "service unavailable: idempotency storage unavailable"
        );
    }

    #[test]
    fn emit_idempotency_telemetry_handles_all_layouts() {
        let _guard = with_debug_tracing();

        let malformed_with_user = IdempotencyTelemetry::new(
            IdempotencyEvent::MalformedHeader,
            "bad header",
            "/p",
            Some("u1"),
        );
        let malformed_without_user = IdempotencyTelemetry::new(
            IdempotencyEvent::MalformedHeader,
            "bad header",
            "/p",
            None::<&str>,
        );
        let cache_hit_with_user =
            IdempotencyTelemetry::new(IdempotencyEvent::CacheHit, "hit", "/p", Some("u1"));
        let cache_hit_without_user =
            IdempotencyTelemetry::new(IdempotencyEvent::CacheHit, "hit", "/p", None::<&str>);

        emit_idempotency_telemetry(&malformed_with_user);
        emit_idempotency_telemetry(&malformed_without_user);
        emit_idempotency_telemetry(&cache_hit_with_user);
        emit_idempotency_telemetry(&cache_hit_without_user);
    }
}
