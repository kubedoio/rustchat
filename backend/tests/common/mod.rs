use once_cell::sync::Lazy;
use rustchat::{api, realtime::WsHub, storage::S3Client};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

// Ensure tracing is initialized only once
static TRACING: Lazy<()> = Lazy::new(|| {
    let log_level = "info";
    // We just call init regardless of TEST_LOG for now, as init() sets global default.
    // In a real scenario we might want to separate subscribers for stdout vs sink.
    rustchat::telemetry::init(log_level);
});

pub struct TestApp {
    pub address: String,
    #[allow(dead_code)]
    pub db_pool: PgPool,
    pub api_client: reqwest::Client,
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let db_url = std::env::var("RUSTCHAT_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://rustchat:rustchat@localhost:5432/rustchat".to_string());

    // Configure database
    let db_pool = configure_database(&db_url).await;

    // Create a random socket address
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    // Initialize dependencies
    let ws_hub = WsHub::new();

    // Dummy S3 client
    let s3_client = S3Client::new(
        Some("http://localhost:9000".to_string()),
        None,
        "test-bucket".to_string(),
        Some("minioadmin".to_string()),
        Some("minioadmin".to_string()),
        "us-east-1".to_string(),
    );

    s3_client
        .ensure_bucket()
        .await
        .expect("Failed to create test bucket");

    let jwt_secret = Uuid::new_v4().to_string();
    let jwt_expiry_hours = 1;

    // Initialize Redis
    let redis_cfg = deadpool_redis::Config::from_url("redis://localhost:6379/");
    let redis_pool = redis_cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Failed to create Redis pool");

    let app = api::router(
        db_pool.clone(),
        redis_pool,
        jwt_secret,
        jwt_expiry_hours,
        ws_hub,
        s3_client,
    );

    let server = axum::serve(listener, app);
    tokio::spawn(async move {
        server.await.expect("Failed to run server");
    });

    TestApp {
        address,
        db_pool,
        api_client: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .cookie_store(true)
            .build()
            .unwrap(),
    }
}

async fn configure_database(database_url: &str) -> PgPool {
    let random_db_name = Uuid::new_v4().to_string();

    // Split URL to get base connection without DB name
    let last_slash = database_url.rfind('/').expect("Invalid database URL");
    let base_url = &database_url[..last_slash];
    // Connect to postgres DB to create new DB
    let maintenance_url = format!("{}/postgres", base_url);

    let mut connection = PgConnection::connect(&maintenance_url)
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}""#, random_db_name).as_str())
        .await
        .expect("Failed to create database");

    // Migrate database
    let new_db_url = format!("{}/{}", base_url, random_db_name);
    let pool = PgPool::connect(&new_db_url)
        .await
        .expect("Failed to connect to new database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate database");

    pool
}
