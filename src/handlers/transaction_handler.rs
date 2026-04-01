//! Transaction handlers: list, list-by-agent, and withdrawal.

use axum::{
    Json,
    extract::{Path, State},
};
use sqlx::PgPool;

use crate::{
    errors::{AppError, ErrorResponse},
    models::transaction::{Transaction, WithdrawalRequest, WithdrawalResponse},
    services::transaction_service,
    state::app_state::RedisPool,
};

/// `GET /api/v1/transactions`
///
/// Lists all financial transactions.
#[utoipa::path(
    get,
    path = "/api/v1/transactions",
    tag = "Transactions",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "List of transactions", body = Vec<Transaction>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
)]
pub async fn list_transactions(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Transaction>>, AppError> {
    let transactions = transaction_service::list(&pool).await?;
    Ok(Json(transactions))
}

/// `GET /api/v1/transactions/by-agent/{agent_ref}`
///
/// Lists transactions for a specific agent.
#[utoipa::path(
    get,
    path = "/api/v1/transactions/by-agent/{agent_ref}",
    tag = "Transactions",
    security(("bearer" = [])),
    params(
        ("agent_ref" = String, Path, description = "Agent reference code"),
    ),
    responses(
        (status = 200, description = "Agent transactions", body = Vec<Transaction>),
    ),
)]
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
