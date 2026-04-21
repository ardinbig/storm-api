//! Transaction handlers: paginated list, unified activity feed,
//! deprecated by-agent list, and withdrawal.

use axum::{
    Json,
    extract::{Path, Query, State},
};
use sqlx::PgPool;

use crate::{
    errors::{AppError, ErrorResponse},
    models::{
        pagination::{
            ActivityQuery, PaginatedActivityResponse, PaginatedTransactionResponse,
            TransactionQuery,
        },
        transaction::{Transaction, WithdrawalRequest, WithdrawalResponse},
    },
    services::transaction_service,
    state::app_state::RedisPool,
};

/// `GET /api/v1/transactions`
///
/// Returns a paginated list of financial transactions (withdrawals), ordered
/// most-recent-first.  Supports optional filtering by `agent_ref` and
/// `station_id`.
#[utoipa::path(
    get,
    path = "/api/v1/transactions",
    tag = "Transactions",
    security(("bearer" = [])),
    params(TransactionQuery),
    responses(
        (status = 200, description = "Paginated list of transactions",
         body = PaginatedTransactionResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
)]
pub async fn list_transactions(
    State(pool): State<PgPool>,
    Query(query): Query<TransactionQuery>,
) -> Result<Json<PaginatedTransactionResponse>, AppError> {
    let result = transaction_service::list_paginated(&pool, &query).await?;
    Ok(Json(result))
}

/// `GET /api/v1/transactions/by-agent/{agent_ref}`
///
/// Lists all transactions for a specific agent.
///
/// **Deprecated** — use `GET /api/v1/transactions?agent_ref={agent_ref}` instead,
/// which returns pagination metadata and supports an additional `station_id` filter.
#[deprecated(note = "Use GET /api/v1/transactions?agent_ref={agent_ref} instead")]
#[utoipa::path(
    get,
    path = "/api/v1/transactions/by-agent/{agent_ref}",
    tag = "Transactions",
    security(("bearer" = [])),
    params(
        ("agent_ref" = String, Path, description = "Agent reference code"),
    ),
    responses(
        (status = 200, description = "Agent transactions (deprecated — prefer paginated endpoint)",
         body = Vec<Transaction>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
)]
#[allow(deprecated)]
pub async fn list_by_agent(
    State(pool): State<PgPool>,
    Path(agent_ref): Path<String>,
) -> Result<Json<Vec<Transaction>>, AppError> {
    let transactions = transaction_service::list_by_agent(&pool, &agent_ref).await?;
    Ok(Json(transactions))
}

/// `POST /api/v1/transactions/withdrawal`
///
/// Performs an atomic withdrawal from a customer card to an agent account.
#[utoipa::path(
    post,
    path = "/api/v1/transactions/withdrawal",
    tag = "Transactions",
    security(("bearer" = [])),
    request_body = WithdrawalRequest,
    responses(
        (status = 200, description = "Withdrawal successful", body = WithdrawalResponse),
        (status = 400, description = "Insufficient balance or invalid request", body = ErrorResponse),
        (status = 404, description = "Card or agent not found", body = ErrorResponse),
    ),
)]
pub async fn withdrawal(
    State(pool): State<PgPool>,
    State(redis): State<RedisPool>,
    Json(input): Json<WithdrawalRequest>,
) -> Result<Json<WithdrawalResponse>, AppError> {
    let response = transaction_service::withdrawal(&pool, &input, &redis).await?;
    Ok(Json(response))
}

/// `GET /api/v1/activity`
///
/// Returns a paginated unified activity feed combining both withdrawals
/// (from the `transactions` table) and fuel consumption events (from the
/// `consumptions` table), ordered most-recent-first.
///
/// ## Filters
///
/// | Parameter    | Description |
/// |--------------|-------------|
/// | `page`       | 1-based page number (default `1`) |
/// | `kind`       | `"WITHDRAWAL"` or `"CONSUMPTION"` |
/// | `agent`  | Agent reference code / operator username |
/// | `station` | Station UUID (system-user ID the agent belongs to) |
#[utoipa::path(
    get,
    path = "/api/v1/activity",
    tag = "Activity",
    security(("bearer" = [])),
    params(ActivityQuery),
    responses(
        (status = 200, description = "Paginated unified activity feed",
         body = PaginatedActivityResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
)]
pub async fn list_activity(
    State(pool): State<PgPool>,
    Query(query): Query<ActivityQuery>,
) -> Result<Json<PaginatedActivityResponse>, AppError> {
    let result = transaction_service::list_activity(&pool, &query).await?;
    Ok(Json(result))
}
