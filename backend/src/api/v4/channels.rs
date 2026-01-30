use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::de::DeserializeOwned;

use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};
use crate::models::post::PostResponse;
use crate::models::Channel;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/channels/{channel_id}/posts", get(get_posts))
        .route("/channels/{channel_id}", get(get_channel).put(update_channel).delete(delete_channel))
        .route("/channels/{channel_id}/members", get(get_channel_members).post(add_channel_member))
        .route("/channels/{channel_id}/members/me", get(get_channel_member_me))
        .route("/channels/{channel_id}/members/{user_id}", delete(remove_channel_member))
        .route("/channels/{channel_id}/timezones", get(get_channel_timezones))
        .route("/channels/{channel_id}/stats", get(get_channel_stats))
        .route("/channels/{channel_id}/unread", get(get_channel_unread))
        .route("/channels/{channel_id}/pinned", get(get_pinned_posts))
        .route("/channels/{channel_id}/posts/{post_id}/pin", post(pin_post))
        .route("/channels/{channel_id}/posts/{post_id}/unpin", post(unpin_post))
        .route("/channels/members/me/view", post(view_channel))
        .route("/channels/direct", post(create_direct_channel))
        .route("/channels/group", post(create_group_channel))
        .route("/channels", post(create_channel))
}



#[derive(Deserialize)]
struct Pagination {
    page: Option<u64>,
    per_page: Option<u64>,
    /// Post ID to fetch posts before (for backward pagination)
    before: Option<String>,
    /// Post ID to fetch posts after (for forward pagination)  
    after: Option<String>,
    /// Timestamp in milliseconds to fetch posts since (for incremental sync)
    since: Option<i64>,
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
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<impl IntoResponse> {
    if body.is_empty() {
        return Ok(Json(serde_json::json!({"status": "OK"})));
    }

    let input = match parse_view_channel_request(&headers, &body) {
        Ok(value) => value,
        Err(_) => return Ok(Json(serde_json::json!({"status": "OK"}))),
    };

    if let Some(channel_id) = parse_mm_or_uuid(&input.channel_id) {
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
    }

    Ok(Json(serde_json::json!({"status": "OK"})))
}

fn parse_view_channel_request(
    headers: &axum::http::HeaderMap,
    body: &Bytes,
) -> ApiResult<ViewChannelRequest> {
    parse_body(headers, body, "Invalid view body")
}

fn parse_body<T: DeserializeOwned>(
    headers: &axum::http::HeaderMap,
    body: &Bytes,
    message: &str,
) -> ApiResult<T> {
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.starts_with("application/json") {
        serde_json::from_slice(body)
            .map_err(|_| crate::error::AppError::BadRequest(message.to_string()))
    } else if content_type.starts_with("application/x-www-form-urlencoded") {
        serde_urlencoded::from_bytes(body)
            .map_err(|_| crate::error::AppError::BadRequest(message.to_string()))
    } else {
        serde_json::from_slice(body)
            .or_else(|_| serde_urlencoded::from_bytes(body))
            .map_err(|_| crate::error::AppError::BadRequest(message.to_string()))
    }
}

async fn get_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<mm::Channel>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;
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

/// GET /channels/{channel_id}/unread - Get unread counts for a channel
async fn get_channel_unread(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;

    // Get the member's last viewed time
    let member: Option<crate::models::ChannelMember> = sqlx::query_as(
        "SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2"
    )
    .bind(channel_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?;

    let member = member.ok_or_else(|| 
        crate::error::AppError::Forbidden("Not a member of this channel".to_string()))?;

    // Count messages since last viewed
    let msg_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM posts
        WHERE channel_id = $1 
          AND deleted_at IS NULL
          AND created_at > $2
        "#
    )
    .bind(channel_id)
    .bind(member.last_viewed_at)
    .fetch_one(&state.db)
    .await?;

    // Get the user's username for mention detection
    let username: Option<String> = sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_optional(&state.db)
        .await?;
    let username = username.unwrap_or_default();

    // Count mentions (posts that mention the user)
    let mention_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM posts
        WHERE channel_id = $1 
          AND deleted_at IS NULL
          AND created_at > $2
          AND (message LIKE '%@' || $3 || '%' OR message LIKE '%@all%' OR message LIKE '%@channel%')
        "#
    )
    .bind(channel_id)
    .bind(member.last_viewed_at)
    .bind(&username)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    Ok(Json(serde_json::json!({
        "team_id": "",
        "channel_id": encode_mm_id(channel_id),
        "msg_count": msg_count,
        "mention_count": mention_count,
        "mention_count_root": mention_count,
        "msg_count_root": msg_count,
        "last_viewed_at": member.last_viewed_at.map(|t| t.timestamp_millis()).unwrap_or(0)
    })))
}

