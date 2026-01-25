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

    // Seed admin user if configured
    if let (Some(admin_email), Some(admin_password)) = (&config.admin_user, &config.admin_password)
    {
        let user_exists = sqlx::query("SELECT 1 FROM users WHERE email = $1")
            .bind(admin_email)
            .fetch_optional(&db_pool)
            .await?;

        if user_exists.is_none() {
            info!("Creating initial admin user: {}", admin_email);
            let password_hash = rustchat::auth::hash_password(admin_password)?;
            let username = admin_email.split('@').next().unwrap_or("admin");

            sqlx::query(
                r#"
                INSERT INTO users (username, email, password_hash, role, is_active, display_name)
                VALUES ($1, $2, $3, 'system_admin', true, 'System Admin')
                "#,
            )
            .bind(username)
            .bind(admin_email)
            .bind(password_hash)
            .execute(&db_pool)
            .await?;

            info!("Admin user created successfully");
        } else {
            info!("Admin user already exists, skipping creation");
        }
    }

    // Create WebSocket hub
    let ws_hub = WsHub::new();
    info!("WebSocket hub initialized");

    // Initialize Redis Pool
    let redis_cfg = deadpool_redis::Config::from_url(&config.redis_url);
    let redis_pool = redis_cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1))?;
    info!("Redis pool initialized");

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
        redis_pool,
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