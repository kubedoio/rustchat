use axum::{
    body::Bytes,
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
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};
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
        .route("/posts/{post_id}/ack", post(ack_post))
        .route("/reactions", post(add_reaction))
        .route("/users/me/posts/{post_id}/reactions/{emoji_name}", delete(remove_reaction))
        .route("/posts/{post_id}/reactions", get(get_reactions))
        .route("/posts/{post_id}/thread", get(get_post_thread))
        .route("/posts/ephemeral", post(create_ephemeral_post))
        .route("/posts/schedule", post(create_scheduled_post))
        .route("/posts/scheduled/team/{team_id}", get(list_scheduled_posts))
        .route("/users/{user_id}/posts/{post_id}/reminder", post(set_post_reminder))
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
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::Post>> {
    let input: CreatePostRequest = parse_body(&headers, &body, "Invalid post body")?;
    let channel_id = parse_mm_or_uuid(&input.channel_id)
        .ok_or_else(|| AppError::Validation("Invalid channel_id".to_string()))?;

    let root_post_id = if !input.root_id.is_empty() {
        Some(
            parse_mm_or_uuid(&input.root_id)
                .ok_or_else(|| AppError::Validation("Invalid root_id".to_string()))?,
        )
    } else {
        None
    };

    let file_ids = input
        .file_ids
        .iter()
        .filter_map(|id| parse_mm_or_uuid(id))
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

fn parse_body<T: serde::de::DeserializeOwned>(
    headers: &axum::http::HeaderMap,
    body: &Bytes,
    message: &str,
) -> ApiResult<T> {
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.starts_with("application/json") {
        serde_json::from_slice(body).map_err(|_| AppError::BadRequest(message.to_string()))
    } else if content_type.starts_with("application/x-www-form-urlencoded") {
        serde_urlencoded::from_bytes(body).map_err(|_| AppError::BadRequest(message.to_string()))
    } else {
        serde_json::from_slice(body)
            .or_else(|_| serde_urlencoded::from_bytes(body))
            .map_err(|_| AppError::BadRequest(message.to_string()))
    }
}

async fn get_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<String>,
) -> ApiResult<Json<mm::Post>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
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
    Path(post_id): Path<String>,
) -> ApiResult<Json<mm::PostList>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
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
    let root_id = encode_mm_id(root_post.id);
    order.push(root_id.clone());
    posts_map.insert(root_id, root_post.into());

    // Add replies
    for r in replies {
        let id = encode_mm_id(r.id);
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
    Path(post_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
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

    Ok(Json(serde_json::json!({"status": "OK", "id": encode_mm_id(post_id)})))
}

#[derive(Deserialize)]
struct PatchPostRequest {
    message: String,
}

async fn patch_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<String>,
    Json(input): Json<PatchPostRequest>,
) -> ApiResult<Json<mm::Post>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
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
    let input_user_id = parse_mm_or_uuid(&input.user_id)
        .ok_or_else(|| AppError::Validation("Invalid user_id".to_string()))?;
    if input_user_id != auth.user_id {
        return Err(AppError::Forbidden("Cannot react for other user".to_string()));
    }

    let post_id = parse_mm_or_uuid(&input.post_id)
        .ok_or_else(|| AppError::Validation("Invalid post_id".to_string()))?;

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
        user_id: encode_mm_id(reaction.user_id),
        post_id: encode_mm_id(reaction.post_id),
        emoji_name: reaction.emoji_name,
        create_at: reaction.created_at.timestamp_millis(),
    }))
}

async fn remove_reaction(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((post_id, emoji_name)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
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
    Path(post_id): Path<String>,
) -> ApiResult<Json<Vec<mm::Reaction>>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
    let reactions: Vec<crate::models::post::Reaction> = sqlx::query_as("SELECT * FROM reactions WHERE post_id = $1")
        .bind(post_id)
        .fetch_all(&state.db)
        .await?;

    let mm_reactions = reactions.into_iter().map(|r| mm::Reaction {
        user_id: encode_mm_id(r.user_id),
        post_id: encode_mm_id(r.post_id),
        emoji_name: r.emoji_name,
        create_at: r.created_at.timestamp_millis(),
    }).collect();

    Ok(Json(mm_reactions))
}

/// POST /posts/{post_id}/ack - Acknowledge a post (push notification receipt)
#[derive(Deserialize)]
#[allow(dead_code)]
struct AckPostRequest {
    #[serde(default)]
    post_id: String,
}

