use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::MM_VERSION;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/system/ping", get(ping))
        .route("/system/version", get(version))
        .route("/system/timezones", get(get_timezones))
        .route("/client_perf", post(client_perf))
        .route("/caches/invalidate", post(invalidate_caches))
        .route("/logs", post(post_logs))
        .route("/database/recycle", post(recycle_database))
        .route("/system/notices/{team_id}", get(get_product_notices))
        .route("/system/notices/view", axum::routing::put(update_viewed_notices))
        .route("/system/support_packet", get(get_support_packet))
        .route("/system/onboarding/complete", get(get_onboarding_status).post(complete_onboarding))
        .route("/system/schema/version", get(get_schema_version))
        .route("/email/test", post(test_email))
        .route("/notifications/test", post(test_notifications))
        .route("/site_url/test", post(test_site_url))
        .route("/file/s3_test", post(test_s3))
        .route("/config", get(get_config))
        .route("/config/reload", post(reload_config))
        .route("/config/environment", get(get_environment_config))
        .route("/config/patch", post(patch_config))
        .route("/license", get(get_license))
        .route("/license/renewal", post(license_renewal))
        .route("/trial-license", post(trial_license))
        .route("/trial-license/prev", get(get_prev_trial_license))
        .route("/license/load_metric", get(get_client_license_load_metric))
        .route("/analytics/old", get(get_analytics_old))
        .route("/server_busy", get(get_server_busy).post(set_server_busy).delete(clear_server_busy))
        .route("/notifications/ack", post(ack_notification))
        .route("/redirect_location", get(get_redirect_location))
        .route("/upgrade_to_enterprise", post(upgrade_to_enterprise))
        .route("/upgrade_to_enterprise/status", get(get_upgrade_to_enterprise_status))
        .route("/upgrade_to_enterprise/allowed", get(get_upgrade_to_enterprise_allowed))
        .route("/restart", post(restart_server))
        .route("/integrity", post(check_integrity))
}

// ... existing code ...

/// GET /system/notices/{team_id}
async fn get_product_notices(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_team_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    // Return empty list of notices for now
    Ok(Json(vec![]))
}

/// PUT /system/notices/view
async fn update_viewed_notices(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_ids): Json<Vec<String>>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// GET /system/support_packet
async fn get_support_packet(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<axum::response::Response> {
    // Return a dummy zip file or 403 if no license as per MM behavior
    // For now, just return a 403 indicating no license
    Err(crate::error::AppError::Forbidden("Support packets require a license".to_string()))
}

/// GET /system/onboarding/complete
async fn get_onboarding_status(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "onboarding_complete": true
    })))
}

/// POST /system/onboarding/complete
async fn complete_onboarding(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// GET /system/schema/version
async fn get_schema_version(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    // Return empty list of migrations for now
    Ok(Json(vec![]))
}

/// POST /email/test
async fn test_email(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_config): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// POST /notifications/test
async fn test_notifications(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// POST /site_url/test
async fn test_site_url(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_props): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// POST /file/s3_test
async fn test_s3(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_config): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// GET /config
async fn get_config(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    // Return a minimal config object
    Ok(Json(serde_json::json!({
        "ServiceSettings": {},
        "TeamSettings": {},
        "SqlSettings": {},
        "LogSettings": {},
        "FileSettings": {},
        "EmailSettings": {},
        "RateLimitSettings": {},
        "PrivacySettings": {},
        "SupportSettings": {},
        "AnnouncementSettings": {},
        "ThemeSettings": {}
    })))
}

/// POST /config/reload
async fn reload_config(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// GET /config/environment
async fn get_environment_config(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

/// POST /config/patch
async fn patch_config(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_patch): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// GET /license
async fn get_license(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

/// POST /license/renewal
async fn license_renewal(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// POST /trial-license
async fn trial_license(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
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

/// GET /system/timezones - Returns a list of supported timezones
async fn get_timezones() -> ApiResult<Json<Vec<String>>> {
    // Returns a standard list of IANA timezone names
    let timezones = vec![
        "Pacific/Midway",
        "Pacific/Honolulu",
        "America/Anchorage",
        "America/Los_Angeles",
        "America/Denver",
        "America/Chicago",
        "America/New_York",
        "America/Toronto",
        "America/Sao_Paulo",
        "Atlantic/Azores",
        "Europe/London",
        "Europe/Paris",
        "Europe/Berlin",
        "Europe/Moscow",
        "Asia/Dubai",
        "Asia/Karachi",
        "Asia/Dhaka",
        "Asia/Bangkok",
        "Asia/Shanghai",
        "Asia/Tokyo",
        "Australia/Sydney",
        "Pacific/Auckland",
        "UTC",
    ].into_iter().map(String::from).collect();

    Ok(Json(timezones))
}

async fn get_prev_trial_license(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn get_client_license_load_metric(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn get_analytics_old(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn get_server_busy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn set_server_busy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn clear_server_busy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn ack_notification(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn get_redirect_location(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"location": ""})))
}

async fn upgrade_to_enterprise(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn get_upgrade_to_enterprise_status(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn get_upgrade_to_enterprise_allowed(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"allowed": false})))
}

async fn restart_server(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn check_integrity(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

