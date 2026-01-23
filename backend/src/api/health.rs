//! Health check endpoints
//!
//! Provides liveness and readiness probes for Kubernetes/Docker.

use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use serde::Serialize;

use super::AppState;
use crate::db;

#[derive(Serialize)]
pub struct LivenessResponse {
    status: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
pub struct ReadinessResponse {
    status: &'static str,
    database: &'static str,
}

/// Build health check routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/live", get(liveness))
        .route("/ready", get(readiness))
}

/// Liveness probe - checks if the application is running
async fn liveness() -> Json<LivenessResponse> {
    Json(LivenessResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Readiness probe - checks if dependencies are available
async fn readiness(State(state): State<AppState>) -> Json<ReadinessResponse> {
    let db_healthy = db::health_check(&state.db).await;

    Json(ReadinessResponse {
        status: if db_healthy { "ok" } else { "degraded" },
        database: if db_healthy { "connected" } else { "disconnected" },
    })
}
