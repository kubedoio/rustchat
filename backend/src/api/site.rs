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
}

pub fn router() -> Router<AppState> {
    Router::new().route("/site/info", get(get_site_info))
}

async fn get_site_info(State(state): State<AppState>) -> ApiResult<Json<PublicConfig>> {
    let config: (sqlx::types::Json<SiteConfig>,) =
        sqlx::query_as("SELECT site FROM server_config WHERE id = 'default'")
            .fetch_one(&state.db)
            .await?;

    Ok(Json(PublicConfig {
        site_name: config.0.site_name.clone(),
        logo_url: config.0.logo_url.clone(),
    }))
}
