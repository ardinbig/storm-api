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
    errors::{AppError, ErrorResponse},
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
#[utoipa::path(
    get,
    path = "/api/v1/agents",
    tag = "Agents",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "List of agents", body = Vec<AgentInfo>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
)]
pub async fn list_agents(State(pool): State<PgPool>) -> Result<Json<Vec<AgentInfo>>, AppError> {
    let agents = agent_service::list(&pool).await?;
    Ok(Json(agents))
}

/// `GET /api/v1/agents/{id}`
///
/// Retrieves a single agent by UUID.
#[utoipa::path(
    get,
    path = "/api/v1/agents/{id}",
    tag = "Agents",
    security(("bearer" = [])),
    params(
        ("id" = Uuid, Path, description = "Agent UUID"),
    ),
    responses(
        (status = 200, description = "Agent found", body = AgentInfo),
        (status = 404, description = "Agent not found", body = ErrorResponse),
    ),
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/agents",
    tag = "Agents",
    security(("bearer" = [])),
    request_body = CreateAgentRequest,
    responses(
        (status = 201, description = "Agent created", body = AgentInfo),
        (status = 409, description = "Agent ref already exists", body = ErrorResponse),
    ),
)]
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
#[utoipa::path(
    delete,
    path = "/api/v1/agents/{id}",
    tag = "Agents",
    security(("bearer" = [])),
    params(
        ("id" = Uuid, Path, description = "Agent UUID"),
    ),
    responses(
        (status = 204, description = "Agent deleted"),
        (status = 400, description = "Cannot delete house account", body = ErrorResponse),
        (status = 404, description = "Agent not found", body = ErrorResponse),
    ),
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/agents/login",
    tag = "Agents",
    request_body = AgentLoginRequest,
    responses(
        (status = 200, description = "Agent login successful", body = AgentAuthResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
    ),
)]
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
#[utoipa::path(
    get,
    path = "/api/v1/agents/cards/{card_id}/balance",
    tag = "Agents",
    security(("bearer" = [])),
    params(
        ("card_id" = String, Path, description = "NFC card reference"),
    ),
    responses(
        (status = 200, description = "Card balance", body = CardDetail),
        (status = 404, description = "Card not found", body = ErrorResponse),
    ),
)]
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
#[utoipa::path(
    get,
    path = "/api/v1/agents/{agent_id}/history",
    tag = "Agents",
    security(("bearer" = [])),
    params(
        ("agent_id" = String, Path, description = "Agent UUID as string"),
    ),
    responses(
        (status = 200, description = "Agent transaction history", body = Vec<AgentHistoryRow>),
    ),
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/agents/customers",
    tag = "Agents",
    security(("bearer" = [])),
    request_body = AgentRegisterCustomerRequest,
    responses(
        (status = 201, description = "Customer registered by agent"),
        (status = 404, description = "Card not found", body = ErrorResponse),
        (status = 409, description = "Card already assigned", body = ErrorResponse),
    ),
)]
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
#[utoipa::path(
    put,
    path = "/api/v1/agents/password",
    tag = "Agents",
    security(("bearer" = [])),
    request_body = UpdateAgentPasswordRequest,
    responses(
        (status = 200, description = "Password updated"),
        (status = 401, description = "Current password incorrect", body = ErrorResponse),
    ),
)]
pub async fn update_password(
    State(pool): State<PgPool>,
    Json(input): Json<UpdateAgentPasswordRequest>,
) -> Result<StatusCode, AppError> {
    agent_service::update_password(&pool, &input).await?;
    Ok(StatusCode::OK)
}
