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
        .route("/config/client", get(get_client_config))
        .route("/license/client", get(get_client_license))
}

#[derive(Deserialize)]
pub struct LicenseQuery {
    pub format: Option<String>,
}

pub async fn get_client_config(
    State(state): State<AppState>,
    Query(query): Query<LicenseQuery>,
) -> ApiResult<impl IntoResponse> {
    if !matches!(query.format.as_deref(), Some("old")) {
        return Ok((
            axum::http::StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "id": "api.config.client.old_format.app_error",
                "message": "The new format for client config is not supported yet. Please provide \"format=old\" in the request.",
                "detailed_error": "",
                "request_id": "",
                "status_code": 501
            })),
        ));
    }

    let site = sqlx::query_as::<_, (sqlx::types::Json<SiteConfig>,)>(
        "SELECT site FROM server_config WHERE id = 'default'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .map(|row| row.0 .0)
    .unwrap_or_default();

    let diagnostic_id = diagnostic_id(&site);
    Ok(Json(legacy_config(&site, &diagnostic_id)))
}

pub async fn get_client_license(
    State(_state): State<AppState>,
    Query(query): Query<LicenseQuery>,
) -> ApiResult<impl IntoResponse> {
    let body = if matches!(query.format.as_deref(), Some("old")) {
        serde_json::json!({
            "IsLicensed": "false"
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

fn legacy_config(site: &SiteConfig, diagnostic_id: &str) -> serde_json::Value {
    use serde_json::{Map, Value};

    let mut map = Map::new();
    let insert = |map: &mut Map<String, Value>, key: &str, value: &str| {
        map.insert(key.to_string(), Value::String(value.to_string()));
    };

    insert(&mut map, "SiteName", &site.site_name);
    insert(&mut map, "SiteURL", &site.site_url);
    insert(&mut map, "Version", MM_VERSION);
    insert(&mut map, "DiagnosticId", diagnostic_id);
    insert(&mut map, "DiagnosticsEnabled", "false");
    insert(&mut map, "EnableDiagnostics", "false");
    insert(&mut map, "NoAccounts", "false");
    insert(&mut map, "AboutLink", "https://docs.mattermost.com/about/product.html/");
    insert(&mut map, "AllowDownloadLogs", "true");
    insert(&mut map, "AndroidAppDownloadLink", "https://mattermost.com/mattermost-android-app/");
    insert(&mut map, "AppDownloadLink", "https://mattermost.com/download/#mattermostApps");
    insert(&mut map, "AppsPluginEnabled", "true");
    insert(&mut map, "EnableCustomBrand", "false");
    insert(&mut map, "EnableCustomEmoji", "false");
    insert(&mut map, "EnableFile", "true");
    insert(&mut map, "EnableUserStatuses", "true");
    insert(&mut map, "IosAppDownloadLink", "https://mattermost.com/mattermost-ios-app/");
    insert(&mut map, "PasswordMinimumLength", "10");
    insert(&mut map, "PluginsEnabled", "true");
    insert(&mut map, "WebsocketPort", "80");
    insert(&mut map, "WebsocketSecurePort", "443");
    
    // Add essential fields for mobile
    insert(&mut map, "EnableMobileFileDownload", "true");
    insert(&mut map, "EnableMobileFileUpload", "true");

    Value::Object(map)
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
