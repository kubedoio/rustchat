#![allow(clippy::needless_borrows_for_generic_args)]
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use rustchat::api::build_router;
use tower::ServiceExt;

#[tokio::test]
async fn config_client_returns_diagnostic_id() {
    // Build a pure router over a minimal test state (no workers spawned).
    let app = build_router(rustchat::testsupport::minimal_app_state());

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
    let app = build_router(rustchat::testsupport::minimal_app_state());

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
    assert!(
        is_licensed.unwrap().as_bool().unwrap_or(false),
        "IsLicensed is not true"
    );
}
