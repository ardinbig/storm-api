//! Current-user handler (requires JWT).
use axum::{Json, extract::Extension};

use crate::models::user::{CurrentUser, MeResponse};

/// `GET /api/v1/users/me`
///
/// Returns the identity of the currently authenticated user (extracted
/// from the JWT by the auth middleware).
#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    tag = "Users",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Current user identity", body = MeResponse),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn me(Extension(user): Extension<CurrentUser>) -> Json<MeResponse> {
    Json(MeResponse {
        id: user.id,
        role: user.role,
    })
}
