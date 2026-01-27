use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::models as mm;
use crate::mattermost_compat::MM_VERSION;
use crate::models::server_config::SiteConfig;
use axum::{extract::{Query, State}, response::IntoResponse, routing::get, Json, Router};
use serde::{Deserialize, Serialize};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/config/client", get(client_config))
        .route("/license/client", get(client_license))
}

#[derive(Serialize)]
struct LegacyConfig {
    #[serde(rename = "Version")]
    version: String,
    #[serde(rename = "BuildNumber")]
    build_number: String,
    #[serde(rename = "SiteName")]
    site_name: String,
}

#[derive(Serialize)]
struct LegacyLicense {
    #[serde(rename = "IsLicensed")]
    is_licensed: String,
    #[serde(rename = "TelemetryId")]
    telemetry_id: String,
}

async fn client_config(
    State(state): State<AppState>,
    Query(query): Query<LicenseQuery>,
) -> ApiResult<impl IntoResponse> {
    let site = sqlx::query_as::<_, (sqlx::types::Json<SiteConfig>,)>(
        "SELECT site FROM server_config WHERE id = 'default'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .map(|row| row.0 .0)
    .unwrap_or_default();

    let body = if matches!(query.format.as_deref(), Some("old")) {
        serde_json::to_value(LegacyConfig {
            version: "9.5.0".to_string(),
            build_number: "dev".to_string(),
            site_name: site.site_name.clone(),
        })
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
    } else {
        serde_json::to_value(mm::Config {
            site_url: site.site_url.clone(),
            version: MM_VERSION.to_string(),
            enable_push_notifications: "false".to_string(),
            // Hardcoded diagnostic ID to satisfy client requirements
            diagnostic_id: "00000000-0000-0000-0000-000000000000".to_string(),
        })
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
    };

    Ok(Json(body))
}

#[derive(Deserialize)]
pub struct LicenseQuery {
    #[allow(dead_code)]
    pub format: Option<String>,
}

async fn client_license(
    State(_state): State<AppState>,
    Query(query): Query<LicenseQuery>,
) -> ApiResult<impl IntoResponse> {
    let body = if matches!(query.format.as_deref(), Some("old")) {
        serde_json::to_value(LegacyLicense {
            is_licensed: "true".to_string(),
            telemetry_id: "12345".to_string(),
        })
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
    } else {
        serde_json::to_value(mm::License {
            is_licensed: false,
            issued_at: 0,
            starts_at: 0,
            expires_at: 0,
        })
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
    };

    Ok(Json(body))
}
