#![allow(dead_code)]
use once_cell::sync::Lazy;
use rustchat::{
    api,
    config::Config,
    models::{
        AgentChannelSettings, AgentConfig, Channel, ChannelType, KnowledgeBase, KnowledgeDocument,
        Organization, Team, User,
    },
    realtime::WsHub,
    services::{agent_runtime::AgentRuntime, llm::ProviderRegistry},
    storage::S3Client,
};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::HeaderValue, Message},
};

pub type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

// Ensure tracing is initialized only once
static TRACING: Lazy<()> = Lazy::new(|| {
    let log_level = "info";
    // We just call init regardless of TEST_LOG for now, as init() sets global default.
    // In a real scenario we might want to separate subscribers for stdout vs sink.
    rustchat::telemetry::init(log_level);
});

#[allow(dead_code)]
pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub redis_pool: deadpool_redis::Pool,
    pub api_client: reqwest::Client,
}

impl TestApp {
    pub async fn connect_ws_v4(&self, token: &str) -> WsStream {
        let ws_base = self.address.replacen("http://", "ws://", 1);
        let ws_url = format!("{ws_base}/api/v4/websocket");

        let mut request = ws_url
            .into_client_request()
            .expect("websocket request should be valid");
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::from_str(token).expect("valid websocket subprotocol token"),
        );

        let (ws_stream, _) = connect_async(request)
            .await
            .expect("websocket connection should succeed");
        ws_stream
    }

    pub async fn wait_for_event(
        &self,
        ws: &mut WsStream,
        expected_event: &str,
        timeout_ms: u64,
    ) -> serde_json::Value {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);

        loop {
            let now = std::time::Instant::now();
            assert!(
                now < deadline,
                "timed out waiting for websocket event {expected_event}"
            );

            let timeout_left = deadline.saturating_duration_since(now);
            let message = tokio::time::timeout(timeout_left, ws.next())
                .await
                .expect("timeout while waiting for websocket frame")
                .expect("websocket closed unexpectedly")
                .expect("websocket frame should be valid");

            if let Message::Text(text) = message {
                let parsed: serde_json::Value =
                    serde_json::from_str(&text).expect("frame should be valid JSON");
                if parsed["event"] == expected_event {
                    return parsed["data"].clone();
                }
            }
        }
    }

    pub async fn send_ws_command(&self, ws: &mut WsStream, command: &str, data: serde_json::Value) {
        let payload = serde_json::json!({
            "type": "command",
            "event": command,
            "data": data,
        });
        ws.send(Message::Text(payload.to_string()))
            .await
            .expect("websocket command should be sent");
    }
}

#[allow(dead_code)]
pub async fn spawn_app() -> TestApp {
    spawn_app_with_config(test_config()).await
}

pub async fn spawn_app_with_config(config: Config) -> TestApp {
    dotenvy::dotenv().ok();
    Lazy::force(&TRACING);

    // Configure database using explicit test URL first, then known local fallbacks.
    let db_pool = configure_database_with_fallback(&collect_test_database_urls()).await;

    // Create a random socket address
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    // Initialize dependencies
    let ws_hub = WsHub::new();

    let s3_endpoint = std::env::var("RUSTCHAT_TEST_S3_ENDPOINT")
        .or_else(|_| std::env::var("RUSTCHAT_S3_ENDPOINT"))
        .unwrap_or_else(|_| "http://localhost:9000".to_string());
    let s3_access_key = std::env::var("RUSTCHAT_TEST_S3_ACCESS_KEY")
        .or_else(|_| std::env::var("RUSTCHAT_S3_ACCESS_KEY"))
        .unwrap_or_else(|_| "testaccesskey".to_string());
    let s3_secret_key = std::env::var("RUSTCHAT_TEST_S3_SECRET_KEY")
        .or_else(|_| std::env::var("RUSTCHAT_S3_SECRET_KEY"))
        .unwrap_or_else(|_| "testsecretkey".to_string());
    let s3_bucket = std::env::var("RUSTCHAT_TEST_S3_BUCKET")
        .or_else(|_| std::env::var("RUSTCHAT_S3_BUCKET"))
        .unwrap_or_else(|_| "test-bucket".to_string());

    let s3_client = S3Client::new(
        Some(s3_endpoint),
        None,
        s3_bucket,
        Some(s3_access_key),
        Some(s3_secret_key),
        "us-east-1".to_string(),
    );

    if let Err(err) = s3_client.ensure_bucket().await {
        tracing::debug!(
            error = %err,
            "Failed to create test bucket; continuing test bootstrap"
        );
    }

    let jwt_secret = Uuid::new_v4().to_string();
    let jwt_expiry_hours = 1;

    // Initialize Redis using explicit test URL first, then known local fallbacks.
    let redis_pool = configure_redis_with_fallback(&collect_test_redis_urls()).await;

    let (app, _state) = api::router(
        db_pool.clone(),
        redis_pool.clone(),
        jwt_secret,
        jwt_expiry_hours,
        ws_hub,
        s3_client,
        config,
        tokio_util::sync::CancellationToken::new(),
    );

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    );
    tokio::spawn(async move {
        server.await.expect("Failed to run server");
    });

    TestApp {
        address,
        db_pool,
        redis_pool,
        api_client: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .cookie_store(true)
            .build()
            .unwrap(),
    }
}

