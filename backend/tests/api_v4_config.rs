#![allow(clippy::needless_borrows_for_generic_args)]
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use rustchat::{api::router, config::Config, realtime::WsHub, storage::S3Client};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

#[tokio::test]
async fn config_client_returns_diagnostic_id() {
    // 1. Create dummy state
    let db = PgPoolOptions::new()
        .connect_lazy("postgres://fake:fake@localhost:5432/fake")
        .expect("Failed to create lazy pool");

    let redis_cfg = deadpool_redis::Config::default();
    let redis = redis_cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .unwrap();

    let ws_hub = WsHub::new();

    let s3_client = S3Client::new(
        Some("http://localhost:9000".to_string()),
        None,
        "test".to_string(),
        Some("a".to_string()),
        Some("s".to_string()),
        "us-east-1".to_string(),
    );

    // 2. Build router using public api
    let (app, _state) = router(
        db,
        redis,
        "secret".to_string(),
        1,
        ws_hub,
        s3_client,
        test_config(),
        tokio_util::sync::CancellationToken::new(),
    );

    // 3. Make request
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v4/config/client?format=old")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Check for DiagnosticId
    let diagnostic_id = body.get("DiagnosticId");
    assert!(diagnostic_id.is_some(), "DiagnosticId field is missing");
    let diagnostic_id_str = diagnostic_id.unwrap().as_str();
    assert!(diagnostic_id_str.is_some(), "DiagnosticId is not a string");
    assert!(
        !diagnostic_id_str.unwrap().is_empty(),
        "DiagnosticId is empty"
    );
}

#[tokio::test]
async fn license_client_returns_boolean() {
    // 1. Create dummy state
    let db = PgPoolOptions::new()
        .connect_lazy("postgres://fake:fake@localhost:5432/fake")
        .expect("Failed to create lazy pool");

    let redis_cfg = deadpool_redis::Config::default();
    let redis = redis_cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .unwrap();

    let ws_hub = WsHub::new();

    let s3_client = S3Client::new(
        Some("http://localhost:9000".to_string()),
        None,
        "test".to_string(),
        Some("a".to_string()),
        Some("s".to_string()),
        "us-east-1".to_string(),
    );

    // 2. Build router using public api
    let (app, _state) = router(
        db,
        redis,
        "secret".to_string(),
        1,
        ws_hub,
        s3_client,
        test_config(),
        tokio_util::sync::CancellationToken::new(),
    );

    // 3. Make request
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v4/license/client")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Check for IsLicensed
    let is_licensed = body.get("IsLicensed");
    assert!(is_licensed.is_some(), "IsLicensed field is missing");

    // This assertion should fail if it returns a string
    assert!(
        is_licensed.unwrap().is_boolean(),
        "IsLicensed is not a boolean: {:?}",
        is_licensed.unwrap()
    );
    assert_eq!(
        is_licensed.unwrap().as_bool(),
        Some(true),
        "IsLicensed is not true"
    );
}

fn test_config() -> Config {
    Config {
        environment: "test".to_string(),
        server_host: "127.0.0.1".to_string(),
        server_port: 3000,
        database_url: "postgres://fake:fake@localhost:5432/fake".to_string(),
        db_pool: Default::default(),
        redis_url: "redis://localhost:6379/".to_string(),
        require_cluster_fanout: false,
        jwt_secret: "secret".to_string(),
        jwt_issuer: None,
        jwt_audience: None,
        encryption_key: "test-encryption-key".to_string(),
        jwt_expiry_hours: 1,
        log_level: "info".to_string(),
        s3_endpoint: Some("http://localhost:9000".to_string()),
        s3_public_endpoint: None,
        s3_bucket: "test".to_string(),
        s3_access_key: Some("a".to_string()),
        s3_secret_key: Some("s".to_string()),
        s3_region: "us-east-1".to_string(),
        admin_user: None,
        admin_password: None,
        cors_allowed_origins: None,
        allow_dev_cors: false,
        turnstile: Default::default(),
        calls: Default::default(),
        security: Default::default(),
        keycloak_sync: Default::default(),
        messaging: Default::default(),
        unread: Default::default(),
        compatibility: rustchat::config::CompatibilityConfig {
            mobile_sso_code_exchange: true,
        },
    }
}
