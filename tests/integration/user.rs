use sqlx::PgPool;
use storm_api::services::user_service;

#[sqlx::test]
async fn seed_super_admin_creates_suadmin_user_on_empty_db(pool: PgPool) {
    user_service::seed_super_admin(&pool).await.unwrap();

    let row: (String, String, Option<String>) =
        sqlx::query_as("SELECT username, name, email FROM users WHERE username = 'suadmin'")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(row.0, "suadmin");
    assert_eq!(row.1, "Super Administrator");
    assert_eq!(row.2.as_deref(), Some("info@ardinbig.com"));
}

#[sqlx::test]
async fn seed_super_admin_is_idempotent_when_called_twice(pool: PgPool) {
    user_service::seed_super_admin(&pool).await.unwrap();
    user_service::seed_super_admin(&pool).await.unwrap();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE username = 'suadmin'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(count.0, 1);
}

#[sqlx::test]
async fn seed_super_admin_does_not_overwrite_existing_suadmin(pool: PgPool) {
    user_service::seed_super_admin(&pool).await.unwrap();

    let original_hash: (String,) =
        sqlx::query_as("SELECT password FROM users WHERE username = 'suadmin'")
            .fetch_one(&pool)
            .await
            .unwrap();

    user_service::seed_super_admin(&pool).await.unwrap();

    let after_hash: (String,) =
        sqlx::query_as("SELECT password FROM users WHERE username = 'suadmin'")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(original_hash.0, after_hash.0);
}

#[sqlx::test]
#[serial_test::serial]
async fn seed_super_admin_password_authenticates_with_env_value(pool: PgPool) {
    unsafe {
        std::env::set_var("SUPER_ADMIN_PASSWORD", "MyEnvP@ssw0rd");
    }
    user_service::seed_super_admin(&pool).await.unwrap();
    unsafe {
        std::env::remove_var("SUPER_ADMIN_PASSWORD");
    }

    let config = storm_api::state::app_state::AuthConfig {
        jwt_secret: "test-secret".into(),
        jwt_expiry_hours: 24,
    };

    let result = user_service::authenticate(&pool, &config, "suadmin", "MyEnvP@ssw0rd").await;
    assert!(result.is_ok());
}

#[sqlx::test]
#[serial_test::serial]
async fn seed_super_admin_password_wrong_password_is_rejected(pool: PgPool) {
    unsafe {
        std::env::set_var("SUPER_ADMIN_PASSWORD", "CorrectHorseBatteryStaple");
    }
    user_service::seed_super_admin(&pool).await.unwrap();
    unsafe {
        std::env::remove_var("SUPER_ADMIN_PASSWORD");
    }

    let config = storm_api::state::app_state::AuthConfig {
        jwt_secret: "test-secret".into(),
        jwt_expiry_hours: 24,
    };

    let result = user_service::authenticate(&pool, &config, "suadmin", "wrong-password").await;
    assert!(result.is_err());
}

#[sqlx::test]
#[serial_test::serial]
async fn seed_super_admin_falls_back_to_default_password_when_env_absent(pool: PgPool) {
    unsafe {
        std::env::remove_var("SUPER_ADMIN_PASSWORD");
    }
    user_service::seed_super_admin(&pool).await.unwrap();

    let config = storm_api::state::app_state::AuthConfig {
        jwt_secret: "test-secret".into(),
        jwt_expiry_hours: 24,
    };

    let result = user_service::authenticate(&pool, &config, "suadmin", "superadminpassword").await;
    assert!(result.is_ok());
}

#[sqlx::test]
#[serial_test::serial]
async fn seed_super_admin_issued_token_carries_user_role(pool: PgPool) {
    unsafe {
        std::env::set_var("SUPER_ADMIN_PASSWORD", "RoleCheckPass1");
    }
    user_service::seed_super_admin(&pool).await.unwrap();
    unsafe {
        std::env::remove_var("SUPER_ADMIN_PASSWORD");
    }

    let config = storm_api::state::app_state::AuthConfig {
        jwt_secret: "role-check-secret".into(),
        jwt_expiry_hours: 24,
    };

    let auth = user_service::authenticate(&pool, &config, "suadmin", "RoleCheckPass1")
        .await
        .unwrap();

    let claims = storm_api::services::auth_service::verify_token(&config, &auth.token).unwrap();

    assert_eq!(claims.role, "user");
}

#[sqlx::test]
async fn seed_super_admin_does_not_affect_other_users(pool: PgPool) {
    user_service::register(
        &pool,
        &storm_api::models::user::RegisterRequest {
            name: "Alice".into(),
            email: None,
            username: "alice".into(),
            password: "alice.pass".into(),
        },
    )
    .await
    .unwrap();

    user_service::seed_super_admin(&pool).await.unwrap();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(count.0, 2);
}
