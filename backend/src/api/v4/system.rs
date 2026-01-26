use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::MM_VERSION;
use axum::{routing::get, Json, Router, response::IntoResponse};
use serde::Serialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/system/ping", get(ping))
        .route("/system/version", get(version))
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
        android_latest_version: "".to_string(),
        android_min_version: "".to_string(),
        desktop_latest_version: "".to_string(),
        desktop_min_version: "".to_string(),
        ios_latest_version: "".to_string(),
        ios_min_version: "".to_string(),
        status: "OK".to_string(),
        version: MM_VERSION.to_string(),
    }))
}

async fn version() -> ApiResult<impl IntoResponse> {
     Ok((
        [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        MM_VERSION.to_string()
    ))
}
