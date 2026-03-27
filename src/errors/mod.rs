//! Unified error handling for the Storm API.
//!
//! All handlers return `Result<_, AppError>`.  [`AppError`] implements Axum's
//! [`IntoResponse`] so that every error variant is automatically serialized as
//! a JSON body of the form:
//!
//! ```json
//! { "error": "message", "code": 400 }
//! ```
//!
//! Automatic conversions are provided via [`From`]:
//!
//! - [`sqlx::Error`] -> [`AppError::Database`]
//! - [`jsonwebtoken::errors::Error`] -> [`AppError::Unauthorized`]

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

/// Application-wide error type.
///
/// Each variant maps to a specific HTTP status code (see
/// [`AppError::status_code`]). The [`Display`](std::fmt::Display)
/// implementation (via `thiserror`) provides the readable message
/// included in the JSON response body.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The requested resource does not exist. -> `404 Not Found`
    #[error("Not found: {0}")]
    NotFound(String),

    /// The request is semantically invalid. -> `400 Bad Request`
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Authentication or authorization failure. -> `401 Unauthorized`
    #[error("Unauthorized")]
    Unauthorized,

    /// A uniqueness or integrity constraint was violated. -> `409 Conflict`
    #[error("Conflict: {0}")]
    Conflict(String),

    /// A database query failed. -> `500 Internal Server Error`
    ///
    /// The inner [`sqlx::Error`] is auto-converted via the derived [`From`]
    /// impl.
    #[error("Database error")]
    Database(#[from] sqlx::Error),

    /// A Redis cache operation failed. -> `500 Internal Server Error`
    ///
    /// The inner [`redis::RedisError`] is auto-converted via the derived
    /// [`From`] impl.
    #[error("Cache error")]
    Cache(#[from] redis::RedisError),

    /// Catch-all for unexpected failures. -> `500 Internal Server Error`
    #[error("Internal server error")]
    Internal,
}

impl AppError {
    /// Maps each variant to the corresponding HTTP status code.
    const fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Database(_) | Self::Cache(_) | Self::Internal => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

/// Wire-format for JSON error responses.
#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Readable error description.
    error: String,
    /// Numeric HTTP status code echoed in the body.
    code: u16,
}

/// Converts an [`AppError`] into an Axum [`Response`] containing a JSON
/// [`ErrorResponse`] body and the appropriate HTTP status code.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ErrorResponse {
            error: self.to_string(),
            code: status.as_u16(),
        };
        (status, Json(body)).into_response()
    }
}

/// Any JWT error is treated as an authentication failure.
impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(_: jsonwebtoken::errors::Error) -> Self {
        Self::Unauthorized
    }
}