async fn get_channel_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<Vec<mm::ChannelMember>>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;
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
            channel_id: encode_mm_id(m.channel_id),
            user_id: encode_mm_id(m.user_id),
            roles: "channel_user".to_string(),
            last_viewed_at: m.last_viewed_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            msg_count: 0,
            mention_count: 0,
            notify_props: normalize_notify_props(m.notify_props),
            last_update_at: 0,
            scheme_guest: false,
            scheme_user: true,
            scheme_admin: false,
        })
        .collect();

    Ok(Json(mm_members))
}

async fn get_channel_member_me(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<mm::ChannelMember>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;
    let member: crate::models::ChannelMember = sqlx::query_as(
        "SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2",
    )
    .bind(channel_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::AppError::Forbidden("Not a member of this channel".to_string()))?;

    Ok(Json(mm::ChannelMember {
        channel_id: encode_mm_id(member.channel_id),
        user_id: encode_mm_id(member.user_id),
        roles: "channel_user".to_string(),
        last_viewed_at: member.last_viewed_at.map(|t| t.timestamp_millis()).unwrap_or(0),
        msg_count: 0,
        mention_count: 0,
        notify_props: normalize_notify_props(member.notify_props),
        last_update_at: 0,
        scheme_guest: false,
        scheme_user: true,
        scheme_admin: false,
    }))
}

fn normalize_notify_props(value: serde_json::Value) -> serde_json::Value {
    if value.is_null() {
        return serde_json::json!({"desktop": "default", "mark_unread": "all"});
    }

    if let Some(obj) = value.as_object() {
        if obj.is_empty() {
            return serde_json::json!({"desktop": "default", "mark_unread": "all"});
        }
    }

    value
}

async fn get_channel_stats(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<mm::ChannelStats>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;
    let is_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM channel_members WHERE channel_id = $1 AND user_id = $2)",
    )
    .bind(channel_id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if !is_member {
        return Err(crate::error::AppError::Forbidden("Not a member of this channel".to_string()));
    }

    let member_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM channel_members WHERE channel_id = $1",
    )
    .bind(channel_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(mm::ChannelStats {
        channel_id: encode_mm_id(channel_id),
        member_count,
    }))
}

