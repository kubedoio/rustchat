use axum::{
    body::Bytes,
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{
    id::{encode_mm_id, parse_mm_or_uuid},
    models as mm,
};
use crate::models::{CreatePost, FileInfo};
use crate::realtime::{EventType, WsBroadcast, WsEnvelope};
use crate::repositories::PostRepository;
use crate::services::posts;

mod reactions;
mod search;
mod unread;

pub(crate) use reactions::reactions_for_posts;
use reactions::{add_reaction, get_reactions, remove_reaction, remove_reaction_for_user};
use search::{search_posts_all_teams, search_team_posts};
use unread::{
    delete_acknowledgement_for_post, get_posts_around_unread, save_acknowledgement_for_post,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/posts", post(create_post_handler))
        .route("/posts/ids", post(get_posts_by_ids))
        .route("/posts/ids/reactions", post(get_reactions_by_post_ids))
        .route(
            "/posts/{post_id}",
            get(get_post).put(update_post).delete(delete_post),
        )
        .route("/posts/{post_id}/files/info", get(get_post_files_info))
        .route("/posts/{post_id}/pin", post(pin_post))
        .route("/posts/{post_id}/unpin", post(unpin_post))
        .route("/posts/{post_id}/patch", put(patch_post))
        .route(
            "/posts/{post_id}/actions/{action_id}",
            post(handle_post_action),
        )
        .route("/posts/{post_id}/move", post(move_post))
        .route(
            "/posts/{post_id}/restore/{restore_version_id}",
            post(restore_post),
        )
        .route(
            "/posts/{post_id}/reveal",
            get(reveal_post).post(reveal_post),
        )
        .route("/posts/{post_id}/burn", delete(burn_post).post(burn_post))
        .route("/posts/rewrite", post(rewrite_post))
        .route(
            "/users/{user_id}/posts/{post_id}/set_unread",
            post(set_post_unread),
        )
        .route("/users/{user_id}/posts/flagged", get(get_flagged_posts))
        .route("/posts/{post_id}/ack", post(ack_post))
        .route("/reactions", post(add_reaction))
        .route(
            "/users/me/posts/{post_id}/reactions/{emoji_name}",
            delete(remove_reaction),
        )
        .route(
            "/users/{user_id}/posts/{post_id}/reactions/{emoji_name}",
            delete(remove_reaction_for_user),
        )
        .route("/posts/{post_id}/reactions", get(get_reactions))
        .route("/posts/{post_id}/thread", get(get_post_thread))
        .route("/posts/ephemeral", post(create_ephemeral_post))
        .route("/posts/schedule", post(create_scheduled_post))
        .route(
            "/posts/schedule/{scheduled_post_id}",
            put(update_scheduled_post).delete(delete_scheduled_post),
        )
        .route("/posts/scheduled/team/{team_id}", get(list_scheduled_posts))
        .route(
            "/users/{user_id}/posts/{post_id}/reminder",
            post(set_post_reminder),
        )
        .route("/posts/search", post(search_posts_all_teams))
        .route("/teams/{team_id}/posts/search", post(search_team_posts))
        .route(
            "/users/{user_id}/channels/{channel_id}/posts/unread",
            get(get_posts_around_unread),
        )
        .route(
            "/users/{user_id}/posts/{post_id}/ack",
            post(save_acknowledgement_for_post).delete(delete_acknowledgement_for_post),
        )
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
        client_msg_id: None,
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

fn status_ok() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "OK"}))
}

async fn get_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<String>,
) -> ApiResult<Json<mm::Post>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
    let repo = PostRepository::new(state.db.clone());
    let mut post: crate::models::post::PostResponse = repo
        .find_by_id(post_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?
        .into();

    repo.require_channel_membership(post.channel_id, auth.user_id).await?;

    posts::normalize_post_avatar_urls(std::slice::from_mut(&mut post));
    let mut mm_post: mm::Post = post.into();
    let reactions_map = reactions_for_posts(&state, &[post_id]).await?;
    if let Some(reactions) = reactions_map.get(&post_id) {
        if !reactions.is_empty() {
            let mut metadata = mm_post.metadata.clone().unwrap_or_else(|| json!({}));
            if let Some(obj) = metadata.as_object_mut() {
                obj.insert("reactions".to_string(), json!(reactions));
            }
            mm_post.metadata = Some(metadata);
        }
    }

    Ok(Json(mm_post))
}

