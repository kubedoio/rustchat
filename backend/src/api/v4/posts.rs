use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::models as mm;
use crate::models::CreatePost;
use crate::realtime::{EventType, WsBroadcast, WsEnvelope};
use crate::services::posts;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/posts", post(create_post_handler))
        .route(
            "/posts/{post_id}",
            get(get_post).delete(delete_post),
        )
        .route("/posts/{post_id}/patch", put(patch_post))
        .route("/reactions", post(add_reaction))
        .route("/users/me/posts/{post_id}/reactions/{emoji_name}", delete(remove_reaction))
        .route("/posts/{post_id}/reactions", get(get_reactions))
        .route("/posts/{post_id}/thread", get(get_post_thread))
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

async fn get_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<Uuid>,
) -> ApiResult<Json<mm::Post>> {
    let post: crate::models::post::PostResponse = sqlx::query_as(
        r#"
        SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
               p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
               p.reply_count::int8 as reply_count,
               p.last_reply_at, p.seq,
               u.username, u.avatar_url, u.email
        FROM posts p
        LEFT JOIN users u ON p.user_id = u.id
        WHERE p.id = $1 AND p.deleted_at IS NULL
        "#,
    )
    .bind(post_id)
    .fetch_one(&state.db)
    .await?;

    let _: crate::models::ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(post.channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| {
                crate::error::AppError::Forbidden("Not a member of this channel".to_string())
            })?;

    Ok(Json(post.into()))
}

async fn get_post_thread(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<Uuid>,
) -> ApiResult<Json<mm::PostList>> {
    use std::collections::HashMap;

    // 1. Get the requested post
    let root_post: crate::models::post::PostResponse = sqlx::query_as(
        r#"
        SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
               p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
               p.reply_count::int8 as reply_count,
               p.last_reply_at, p.seq,
               u.username, u.avatar_url, u.email
        FROM posts p
        LEFT JOIN users u ON p.user_id = u.id
        WHERE p.id = $1 AND p.deleted_at IS NULL
        "#,
    )
    .bind(post_id)
    .fetch_one(&state.db)
    .await?;

    // 2. Check permissions
    let _: crate::models::ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(root_post.channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| {
                crate::error::AppError::Forbidden("Not a member of this channel".to_string())
            })?;

    // 3. Get replies
    let replies: Vec<crate::models::post::PostResponse> = sqlx::query_as(
        r#"
        SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
               p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
               p.reply_count::int8 as reply_count,
               p.last_reply_at, p.seq,
               u.username, u.avatar_url, u.email
        FROM posts p
        LEFT JOIN users u ON p.user_id = u.id
        WHERE p.root_post_id = $1 AND p.deleted_at IS NULL
        ORDER BY p.created_at ASC
        "#,
    )
    .bind(post_id)
    .fetch_all(&state.db)
    .await?;

    // 4. Construct response
    let mut order = Vec::new();
    let mut posts_map = HashMap::new();

    // Add root post
    let root_id = root_post.id.to_string();
    order.push(root_id.clone());
    posts_map.insert(root_id, root_post.into());

    // Add replies
    for r in replies {
        let id = r.id.to_string();
        order.push(id.clone());
        posts_map.insert(id, r.into());
    }

    Ok(Json(mm::PostList {
        order,
        posts: posts_map,
        next_post_id: "".to_string(),
        prev_post_id: "".to_string(),
    }))
}

