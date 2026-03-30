//! Consumption handlers: list, list-by-client, and create.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::PgPool;

use crate::{
    errors::AppError,
    models::consumption::{Consumption, CreateConsumptionRequest},
    services::consumption_service,
};

/// `GET /api/v1/consumptions`
///
/// Lists all fuel consumption records.
pub async fn list_consumptions(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Consumption>>, AppError> {
    let consumptions = consumption_service::list(&pool).await?;
    Ok(Json(consumptions))
}

/// `GET /api/v1/consumptions/by-client/{client_ref}`
///
/// Lists consumptions for a specific client.
pub async fn list_by_client(
    State(pool): State<PgPool>,
    Path(client_ref): Path<String>,
) -> Result<Json<Vec<Consumption>>, AppError> {
    let consumptions = consumption_service::list_by_client(&pool, &client_ref).await?;
    Ok(Json(consumptions))
}

/// `POST /api/v1/consumptions`
///
/// Records a new fuel consumption event. Returns `201 Created`.
pub async fn create(
    State(pool): State<PgPool>,
    Json(input): Json<CreateConsumptionRequest>,
) -> Result<StatusCode, AppError> {
    consumption_service::create(&pool, &input).await?;
    Ok(StatusCode::CREATED)
}
