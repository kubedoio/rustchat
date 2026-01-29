use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::HashMap;

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};
use crate::models::post::PostResponse;
use crate::models::Channel;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/channels/{channel_id}/posts", get(get_posts))
        .route("/channels/{channel_id}", get(get_channel))
        .route("/channels/{channel_id}/members", get(get_channel_members))
        .route("/channels/{channel_id}/members/me", get(get_channel_member_me))
        .route("/channels/{channel_id}/timezones", get(get_channel_timezones))
        .route("/channels/{channel_id}/stats", get(get_channel_stats))
        .route("/channels/members/me/view", post(view_channel))
        .route("/channels/direct", post(create_direct_channel))
}

#[derive(Deserialize)]
struct Pagination {
    page: Option<u64>,
    per_page: Option<u64>,
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
    let input: DirectChannelRequest = parse_body(&headers, &body, "Invalid user_ids")?;

    if input.user_ids.len() != 2 {
        return Err(crate::error::AppError::BadRequest("user_ids must contain 2 users".to_string()));
    }

    let mut ids: Vec<Uuid> = input
        .user_ids
        .iter()
        .filter_map(|id| parse_mm_or_uuid(id))
        .collect();

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

async fn get_posts(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Query(pagination): Query<Pagination>,
) -> ApiResult<Json<mm::PostList>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;
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

    let page = pagination.page.unwrap_or(0);
    let per_page = pagination.per_page.unwrap_or(60).min(200);
    let offset = page * per_page;

    // Fetch posts
    let posts: Vec<PostResponse> = sqlx::query_as(
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
    .bind(per_page as i64)
    .bind(offset as i64)
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
        next_post_id: "".to_string(),
        prev_post_id: "".to_string(),
    }))
}
