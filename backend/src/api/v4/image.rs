use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use serde::Deserialize;

pub fn router() -> Router<AppState> {
    Router::new().route("/image", get(get_image))
}

#[derive(Deserialize)]
pub struct ImageQuery {
    pub _url: String,
}

/// GET /api/v4/image
async fn get_image(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Query(_query): Query<ImageQuery>,
) -> ApiResult<axum::response::Response> {
    // Mattermost behavior:
    // 1. If image proxy is enabled, it fetches and proxies the image.
    // 2. If disabled, it returns 400 Bad Request for external images.
    
    // For now, we don't have an image proxy implemented.
    // We return 400 Bad Request to indicate we don't support external image proxying yet.
    // This is safer than redirecting (which MM stopped doing for security).
    
    Err(crate::error::AppError::BadRequest("Image proxy is disabled".to_string()))
}
