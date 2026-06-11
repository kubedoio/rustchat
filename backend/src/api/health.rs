//! Health check endpoints
//!
//! Provides liveness and readiness probes for Kubernetes/Docker,
//! plus metrics endpoint for Prometheus.

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use std::collections::HashMap;

use super::AppState;
use crate::db;
use crate::db::EXPECTED_SCHEMA_VERSION;
use crate::telemetry::metrics;

#[derive(Serialize)]
pub struct LivenessResponse {
    status: &'static str,
    version: &'static str,
    uptime_seconds: u64,
}

#[derive(Serialize)]
pub struct ReadinessResponse {
    status: &'static str,
    database: &'static str,
    redis: &'static str,
    s3: &'static str,
    checks: HashMap<String, String>,
}

#[derive(Serialize)]
pub struct MetricsResponse {
    websocket_connections: i64,
    active_users: i64,
    db_pool_saturation: f64,
}

/// Build health check routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/live", get(liveness))
        .route("/ready", get(readiness))
        .route("/metrics", get(metrics_endpoint))
        .route("/stats", get(stats))
}

/// Liveness probe - checks if the application is running
async fn liveness(State(state): State<AppState>) -> Json<LivenessResponse> {
    let uptime = state.start_time.elapsed().as_secs();

    Json(LivenessResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime,
    })
}

/// Readiness probe - checks if dependencies are available
async fn readiness(State(state): State<AppState>) -> Result<Json<ReadinessResponse>, StatusCode> {
    let mut checks = HashMap::new();

    // Check database
    let db_healthy = db::health_check(&state.db).await;
    checks.insert(
        "database".to_string(),
        if db_healthy {
            "ok".to_string()
        } else {
            "error".to_string()
        },
    );

    // Check Redis
    let redis_healthy = check_redis(&state.redis).await;
    checks.insert(
        "redis".to_string(),
        if redis_healthy {
            "ok".to_string()
        } else {
            "error".to_string()
        },
    );

    // WebSocket hub process-local health detail (informational)
    let ws_connections = state.ws_hub.count_connections().await;
    checks.insert(
        "websocket_hub".to_string(),
        format!("ok(connections={})", ws_connections),
    );

    // Check object storage
    let s3_healthy = state.s3_client.health_check().await;
    checks.insert(
        "s3".to_string(),
        if s3_healthy {
            "ok".to_string()
        } else {
            "error".to_string()
        },
    );

    // Check migration state
    let migration_healthy =
        match sqlx::query_scalar::<_, i64>("SELECT MAX(version) FROM _sqlx_migrations")
            .fetch_optional(&state.db)
            .await
        {
            Ok(Some(max_version)) => max_version == EXPECTED_SCHEMA_VERSION,
            Ok(None) => false,
            Err(err) => {
                tracing::warn!(error = %err, "Failed to query migration state");
                false
            }
        };
    checks.insert(
        "migrations".to_string(),
        if migration_healthy {
            "ok".to_string()
        } else {
            "pending".to_string()
        },
    );

    // Check email outbox lag/pressure (operational readiness signal).
    let outbox_healthy = match check_email_outbox_pressure(&state.db).await {
        Ok((queued_old, oldest_age_secs)) => {
            let healthy = queued_old < 1000 && oldest_age_secs.unwrap_or(0) < 3600;
            checks.insert(
                "email_outbox".to_string(),
                if healthy {
                    format!(
                        "ok(queued_old={}, oldest_age_secs={})",
                        queued_old,
                        oldest_age_secs.unwrap_or(0)
                    )
                } else {
                    format!(
                        "degraded(queued_old={}, oldest_age_secs={})",
                        queued_old,
                        oldest_age_secs.unwrap_or(0)
                    )
                },
            );
            healthy
        }
        Err(err) => {
            tracing::warn!(error = %err, "Failed to evaluate email outbox readiness");
            checks.insert("email_outbox".to_string(), "error".to_string());
            false
        }
    };

    let all_healthy =
        db_healthy && redis_healthy && s3_healthy && outbox_healthy && migration_healthy;

    let response = ReadinessResponse {
        status: if all_healthy { "ok" } else { "degraded" },
        database: if db_healthy {
            "connected"
        } else {
            "disconnected"
        },
        redis: if redis_healthy {
            "connected"
        } else {
            "disconnected"
        },
        s3: if s3_healthy {
            "connected"
        } else {
            "disconnected"
        },
        checks,
    };

    if all_healthy {
        Ok(Json(response))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// Prometheus metrics endpoint
async fn metrics_endpoint() -> impl IntoResponse {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::gather();

    match encoder.encode_to_string(&metric_families) {
        Ok(metrics) => (StatusCode::OK, metrics),
        Err(e) => {
            tracing::error!("Failed to encode metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to encode metrics".to_string(),
            )
        }
    }
}

/// Internal stats endpoint (JSON format)
async fn stats(State(state): State<AppState>) -> Json<MetricsResponse> {
    let ws_connections = state.ws_hub.count_connections().await as i64;
    let active_users = state.ws_hub.online_users().await.len() as i64;

    let max_connections = state.config.db_pool.max_connections.max(1) as f64;
    let current_size = state.db.size() as f64;
    let idle = state.db.num_idle() as f64;
    let in_use = (current_size - idle).max(0.0);
    let db_pool_saturation = (in_use / max_connections).clamp(0.0, 1.0);

    // Update gauges
    metrics::WS_ACTIVE_CONNECTIONS.set(ws_connections);
    metrics::ACTIVE_USERS.set(active_users);
    metrics::DB_CONNECTIONS_ACTIVE.set(in_use as i64);
    metrics::DB_POOL_SATURATION.set(db_pool_saturation);

    Json(MetricsResponse {
        websocket_connections: ws_connections,
        active_users,
        db_pool_saturation,
    })
}

/// Check Redis connectivity
pub async fn check_redis(redis: &deadpool_redis::Pool) -> bool {
    match redis.get().await {
        Ok(mut conn) => deadpool_redis::redis::cmd("PING")
            .query_async::<Option<String>>(&mut conn)
            .await
            .is_ok(),
        Err(_) => false,
    }
}

/// Check email outbox pressure.
///
/// Returns:
/// - Number of queued emails older than 15 minutes.
/// - Age (seconds) of the oldest queued email, if any.
async fn check_email_outbox_pressure(db: &sqlx::PgPool) -> Result<(i64, Option<i64>), sqlx::Error> {
    let queued_old: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM email_outbox
        WHERE status = 'queued'
          AND created_at < NOW() - INTERVAL '15 minutes'
        "#,
    )
    .fetch_one(db)
    .await?;

    let oldest_age_secs: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT EXTRACT(EPOCH FROM (NOW() - MIN(created_at)))::bigint
        FROM email_outbox
        WHERE status = 'queued'
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok((queued_old, oldest_age_secs))
}