async fn get_posts_by_ids(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Vec<mm::Post>>> {
    let input: Vec<String> = parse_body(&headers, &body, "Invalid post ids")?;
    if input.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let mut post_ids = Vec::new();
    for id in &input {
        let parsed = parse_mm_or_uuid(id)
            .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
        post_ids.push(parsed);
    }

    let repo = PostRepository::new(state.db.clone());
    let mut posts: Vec<crate::models::post::PostResponse> = repo
        .get_posts_by_ids_for_user(&post_ids, auth.user_id)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    posts::normalize_post_avatar_urls(&mut posts);

    let mut map = std::collections::HashMap::new();
    for post in posts {
        map.insert(post.id, mm::Post::from(post));
    }

    let mut ordered = Vec::new();
    for id in post_ids {
        if let Some(post) = map.remove(&id) {
            ordered.push(post);
        }
    }

    Ok(Json(ordered))
}

async fn get_reactions_by_post_ids(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<std::collections::HashMap<String, Vec<mm::Reaction>>>> {
    let input: Vec<String> = parse_body(&headers, &body, "Invalid post ids")?;
    if input.is_empty() {
        return Ok(Json(std::collections::HashMap::new()));
    }

    let mut post_ids = Vec::new();
    for id in &input {
        let parsed = parse_mm_or_uuid(id)
            .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
        post_ids.push(parsed);
    }

    let repo = PostRepository::new(state.db.clone());
    let visible_ids = repo.get_visible_post_ids(&post_ids, auth.user_id).await?;

    let reactions_map = reactions_for_posts(&state, &visible_ids).await?;
    let mut output = std::collections::HashMap::new();
    for (post_id, reactions) in reactions_map {
        output.insert(encode_mm_id(post_id), reactions);
    }

    Ok(Json(output))
}

async fn get_post_files_info(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<String>,
) -> ApiResult<Json<Vec<mm::FileInfo>>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;

    let repo = PostRepository::new(state.db.clone());
    let post = repo
        .find_by_id(post_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    repo.require_channel_membership(post.channel_id, auth.user_id).await?;

    if post.file_ids.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let files = repo.get_post_files(&post.file_ids).await?;
    let mm_files: Vec<mm::FileInfo> = files.into_iter().map(|f| f.into()).collect();
    Ok(Json(mm_files))
}

async fn pin_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;

    let repo = PostRepository::new(state.db.clone());
    let channel_id = repo.get_post_channel_id(post_id).await?;
    repo.require_channel_membership(channel_id, auth.user_id).await?;
    repo.pin_post(post_id).await?;

    Ok(status_ok())
}

async fn unpin_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;

    let repo = PostRepository::new(state.db.clone());
    let channel_id = repo.get_post_channel_id(post_id).await?;
    repo.require_channel_membership(channel_id, auth.user_id).await?;
    repo.unpin_post(post_id).await?;

    Ok(status_ok())
}

/// Query parameters for thread endpoint
#[derive(Debug, Deserialize)]
pub struct ThreadQuery {
    /// Cursor for pagination (post ID to start after)
    pub cursor: Option<String>,
    /// Maximum number of replies to return (1-100, default 60)
    #[serde(default = "default_thread_limit")]
    pub limit: i64,
}

fn default_thread_limit() -> i64 {
    60
}

/// Mattermost-compatible thread response
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThreadResponseMm {
    pub order: Vec<String>,
    pub posts: std::collections::HashMap<String, mm::Post>,
    pub next_cursor: Option<String>,
}

