//! Fuel consumption business logic: listing and recording dispensing events.
//!
//! A database trigger (`fn_consumption_bonus_tree`) fires after each insert
//! to calculate MLM loyalty bonuses based on the consumption amount and the
//! applicable commission tier.

use sqlx::{PgPool, QueryBuilder};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        consumption::{Consumption, CreateConsumptionRequest},
        pagination::{ConsumptionQuery, PAGE_SIZE, PaginatedConsumptionResponse},
    },
};

/// Shared SELECT + FROM for consumption queries (aliased table `c`).
const CONS_SELECT: &str = "\
    SELECT c.client_ref, c.consumption_type, \
           c.quantity::FLOAT8 AS quantity, c.price::FLOAT8 AS price, \
           c.username, c.consumption_date, c.status \
    FROM consumptions c \
    LEFT JOIN agent_accounts a ON a.agent_ref = c.username \
    WHERE 1=1";

/// Appends optional `agent_ref` and `station_id` WHERE clauses to a
/// consumption `QueryBuilder`.  The builder must already end with `WHERE 1=1`.
fn push_consumption_filters<'q>(
    qb: &mut QueryBuilder<'q, sqlx::Postgres>,
    query: &'q ConsumptionQuery,
) {
    if let Some(ref ar) = query.agent {
        qb.push(" AND c.username = ").push_bind(ar.as_str());
    }
    if let Some(sid) = query.station {
        qb.push(" AND a.station_id = ").push_bind(sid);
    }
}

/// Lists all consumption records, most recent first.
///
/// # Deprecated
///
/// Prefer [`list_paginated`] which includes pagination metadata and supports
/// `agent_ref` / `station_id` filters.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
#[deprecated(note = "Use list_paginated instead")]
pub async fn list(pool: &PgPool) -> Result<Vec<Consumption>, AppError> {
    let consumptions = sqlx::query_as::<_, Consumption>(
        "SELECT client_ref, consumption_type, quantity::FLOAT8 AS quantity,
                price::FLOAT8 AS price, username, consumption_date, status
         FROM consumptions ORDER BY consumption_date DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(consumptions)
}

/// Lists consumption records for a specific client, most recent first.
///
/// # Deprecated
///
/// Prefer `list_paginated` with `agent_ref` query parameter.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
#[deprecated(note = "Use list_paginated with agent_ref filter instead")]
pub async fn list_by_client(pool: &PgPool, client_ref: &str) -> Result<Vec<Consumption>, AppError> {
    let consumptions = sqlx::query_as::<_, Consumption>(
        "SELECT client_ref, consumption_type, quantity::FLOAT8 AS quantity,
                price::FLOAT8 AS price, username, consumption_date, status
         FROM consumptions WHERE client_ref = $1 ORDER BY consumption_date DESC",
    )
    .bind(client_ref)
    .fetch_all(pool)
    .await?;

    Ok(consumptions)
}

/// Returns a paginated list of consumptions, optionally filtered by the
/// operator's agent reference code and/or the station they belong to.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
pub async fn list_paginated(
    pool: &PgPool,
    query: &ConsumptionQuery,
) -> Result<PaginatedConsumptionResponse, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let offset = ((page - 1) as i64) * (PAGE_SIZE as i64);

    // Data query
    let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(CONS_SELECT);
    push_consumption_filters(&mut qb, query);
    qb.push(" ORDER BY c.consumption_date DESC LIMIT ")
        .push_bind(PAGE_SIZE as i64)
        .push(" OFFSET ")
        .push_bind(offset);
    let data = qb.build_query_as::<Consumption>().fetch_all(pool).await?;

    // Count query
    let mut cqb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        "SELECT COUNT(*) FROM consumptions c \
         LEFT JOIN agent_accounts a ON a.agent_ref = c.username \
         WHERE 1=1",
    );
    push_consumption_filters(&mut cqb, query);
    let (total_items,): (i64,) = cqb.build_query_as().fetch_one(pool).await?;

    Ok(PaginatedConsumptionResponse::new(data, page, total_items))
}

/// Records a new fuel consumption event.
///
/// The `date` field from the request is cast to `TIMESTAMPTZ` at the SQL
/// level.  After insertion, a database trigger calculates any applicable
/// MLM loyalty bonuses.
///
/// # Errors
///
/// Returns [`AppError::Database`] on constraint violation or query failure.
pub async fn create(pool: &PgPool, input: &CreateConsumptionRequest) -> Result<(), AppError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO consumptions (id, client_ref, consumption_type, quantity, price, username, consumption_date, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7::TIMESTAMPTZ, 1)",
    )
    .bind(id)
    .bind(&input.client_ref)
    .bind(&input.consumption_type)
    .bind(input.quantity)
    .bind(input.price)
    .bind(&input.username)
    .bind(&input.date)
    .execute(pool)
    .await?;

    Ok(())
}
