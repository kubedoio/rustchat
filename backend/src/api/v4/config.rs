use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::models as mm;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/config/client", get(client_config))
        .route("/license/client", get(client_license))
}

async fn client_config(State(_state): State<AppState>) -> ApiResult<Json<mm::Config>> {
    Ok(Json(mm::Config {
        site_url: "".to_string(), // Mobile client usually ignores this if connected
        version: "10.0.0-rustchat".to_string(),
        enable_push_notifications: "false".to_string(),
    }))
}

async fn client_license(State(_state): State<AppState>) -> ApiResult<Json<mm::License>> {
    Ok(Json(mm::License {
        is_licensed: "false".to_string(),
    }))
}
