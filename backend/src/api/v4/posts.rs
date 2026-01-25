use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::models as mm;
use crate::models::CreatePost;
use crate::services::posts;

pub fn router() -> Router<AppState> {
    Router::new().route("/posts", post(create_post_handler))
}

#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub channel_id: String,
    pub message: String,
    #[serde(default)]
    pub root_id: String,
    #[serde(default)]
    pub file_ids: Vec<String>,
    #[serde(default)]
    pub props: serde_json::Value,
    #[serde(default)]
    pub pending_post_id: String,
}

async fn create_post_handler(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<CreatePostRequest>,
) -> ApiResult<Json<mm::Post>> {
    let channel_id = Uuid::parse_str(&input.channel_id)
        .map_err(|_| AppError::Validation("Invalid channel_id".to_string()))?;

    let root_post_id = if !input.root_id.is_empty() {
        Some(
            Uuid::parse_str(&input.root_id)
                .map_err(|_| AppError::Validation("Invalid root_id".to_string()))?,
        )
    } else {
        None
    };

    let file_ids = input
        .file_ids
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect();

    let create_payload = CreatePost {
        message: input.message,
        root_post_id,
        props: Some(input.props),
        file_ids,
    };

    let client_msg_id = if !input.pending_post_id.is_empty() {
        Some(input.pending_post_id)
    } else {
        None
    };

    let post_resp = posts::create_post(
        &state,
        auth.user_id,
        channel_id,
        create_payload,
        client_msg_id,
    )
    .await?;

    Ok(Json(post_resp.into()))
}
