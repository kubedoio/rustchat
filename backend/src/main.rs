use rustchat::{api, config::Config, db, realtime::WsHub, storage::S3Client, telemetry};
use std::net::SocketAddr;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment from .env file if present
    dotenvy::dotenv().ok();

    // Load configuration
    let config = Config::load()?;

    // Initialize telemetry (logging/tracing)
    telemetry::init(&config.log_level);

    info!("Starting rustchat server v{}", env!("CARGO_PKG_VERSION"));

    // Connect to database and run migrations
    let db_pool = db::connect(&config.database_url).await?;
    info!("Database connected and migrations applied");

    // Create WebSocket hub
    let ws_hub = WsHub::new();
    info!("WebSocket hub initialized");

    // Create S3 client
    let s3_client = S3Client::new(
        config.s3_endpoint.clone(),
        config.s3_bucket.clone(),
        config.s3_access_key.clone(),
        config.s3_secret_key.clone(),
        config.s3_region.clone(),
    );
    info!("S3 client initialized");

    // Spawn background jobs
    rustchat::jobs::spawn_retention_job(db_pool.clone());

    // Build application router
    let app = api::router(
        db_pool.clone(),
        config.jwt_secret.clone(),
        config.jwt_expiry_hours,
        ws_hub,
        s3_client,
    );

    // Start server
    let addr: SocketAddr = format!("{}:{}", config.server_host, config.server_port)
        .parse()
        .expect("Invalid server address");

    info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