pub fn test_config() -> Config {
    Config {
        environment: "test".to_string(),
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        database_url: "postgres://rustchat:rustchat@localhost:5432/rustchat".to_string(),
        db_pool: Default::default(),
        redis_url: "redis://localhost:6379/".to_string(),
        require_cluster_fanout: false,
        jwt_secret: "test-secret".to_string(),
        jwt_issuer: None,
        jwt_audience: None,
        encryption_key: "test-encryption-key".to_string(),
        jwt_expiry_hours: 1,
        log_level: "info".to_string(),
        s3_endpoint: Some("http://localhost:9000".to_string()),
        s3_public_endpoint: None,
        s3_bucket: "test-bucket".to_string(),
        s3_access_key: Some("testaccesskey".to_string()),
        s3_secret_key: Some("testsecretkey".to_string()),
        s3_region: "us-east-1".to_string(),
        admin_user: None,
        admin_password: None,
        cors_allowed_origins: None,
        allow_dev_cors: false,
        turnstile: Default::default(),
        calls: Default::default(),
        security: rustchat::config::SecurityConfig {
            rate_limit_enabled: false,
            ..Default::default()
        },
        keycloak_sync: Default::default(),
        messaging: Default::default(),
        unread: Default::default(),
        compatibility: rustchat::config::CompatibilityConfig {
            mobile_sso_code_exchange: true,
        },
    }
}

fn collect_test_database_urls() -> Vec<String> {
    let mut urls = Vec::new();

    for env_key in [
        "RUSTCHAT_TEST_DATABASE_URL",
        "RUSTCHAT_DATABASE_URL",
        "DATABASE_URL",
    ] {
        if let Ok(url) = std::env::var(env_key) {
            let trimmed = url.trim();
            if !trimmed.is_empty() && !urls.iter().any(|existing| existing == trimmed) {
                urls.push(trimmed.to_string());
            }
        }
    }

    for fallback in [
        "postgres://rustchat:rustchat@127.0.0.1:55432/rustchat",
        "postgres://rustchat:rustchat@localhost:5432/rustchat",
        "postgres://postgres:postgres@localhost:5432/postgres",
        "postgres://postgres@localhost:5432/postgres",
    ] {
        if !urls.iter().any(|existing| existing == fallback) {
            urls.push(fallback.to_string());
        }
    }

    urls
}

async fn configure_database_with_fallback(candidates: &[String]) -> PgPool {
    let mut failures = Vec::new();

    for candidate in candidates {
        match configure_database(candidate).await {
            Ok(pool) => {
                tracing::info!(
                    database_url = %redact_url(candidate),
                    "Using PostgreSQL test bootstrap URL"
                );
                return pool;
            }
            Err(err) => {
                failures.push(format!("{} => {}", redact_url(candidate), err));
            }
        }
    }

    panic!(
        "Failed to bootstrap PostgreSQL for integration tests.\n\
Set RUSTCHAT_TEST_DATABASE_URL to a superuser-capable database URL.\n\
Tried:\n{}",
        failures.join("\n")
    );
}