async fn get_channel_timezones(
    Path(_channel_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

#[derive(serde::Deserialize)]
struct DirectChannelRequest {
    user_ids: Vec<String>,
}

async fn create_direct_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::Channel>> {
    // Mattermost sends either a plain array ["id1", "id2"] or an object {"user_ids": ["id1", "id2"]}
    // Try parsing as plain array first, then fall back to object format
    let user_ids: Vec<String> = serde_json::from_slice::<Vec<String>>(&body)
        .or_else(|_| {
            parse_body::<DirectChannelRequest>(&headers, &body, "Invalid user_ids")
                .map(|req| req.user_ids)
        })?;

    if user_ids.len() != 2 {
        return Err(crate::error::AppError::BadRequest("Request body must contain exactly 2 user IDs".to_string()));
    }

    let mut ids: Vec<Uuid> = user_ids
        .iter()
        .filter_map(|id| parse_mm_or_uuid(id))
        .collect();

    if ids.len() != 2 {
        return Err(crate::error::AppError::BadRequest("Invalid user IDs provided".to_string()));
    }

    if !ids.contains(&auth.user_id) {
        return Err(crate::error::AppError::Forbidden("Must include your user id".to_string()));
    }

    ids.sort();
    let name = format!("dm_{}_{}", ids[0], ids[1]);

    let team_id: Uuid = sqlx::query_scalar(
        "SELECT team_id FROM team_members WHERE user_id = $1 ORDER BY created_at ASC LIMIT 1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::AppError::BadRequest("User has no team".to_string()))?;

    let channel: Channel = sqlx::query_as(
        r#"
        INSERT INTO channels (team_id, type, name, display_name, purpose, header, creator_id)
        VALUES ($1, 'direct', $2, '', '', '', $3)
        ON CONFLICT (team_id, name) DO UPDATE SET name = EXCLUDED.name
        RETURNING *
        "#,
    )
    .bind(team_id)
    .bind(&name)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    for user_id in ids {
        sqlx::query(
            "INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, 'member') ON CONFLICT DO NOTHING",
        )
        .bind(channel.id)
        .bind(user_id)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(channel.into()))
}

/// POST /channels/group - Create group DM (3+ users)
async fn create_group_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::Channel>> {
    // Group DMs also use array format
    let input: DirectChannelRequest = parse_body(&headers, &body, "Invalid user_ids")?;

    if input.user_ids.len() < 2 {
        return Err(crate::error::AppError::BadRequest("user_ids must contain at least 2 users".to_string()));
    }

    let mut ids: Vec<Uuid> = input
        .user_ids
        .iter()
        .filter_map(|id| parse_mm_or_uuid(id))
        .collect();

    if !ids.contains(&auth.user_id) {
        ids.push(auth.user_id);
    }

    ids.sort();
    let name = format!("gm_{}", ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join("_"));

    let team_id: Uuid = sqlx::query_scalar(
        "SELECT team_id FROM team_members WHERE user_id = $1 ORDER BY created_at ASC LIMIT 1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::AppError::BadRequest("User has no team".to_string()))?;

    // Generate display name from usernames
    let usernames: Vec<String> = sqlx::query_scalar("SELECT username FROM users WHERE id = ANY($1)")
        .bind(&ids)
        .fetch_all(&state.db)
        .await?;
    let display_name = usernames.join(", ");

    let channel: Channel = sqlx::query_as(
        r#"
        INSERT INTO channels (team_id, type, name, display_name, purpose, header, creator_id)
        VALUES ($1, 'group', $2, $3, '', '', $4)
        ON CONFLICT (team_id, name) DO UPDATE SET name = EXCLUDED.name
        RETURNING *
        "#,
    )
    .bind(team_id)
    .bind(&name)
    .bind(&display_name)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    for user_id in ids {
        sqlx::query(
            "INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, 'member') ON CONFLICT DO NOTHING",
        )
        .bind(channel.id)
        .bind(user_id)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(channel.into()))
}

/// POST /channels - Create a new channel
#[derive(serde::Deserialize)]
struct CreateChannelRequest {
    team_id: String,
    name: String,
    display_name: String,
    #[serde(rename = "type", default = "default_channel_type")]
    channel_type: String,
    #[serde(default)]
    purpose: String,
    #[serde(default)]
    header: String,
}

fn default_channel_type() -> String {
    "O".to_string()
}

async fn create_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::Channel>> {
    let input: CreateChannelRequest = parse_body(&headers, &body, "Invalid channel body")?;

    let team_id = parse_mm_or_uuid(&input.team_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;

    // Verify team membership
    let is_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2)"
    )
    .bind(team_id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if !is_member {
        return Err(crate::error::AppError::Forbidden("Not a member of this team".to_string()));
    }

    // Map MM channel type to RustChat type
    let channel_type = match input.channel_type.as_str() {
        "O" => "public",
        "P" => "private",
        _ => "public",
    };

    let channel: Channel = sqlx::query_as(
        r#"
        INSERT INTO channels (team_id, type, name, display_name, purpose, header, creator_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(team_id)
    .bind(channel_type)
    .bind(&input.name)
    .bind(&input.display_name)
    .bind(&input.purpose)
    .bind(&input.header)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    // Add creator as member
    sqlx::query(
        "INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, 'admin') ON CONFLICT DO NOTHING",
    )
    .bind(channel.id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(channel.into()))
}

/// PUT /channels/{channel_id} - Update channel
#[derive(serde::Deserialize)]
struct UpdateChannelRequest {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    purpose: Option<String>,
    #[serde(default)]
    header: Option<String>,
}

async fn update_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::Channel>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;

    // Verify membership
    let _membership: crate::models::ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| crate::error::AppError::Forbidden("Not a member of this channel".to_string()))?;

    let input: UpdateChannelRequest = parse_body(&headers, &body, "Invalid channel update")?;

    // Build update query dynamically
    let channel: Channel = sqlx::query_as(
        r#"
        UPDATE channels SET
            display_name = COALESCE($2, display_name),
            name = COALESCE($3, name),
            purpose = COALESCE($4, purpose),
            header = COALESCE($5, header),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(channel_id)
    .bind(&input.display_name)
    .bind(&input.name)
    .bind(&input.purpose)
    .bind(&input.header)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(channel.into()))
}

/// DELETE /channels/{channel_id} - Delete/archive channel
async fn delete_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;

    // Verify membership (should be admin but simplified for now)
    let _membership: crate::models::ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| crate::error::AppError::Forbidden("Not a member of this channel".to_string()))?;

    // Soft delete the channel
    sqlx::query("UPDATE channels SET deleted_at = NOW() WHERE id = $1")
        .bind(channel_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// GET /channels/{channel_id}/pinned - Get pinned posts
async fn get_pinned_posts(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<mm::PostList>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;

    // Verify membership
    let _membership: crate::models::ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| crate::error::AppError::Forbidden("Not a member of this channel".to_string()))?;

    let posts: Vec<PostResponse> = sqlx::query_as(
        r#"
        SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
               p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
               p.reply_count::int8 as reply_count,
               p.last_reply_at, p.seq,
               u.username, u.avatar_url, u.email
        FROM posts p
        LEFT JOIN users u ON p.user_id = u.id
        WHERE p.channel_id = $1 AND p.is_pinned = true AND p.deleted_at IS NULL
        ORDER BY p.created_at DESC
        "#,
    )
    .bind(channel_id)
    .fetch_all(&state.db)
    .await?;

    let mut order = Vec::new();
    let mut posts_map = HashMap::new();

    for p in posts {
        let id = encode_mm_id(p.id);
        order.push(id.clone());
        posts_map.insert(id, p.into());
    }

    Ok(Json(mm::PostList {
        order,
        posts: posts_map,
        next_post_id: String::new(),
        prev_post_id: String::new(),
    }))
}

/// Path for pin/unpin operations
#[derive(serde::Deserialize)]
struct PinPath {
    channel_id: String,
    post_id: String,
}

/// POST /channels/{channel_id}/posts/{post_id}/pin - Pin a post
async fn pin_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<PinPath>,
) -> ApiResult<impl IntoResponse> {
    let channel_id = parse_mm_or_uuid(&path.channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;
    let post_id = parse_mm_or_uuid(&path.post_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid post_id".to_string()))?;

    // Verify membership
    let _membership: crate::models::ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| crate::error::AppError::Forbidden("Not a member of this channel".to_string()))?;

    // Pin the post
    sqlx::query("UPDATE posts SET is_pinned = true WHERE id = $1 AND channel_id = $2")
        .bind(post_id)
        .bind(channel_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// POST /channels/{channel_id}/posts/{post_id}/unpin - Unpin a post
async fn unpin_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<PinPath>,
) -> ApiResult<impl IntoResponse> {
    let channel_id = parse_mm_or_uuid(&path.channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;
    let post_id = parse_mm_or_uuid(&path.post_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid post_id".to_string()))?;

    // Verify membership
    let _membership: crate::models::ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| crate::error::AppError::Forbidden("Not a member of this channel".to_string()))?;

    // Unpin the post
    sqlx::query("UPDATE posts SET is_pinned = false WHERE id = $1 AND channel_id = $2")
        .bind(post_id)
        .bind(channel_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

#[derive(serde::Deserialize)]
struct AddMemberRequest {
    user_id: String,
}

async fn add_channel_member(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::ChannelMember>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;

    let input: AddMemberRequest = parse_body(&headers, &body, "Invalid member body")?;

    let user_id = parse_mm_or_uuid(&input.user_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid user_id".to_string()))?;

    // Verify caller is member of the channel
    let _caller_member: crate::models::ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| crate::error::AppError::Forbidden("Not a member of this channel".to_string()))?;

    // Add the user
    sqlx::query(
        "INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, 'member') ON CONFLICT DO NOTHING",
    )
    .bind(channel_id)
    .bind(user_id)
    .execute(&state.db)
    .await?;

    // Fetch and return the new member
    let member: crate::models::ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(user_id)
            .fetch_one(&state.db)
            .await?;

    Ok(Json(mm::ChannelMember {
        channel_id: encode_mm_id(member.channel_id),
        user_id: encode_mm_id(member.user_id),
        roles: "channel_user".to_string(),
        last_viewed_at: member.last_viewed_at.map(|t| t.timestamp_millis()).unwrap_or(0),
        msg_count: 0,
        mention_count: 0,
        notify_props: normalize_notify_props(member.notify_props),
        last_update_at: 0,
        scheme_guest: false,
        scheme_user: true,
        scheme_admin: false,
    }))
}

/// DELETE /channels/{channel_id}/members/{user_id} - Remove a member from a channel
#[derive(serde::Deserialize)]
struct ChannelMemberPath {
    channel_id: String,
    user_id: String,
}

async fn remove_channel_member(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ChannelMemberPath>,
) -> ApiResult<impl IntoResponse> {
    let channel_id = parse_mm_or_uuid(&path.channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;

    let user_id = parse_mm_or_uuid(&path.user_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid user_id".to_string()))?;

    // Verify caller is member of the channel (or is the user being removed)
    if auth.user_id != user_id {
        let _caller_member: crate::models::ChannelMember =
            sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
                .bind(channel_id)
                .bind(auth.user_id)
                .fetch_optional(&state.db)
                .await?
                .ok_or_else(|| crate::error::AppError::Forbidden("Not a member of this channel".to_string()))?;
    }

    // Remove the user
    sqlx::query("DELETE FROM channel_members WHERE channel_id = $1 AND user_id = $2")
        .bind(channel_id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn get_posts(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Query(pagination): Query<Pagination>,
) -> ApiResult<Json<mm::PostList>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;
    
    // Check channel membership first
    let _membership: crate::models::ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| {
                crate::error::AppError::Forbidden("Not a member of this channel".to_string())
            })?;

    let per_page = pagination.per_page.unwrap_or(60).min(200) as i64;

    // Determine query type based on pagination params
    let posts: Vec<PostResponse> = if let Some(since) = pagination.since {
        // Incremental sync: get posts created or edited since timestamp
        let since_time = chrono::DateTime::from_timestamp_millis(since)
            .unwrap_or_else(|| chrono::Utc::now());
        
        sqlx::query_as(
            r#"
            SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                   p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                   p.reply_count::int8 as reply_count,
                   p.last_reply_at, p.seq,
                   u.username, u.avatar_url, u.email
            FROM posts p
            LEFT JOIN users u ON p.user_id = u.id
            WHERE p.channel_id = $1 
              AND (p.created_at >= $2 OR p.edited_at >= $2)
            ORDER BY p.created_at ASC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(since_time)
        .bind(per_page)
        .fetch_all(&state.db)
        .await?
    } else if let Some(before) = &pagination.before {
        // Cursor pagination: get posts before a specific post
        let before_id = parse_mm_or_uuid(before)
            .ok_or_else(|| crate::error::AppError::BadRequest("Invalid before post_id".to_string()))?;
        
        // Get the created_at of the before post
        let before_time: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
            "SELECT created_at FROM posts WHERE id = $1"
        )
        .bind(before_id)
        .fetch_optional(&state.db)
        .await?;

        let before_time = before_time.ok_or_else(|| 
            crate::error::AppError::NotFound("Before post not found".to_string()))?;

        sqlx::query_as(
            r#"
            SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                   p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                   p.reply_count::int8 as reply_count,
                   p.last_reply_at, p.seq,
                   u.username, u.avatar_url, u.email
            FROM posts p
            LEFT JOIN users u ON p.user_id = u.id
            WHERE p.channel_id = $1 
              AND p.deleted_at IS NULL
              AND p.created_at < $2
            ORDER BY p.created_at DESC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(before_time)
        .bind(per_page)
        .fetch_all(&state.db)
        .await?
    } else if let Some(after) = &pagination.after {
        // Cursor pagination: get posts after a specific post
        let after_id = parse_mm_or_uuid(after)
            .ok_or_else(|| crate::error::AppError::BadRequest("Invalid after post_id".to_string()))?;
        
        // Get the created_at of the after post
        let after_time: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
            "SELECT created_at FROM posts WHERE id = $1"
        )
        .bind(after_id)
        .fetch_optional(&state.db)
        .await?;

        let after_time = after_time.ok_or_else(|| 
            crate::error::AppError::NotFound("After post not found".to_string()))?;

        sqlx::query_as(
            r#"
            SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                   p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                   p.reply_count::int8 as reply_count,
                   p.last_reply_at, p.seq,
                   u.username, u.avatar_url, u.email
            FROM posts p
            LEFT JOIN users u ON p.user_id = u.id
            WHERE p.channel_id = $1 
              AND p.deleted_at IS NULL
              AND p.created_at > $2
            ORDER BY p.created_at ASC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(after_time)
        .bind(per_page)
        .fetch_all(&state.db)
        .await?
    } else {
        // Standard page-based pagination
        let page = pagination.page.unwrap_or(0);
        let offset = (page * per_page as u64) as i64;

        sqlx::query_as(
            r#"
            SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                   p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                   p.reply_count::int8 as reply_count,
                   p.last_reply_at, p.seq,
                   u.username, u.avatar_url, u.email
            FROM posts p
            LEFT JOIN users u ON p.user_id = u.id
            WHERE p.channel_id = $1 AND p.deleted_at IS NULL
            ORDER BY p.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(channel_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    };

    let mut order = Vec::new();
    let mut posts_map = HashMap::new();

    // Determine prev/next post IDs for pagination hints
    let (prev_post_id, next_post_id) = if !posts.is_empty() {
        let first_id = encode_mm_id(posts.first().unwrap().id);
        let last_id = encode_mm_id(posts.last().unwrap().id);
        // If using before/after, provide the opposite cursor
        if pagination.before.is_some() {
            (last_id, String::new())
        } else if pagination.after.is_some() {
            (String::new(), first_id)
        } else {
            (String::new(), String::new())
        }
    } else {
        (String::new(), String::new())
    };

    for p in posts {
        let id = encode_mm_id(p.id);
        order.push(id.clone());
        posts_map.insert(id, p.into());
    }

    Ok(Json(mm::PostList {
        order,
        posts: posts_map,
        next_post_id,
        prev_post_id,
    }))
}

