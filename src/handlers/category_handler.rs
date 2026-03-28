//! Category handlers: list, get, and create.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::category::{Category, CreateCategoryRequest},
    services::category_service,
};

/// `GET /api/v1/categories`
///
/// Lists all vehicle/customer categories.
pub async fn list_categories(State(pool): State<PgPool>) -> Result<Json<Vec<Category>>, AppError> {
    let categories = category_service::list(&pool).await?;
    Ok(Json(categories))
}

/// `GET /api/v1/categories/{id}`
///
/// Retrieves a single category by UUID.
pub async fn get_category(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Category>, AppError> {
    let category = category_service::get_by_id(&pool, id).await?;
    Ok(Json(category))
}

/// `POST /api/v1/categories`
///
/// Creates a new category. Returns `201 Created`.
pub async fn create_category(
    State(pool): State<PgPool>,
    Json(input): Json<CreateCategoryRequest>,
) -> Result<(StatusCode, Json<Category>), AppError> {
    let category = category_service::create(&pool, &input).await?;
    Ok((StatusCode::CREATED, Json(category)))
}
