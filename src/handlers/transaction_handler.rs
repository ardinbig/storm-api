//! Transaction handlers: list, list-by-agent, and withdrawal.

use axum::{
    Json,
    extract::{Path, State},
};
use sqlx::PgPool;

use crate::{
    errors::AppError,
    models::transaction::{Transaction, WithdrawalRequest, WithdrawalResponse},
    services::transaction_service,
    state::app_state::RedisPool,
};

/// `GET /api/v1/transactions`
///
/// Lists all financial transactions.
pub async fn list_transactions(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Transaction>>, AppError> {
    let transactions = transaction_service::list(&pool).await?;
    Ok(Json(transactions))
}

/// `GET /api/v1/transactions/by-agent/{agent_ref}`
///
/// Lists transactions for a specific agent.
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
pub async fn withdrawal(
    State(pool): State<PgPool>,
    State(redis): State<RedisPool>,
    Json(input): Json<WithdrawalRequest>,
) -> Result<Json<WithdrawalResponse>, AppError> {
    let response = transaction_service::withdrawal(&pool, &input, &redis).await?;
    Ok(Json(response))
}