fn collect_test_redis_urls() -> Vec<String> {
    let mut urls = Vec::new();

    for env_key in ["RUSTCHAT_TEST_REDIS_URL", "RUSTCHAT_REDIS_URL", "REDIS_URL"] {
        if let Ok(url) = std::env::var(env_key) {
            let trimmed = url.trim();
            if !trimmed.is_empty() && !urls.iter().any(|existing| existing == trimmed) {
                urls.push(trimmed.to_string());
            }
        }
    }

    for fallback in ["redis://127.0.0.1:56379/", "redis://localhost:6379/"] {
        if !urls.iter().any(|existing| existing == fallback) {
            urls.push(fallback.to_string());
        }
    }

    urls
}

async fn configure_redis_with_fallback(candidates: &[String]) -> deadpool_redis::Pool {
    let mut failures = Vec::new();

    for candidate in candidates {
        let redis_cfg = deadpool_redis::Config::from_url(candidate.to_string());
        let pool = match redis_cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1)) {
            Ok(pool) => pool,
            Err(err) => {
                failures.push(format!("{} => {}", redact_url(candidate), err));
                continue;
            }
        };

        let mut conn = match pool.get().await {
            Ok(conn) => conn,
            Err(err) => {
                failures.push(format!("{} => {}", redact_url(candidate), err));
                continue;
            }
        };

        match deadpool_redis::redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
        {
            Ok(reply) if reply.eq_ignore_ascii_case("PONG") => {
                tracing::info!(redis_url = %redact_url(candidate), "Using Redis test URL");
                return pool;
            }
            Ok(reply) => {
                failures.push(format!(
                    "{} => unexpected PING reply {}",
                    redact_url(candidate),
                    reply
                ));
            }
            Err(err) => {
                failures.push(format!("{} => {}", redact_url(candidate), err));
            }
        }
    }

    panic!(
        "Failed to bootstrap Redis for integration tests.\n\
Set RUSTCHAT_TEST_REDIS_URL to a reachable redis URL.\n\
Tried:\n{}",
        failures.join("\n")
    );
}

fn redact_url(database_url: &str) -> String {
    let mut redacted = database_url.to_string();
    if let Some(scheme_end) = redacted.find("://") {
        let auth_start = scheme_end + 3;
        if let Some(at_rel) = redacted[auth_start..].find('@') {
            let at = auth_start + at_rel;
            if let Some(colon_rel) = redacted[auth_start..at].find(':') {
                let colon = auth_start + colon_rel;
                redacted.replace_range((colon + 1)..at, "***");
            }
        }
    }
    redacted
}

