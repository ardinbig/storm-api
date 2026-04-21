use sqlx::PgPool;
use storm_api::services::user_service;

fn set_default_admin_password() {
    unsafe {
        std::env::set_var("SUPER_ADMIN_PASSWORD", "TestAdminPass1!");
    }
}

// The database trigger installed by 003_protect_suadmin.sql must reject any
// attempt to DELETE the suadmin row.
#[sqlx::test]
#[serial_test::serial]
async fn suadmin_cannot_be_deleted_from_users_table(pool: PgPool) {
    set_default_admin_password();
    user_service::seed_super_admin(&pool).await.unwrap();

    let result = sqlx::query("DELETE FROM users WHERE username = 'suadmin'")
        .execute(&pool)
        .await;

    assert!(
        result.is_err(),
        "DELETE suadmin should be rejected by the trigger"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("suadmin account is protected"),
        "unexpected error message: {err}"
    );

    // The row must still be present.
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE username = 'suadmin'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(count.0, 1);
}

#[sqlx::test]
#[serial_test::serial]
async fn seed_super_admin_creates_suadmin_user_on_empty_db(pool: PgPool) {
    set_default_admin_password();
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
#[serial_test::serial]
async fn seed_super_admin_is_idempotent_when_called_twice(pool: PgPool) {
    set_default_admin_password();
    user_service::seed_super_admin(&pool).await.unwrap();
    user_service::seed_super_admin(&pool).await.unwrap();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE username = 'suadmin'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(count.0, 1);
}

// The upsert re-hashes with a fresh salt on every call, so the stored hash
// bytes change — but there must still be exactly one suadmin row.
#[sqlx::test]
#[serial_test::serial]
async fn seed_super_admin_upsert_keeps_exactly_one_row(pool: PgPool) {
    set_default_admin_password();
    user_service::seed_super_admin(&pool).await.unwrap();
    user_service::seed_super_admin(&pool).await.unwrap();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE username = 'suadmin'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(count.0, 1);
}

// Changing SUPER_ADMIN_PASSWORD and re-seeding must let the new password work.
#[sqlx::test]
#[serial_test::serial]
async fn seed_super_admin_picks_up_new_password_after_env_change(pool: PgPool) {
    unsafe {
        std::env::set_var("SUPER_ADMIN_PASSWORD", "OldPassword1");
    }
    user_service::seed_super_admin(&pool).await.unwrap();

    unsafe {
        std::env::set_var("SUPER_ADMIN_PASSWORD", "NewPassword2");
    }
    user_service::seed_super_admin(&pool).await.unwrap();
    unsafe {
        std::env::remove_var("SUPER_ADMIN_PASSWORD");
    }

    let config = storm_api::state::app_state::AuthConfig {
        jwt_secret: "test-secret".into(),
        jwt_expiry_hours: 24,
    };

    assert!(
        user_service::authenticate(&pool, &config, "suadmin", "NewPassword2")
            .await
            .is_ok()
    );

    assert!(
        user_service::authenticate(&pool, &config, "suadmin", "OldPassword1")
            .await
            .is_err()
    );
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

// When the env var is absent the seed must fail.
#[sqlx::test]
#[serial_test::serial]
async fn seed_super_admin_fails_when_env_absent(pool: PgPool) {
    unsafe {
        std::env::remove_var("SUPER_ADMIN_PASSWORD");
    }

    let result = user_service::seed_super_admin(&pool).await;
    assert!(result.is_err());
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
#[serial_test::serial]
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

    set_default_admin_password();
    user_service::seed_super_admin(&pool).await.unwrap();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(count.0, 2);
}
