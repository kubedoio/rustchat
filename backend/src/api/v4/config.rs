use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::models as mm;
use crate::mattermost_compat::{id::encode_mm_id, MM_VERSION};
use crate::models::server_config::SiteConfig;
use axum::{extract::{Query, State}, response::IntoResponse, routing::get, Json, Router};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/config/client", get(client_config))
        .route("/license/client", get(client_license))
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
        let diagnostic_id = diagnostic_id(&site);
        serde_json::json!({
            "Version": "9.5.0",
            "DiagnosticId": diagnostic_id,
            "TelemetryId": diagnostic_id,
            "EnableDiagnostics": "false",
            "BuildNumber": "dev",
            "SiteName": site.site_name,
            "EnableFilePost": "true",
            "EnableCommands": "false",
            "EnableCustomEmoji": "false",
            "ExperimentalEnableDefaultChannelLeaveJoinMessages": "true"
        })
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
        serde_json::json!({
            "IsLicensed": "false",
            "Cloud": "false"
        })
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

fn diagnostic_id(site: &SiteConfig) -> String {
    let seed = if !site.site_url.is_empty() {
        site.site_url.as_bytes()
    } else if !site.site_name.is_empty() {
        site.site_name.as_bytes()
    } else {
        b"rustchat"
    };

    let mut hasher = Sha256::new();
    hasher.update(seed);
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);

    Uuid::from_slice(&bytes)
        .map(encode_mm_id)
        .unwrap_or_else(|_| encode_mm_id(Uuid::new_v4()))
}
