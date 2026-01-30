use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::MM_VERSION;
use axum::{extract::{Query, State}, routing::{get, post}, Json, Router, response::IntoResponse};
use serde::Serialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/system/ping", get(ping))
        .route("/system/version", get(version))
        .route("/client_perf", post(client_perf))
        .route("/caches/invalidate", post(invalidate_caches))
        .route("/logs", post(post_logs))
        .route("/database/recycle", post(recycle_database))
}

#[derive(Serialize)]
struct SystemStatus {
    #[serde(rename = "AndroidLatestVersion")]
    android_latest_version: String,
    #[serde(rename = "AndroidMinVersion")]
    android_min_version: String,
    #[serde(rename = "DesktopLatestVersion")]
    desktop_latest_version: String,
    #[serde(rename = "DesktopMinVersion")]
    desktop_min_version: String,
    #[serde(rename = "IosLatestVersion")]
    ios_latest_version: String,
    #[serde(rename = "IosMinVersion")]
    ios_min_version: String,
    status: String,
    version: String,
}

#[derive(serde::Deserialize)]
struct PingQuery {
    format: Option<String>,
}

async fn ping(Query(query): Query<PingQuery>) -> ApiResult<Json<serde_json::Value>> {
    if matches!(query.format.as_deref(), Some("old")) {
        return Ok(Json(serde_json::json!({
            "ActiveSearchBackend": "database",
            "AndroidLatestVersion": "",
            "AndroidMinVersion": "",
            "IosLatestVersion": "",
            "IosMinVersion": "",
            "status": "OK"
        })));
    }

    let body = serde_json::to_value(SystemStatus {
        android_latest_version: "".to_string(),
        android_min_version: "".to_string(),
        desktop_latest_version: "".to_string(),
        desktop_min_version: "".to_string(),
        ios_latest_version: "".to_string(),
        ios_min_version: "".to_string(),
        status: "OK".to_string(),
        version: MM_VERSION.to_string(),
    })
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    Ok(Json(body))
}

async fn client_perf(
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let _payload: serde_json::Value = if body.is_empty() {
        serde_json::json!({})
    } else {
        let content_type = headers
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if content_type.starts_with("application/json") {
            serde_json::from_slice(&body)
                .unwrap_or_else(|_| serde_json::json!({}))
        } else if content_type.starts_with("application/x-www-form-urlencoded") {
            serde_urlencoded::from_bytes(&body)
                .unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::from_slice(&body)
                .or_else(|_| serde_urlencoded::from_bytes(&body))
                .unwrap_or_else(|_| serde_json::json!({}))
        }
    };

    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn version() -> ApiResult<impl IntoResponse> {
     Ok((
        [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        MM_VERSION.to_string()
    ))
}

pub async fn invalidate_caches(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

pub async fn recycle_database(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

pub async fn post_logs(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(input): Json<Vec<String>>,
) -> ApiResult<Json<serde_json::Value>> {
    for log in input {
        tracing::info!("Client log: {}", log);
    }
    Ok(Json(serde_json::json!({"status": "OK"})))
}
