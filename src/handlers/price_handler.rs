//! Fuel price handlers: list, get-by-type, and create.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::PgPool;

use crate::{
    errors::{AppError, ErrorResponse},
    models::price::{CreatePriceRequest, FuelPrice},
    services::price_service,
    state::app_state::RedisPool,
};

/// `GET /api/v1/prices`
///
/// Lists all fuel price records.
#[utoipa::path(
    get,
    path = "/api/v1/prices",
    tag = "Prices",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "List of fuel prices", body = Vec<FuelPrice>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
)]
pub async fn list_prices(State(pool): State<PgPool>) -> Result<Json<Vec<FuelPrice>>, AppError> {
    let prices = price_service::list(&pool).await?;
    Ok(Json(prices))
}

/// `GET /api/v1/prices/by-type/{consumption_type}`
///
/// Retrieves the current price for a specific fuel type.
#[utoipa::path(
    get,
    path = "/api/v1/prices/by-type/{consumption_type}",
    tag = "Prices",
    security(("bearer" = [])),
    params(
        ("consumption_type" = String, Path, description = "Fuel type (e.g. diesel, essence)"),
    ),
    responses(
        (status = 200, description = "Current fuel price", body = FuelPrice),
        (status = 404, description = "Price not found for type", body = ErrorResponse),
    ),
)]
pub async fn get_by_type(
    State(pool): State<PgPool>,
    State(redis): State<RedisPool>,
    Path(consumption_type): Path<String>,
) -> Result<Json<FuelPrice>, AppError> {
    let price = price_service::get_by_type(&pool, &consumption_type, &redis).await?;
    Ok(Json(price))
}

/// `POST /api/v1/prices`
///
/// Creates a new fuel price record. Returns `201 Created`.
#[utoipa::path(
    post,
    path = "/api/v1/prices",
    tag = "Prices",
    security(("bearer" = [])),
    request_body = CreatePriceRequest,
    responses(
        (status = 201, description = "Price created", body = FuelPrice),
    ),
)]
pub async fn create_price(
    State(pool): State<PgPool>,
    State(redis): State<RedisPool>,
    Json(input): Json<CreatePriceRequest>,
) -> Result<(StatusCode, Json<FuelPrice>), AppError> {
    let price = price_service::create(&pool, &input, &redis).await?;
    Ok((StatusCode::CREATED, Json(price)))
}
