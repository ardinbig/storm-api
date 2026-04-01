//! Category handlers: list, get, and create.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{AppError, ErrorResponse},
    models::category::{Category, CreateCategoryRequest},
    services::category_service,
};

/// `GET /api/v1/categories`
///
/// Lists all vehicle/customer categories.
#[utoipa::path(
    get,
    path = "/api/v1/categories",
    tag = "Categories",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "List of categories", body = Vec<Category>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
)]
pub async fn list_categories(State(pool): State<PgPool>) -> Result<Json<Vec<Category>>, AppError> {
    let categories = category_service::list(&pool).await?;
    Ok(Json(categories))
}

/// `GET /api/v1/categories/{id}`
///
/// Retrieves a single category by UUID.
#[utoipa::path(
    get,
    path = "/api/v1/categories/{id}",
    tag = "Categories",
    security(("bearer" = [])),
    params(
        ("id" = Uuid, Path, description = "Category UUID"),
    ),
    responses(
        (status = 200, description = "Category found", body = Category),
        (status = 404, description = "Category not found", body = ErrorResponse),
    ),
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/categories",
    tag = "Categories",
    security(("bearer" = [])),
    request_body = CreateCategoryRequest,
    responses(
        (status = 201, description = "Category created", body = Category),
    ),
)]
pub async fn create_category(
    State(pool): State<PgPool>,
    Json(input): Json<CreateCategoryRequest>,
) -> Result<(StatusCode, Json<Category>), AppError> {
    let category = category_service::create(&pool, &input).await?;
    Ok((StatusCode::CREATED, Json(category)))
}
