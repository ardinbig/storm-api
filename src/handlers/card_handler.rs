//! NFC card handlers: list, get, create, and balance check.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::card::{BalanceCheckRequest, BalanceResponse, Card, CreateCardRequest},
    services::card_service,
};

/// `GET /api/v1/cards`
///
/// Lists all NFC cards.
pub async fn list_cards(State(pool): State<PgPool>) -> Result<Json<Vec<Card>>, AppError> {
    let cards = card_service::list(&pool).await?;
    Ok(Json(cards))
}

/// `GET /api/v1/cards/{id}`
///
/// Retrieves a single card by UUID.
pub async fn get_card(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Card>, AppError> {
    let card = card_service::get_by_id(&pool, id).await?;
    Ok(Json(card))
}

/// `POST /api/v1/cards`
///
/// Creates a new NFC card. Returns `201 Created`.
pub async fn create_card(
    State(pool): State<PgPool>,
    Json(input): Json<CreateCardRequest>,
) -> Result<(StatusCode, Json<Card>), AppError> {
    let card = card_service::create(&pool, &input).await?;
    Ok((StatusCode::CREATED, Json(card)))
}

/// `POST /api/v1/cards/{nfc_ref}/balance`
///
/// Password-protected card balance inquiry.
pub async fn check_balance(
    State(pool): State<PgPool>,
    Path(nfc_ref): Path<String>,
    Json(input): Json<BalanceCheckRequest>,
) -> Result<Json<BalanceResponse>, AppError> {
    let balance = card_service::check_balance(&pool, &nfc_ref, &input.password).await?;
    Ok(Json(balance))
}
