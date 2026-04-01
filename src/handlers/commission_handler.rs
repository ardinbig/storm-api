//! Commission rate handlers: list, get-current, and create.

use axum::{Json, extract::State, http::StatusCode};
use sqlx::PgPool;

use crate::{
    errors::{AppError, ErrorResponse},
    models::commission::{Commission, CreateCommissionRequest},
    services::commission_service,
};

/// `GET /api/v1/commissions`
///
/// Lists all commission rate records.
#[utoipa::path(
    get,
    path = "/api/v1/commissions",
    tag = "Commissions",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "List of commission rates", body = Vec<Commission>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
)]
pub async fn list_commissions(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Commission>>, AppError> {
    let commissions = commission_service::list(&pool).await?;
    Ok(Json(commissions))
}

/// `GET /api/v1/commissions/current`
///
/// Retrieves the currently active commission rate.
#[utoipa::path(
    get,
    path = "/api/v1/commissions/current",
    tag = "Commissions",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Current commission rate", body = Commission),
        (status = 404, description = "No commission rate set", body = ErrorResponse),
    ),
)]
pub async fn get_current(State(pool): State<PgPool>) -> Result<Json<Commission>, AppError> {
    let commission = commission_service::get_current(&pool).await?;
    Ok(Json(commission))
}

/// `POST /api/v1/commissions`
///
/// Creates a new commission rate. Returns `201 Created`.
#[utoipa::path(
    post,
    path = "/api/v1/commissions",
    tag = "Commissions",
    security(("bearer" = [])),
    request_body = CreateCommissionRequest,
    responses(
        (status = 201, description = "Commission rate created", body = Commission),
    ),
)]
pub async fn create_commission(
    State(pool): State<PgPool>,
    Json(input): Json<CreateCommissionRequest>,
) -> Result<(StatusCode, Json<Commission>), AppError> {
    let commission = commission_service::create(&pool, &input).await?;
    Ok((StatusCode::CREATED, Json(commission)))
}
