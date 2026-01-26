use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
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
    Router::new()
        .route("/channels/{channel_id}/posts", get(get_posts))
        .route("/channels/{channel_id}", get(get_channel))
        .route("/channels/{channel_id}/members", get(get_channel_members))
        .route("/channels/members/me/view", post(view_channel))
}

#[derive(serde::Deserialize)]
struct ViewChannelRequest {
    channel_id: String,
    #[allow(dead_code)]
    prev_channel_id: Option<String>,
}

async fn view_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<ViewChannelRequest>,
) -> ApiResult<impl IntoResponse> {
    if let Ok(channel_id) = Uuid::parse_str(&input.channel_id) {
        // Update last_viewed_at
        sqlx::query(
            "UPDATE channel_members SET last_viewed_at = NOW() WHERE channel_id = $1 AND user_id = $2"
        )
        .bind(channel_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

        // Broadcast channel_viewed
        let broadcast = crate::realtime::WsEnvelope::event(
            crate::realtime::EventType::ChannelUpdated, // Closest match, usually handled by client logic
            serde_json::json!({
                "channel_id": channel_id,
                "user_id": auth.user_id
            }),
            Some(channel_id)
        );
        // We don't usually broadcast view events to EVERYONE, just to the user's other sessions.
        // But Mattermost sends 'channel_viewed' to the user.
        // My WsHub broadcasts to channel subscribers.
        // I'll skip broadcasting this generally to avoid noise, OR target only the user.
        // WsHub targeting user only:
        let broadcast = broadcast.with_broadcast(crate::realtime::WsBroadcast {
            channel_id: None,
            team_id: None,
            user_id: Some(auth.user_id),
            exclude_user_id: None,
        });
        state.ws_hub.broadcast(broadcast).await;

        Ok(Json(serde_json::json!({"status": "OK"})))
    } else {
        Err(crate::error::AppError::BadRequest("Invalid channel_id".to_string()))
    }
}

async fn get_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<Uuid>,
) -> ApiResult<Json<mm::Channel>> {
    // Verify membership
    let _membership: crate::models::ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| {
                crate::error::AppError::Forbidden("Not a member of this channel".to_string())
            })?;

    let channel: crate::models::Channel = sqlx::query_as("SELECT * FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(channel.into()))
}

async fn get_channel_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<Uuid>,
) -> ApiResult<Json<Vec<mm::ChannelMember>>> {
    // Verify membership
    let _membership: crate::models::ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| {
                crate::error::AppError::Forbidden("Not a member of this channel".to_string())
            })?;

    let members: Vec<crate::models::ChannelMember> =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1")
            .bind(channel_id)
            .fetch_all(&state.db)
            .await?;

    let mm_members = members
        .into_iter()
        .map(|m| mm::ChannelMember {
            channel_id: m.channel_id.to_string(),
            user_id: m.user_id.to_string(),
            roles: "channel_user".to_string(),
            last_viewed_at: m.last_viewed_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            msg_count: 0,
            mention_count: 0,
            notify_props: m.notify_props,
            last_update_at: 0,
            scheme_guest: false,
            scheme_user: true,
            scheme_admin: false,
        })
        .collect();

    Ok(Json(mm_members))
}

async fn get_posts(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(channel_id): Path<Uuid>,
) -> ApiResult<Json<mm::PostList>> {
    // Check channel membership first
    let _membership: crate::models::ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(_auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| {
                crate::error::AppError::Forbidden("Not a member of this channel".to_string())
            })?;

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
