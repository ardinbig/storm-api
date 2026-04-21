//! Commission rate handlers: list, get-current, and create.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::PgPool;
use uuid::Uuid;

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

/// `DELETE /api/v1/commissions/{id}`
///
/// Deletes a commission rate. Returns `204 No Content`.
#[utoipa::path(
    delete,
    path = "/api/v1/commissions/{id}",
    tag = "Commissions",
    security(("bearer" = [])),
    params(
        ("id" = Uuid, Path, description = "Commission UUID"),
    ),
    responses(
        (status = 204, description = "Commission deleted"),
        (status = 400, description = "Cannot delete the last remaining commission", body = ErrorResponse),
        (status = 404, description = "Commission not found", body = ErrorResponse),
    ),
)]
pub async fn delete_commission(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    commission_service::delete(&pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
