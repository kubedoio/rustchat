//! Unreads API endpoints

use axum::{
    extract::State,
    routing::get,
    Json, Router,
};

use super::AppState;
use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::services::unreads::{UnreadOverview, get_unread_overview};

/// Build unreads routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/overview", get(get_unreads_overview))
        .route("/mark_all_read", axum::routing::post(mark_all_read))
}

/// Get unread overview for all channels and teams
async fn get_unreads_overview(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<UnreadOverview>> {
    let overview = get_unread_overview(&state, auth.user_id).await?;
    Ok(Json(overview))
}

/// Mark all channels as read
async fn mark_all_read(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    crate::services::unreads::mark_all_as_read(&state, auth.user_id).await?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}