async fn ack_post(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(post_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Parse and validate the post ID
    let _post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;

    // Acknowledgments are typically used for:
    // 1. Confirming push notification receipt
    // 2. Analytics/delivery tracking
    // For now, we just return success - can be extended to track delivery status

    Ok(Json(serde_json::json!({"status": "OK"})))
}

#[derive(serde::Deserialize)]
pub struct CreateScheduledPostRequest {
    pub channel_id: String,
    pub message: String,
    #[serde(default)]
    pub root_id: String,
    #[serde(default)]
    pub props: serde_json::Value,
    #[serde(default)]
    pub file_ids: Vec<String>,
    pub scheduled_at: i64,
}

async fn list_scheduled_posts(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id_str): Path<String>,
) -> ApiResult<Json<Vec<mm::ScheduledPost>>> {
    let team_id = parse_mm_or_uuid(&team_id_str)
        .ok_or_else(|| AppError::Validation("Invalid team_id".to_string()))?;

    let rows: Vec<(Uuid, Uuid, Uuid, Option<Uuid>, String, serde_json::Value, Vec<Uuid>, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        r#"
        SELECT id, user_id, channel_id, root_id, message, props, file_ids, scheduled_at, created_at, updated_at
        FROM scheduled_posts
        WHERE user_id = $1 AND channel_id IN (SELECT id FROM channels WHERE team_id = $2)
        AND state = 'pending'
        "#
    )
    .bind(auth.user_id)
    .bind(team_id)
    .fetch_all(&state.db)
    .await?;

    let posts = rows.into_iter().map(|r| mm::ScheduledPost {
        id: encode_mm_id(r.0),
        user_id: encode_mm_id(r.1),
        channel_id: encode_mm_id(r.2),
        root_id: r.3.map(encode_mm_id).unwrap_or_default(),
        message: r.4,
        props: r.5,
        file_ids: r.6.into_iter().map(encode_mm_id).collect(),
        scheduled_at: r.7.timestamp_millis(),
        create_at: r.8.timestamp_millis(),
        update_at: r.9.timestamp_millis(),
    }).collect();

    Ok(Json(posts))
}

async fn create_scheduled_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<CreateScheduledPostRequest>,
) -> ApiResult<Json<mm::ScheduledPost>> {
    let channel_id = parse_mm_or_uuid(&input.channel_id)
        .ok_or_else(|| AppError::Validation("Invalid channel_id".to_string()))?;

    let root_id = if !input.root_id.is_empty() {
        Some(parse_mm_or_uuid(&input.root_id)
            .ok_or_else(|| AppError::Validation("Invalid root_id".to_string()))?)
    } else {
        None
    };

    let file_ids = input.file_ids.iter().filter_map(|id| parse_mm_or_uuid(id)).collect::<Vec<_>>();
    let scheduled_at = chrono::DateTime::from_timestamp_millis(input.scheduled_at)
        .ok_or_else(|| AppError::Validation("Invalid scheduled_at".to_string()))?;

    let row: (Uuid, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        r#"
        INSERT INTO scheduled_posts (user_id, channel_id, root_id, message, props, file_ids, scheduled_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, created_at, updated_at
        "#
    )
    .bind(auth.user_id)
    .bind(channel_id)
    .bind(root_id)
    .bind(&input.message)
    .bind(&input.props)
    .bind(&file_ids)
    .bind(scheduled_at)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(mm::ScheduledPost {
        id: encode_mm_id(row.0),
        user_id: encode_mm_id(auth.user_id),
        channel_id: input.channel_id,
        root_id: input.root_id,
        message: input.message,
        props: input.props,
        file_ids: input.file_ids,
        scheduled_at: input.scheduled_at,
        create_at: row.1.timestamp_millis(),
        update_at: row.2.timestamp_millis(),
    }))
}

#[derive(serde::Deserialize)]
pub struct EphemeralPostRequest {
    pub user_id: String,
    pub post: CreatePostRequest,
}

async fn create_ephemeral_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<EphemeralPostRequest>,
) -> ApiResult<Json<mm::Post>> {
    let target_user_id = parse_mm_or_uuid(&input.user_id)
        .ok_or_else(|| AppError::Validation("Invalid user_id".to_string()))?;

    if target_user_id != auth.user_id && input.user_id != "me" {
        return Err(AppError::Forbidden("Cannot send ephemeral post to others".to_string()));
    }

    let channel_id = parse_mm_or_uuid(&input.post.channel_id)
        .ok_or_else(|| AppError::Validation("Invalid channel_id".to_string()))?;

    let post_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();
    
    let ephemeral_post = mm::Post {
        id: encode_mm_id(post_id),
        create_at: now,
        update_at: now,
        delete_at: 0,
        edit_at: 0,
        user_id: encode_mm_id(auth.user_id),
        channel_id: input.post.channel_id,
        root_id: input.post.root_id,
        original_id: "".to_string(),
        message: input.post.message,
        post_type: "ephemeral".to_string(),
        props: input.post.props,
        hashtags: "".to_string(),
        file_ids: input.post.file_ids,
        pending_post_id: input.post.pending_post_id,
        metadata: None,
    };

    let broadcast = WsEnvelope::event(
        EventType::EphemeralMessage,
        ephemeral_post.clone(),
        Some(channel_id),
    )
    .with_broadcast(WsBroadcast {
        channel_id: Some(channel_id),
        team_id: None,
        user_id: Some(auth.user_id),
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(ephemeral_post))
}

#[derive(serde::Deserialize)]
pub struct PostReminderRequest {
    pub target_at: i64,
}

async fn set_post_reminder(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((user_id_str, post_id_str)): Path<(String, String)>,
    Json(input): Json<PostReminderRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let target_user_id = parse_mm_or_uuid(&user_id_str)
        .ok_or_else(|| AppError::Validation("Invalid user_id".to_string()))?;

    if target_user_id != auth.user_id && user_id_str != "me" {
        return Err(AppError::Forbidden("Cannot set reminder for others".to_string()));
    }

    let post_id = parse_mm_or_uuid(&post_id_str)
        .ok_or_else(|| AppError::Validation("Invalid post_id".to_string()))?;

    let target_at = chrono::DateTime::from_timestamp_millis(input.target_at)
        .ok_or_else(|| AppError::Validation("Invalid target_at".to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO post_reminders (user_id, post_id, target_at)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, post_id) DO UPDATE SET target_at = $3
        "#
    )
    .bind(auth.user_id)
    .bind(post_id)
    .bind(target_at)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

