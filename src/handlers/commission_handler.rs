//! Commission rate handlers: list, get-current, and create.

use axum::{Json, extract::State, http::StatusCode};
use sqlx::PgPool;

use crate::{
    errors::AppError,
    models::commission::{Commission, CreateCommissionRequest},
    services::commission_service,
};

/// `GET /api/v1/commissions`
///
/// Lists all commission rate records.
pub async fn list_commissions(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Commission>>, AppError> {
    let commissions = commission_service::list(&pool).await?;
    Ok(Json(commissions))
}

/// `GET /api/v1/commissions/current`
///
/// Retrieves the currently active commission rate.
pub async fn get_current(State(pool): State<PgPool>) -> Result<Json<Commission>, AppError> {
    let commission = commission_service::get_current(&pool).await?;
    Ok(Json(commission))
}

/// `POST /api/v1/commissions`
///
/// Creates a new commission rate. Returns `201 Created`.
pub async fn create_commission(
    State(pool): State<PgPool>,
    Json(input): Json<CreateCommissionRequest>,
) -> Result<(StatusCode, Json<Commission>), AppError> {
    let commission = commission_service::create(&pool, &input).await?;
    Ok((StatusCode::CREATED, Json(commission)))
}
