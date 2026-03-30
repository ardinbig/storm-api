use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

/// Spin up a disposable container, connect via `create_pool`, and run a query.
#[tokio::test]
#[serial_test::serial]
async fn create_pool_success() {
    let container = Postgres::default()
        .start()
        .await
        .expect("Failed to start container");

    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get port");

    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let pool = storm_api::db::connection::create_pool(&url).await;

    let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await.unwrap();
    assert_eq!(row.0, 1);

    drop(pool);
}

/// Verify that `create_pool` with a MAX_DB_CONNECTIONS env var still works.
#[tokio::test]
#[serial_test::serial]
async fn create_pool_with_max_connections_env() {
    let container = Postgres::default()
        .start()
        .await
        .expect("Failed to start container");

    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get port");

    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    unsafe { std::env::set_var("MAX_DB_CONNECTIONS", "3") };
    let pool = storm_api::db::connection::create_pool(&url).await;
    let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await.unwrap();
    assert_eq!(row.0, 1);
    unsafe { std::env::remove_var("MAX_DB_CONNECTIONS") };

    drop(pool);
}

/// Bad URL should panic.
#[tokio::test]
#[should_panic(expected = "Failed to connect to database")]
async fn create_pool_with_bad_url() {
    storm_api::db::connection::create_pool("postgres://invalid:invalid@localhost:1/none").await;
}
