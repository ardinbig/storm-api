//! Customer business logic: CRUD, card-based lookup, registration, and
//! partial updates.
//!
//! The `update` function demonstrates the *partial-update* pattern: every
//! field in the request body is `Option<T>`, and the SQL uses
//! `COALESCE($n, column)` so that `None` values leave the existing data
//! untouched.

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::customer::{
        Customer, CustomerByCardResponse, RegisterCustomerRequest, UpdateCustomerRequest,
    },
};

fn generate_client_code() -> String {
    use chrono::Utc;
    let now = Utc::now();
    format!("STORM-{}", now.format("%Y%m%d-%H%M%S"))
}

/// Lists all customers ordered by last_name, first_name.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
pub async fn list(pool: &PgPool) -> Result<Vec<Customer>, AppError> {
    let customers = sqlx::query_as::<_, Customer>(
        "SELECT id, status, client_code, first_name, middle_name, last_name, address, networks, phone, category_ref, card_id, gender, marital_status, affiliation, created_at, updated_at FROM customers ORDER BY last_name, first_name",
    )
    .fetch_all(pool)
    .await?;

    Ok(customers)
}

/// Retrieves a single customer by primary key.
///
/// # Errors
///
/// - [`AppError::NotFound`] — no customer with this `id`.
/// - [`AppError::Database`] — query failure.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Customer, AppError> {
    let customer = sqlx::query_as::<_, Customer>(
        "SELECT id, status, client_code, first_name, middle_name, last_name, address, networks, phone, category_ref, card_id, gender, marital_status, affiliation, created_at, updated_at FROM customers WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Customer not found".to_string()))?;

    Ok(customer)
}

/// Looks up a customer by their NFC `card_id` and returns their
/// `client_code`.
///
/// # Errors
///
/// - [`AppError::NotFound`] — no customer associated with this card.
/// - [`AppError::Database`] — query failure.
pub async fn get_by_card(pool: &PgPool, card_id: &str) -> Result<CustomerByCardResponse, AppError> {
    let row: (String,) =
        sqlx::query_as("SELECT client_code FROM customers WHERE card_id = $1 LIMIT 1")
            .bind(card_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Customer not found for this card".to_string()))?;

    Ok(CustomerByCardResponse { client_code: row.0 })
}

/// Registers a new customer via the customer endpoint.
///
/// # Errors
///
/// - [`AppError::Database`] — constraint violation or query failure.
pub async fn register(
    pool: &PgPool,
    input: &RegisterCustomerRequest,
) -> Result<Customer, AppError> {
    let id = Uuid::new_v4();
    let client_code = generate_client_code();
    let customer = sqlx::query_as::<_, Customer>(
        "INSERT INTO customers (id, status, client_code, first_name, middle_name, last_name, address, networks, phone, category_ref, card_id, gender, marital_status, affiliation)
         VALUES ($1, 1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         RETURNING id, status, client_code, first_name, middle_name, last_name, address, networks, phone, category_ref, card_id, gender, marital_status, affiliation, created_at, updated_at",
    )
    .bind(id)
    .bind(&client_code)
    .bind(&input.first_name)
    .bind(&input.middle_name)
    .bind(&input.last_name)
    .bind(&input.address)
    .bind(&input.networks)
    .bind(&input.phone)
    .bind(input.category_ref)
    .bind(&input.card_id)
    .bind(&input.gender)
    .bind(&input.marital_status)
    .bind(&input.affiliation)
    .fetch_one(pool)
    .await?;

    Ok(customer)
}

/// Partially updates a customer's profile.
///
/// Only non-`None` fields in `input` are applied; existing values are
/// preserved via `COALESCE`.
///
/// # Errors
///
/// - [`AppError::NotFound`] — no customer with this `id`.
/// - [`AppError::Database`] — query failure.
pub async fn update(
    pool: &PgPool,
    id: Uuid,
    input: &UpdateCustomerRequest,
) -> Result<Customer, AppError> {
    let customer = sqlx::query_as::<_, Customer>(
        "UPDATE customers SET
            first_name = COALESCE($2, first_name),
            middle_name = COALESCE($3, middle_name),
            last_name = COALESCE($4, last_name),
            address = COALESCE($5, address),
            phone = COALESCE($6, phone),
            gender = COALESCE($7, gender),
            marital_status = COALESCE($8, marital_status),
            affiliation = COALESCE($9, affiliation),
            networks = COALESCE($10, networks),
            card_id = COALESCE($11, card_id),
            category_ref = COALESCE($12, category_ref)
         WHERE id = $1
         RETURNING id, status, client_code, first_name, middle_name, last_name, address, networks, phone, category_ref, card_id, gender, marital_status, affiliation, created_at, updated_at",
    )
    .bind(id)
    .bind(input.first_name.as_ref())
    .bind(input.middle_name.as_ref())
    .bind(input.last_name.as_ref())
    .bind(input.address.as_ref())
    .bind(input.phone.as_ref())
    .bind(input.gender.as_ref())
    .bind(input.marital_status.as_ref())
    .bind(input.affiliation.as_ref())
    .bind(input.networks.as_ref())
    .bind(input.card_id.as_ref())
    .bind(input.category_ref)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Customer not found".to_string()))?;

    Ok(customer)
}

/// Deletes a customer by primary key.
///
/// # Errors
///
/// - [`AppError::NotFound`] — no customer with this `id`.
/// - [`AppError::Database`] — query failure.
pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM customers WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Customer not found".to_string()));
    }

    Ok(())
}
