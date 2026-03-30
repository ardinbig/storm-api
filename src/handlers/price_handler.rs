//! Fuel price handlers: list, get-by-type, and create.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::PgPool;

use crate::{
    errors::AppError,
    models::price::{CreatePriceRequest, FuelPrice},
    services::price_service,
    state::app_state::RedisPool,
};

/// `GET /api/v1/prices`
///
/// Lists all fuel price records.
pub async fn list_prices(State(pool): State<PgPool>) -> Result<Json<Vec<FuelPrice>>, AppError> {
    let prices = price_service::list(&pool).await?;
    Ok(Json(prices))
}

/// `GET /api/v1/prices/by-type/{consumption_type}`
///
/// Retrieves the current price for a specific fuel type.
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
pub async fn create_price(
    State(pool): State<PgPool>,
    State(redis): State<RedisPool>,
    Json(input): Json<CreatePriceRequest>,
) -> Result<(StatusCode, Json<FuelPrice>), AppError> {
    let price = price_service::create(&pool, &input, &redis).await?;
    Ok((StatusCode::CREATED, Json(price)))
}
