//! Customer handlers: list, get, get-by-card, register, update, and delete.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::customer::{
        Customer, CustomerByCardResponse, RegisterCustomerRequest, UpdateCustomerRequest,
    },
    services::customer_service,
};

/// `GET /api/v1/customers`
///
/// Lists all customer profiles.
pub async fn list_customers(State(pool): State<PgPool>) -> Result<Json<Vec<Customer>>, AppError> {
    let customers = customer_service::list(&pool).await?;
    Ok(Json(customers))
}

/// `GET /api/v1/customers/{id}`
///
/// Retrieves a single customer by UUID.
pub async fn get_customer(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Customer>, AppError> {
    let customer = customer_service::get_by_id(&pool, id).await?;
    Ok(Json(customer))
}

/// `GET /api/v1/customers/by-card/{card_id}`
///
/// Looks up a customer by NFC card identifier and returns the client code.
pub async fn get_by_card(
    State(pool): State<PgPool>,
    Path(card_id): Path<String>,
) -> Result<Json<CustomerByCardResponse>, AppError> {
    let customer = customer_service::get_by_card(&pool, &card_id).await?;
    Ok(Json(customer))
}

/// `POST /api/v1/customers`
///
/// Registers a new customer (sync endpoint). Returns `201 Created`.
pub async fn register(
    State(pool): State<PgPool>,
    Json(input): Json<RegisterCustomerRequest>,
) -> Result<(StatusCode, Json<Customer>), AppError> {
    let registration = customer_service::register(&pool, &input).await?;
    Ok((StatusCode::CREATED, Json(registration)))
}

/// `PUT /api/v1/customers/{id}`
///
/// Partially updates a customer profile.
pub async fn update_customer(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateCustomerRequest>,
) -> Result<Json<Customer>, AppError> {
    let customer = customer_service::update(&pool, id, &input).await?;
    Ok(Json(customer))
}

/// `DELETE /api/v1/customers/{id}`
///
/// Deletes a customer. Returns `204 No Content`.
pub async fn delete_customer(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    customer_service::delete(&pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