async fn delete_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let post: crate::models::post::Post = sqlx::query_as("SELECT * FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(&state.db)
        .await?;

    if post.user_id != auth.user_id {
        return Err(AppError::Forbidden("Cannot delete others' posts".to_string()));
    }

    let deleted_post: crate::models::post::PostResponse = sqlx::query_as(
        r#"
        WITH updated_post AS (
            UPDATE posts SET deleted_at = NOW() WHERE id = $1
            RETURNING *
        )
        SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
               p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
               p.reply_count::int8 as reply_count,
               p.last_reply_at, p.seq,
               u.username, u.avatar_url, u.email
        FROM updated_post p
        LEFT JOIN users u ON p.user_id = u.id
        "#,
    )
    .bind(post_id)
    .fetch_one(&state.db)
    .await?;

    let broadcast = WsEnvelope::event(
        EventType::MessageDeleted,
        deleted_post,
        Some(post.channel_id),
    )
    .with_broadcast(WsBroadcast {
        channel_id: Some(post.channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(serde_json::json!({"status": "OK", "id": post_id.to_string()})))
}

#[derive(Deserialize)]
struct PatchPostRequest {
    message: String,
}

async fn patch_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<Uuid>,
    Json(input): Json<PatchPostRequest>,
) -> ApiResult<Json<mm::Post>> {
    let post: crate::models::post::Post = sqlx::query_as("SELECT * FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(&state.db)
        .await?;

    if post.user_id != auth.user_id {
        return Err(AppError::Forbidden("Cannot edit others' posts".to_string()));
    }

    let updated: crate::models::post::PostResponse = sqlx::query_as(
        r#"
        WITH updated_post AS (
            UPDATE posts SET message = $1, edited_at = NOW()
            WHERE id = $2
            RETURNING *
        )
        SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
               p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
               p.reply_count::int8 as reply_count,
               p.last_reply_at, p.seq,
               u.username, u.avatar_url, u.email
        FROM updated_post p
        LEFT JOIN users u ON p.user_id = u.id
        "#,
    )
    .bind(input.message)
    .bind(post_id)
    .fetch_one(&state.db)
    .await?;

    let broadcast = WsEnvelope::event(
        EventType::MessageUpdated,
        updated.clone(),
        Some(post.channel_id),
    )
    .with_broadcast(WsBroadcast {
        channel_id: Some(post.channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(updated.into()))
}

#[derive(Deserialize)]
struct ReactionRequest {
    user_id: String,
    post_id: String,
    emoji_name: String,
}

async fn add_reaction(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<ReactionRequest>,
) -> ApiResult<Json<mm::Reaction>> {
    if input.user_id != auth.user_id.to_string() {
        return Err(AppError::Forbidden("Cannot react for other user".to_string()));
    }

    let post_id = Uuid::parse_str(&input.post_id).map_err(|_| AppError::Validation("Invalid post_id".to_string()))?;

    let reaction: crate::models::post::Reaction = sqlx::query_as(
        r#"
        INSERT INTO reactions (user_id, post_id, emoji_name)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, post_id, emoji_name) DO UPDATE SET emoji_name = $3
        RETURNING *
        "#
    )
    .bind(auth.user_id)
    .bind(post_id)
    .bind(&input.emoji_name)
    .fetch_one(&state.db)
    .await?;

    let channel_id: Uuid = sqlx::query_scalar("SELECT channel_id FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(&state.db)
        .await?;

    let broadcast = WsEnvelope::event(
        EventType::ReactionAdded,
        reaction.clone(),
        Some(channel_id),
    )
    .with_broadcast(WsBroadcast {
        channel_id: Some(channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(mm::Reaction {
        user_id: reaction.user_id.to_string(),
        post_id: reaction.post_id.to_string(),
        emoji_name: reaction.emoji_name,
        create_at: reaction.created_at.timestamp_millis(),
    }))
}

async fn remove_reaction(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((post_id, emoji_name)): Path<(Uuid, String)>,
) -> ApiResult<impl IntoResponse> {
    let reaction: Option<crate::models::post::Reaction> = sqlx::query_as(
        "SELECT * FROM reactions WHERE user_id = $1 AND post_id = $2 AND emoji_name = $3",
    )
    .bind(auth.user_id)
    .bind(post_id)
    .bind(&emoji_name)
    .fetch_optional(&state.db)
    .await?;

    if let Some(r) = reaction {
        sqlx::query("DELETE FROM reactions WHERE user_id = $1 AND post_id = $2 AND emoji_name = $3")
            .bind(auth.user_id)
            .bind(post_id)
            .bind(&emoji_name)
            .execute(&state.db)
            .await?;

        let channel_id: Uuid = sqlx::query_scalar("SELECT channel_id FROM posts WHERE id = $1")
            .bind(post_id)
            .fetch_one(&state.db)
            .await?;

        let broadcast = WsEnvelope::event(EventType::ReactionRemoved, r, Some(channel_id))
            .with_broadcast(WsBroadcast {
                channel_id: Some(channel_id),
                team_id: None,
                user_id: None,
                exclude_user_id: None,
            });
        state.ws_hub.broadcast(broadcast).await;
    }

    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn get_reactions(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
) -> ApiResult<Json<Vec<mm::Reaction>>> {
    let reactions: Vec<crate::models::post::Reaction> = sqlx::query_as("SELECT * FROM reactions WHERE post_id = $1")
        .bind(post_id)
        .fetch_all(&state.db)
        .await?;

    let mm_reactions = reactions.into_iter().map(|r| mm::Reaction {
        user_id: r.user_id.to_string(),
        post_id: r.post_id.to_string(),
        emoji_name: r.emoji_name,
        create_at: r.created_at.timestamp_millis(),
    }).collect();

    Ok(Json(mm_reactions))
}
