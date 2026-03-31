//! Agent account handlers: CRUD, login, balance check, history, customer
//! registration, and password update.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        agent::{
            AgentAuthResponse, AgentHistoryRow, AgentInfo, AgentLoginRequest,
            AgentRegisterCustomerRequest, CreateAgentRequest, UpdateAgentPasswordRequest,
        },
        card::CardDetail,
    },
    services::agent_service,
    state::app_state::{AuthConfig, RedisPool},
};

/// `GET /api/v1/agents`
///
/// Lists all agent accounts.
pub async fn list_agents(State(pool): State<PgPool>) -> Result<Json<Vec<AgentInfo>>, AppError> {
    let agents = agent_service::list(&pool).await?;
    Ok(Json(agents))
}

/// `GET /api/v1/agents/{id}`
///
/// Retrieves a single agent by UUID.
pub async fn get_agent(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<AgentInfo>, AppError> {
    let agent = agent_service::get_by_id(&pool, id).await?;
    Ok(Json(agent))
}

/// `POST /api/v1/agents`
///
/// Creates a new agent account. Returns `201 Created`.
pub async fn create_agent(
    State(pool): State<PgPool>,
    Json(input): Json<CreateAgentRequest>,
) -> Result<(StatusCode, Json<AgentInfo>), AppError> {
    let agent = agent_service::create(&pool, &input).await?;
    Ok((StatusCode::CREATED, Json(agent)))
}

/// `DELETE /api/v1/agents/{id}`
///
/// Deletes an agent account. Returns `204 No Content`.
pub async fn delete_agent(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    agent_service::delete(&pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/agents/login` (public route)
///
/// Authenticates an agent and returns a JWT.
pub async fn login(
    State(pool): State<PgPool>,
    State(config): State<Arc<AuthConfig>>,
    Json(input): Json<AgentLoginRequest>,
) -> Result<Json<AgentAuthResponse>, AppError> {
    let response = agent_service::login(&pool, &config, &input).await?;
    Ok(Json(response))
}

/// `GET /api/v1/agents/cards/{card_id}/balance`
///
/// Returns the card detail (balance) for a given NFC reference.
pub async fn check_balance(
    State(pool): State<PgPool>,
    State(redis): State<RedisPool>,
    Path(card_id): Path<String>,
) -> Result<Json<CardDetail>, AppError> {
    let card = agent_service::check_balance(&pool, &card_id, &redis).await?;
    Ok(Json(card))
}

/// `GET /api/v1/agents/{agent_id}/history`
///
/// Returns the transaction history for a given agent.
pub async fn history(
    State(pool): State<PgPool>,
    Path(agent_id): Path<String>,
) -> Result<Json<Vec<AgentHistoryRow>>, AppError> {
    let rows = agent_service::history(&pool, &agent_id).await?;
    Ok(Json(rows))
}

/// `POST /api/v1/agents/customers`
///
/// Agent-initiated customer registration. Returns `201 Created`.
pub async fn register_customer(
    State(pool): State<PgPool>,
    State(redis): State<RedisPool>,
    Json(input): Json<AgentRegisterCustomerRequest>,
) -> Result<StatusCode, AppError> {
    agent_service::register_customer(&pool, &input, &redis).await?;
    Ok(StatusCode::CREATED)
}

/// `PUT /api/v1/agents/password`
///
/// Changes an agent's password after verifying the current one.
pub async fn update_password(
    State(pool): State<PgPool>,
    Json(input): Json<UpdateAgentPasswordRequest>,
) -> Result<StatusCode, AppError> {
    agent_service::update_password(&pool, &input).await?;
    Ok(StatusCode::OK)
}
