use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::models as mm;
use crate::mattermost_compat::MM_VERSION;
use crate::models::server_config::SiteConfig;
use axum::{extract::{Query, State}, routing::get, Json, Router};
use serde::Deserialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/config/client", get(client_config))
        .route("/license/client", get(client_license))
}

async fn client_config(State(state): State<AppState>) -> ApiResult<Json<mm::Config>> {
    let site: (sqlx::types::Json<SiteConfig>,) =
        sqlx::query_as("SELECT site FROM server_config WHERE id = 'default'")
            .fetch_one(&state.db)
            .await?;

    Ok(Json(mm::Config {
        site_url: site.0.site_url,
        version: MM_VERSION.to_string(),
        enable_push_notifications: "false".to_string(),
        // Hardcoded diagnostic ID to satisfy client requirements
        diagnostic_id: "00000000-0000-0000-0000-000000000000".to_string(),
    }))
}

#[derive(Deserialize)]
pub struct LicenseQuery {
    #[allow(dead_code)]
    pub format: Option<String>,
}

async fn client_license(
    State(_state): State<AppState>,
    Query(_query): Query<LicenseQuery>,
) -> ApiResult<Json<mm::License>> {
    Ok(Json(mm::License {
        is_licensed: false,
        issued_at: 0,
        starts_at: 0,
        expires_at: 0,
    }))
}