async fn get_post_thread(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<String>,
    axum::extract::Query(query): axum::extract::Query<ThreadQuery>,
) -> ApiResult<Json<ThreadResponseMm>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;

    // Parse cursor if provided
    let cursor = match query.cursor {
        Some(cursor_str) => Some(
            parse_mm_or_uuid(&cursor_str)
                .ok_or_else(|| AppError::BadRequest("Invalid cursor".to_string()))?,
        ),
        None => None,
    };

    // Call the service method
    let thread_response =
        crate::services::posts::get_thread(&state, post_id, cursor, query.limit).await?;

    // Check channel membership permission
    let first_post = thread_response
        .posts
        .values()
        .next()
        .ok_or_else(|| AppError::NotFound("Thread not found".to_string()))?;

    let repo = PostRepository::new(state.db.clone());
    repo.require_channel_membership(first_post.channel_id, auth.user_id).await?;

    // Convert to Mattermost-compatible format
    let order: Vec<String> = thread_response
        .order
        .iter()
        .map(|id| encode_mm_id(uuid::Uuid::parse_str(id).unwrap_or_default()))
        .collect();

    let posts: std::collections::HashMap<String, mm::Post> = thread_response
        .posts
        .into_iter()
        .map(|(id, post)| {
            let mm_id = encode_mm_id(uuid::Uuid::parse_str(&id).unwrap_or_default());
            let mm_post = mm::Post {
                id: mm_id.clone(),
                create_at: post.created_at.timestamp_millis(),
                update_at: post
                    .edited_at
                    .map(|dt| dt.timestamp_millis())
                    .unwrap_or_else(|| post.created_at.timestamp_millis()),
                delete_at: post.deleted_at.map(|dt| dt.timestamp_millis()).unwrap_or(0),
                edit_at: post.edited_at.map(|dt| dt.timestamp_millis()).unwrap_or(0),
                user_id: encode_mm_id(post.user_id),
                channel_id: encode_mm_id(post.channel_id),
                root_id: post.root_post_id.map(encode_mm_id).unwrap_or_default(),
                original_id: String::new(),
                message: post.message,
                post_type: String::new(),
                props: post.props,
                hashtags: String::new(),
                file_ids: post.file_ids.into_iter().map(encode_mm_id).collect(),
                pending_post_id: post.client_msg_id.unwrap_or_default(),
                metadata: None,
            };
            (mm_id, mm_post)
        })
        .collect();

    let next_cursor = thread_response
        .next_cursor
        .map(|c| encode_mm_id(uuid::Uuid::parse_str(&c).unwrap_or_default()));

    Ok(Json(ThreadResponseMm {
        order,
        posts,
        next_cursor,
    }))
}

async fn handle_post_action(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((post_id, _action_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
    let _value: serde_json::Value = parse_body(&headers, &body, "Invalid action body")?;

    let repo = PostRepository::new(state.db.clone());
    let channel_id = repo.get_post_channel_id(post_id).await?;
    repo.require_channel_membership(channel_id, auth.user_id).await?;

    Ok(status_ok())
}

#[derive(Deserialize)]
struct MovePostRequest {
    #[serde(rename = "channel_id")]
    _channel_id: String,
}

async fn move_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
    let _input: MovePostRequest = parse_body(&headers, &body, "Invalid move body")?;

    let repo = PostRepository::new(state.db.clone());
    let channel_id = repo.get_post_channel_id(post_id).await?;
    repo.require_channel_membership(channel_id, auth.user_id).await?;

    Ok(status_ok())
}

async fn restore_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((post_id, _restore_version_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
    let repo = PostRepository::new(state.db.clone());
    let channel_id = repo.get_post_channel_id(post_id).await?;
    repo.require_channel_membership(channel_id, auth.user_id).await?;
    Ok(status_ok())
}

async fn reveal_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
    let repo = PostRepository::new(state.db.clone());
    let channel_id = repo.get_post_channel_id(post_id).await?;
    repo.require_channel_membership(channel_id, auth.user_id).await?;
    Ok(status_ok())
}

async fn burn_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
    let repo = PostRepository::new(state.db.clone());
    let channel_id = repo.get_post_channel_id(post_id).await?;
    repo.require_channel_membership(channel_id, auth.user_id).await?;
    Ok(status_ok())
}

#[derive(Deserialize)]
struct RewriteRequest {
    message: String,
}

