use sqlx::PgPool;

use crate::common::{register_and_login, test_config};

/// A self-contained test server backed by disposable PostgreSQL and Redis
/// containers.
pub struct TestApp {
    pub addr: String,
    pub client: reqwest::Client,
    pub pool: PgPool,
    _pg_container: testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>,
    _redis_container: testcontainers::ContainerAsync<testcontainers_modules::redis::Redis>,
}

impl TestApp {
    //noinspection HttpUrlsUsage
    /// Spin up fresh PostgreSQL and Redis containers, run migrations, start
    /// the Axum server on an OS-assigned port, and return the ready-to-use
    /// context.
    pub async fn spawn() -> Self {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, AtomicU64},
        };

        use testcontainers::ImageExt;
        use testcontainers::runners::AsyncRunner;
        use testcontainers_modules::postgres::Postgres;
        use testcontainers_modules::redis::Redis;

        use storm_api::state::app_state::AppState;

        let pg_container = Postgres::default()
            .with_tag("18-bookworm")
            .start()
            .await
            .expect("Failed to start PostgreSQL container");

        let pg_port = pg_container
            .get_host_port_ipv4(5432)
            .await
            .expect("Failed to get Postgres container port");

        let db_url = format!("postgres://postgres:postgres@127.0.0.1:{pg_port}/postgres");

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .expect("Failed to connect to container database");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        let redis_container = Redis::default()
            .with_tag("8-bookworm")
            .start()
            .await
            .expect("Failed to start Redis container");

        let redis_port = redis_container
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get Redis container port");

        let redis_url = format!("redis://127.0.0.1:{redis_port}");
        let redis_client = redis::Client::open(redis_url).expect("Failed to create Redis client");
        let redis_conn = redis::aio::ConnectionManager::new(redis_client)
            .await
            .expect("Failed to connect to Redis container");

        let state = AppState {
            pool: pool.clone(),
            redis: Some(redis_conn),
            auth_config: Arc::new(test_config()),
            ready: Arc::new(AtomicBool::new(true)),
            request_count: Arc::new(AtomicU64::new(0)),
        };

        let app = storm_api::app::create_app(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind");
        let addr = format!("http://{}", listener.local_addr().unwrap());

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Self {
            addr,
            client: reqwest::Client::new(),
            pool,
            _pg_container: pg_container,
            _redis_container: redis_container,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.addr)
    }

    /// Register + login via the service layer and return a JWT.
    pub async fn token(&self) -> String {
        register_and_login(&self.pool, &test_config()).await
    }

    pub async fn get(&self, path: &str) -> reqwest::Response {
        self.client.get(self.url(path)).send().await.unwrap()
    }

    pub async fn get_auth(&self, path: &str, token: &str) -> reqwest::Response {
        self.client
            .get(self.url(path))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
    }

    pub async fn post_json(&self, path: &str, body: &serde_json::Value) -> reqwest::Response {
        self.client
            .post(self.url(path))
            .json(body)
            .send()
            .await
            .unwrap()
    }

    pub async fn post_json_auth(
        &self,
        path: &str,
        body: &serde_json::Value,
        token: &str,
    ) -> reqwest::Response {
        self.client
            .post(self.url(path))
            .bearer_auth(token)
            .json(body)
            .send()
            .await
            .unwrap()
    }

    pub async fn post_auth(&self, path: &str, token: &str) -> reqwest::Response {
        self.client
            .post(self.url(path))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
    }

    pub async fn put_json_auth(
        &self,
        path: &str,
        body: &serde_json::Value,
        token: &str,
    ) -> reqwest::Response {
        self.client
            .put(self.url(path))
            .bearer_auth(token)
            .json(body)
            .send()
            .await
            .unwrap()
    }

    pub async fn patch_json_auth(
        &self,
        path: &str,
        body: &serde_json::Value,
        token: &str,
    ) -> reqwest::Response {
        self.client
            .patch(self.url(path))
            .bearer_auth(token)
            .json(body)
            .send()
            .await
            .unwrap()
    }

    pub async fn delete_auth(&self, path: &str, token: &str) -> reqwest::Response {
        self.client
            .delete(self.url(path))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
    }
}
