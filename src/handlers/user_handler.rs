//! Current-user handler (requires JWT).
use axum::{Json, extract::Extension};

use crate::models::user::{CurrentUser, MeResponse};

/// `GET /api/v1/users/me`
///
/// Returns the identity of the currently authenticated user (extracted
/// from the JWT by the auth middleware).
pub async fn me(Extension(user): Extension<CurrentUser>) -> Json<MeResponse> {
    Json(MeResponse {
        id: user.id,
        role: user.role,
    })
}
