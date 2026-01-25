use crate::api::AppState;
use crate::error::ApiResult;
use axum::{routing::get, Json, Router};
use serde::Serialize;

pub fn router() -> Router<AppState> {
    Router::new().route("/system/ping", get(ping))
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

async fn ping() -> ApiResult<Json<SystemStatus>> {
    Ok(Json(SystemStatus {
        android_latest_version: "2.0.0".to_string(),
        android_min_version: "1.0.0".to_string(),
        desktop_latest_version: "5.0.0".to_string(),
        desktop_min_version: "4.0.0".to_string(),
        ios_latest_version: "2.0.0".to_string(),
        ios_min_version: "1.0.0".to_string(),
        status: "OK".to_string(),
        version: "5.35.0".to_string(),
    }))
}
