use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use rustchat::{api::router, realtime::WsHub, storage::S3Client};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

#[tokio::test]
async fn config_client_returns_diagnostic_id() {
    // 1. Create dummy state
    let db = PgPoolOptions::new()
        .connect_lazy("postgres://fake:fake@localhost:5432/fake")
        .expect("Failed to create lazy pool");

    let redis_cfg = deadpool_redis::Config::default();
    let redis = redis_cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1)).unwrap();

    let ws_hub = WsHub::new();

    let s3_client = S3Client::new(
        Some("http://localhost:9000".to_string()),
        "test".to_string(),
        Some("a".to_string()),
        Some("s".to_string()),
        "us-east-1".to_string(),
    );

    // 2. Build router using public api
    let app = router(
        db,
        redis,
        "secret".to_string(),
        1,
        ws_hub,
        s3_client
    );

    // 3. Make request
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v4/config/client")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Check for DiagnosticId
    let diagnostic_id = body.get("DiagnosticId");
    assert!(diagnostic_id.is_some(), "DiagnosticId field is missing");
    let diagnostic_id_str = diagnostic_id.unwrap().as_str();
    assert!(diagnostic_id_str.is_some(), "DiagnosticId is not a string");
    assert!(!diagnostic_id_str.unwrap().is_empty(), "DiagnosticId is empty");
}
