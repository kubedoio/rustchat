use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use std::collections::HashMap;
use uuid::Uuid;

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::models as mm;
use crate::models::post::PostResponse;

pub fn router() -> Router<AppState> {
    Router::new().route("/channels/{channel_id}/posts", get(get_posts))
}

async fn get_posts(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(channel_id): Path<Uuid>,
) -> ApiResult<Json<mm::PostList>> {
    // Check channel membership first
    let _membership: crate::models::ChannelMember = sqlx::query_as(
        "SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2"
    )
    .bind(channel_id)
    .bind(_auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::AppError::Forbidden("Not a member of this channel".to_string()))?;

    // Fetch posts
    // Limit to 60 by default in MM
    let posts: Vec<PostResponse> = sqlx::query_as(
        r#"
        SELECT p.*, u.username, u.email, u.avatar_url,
        (SELECT COUNT(*) FROM posts r WHERE r.root_post_id = p.id) as reply_count,
        (SELECT MAX(created_at) FROM posts r WHERE r.root_post_id = p.id) as last_reply_at
        FROM posts p
        LEFT JOIN users u ON p.user_id = u.id
        WHERE p.channel_id = $1 AND p.deleted_at IS NULL
        ORDER BY p.created_at DESC
        LIMIT 60
        "#,
    )
    .bind(channel_id)
    .fetch_all(&state.db)
    .await?;

    let mut order = Vec::new();
    let mut posts_map = HashMap::new();

    for p in posts {
        let id = p.id.to_string();
        order.push(id.clone());
        posts_map.insert(id, p.into());
    }

    Ok(Json(mm::PostList {
        order,
        posts: posts_map,
        next_post_id: "".to_string(),
        prev_post_id: "".to_string(),
    }))
}