async fn configure_database(database_url: &str) -> Result<PgPool, String> {
    let random_db_name = Uuid::new_v4().to_string();

    // Split URL to get base connection without DB name
    let last_slash = database_url
        .rfind('/')
        .ok_or_else(|| format!("invalid database URL: {}", redact_url(database_url)))?;
    let base_url = &database_url[..last_slash];
    // Connect to postgres DB to create new DB
    let maintenance_url = format!("{}/postgres", base_url);

    let mut connection = PgConnection::connect(&maintenance_url)
        .await
        .map_err(|err| {
            format!(
                "failed maintenance connection ({}): {}",
                redact_url(&maintenance_url),
                err
            )
        })?;

    connection
        .execute(format!(r#"CREATE DATABASE "{}""#, random_db_name).as_str())
        .await
        .map_err(|err| format!("failed to create database {}: {}", random_db_name, err))?;

    // Migrate database
    let new_db_url = format!("{}/{}", base_url, random_db_name);
    let pool = PgPool::connect(&new_db_url).await.map_err(|err| {
        format!(
            "failed to connect new database ({}): {}",
            redact_url(&new_db_url),
            err
        )
    })?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|err| format!("failed to migrate database {}: {}", random_db_name, err))?;

    Ok(pool)
}

/// Create a minimal AppState for testing extractors
#[allow(dead_code)]
pub async fn create_test_state(pool: PgPool) -> anyhow::Result<rustchat::api::AppState> {
    dotenvy::dotenv().ok();
    let redis_pool = configure_redis_with_fallback(&collect_test_redis_urls()).await;
    let config = test_config();
    let ws_hub = WsHub::new();
    let s3_client = S3Client::new(
        Some("http://localhost:9000".to_string()),
        None,
        "test-bucket".to_string(),
        Some("testaccesskey".to_string()),
        Some("testsecretkey".to_string()),
        "us-east-1".to_string(),
    );

    let jwt_secret = Uuid::new_v4().to_string();
    let jwt_expiry_hours = 1;

    // Build a temporary router to get properly initialized managers
    // This is cleaner than trying to construct them directly
    let (_temp_router, _temp_state) = rustchat::api::router(
        pool.clone(),
        redis_pool.clone(),
        jwt_secret.clone(),
        jwt_expiry_hours,
        ws_hub.clone(),
        s3_client.clone(),
        config.clone(),
        tokio_util::sync::CancellationToken::new(),
    );

    // Extract state from the router
    // The router construction already created all the necessary managers
    // We'll create a new state that matches what the router has
    Ok(rustchat::api::AppState {
        db: pool,
        redis: redis_pool.clone(),
        jwt_secret,
        jwt_issuer: config.jwt_issuer.clone(),
        jwt_audience: config.jwt_audience.clone(),
        jwt_expiry_hours,
        ws_hub,
        connection_store: rustchat::realtime::ConnectionStore::new(
            tokio_util::sync::CancellationToken::new(),
        ),
        s3_client,
        http_client: reqwest::Client::new(),
        start_time: std::time::Instant::now(),
        config: config.clone(),
        // Extract from router's state - but since we can't access it directly,
        // we'll just drop the router and create dummy managers that won't be used
        // The extractor tests don't need SFU or call state functionality
        sfu_manager: {
            let (voice_tx, _) = tokio::sync::mpsc::channel(1);
            use rustchat::api::v4::calls_plugin::sfu::SFUManager;
            SFUManager::new(config.calls.clone(), voice_tx)
        },
        call_state_manager: {
            use rustchat::api::v4::calls_plugin::state::{CallStateBackend, CallStateManager};
            std::sync::Arc::new(CallStateManager::with_backend(
                Some(redis_pool.clone()),
                CallStateBackend::parse(&config.calls.state_backend),
            ))
        },
        circuit_breakers: std::sync::Arc::new(
            rustchat::middleware::reliability::ServiceCircuitBreakers::new(),
        ),
        reconciliation_tx: None,
        agent_runtime: None,
        shutdown: tokio_util::sync::CancellationToken::new(),
    })
}

/// Create a minimal AppState with an injected agent runtime.
#[allow(dead_code)]
pub async fn create_test_state_with_agent_runtime(
    pool: PgPool,
    provider_registry: Arc<ProviderRegistry>,
) -> anyhow::Result<rustchat::api::AppState> {
    let mut state = create_test_state(pool).await?;
    state.agent_runtime = Some(Arc::new(AgentRuntime::new(
        state.db.clone(),
        state.ws_hub.clone(),
        provider_registry,
        None,
        None,
        None,
    )));
    Ok(state)
}

#[allow(dead_code)]
pub async fn create_test_organization(db: &PgPool, name: &str) -> Organization {
    let slug = unique_slug(name);
    sqlx::query_as::<_, Organization>(
        r#"
        INSERT INTO organizations (name, display_name, description)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(&slug)
    .bind(name)
    .bind("Test organization")
    .fetch_one(db)
    .await
    .expect("test organization should be created")
}

#[allow(dead_code)]
pub async fn create_test_user(db: &PgPool, username: &str) -> User {
    let org = create_test_organization(db, &format!("{username}-org")).await;
    create_test_user_in_org(db, org.id, username, "member").await
}

#[allow(dead_code)]
pub async fn create_test_admin_user(db: &PgPool, username: &str) -> User {
    let org = create_test_organization(db, &format!("{username}-org")).await;
    create_test_user_in_org(db, org.id, username, "system_admin").await
}

#[allow(dead_code)]
pub async fn create_test_user_in_org(
    db: &PgPool,
    org_id: Uuid,
    username: &str,
    role: &str,
) -> User {
    let slug = unique_slug(username);
    let email = format!("{slug}@example.test");
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (
            org_id, username, email, password_hash, display_name, is_bot, is_active, role,
            entity_type, entity_metadata, rate_limit_tier, email_verified
        )
        VALUES ($1, $2, $3, 'test-password-hash', $4, false, true, $5, 'human', '{}'::jsonb, 'human_standard', true)
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(&slug)
    .bind(&email)
    .bind(username)
    .bind(role)
    .fetch_one(db)
    .await
    .expect("test user should be created")
}

#[allow(dead_code)]
pub async fn create_test_team(db: &PgPool, owner: &User, name: &str) -> Team {
    let slug = unique_slug(name);
    let team = sqlx::query_as::<_, Team>(
        r#"
        INSERT INTO teams (org_id, name, display_name, description)
        VALUES ($1, $2, $3, 'Test team')
        RETURNING *
        "#,
    )
    .bind(owner.org_id.expect("test owner should have an org"))
    .bind(&slug)
    .bind(name)
    .fetch_one(db)
    .await
    .expect("test team should be created");

    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'team_admin')")
        .bind(team.id)
        .bind(owner.id)
        .execute(db)
        .await
        .expect("test team owner should be added");

    team
}

#[allow(dead_code)]
pub async fn create_test_channel(db: &PgPool, team: &Team, creator: &User, name: &str) -> Channel {
    let slug = unique_slug(name);
    let channel = sqlx::query_as::<_, Channel>(
        r#"
        INSERT INTO channels (team_id, type, name, display_name, creator_id)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(team.id)
    .bind(ChannelType::Public)
    .bind(&slug)
    .bind(name)
    .bind(creator.id)
    .fetch_one(db)
    .await
    .expect("test channel should be created");

    sqlx::query(
        "INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, 'channel_admin')",
    )
    .bind(channel.id)
    .bind(creator.id)
    .execute(db)
    .await
    .expect("test channel creator should be added");

    channel
}

#[allow(dead_code)]
pub async fn create_test_agent(db: &PgPool, name: &str) -> AgentConfig {
    let creator = create_test_admin_user(db, &format!("{name}-creator")).await;
    create_test_agent_for_creator(db, &creator, name).await
}

#[allow(dead_code)]
pub async fn create_test_agent_for_creator(db: &PgPool, creator: &User, name: &str) -> AgentConfig {
    let slug = unique_slug(name);
    let email = format!("{slug}-agent@example.test");
    let agent_user: User = sqlx::query_as(
        r#"
        INSERT INTO users (
            org_id, username, email, password_hash, display_name, is_bot, is_active, role,
            entity_type, entity_metadata, rate_limit_tier, email_verified
        )
        VALUES ($1, $2, $3, NULL, $4, true, true, 'member', 'agent', '{}'::jsonb, 'agent_high', true)
        RETURNING *
        "#,
    )
    .bind(creator.org_id)
    .bind(&slug)
    .bind(&email)
    .bind(name)
    .fetch_one(db)
    .await
    .expect("test agent user should be created");

    sqlx::query_as::<_, AgentConfig>(
        r#"
        INSERT INTO agent_configs (
            user_id, title, description, system_prompt, provider, model,
            temperature, max_context_messages, max_output_tokens, capabilities,
            rag_enabled, rag_top_k, is_active, created_by
        )
        VALUES (
            $1, $2, 'Test agent', 'You are a deterministic test agent.', 'mock', 'mock-model',
            0.0, 10, 256,
            '{"respond_to_mentions": true, "respond_to_all": false, "use_memory": true, "use_rag": false}'::jsonb,
            false, 5, true, $3
        )
        RETURNING *
        "#,
    )
    .bind(agent_user.id)
    .bind(name)
    .bind(creator.id)
    .fetch_one(db)
    .await
    .expect("test agent config should be created")
}

#[allow(dead_code)]
pub async fn create_test_knowledge_base(db: &PgPool, name: &str) -> KnowledgeBase {
    let owner = create_test_admin_user(db, &format!("{name}-owner")).await;
    let team = create_test_team(db, &owner, &format!("{name}-team")).await;
    create_test_knowledge_base_for_team(db, &team, &owner, name).await
}

#[allow(dead_code)]
pub async fn create_test_knowledge_base_for_team(
    db: &PgPool,
    team: &Team,
    created_by: &User,
    name: &str,
) -> KnowledgeBase {
    sqlx::query_as::<_, KnowledgeBase>(
        r#"
        INSERT INTO knowledge_bases (
            team_id, name, description, embedding_model, embedding_dimensions,
            chunk_size, chunk_overlap, is_active, created_by
        )
        VALUES ($1, $2, 'Test knowledge base', 'text-embedding-3-small', 1536, 512, 50, true, $3)
        RETURNING *
        "#,
    )
    .bind(team.id)
    .bind(name)
    .bind(created_by.id)
    .fetch_one(db)
    .await
    .expect("test knowledge base should be created")
}

#[allow(dead_code)]
pub async fn create_test_document(db: &PgPool, kb_id: Uuid, filename: &str) -> KnowledgeDocument {
    let kb: KnowledgeBase = sqlx::query_as("SELECT * FROM knowledge_bases WHERE id = $1")
        .bind(kb_id)
        .fetch_one(db)
        .await
        .expect("test knowledge base should exist");
    create_test_document_for_kb(db, &kb, kb.created_by, filename).await
}

#[allow(dead_code)]
pub async fn create_test_document_for_kb(
    db: &PgPool,
    kb: &KnowledgeBase,
    created_by: Uuid,
    filename: &str,
) -> KnowledgeDocument {
    let document_id = Uuid::new_v4();
    sqlx::query_as::<_, KnowledgeDocument>(
        r#"
        INSERT INTO knowledge_documents (
            id, knowledge_base_id, team_id, title, source_type, s3_key, s3_bucket,
            content_hash, mime_type, size_bytes, extracted_text, is_indexed,
            chunk_count, created_by
        )
        VALUES (
            $1, $2, $3, $4, 'upload', $5, 'test-bucket',
            $6, 'text/plain', 12, 'test document content', false, 0, $7
        )
        RETURNING *
        "#,
    )
    .bind(document_id)
    .bind(kb.id)
    .bind(kb.team_id)
    .bind(filename)
    .bind(format!("test-documents/{document_id}/{filename}"))
    .bind(format!("{:064x}", document_id.as_u128()))
    .bind(created_by)
    .fetch_one(db)
    .await
    .expect("test knowledge document should be created")
}

#[allow(dead_code)]
pub async fn create_test_agent_channel_setting(
    db: &PgPool,
    agent_id: Uuid,
    channel_id: Uuid,
) -> AgentChannelSettings {
    sqlx::query(
        "INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, 'member') ON CONFLICT DO NOTHING",
    )
    .bind(channel_id)
    .bind(agent_id)
    .execute(db)
    .await
    .expect("test agent should be added to channel members");

    sqlx::query_as::<_, AgentChannelSettings>(
        r#"
        INSERT INTO agent_channel_settings (agent_id, channel_id, is_active)
        VALUES ($1, $2, true)
        ON CONFLICT (agent_id, channel_id)
        DO UPDATE SET is_active = EXCLUDED.is_active
        RETURNING *
        "#,
    )
    .bind(agent_id)
    .bind(channel_id)
    .fetch_one(db)
    .await
    .expect("test agent channel setting should be created")
}

fn unique_slug(input: &str) -> String {
    let normalized: String = input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = normalized.trim_matches('-');
    let base = if trimmed.is_empty() { "test" } else { trimmed };
    let max_base_len = 40.min(base.len());
    format!("{}-{}", &base[..max_base_len], Uuid::new_v4().simple())
        .chars()
        .take(64)
        .collect()
}
