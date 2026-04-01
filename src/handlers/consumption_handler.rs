//! Consumption handlers: list, list-by-client, and create.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::PgPool;

use crate::{
    errors::{AppError, ErrorResponse},
    models::consumption::{Consumption, CreateConsumptionRequest},
    services::consumption_service,
};

/// `GET /api/v1/consumptions`
///
/// Lists all fuel consumption records.
#[utoipa::path(
    get,
    path = "/api/v1/consumptions",
    tag = "Consumptions",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "List of consumptions", body = Vec<Consumption>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
)]
pub async fn list_consumptions(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Consumption>>, AppError> {
    let consumptions = consumption_service::list(&pool).await?;
    Ok(Json(consumptions))
}

/// `GET /api/v1/consumptions/by-client/{client_ref}`
///
/// Lists consumptions for a specific client.
#[utoipa::path(
    get,
    path = "/api/v1/consumptions/by-client/{client_ref}",
    tag = "Consumptions",
    security(("bearer" = [])),
    params(
        ("client_ref" = String, Path, description = "Client reference code"),
    ),
    responses(
        (status = 200, description = "Client consumptions", body = Vec<Consumption>),
    ),
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/consumptions",
    tag = "Consumptions",
    security(("bearer" = [])),
    request_body = CreateConsumptionRequest,
    responses(
        (status = 201, description = "Consumption recorded", body = ()),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
)]
pub async fn create(
    State(pool): State<PgPool>,
    Json(input): Json<CreateConsumptionRequest>,
) -> Result<StatusCode, AppError> {
    consumption_service::create(&pool, &input).await?;
    Ok(StatusCode::CREATED)
}