async fn rewrite_post(
    _auth: MmAuthUser,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let input: RewriteRequest = parse_body(&headers, &body, "Invalid rewrite body")?;
    Ok(Json(serde_json::json!({"rewritten_text": input.message})))
}

#[derive(Deserialize)]
struct SetUnreadPath {
    user_id: String,
    post_id: String,
}

#[derive(Deserialize, Default)]
struct SetPostUnreadRequest {
    #[serde(default)]
    collapsed_threads_supported: bool,
}

async fn set_post_unread(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<SetUnreadPath>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::ChannelUnreadAt>> {
    let user_id = super::users::resolve_user_id(&path.user_id, &auth)
        .map_err(|_| AppError::Forbidden("Cannot access another user's posts".to_string()))?;
    let post_id = parse_mm_or_uuid(&path.post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
    let request: SetPostUnreadRequest = if body.is_empty() {
        SetPostUnreadRequest::default()
    } else {
        parse_body(&headers, &body, "Invalid set unread body")?
    };

    let repo = PostRepository::new(state.db.clone());
    let (channel_id, team_id, seq, root_post_id, post_created_at) = repo
        .get_post_with_channel(post_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    repo.require_channel_membership(channel_id, user_id).await?;

    let last_read_id = if seq > 0 { seq - 1 } else { 0 };
    let mark_view_at = post_created_at - chrono::Duration::milliseconds(1);
    let crt_enabled_for_user =
        repo.is_crt_enabled_for_user(user_id, state.config.unread.collapsed_threads_enabled)
            .await?;
    let crt_supported_request = request.collapsed_threads_supported && crt_enabled_for_user;
    let is_reply = root_post_id.is_some();

    repo.upsert_channel_read(user_id, channel_id, last_read_id).await?;

    let username = repo.get_username(user_id).await?;

    let mut stats = repo
        .compute_channel_unread(channel_id, last_read_id, &username)
        .await?;

    if !state.config.unread.post_priority_enabled {
        stats.urgent_mention_count = 0;
    }

    // CRT unsupported + reply follows Mattermost behavior:
    // unread root/urgent counters for the channel are intentionally zeroed.
    let set_unread_count_root = !is_reply || crt_supported_request;
    if !set_unread_count_root {
        stats.unread_msg_count_root = 0;
        stats.mention_count_root = 0;
        stats.urgent_mention_count = 0;
    }

    if is_reply && !crt_supported_request && state.config.unread.thread_auto_follow {
        let thread_root_id = root_post_id.unwrap_or(post_id);
        let (thread_unread_replies, thread_unread_mentions) = repo
            .get_thread_unread_counts(thread_root_id, &username, mark_view_at)
            .await?;

        let unread_replies_count = i32::try_from(thread_unread_replies).unwrap_or(i32::MAX);
        let mention_count = i32::try_from(thread_unread_mentions).unwrap_or(i32::MAX);

        repo.upsert_thread_membership(
            user_id,
            thread_root_id,
            mark_view_at,
            mention_count,
            unread_replies_count,
        )
        .await?;

        // Match Mattermost: only send thread_updated when user CRT is enabled but request
        // came from a CRT-unsupported client.
        if crt_enabled_for_user && !request.collapsed_threads_supported {
            if let Some(thread_row) = repo.fetch_thread_snapshot(thread_root_id, user_id).await? {
                let thread = mm::Thread {
                    id: encode_mm_id(thread_row.id),
                    reply_count: thread_row.reply_count,
                    last_reply_at: thread_row
                        .last_reply_at
                        .map(|dt| dt.timestamp_millis())
                        .unwrap_or(0),
                    last_viewed_at: thread_row
                        .last_read_at
                        .map(|dt| dt.timestamp_millis())
                        .unwrap_or(0),
                    participants: vec![],
                    post: mm::PostInThread {
                        id: encode_mm_id(thread_row.id),
                        channel_id: encode_mm_id(thread_row.channel_id),
                        user_id: encode_mm_id(thread_row.user_id),
                        message: thread_row.message,
                        create_at: thread_row.created_at.timestamp_millis(),
                    },
                    unread_replies: i64::from(thread_row.unread_replies_count),
                    unread_mentions: i64::from(thread_row.mention_count),
                    is_following: Some(thread_row.following),
                };
                if let Ok(payload) = serde_json::to_string(&thread) {
                    let thread_updated = WsEnvelope::event(
                        EventType::ThreadUpdated,
                        serde_json::json!({ "thread": payload }),
                        None,
                    )
                    .with_broadcast(WsBroadcast {
                        channel_id: None,
                        team_id: Some(team_id),
                        user_id: Some(user_id),
                        exclude_user_id: None,
                    });
                    state.ws_hub.broadcast(thread_updated).await;
                }
            }
        }
    }

    let msg_count = (stats.total_msg_count - stats.unread_msg_count).max(0);
    let msg_count_root = (stats.total_msg_count_root - stats.unread_msg_count_root).max(0);
    let mention_count = stats.mention_count.max(0);
    let mention_count_root = stats.mention_count_root.max(0);
    let urgent_mention_count = stats.urgent_mention_count.max(0);

    repo.update_channel_member_unread(
        channel_id,
        user_id,
        mark_view_at,
        msg_count,
        mention_count,
        msg_count_root,
        mention_count_root,
        urgent_mention_count,
    )
    .await?;

    let payload = mm::ChannelUnreadAt {
        team_id: encode_mm_id(team_id),
        user_id: encode_mm_id(user_id),
        channel_id: encode_mm_id(channel_id),
        msg_count,
        mention_count,
        mention_count_root,
        urgent_mention_count,
        msg_count_root,
        last_viewed_at: mark_view_at.timestamp_millis(),
    };

    if state.config.unread.post_unread_ws_enabled {
        let broadcast = WsEnvelope::event(
            EventType::PostUnread,
            serde_json::json!({
                "team_id": payload.team_id,
                "user_id": payload.user_id,
                "channel_id": payload.channel_id,
                "msg_count": payload.msg_count,
                "msg_count_root": payload.msg_count_root,
                "mention_count": payload.mention_count,
                "mention_count_root": payload.mention_count_root,
                "urgent_mention_count": payload.urgent_mention_count,
                "last_viewed_at": payload.last_viewed_at,
                "post_id": encode_mm_id(post_id),
            }),
            Some(channel_id),
        )
        .with_broadcast(WsBroadcast {
            channel_id: Some(channel_id),
            team_id: Some(team_id),
            user_id: Some(user_id),
            exclude_user_id: None,
        });
        state.ws_hub.broadcast(broadcast).await;
    }

    Ok(Json(payload))
}

async fn get_flagged_posts(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<mm::PostList>> {
    let user_id = if user_id == "me" {
        auth.user_id
    } else {
        let parsed = parse_mm_or_uuid(&user_id)
            .ok_or_else(|| AppError::BadRequest("Invalid user_id".to_string()))?;
        if !auth.can_access_owned(parsed, &permissions::USER_MANAGE) {
            return Err(AppError::Forbidden(
                "Cannot access another user's posts".to_string(),
            ));
        }
        parsed
    };

    let repo = PostRepository::new(state.db.clone());
    let mut posts: Vec<crate::models::post::PostResponse> = repo
        .get_flagged_posts(user_id)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    posts::normalize_post_avatar_urls(&mut posts);

    let mut order = Vec::new();
    let mut posts_map: std::collections::HashMap<String, mm::Post> =
        std::collections::HashMap::new();
    let mut post_ids = Vec::new();
    let mut id_map = Vec::new();

    for p in posts {
        let id = encode_mm_id(p.id);
        post_ids.push(p.id);
        id_map.push((p.id, id.clone()));
        order.push(id.clone());
        posts_map.insert(id, p.into());
    }

    let reactions_map = reactions_for_posts(&state, &post_ids).await?;
    for (post_uuid, post_id) in id_map {
        if let Some(reactions) = reactions_map.get(&post_uuid) {
            if !reactions.is_empty() {
                if let Some(post) = posts_map.get_mut(&post_id) {
                    post.metadata = Some(json!({ "reactions": reactions }));
                }
            }
        }
    }

    Ok(Json(mm::PostList {
        order,
        posts: posts_map,
        next_post_id: String::new(),
        prev_post_id: String::new(),
    }))
}

async fn delete_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
    let repo = PostRepository::new(state.db.clone());
    let (post_user_id, post_channel_id, _, _) = repo
        .get_post_basic(post_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    if post_user_id != auth.user_id {
        return Err(AppError::Forbidden(
            "Cannot delete others' posts".to_string(),
        ));
    }

    let mut deleted_post: crate::models::post::PostResponse =
        repo.soft_delete_post(post_id).await?.into();

    posts::normalize_post_avatar_urls(std::slice::from_mut(&mut deleted_post));

    let broadcast = WsEnvelope::event(
        EventType::MessageDeleted,
        serde_json::json!({
            "post_id": post_id,
            "channel_id": post_channel_id
        }),
        Some(post_channel_id),
    )
    .with_broadcast(WsBroadcast {
        channel_id: Some(post_channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(
        serde_json::json!({"status": "OK", "id": encode_mm_id(post_id)}),
    ))
}

#[derive(Deserialize)]
struct PatchPostRequest {
    message: String,
}

#[derive(Deserialize)]
struct UpdatePostRequest {
    id: String,
    #[serde(default)]
    message: String,
}

async fn update_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<String>,
    Json(input): Json<UpdatePostRequest>,
) -> ApiResult<Json<mm::Post>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
    let body_post_id = parse_mm_or_uuid(&input.id)
        .ok_or_else(|| AppError::BadRequest("Invalid id".to_string()))?;

    if post_id != body_post_id {
        return Err(AppError::BadRequest("Invalid id".to_string()));
    }

    update_post_message(&state, auth.user_id, post_id, input.message).await
}

async fn patch_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<String>,
    Json(input): Json<PatchPostRequest>,
) -> ApiResult<Json<mm::Post>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;
    update_post_message(&state, auth.user_id, post_id, input.message).await
}

async fn update_post_message(
    state: &AppState,
    acting_user_id: Uuid,
    post_id: Uuid,
    message: String,
) -> ApiResult<Json<mm::Post>> {
    let repo = PostRepository::new(state.db.clone());
    let (post_user_id, post_channel_id, post_created_at, original_message) = repo
        .get_post_basic(post_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    if post_user_id != acting_user_id {
        return Err(AppError::Forbidden("Cannot edit others' posts".to_string()));
    }

    if message != original_message {
        let post_edit_time_limit_seconds = repo.load_post_edit_time_limit_seconds().await?;
        if post_edit_time_limit_seconds == 0 {
            return Err(AppError::BadRequest(
                "Message editing is disabled by server policy".to_string(),
            ));
        }
        if post_edit_time_limit_seconds > 0 {
            let post_age_seconds = Utc::now()
                .signed_duration_since(post_created_at)
                .num_seconds();
            if post_age_seconds >= post_edit_time_limit_seconds {
                return Err(AppError::BadRequest(format!(
                    "Message edit window expired after {} seconds",
                    post_edit_time_limit_seconds
                )));
            }
        }
    }

    let mut updated: crate::models::post::PostResponse =
        repo.update_post_message(post_id, message).await?.into();

    posts::normalize_post_avatar_urls(std::slice::from_mut(&mut updated));

    let broadcast = WsEnvelope::event(
        EventType::MessageUpdated,
        updated.clone(),
        Some(post_channel_id),
    )
    .with_broadcast(WsBroadcast {
        channel_id: Some(post_channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(updated.into()))
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

    let repo = PostRepository::new(state.db.clone());
    let rows = repo.list_scheduled_posts(auth.user_id, team_id).await?;

    let posts = rows
        .into_iter()
        .map(|r| mm::ScheduledPost {
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
        })
        .collect();

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
        .collect::<Vec<_>>();
    let scheduled_at = chrono::DateTime::from_timestamp_millis(input.scheduled_at)
        .ok_or_else(|| AppError::Validation("Invalid scheduled_at".to_string()))?;

    let repo = PostRepository::new(state.db.clone());
    let row = repo
        .create_scheduled_post(
            auth.user_id,
            channel_id,
            root_id,
            &input.message,
            &input.props,
            &file_ids,
            scheduled_at,
        )
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

#[derive(Deserialize)]
struct UpdateScheduledPostRequest {
    id: String,
    channel_id: String,
    user_id: String,
    message: String,
    scheduled_at: i64,
    #[serde(default)]
    root_id: String,
    #[serde(default)]
    props: serde_json::Value,
    #[serde(default)]
    file_ids: Vec<String>,
}

async fn update_scheduled_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(scheduled_post_id): Path<String>,
    Json(input): Json<UpdateScheduledPostRequest>,
) -> ApiResult<Json<mm::ScheduledPost>> {
    if input.id != scheduled_post_id {
        return Err(AppError::BadRequest(
            "Scheduled post id mismatch".to_string(),
        ));
    }

    let scheduled_id = parse_mm_or_uuid(&scheduled_post_id)
        .ok_or_else(|| AppError::Validation("Invalid scheduled_post_id".to_string()))?;
    let channel_id = parse_mm_or_uuid(&input.channel_id)
        .ok_or_else(|| AppError::Validation("Invalid channel_id".to_string()))?;
    let user_id = parse_mm_or_uuid(&input.user_id)
        .ok_or_else(|| AppError::Validation("Invalid user_id".to_string()))?;

    if user_id != auth.user_id {
        return Err(AppError::Forbidden(
            "Cannot update another user's scheduled post".to_string(),
        ));
    }

    let root_id = if !input.root_id.is_empty() {
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
        .collect::<Vec<_>>();
    let scheduled_at = chrono::DateTime::from_timestamp_millis(input.scheduled_at)
        .ok_or_else(|| AppError::Validation("Invalid scheduled_at".to_string()))?;

    let repo = PostRepository::new(state.db.clone());
    let row = repo
        .update_scheduled_post(
            scheduled_id,
            auth.user_id,
            channel_id,
            root_id,
            &input.message,
            &input.props,
            &file_ids,
            scheduled_at,
        )
        .await?
        .ok_or_else(|| AppError::NotFound("Scheduled post not found".to_string()))?;

    Ok(Json(mm::ScheduledPost {
        id: scheduled_post_id,
        user_id: input.user_id,
        channel_id: input.channel_id,
        root_id: input.root_id,
        message: input.message,
        props: input.props,
        file_ids: input.file_ids,
        scheduled_at: input.scheduled_at,
        create_at: row.0.timestamp_millis(),
        update_at: row.1.timestamp_millis(),
    }))
}

async fn delete_scheduled_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(scheduled_post_id): Path<String>,
) -> ApiResult<Json<mm::ScheduledPost>> {
    let scheduled_id = parse_mm_or_uuid(&scheduled_post_id)
        .ok_or_else(|| AppError::Validation("Invalid scheduled_post_id".to_string()))?;

    let repo = PostRepository::new(state.db.clone());
    let row = repo
        .delete_scheduled_post(scheduled_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Scheduled post not found".to_string()))?;

    Ok(Json(mm::ScheduledPost {
        id: scheduled_post_id,
        user_id: encode_mm_id(row.1),
        channel_id: encode_mm_id(row.0),
        root_id: row.2.clone(),
        message: row.3.clone(),
        props: row.4.clone(),
        file_ids: row.5.iter().map(|id| encode_mm_id(*id)).collect(),
        scheduled_at: row.6,
        create_at: row.7.timestamp_millis(),
        update_at: row.8.timestamp_millis(),
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
        return Err(AppError::Forbidden(
            "Cannot send ephemeral post to others".to_string(),
        ));
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
        return Err(AppError::Forbidden(
            "Cannot set reminder for others".to_string(),
        ));
    }

    let post_id = parse_mm_or_uuid(&post_id_str)
        .ok_or_else(|| AppError::Validation("Invalid post_id".to_string()))?;

    let target_at = chrono::DateTime::from_timestamp_millis(input.target_at)
        .ok_or_else(|| AppError::Validation("Invalid target_at".to_string()))?;

    let repo = PostRepository::new(state.db.clone());
    repo.set_post_reminder(auth.user_id, post_id, target_at).await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}
