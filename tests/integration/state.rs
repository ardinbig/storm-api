use std::sync::Arc;

use axum::extract::FromRef;
use sqlx::PgPool;

use crate::common::{JWT_SECRET, test_state};
use storm_api::state::app_state::AuthConfig;

#[sqlx::test]
async fn from_ref_pool(pool: PgPool) {
    let state = test_state(pool.clone());
    let extracted: PgPool = PgPool::from_ref(&state);
    assert_eq!(format!("{:?}", extracted), format!("{:?}", pool));
}

#[sqlx::test]
async fn from_ref_auth_config(pool: PgPool) {
    let state = test_state(pool);
    let extracted: Arc<AuthConfig> = Arc::<AuthConfig>::from_ref(&state);
    assert_eq!(extracted.jwt_secret, JWT_SECRET);
    assert_eq!(extracted.jwt_expiry_hours, 24);
}
