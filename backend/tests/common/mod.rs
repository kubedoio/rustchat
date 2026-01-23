use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::net::TcpListener;
use rustchat::config::Config;
use rustchat::api;
use rustchat::realtime::WsHub;
use rustchat::storage::S3Client;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

pub async fn spawn_app() -> TestApp {
    // 1. Setup Configuration
    let mut config = Config::load().expect("Failed to load configuration");

    // Bind to port 0 to let OS select random port
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    // 2. Setup Database Connection
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to Postgres");

    // 3. Setup Dependencies
    // WsHub::new() returns Arc<WsHub> directly
    let ws_hub = WsHub::new();
    let s3_client = S3Client::new(
        config.s3_endpoint.clone(),
        config.s3_bucket.clone(),
        config.s3_access_key.clone(),
        config.s3_secret_key.clone(),
        config.s3_region.clone(),
    );

    // 4. Build App
    let app = api::router(
        db_pool.clone(),
        config.jwt_secret.clone(),
        config.jwt_expiry_hours,
        ws_hub,
        s3_client,
    );

    // 5. Spawn Server in Background
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    TestApp {
        address,
        db_pool,
    }
}
