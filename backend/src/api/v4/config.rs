use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::models as mm;
use axum::{extract::State, routing::get, Json, Router};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/config/client", get(client_config))
        .route("/license/client", get(client_license))
}

async fn client_config(State(_state): State<AppState>) -> ApiResult<Json<mm::Config>> {
    Ok(Json(mm::Config {
        site_url: "".to_string(),
        version: "5.35.0".to_string(),
        enable_push_notifications: "false".to_string(),
        // Hardcoded diagnostic ID to satisfy client requirements
        diagnostic_id: "00000000-0000-0000-0000-000000000000".to_string(),
    }))
}

async fn client_license(State(_state): State<AppState>) -> ApiResult<Json<mm::License>> {
    Ok(Json(mm::License {
        is_licensed: "false".to_string(),
    }))
}
