//! Consumption handlers: paginated list, deprecated by-client list, and create.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use sqlx::PgPool;

use crate::{
    errors::{AppError, ErrorResponse},
    models::{
        consumption::{Consumption, CreateConsumptionRequest},
        pagination::{ConsumptionQuery, PaginatedConsumptionResponse},
    },
    services::consumption_service,
};

/// `GET /api/v1/consumptions`
///
/// Returns a paginated list of fuel consumption records, ordered
/// most-recent-first.  Supports optional filtering by the operator's
/// `agent_ref` and by `station_id`.
#[utoipa::path(
    get,
    path = "/api/v1/consumptions",
    tag = "Consumptions",
    security(("bearer" = [])),
    params(ConsumptionQuery),
    responses(
        (status = 200, description = "Paginated list of consumptions",
         body = PaginatedConsumptionResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
)]
pub async fn list_consumptions(
    State(pool): State<PgPool>,
    Query(query): Query<ConsumptionQuery>,
) -> Result<Json<PaginatedConsumptionResponse>, AppError> {
    let result = consumption_service::list_paginated(&pool, &query).await?;
    Ok(Json(result))
}

/// `GET /api/v1/consumptions/by-client/{client_ref}`
///
/// Lists consumptions for a specific client.
///
/// # Deprecated
///
/// Use `GET /api/v1/consumptions?agent_ref={agent_ref}` or the unified
/// `GET /api/v1/activity?kind=CONSUMPTION` instead.
#[deprecated(note = "Use GET /api/v1/consumptions?agent_ref=... or GET /api/v1/activity instead")]
#[utoipa::path(
    get,
    path = "/api/v1/consumptions/by-client/{client_ref}",
    tag = "Consumptions",
    security(("bearer" = [])),
    params(
        ("client_ref" = String, Path, description = "Client reference code"),
    ),
    responses(
        (status = 200, description = "Client consumptions (deprecated — prefer paginated endpoint)",
         body = Vec<Consumption>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
)]
#[allow(deprecated)]
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
