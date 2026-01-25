//! Public site configuration and metadata
use super::AppState;
use crate::error::ApiResult;
use crate::models::server_config::SiteConfig;
use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
pub struct PublicConfig {
    pub site_name: String,
    pub logo_url: Option<String>,
    pub mirotalk_enabled: bool,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/site/info", get(get_site_info))
}

async fn get_site_info(State(state): State<AppState>) -> ApiResult<Json<PublicConfig>> {
    let config: (sqlx::types::Json<SiteConfig>,) =
        sqlx::query_as("SELECT site FROM server_config WHERE id = 'default'")
            .fetch_one(&state.db)
            .await?;

    let mirotalk_mode: Option<String> =
        sqlx::query_scalar("SELECT mode FROM mirotalk_config WHERE is_active = true")
            .fetch_optional(&state.db)
            .await?;

    let mirotalk_enabled = mirotalk_mode.map(|m| m != "disabled").unwrap_or(false);

    Ok(Json(PublicConfig {
        site_name: config.0.site_name.clone(),
        logo_url: config.0.logo_url.clone(),
        mirotalk_enabled,
    }))
}
