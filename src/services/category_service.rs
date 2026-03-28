//! Vehicle/customer category business logic.

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::category::{Category, CreateCategoryRequest},
};

/// Lists all categories ordered by name.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
pub async fn list(pool: &PgPool) -> Result<Vec<Category>, AppError> {
    let categories =
        sqlx::query_as::<_, Category>("SELECT id, name, created_at FROM categories ORDER BY name")
            .fetch_all(pool)
            .await?;

    Ok(categories)
}

/// Retrieves a single category by primary key.
///
/// # Errors
///
/// - [`AppError::NotFound`] — no category with this `id`.
/// - [`AppError::Database`] — query failure.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Category, AppError> {
    let category =
        sqlx::query_as::<_, Category>("SELECT id, name, created_at FROM categories WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;

    Ok(category)
}

/// Creates a new category.
///
/// # Errors
///
/// Returns [`AppError::Database`] on duplicate name or other constraint
/// violation.
pub async fn create(pool: &PgPool, input: &CreateCategoryRequest) -> Result<Category, AppError> {
    let id = Uuid::new_v4();
    let category = sqlx::query_as::<_, Category>(
        "INSERT INTO categories (id, name) VALUES ($1, $2)
         RETURNING id, name, created_at",
    )
    .bind(id)
    .bind(&input.name)
    .fetch_one(pool)
    .await?;

    Ok(category)
}
