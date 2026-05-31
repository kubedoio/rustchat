use axum::{body::Bytes, extract::State, http::HeaderMap, Json};

use super::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{
    id::{encode_mm_id, parse_mm_or_uuid},
    models as mm,
};
use crate::models::channel::Channel;

pub async fn user_typing(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Path((user_id, channel_id)): axum::extract::Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    let user_id = parse_mm_or_uuid(&user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user ID".to_string()))?;
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| AppError::BadRequest("Invalid channel ID".to_string()))?;
    if user_id != auth.user_id {
        return Err(AppError::Forbidden("Mismatch user_id".to_string()));
    }

    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::UserTyping,
        crate::realtime::TypingEvent {
            user_id: auth.user_id,
            display_name: "".to_string(), // Fetched by client usually
            thread_root_id: None,
        },
        Some(channel_id),
    )
    .with_broadcast(crate::realtime::WsBroadcast {
        channel_id: Some(channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: Some(auth.user_id),
    });

    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

pub async fn get_user_group_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let channels: Vec<Channel> = sqlx::query_as(
        r#"
        SELECT c.* FROM channels c
        JOIN channel_members cm ON c.id = cm.channel_id
        WHERE cm.user_id = $1 AND c.type = 'group'
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(channels.into_iter().map(|c| c.into()).collect()))
}

/// POST /users/group_channels - Get user profiles for multiple group/DM channels
/// Mobile client sends array of channel IDs and expects map of channel_id -> [user profiles]
pub async fn get_profiles_in_group_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    _headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<std::collections::HashMap<String, Vec<mm::User>>>> {
    // Parse channel IDs from body (sent as raw JSON array)
    let channel_ids: Vec<String> = serde_json::from_slice(&body)
        .map_err(|_| AppError::BadRequest("Expected array of channel IDs".to_string()))?;

    if channel_ids.is_empty() {
        return Ok(Json(std::collections::HashMap::new()));
    }

    // Convert to UUIDs
    let channel_uuids: Vec<uuid::Uuid> = channel_ids
        .iter()
        .filter_map(|id| parse_mm_or_uuid(id))
        .collect();

    if channel_uuids.is_empty() {
        return Ok(Json(std::collections::HashMap::new()));
    }

    // Query users for each channel the requesting user is a member of
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        uuid::Uuid,
        uuid::Uuid,
        String,
        String,
        Option<String>,
        Option<String>,
        bool,
    )> = sqlx::query_as(
        r#"
        SELECT 
            cm.channel_id,
            u.id,
            u.username,
            u.email,
            u.display_name,
            u.avatar_url,
            u.is_active
        FROM channel_members cm
        JOIN users u ON cm.user_id = u.id
        WHERE cm.channel_id = ANY($1)
          AND EXISTS (
              SELECT 1 FROM channel_members cm2 
              WHERE cm2.channel_id = cm.channel_id AND cm2.user_id = $2
          )
        ORDER BY cm.channel_id, u.username
        "#,
    )
    .bind(&channel_uuids)
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    // Group by channel_id
    let mut result: std::collections::HashMap<String, Vec<mm::User>> =
        std::collections::HashMap::new();
    for (channel_id, user_id, username, email, display_name, _avatar_url, _is_active) in rows {
        let mm_user = mm::User {
            id: encode_mm_id(user_id),
            username,
            email,
            nickname: display_name.clone().unwrap_or_default(),
            first_name: display_name.unwrap_or_default(),
            last_name: "".to_string(),
            email_verified: true,
            auth_service: "".to_string(),
            roles: "system_user".to_string(),
            locale: "en".to_string(),
            timezone: serde_json::json!({}),
            create_at: 0,
            update_at: 0,
            delete_at: 0,
            props: serde_json::json!({}),
            notify_props: serde_json::json!({}),
            last_password_update: 0,
            last_picture_update: 0,
            failed_attempts: 0,
            mfa_active: false,
        };
        result
            .entry(encode_mm_id(channel_id))
            .or_default()
            .push(mm_user);
    }

    Ok(Json(result))
}

pub async fn set_user_typing(
    auth: MmAuthUser,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let _ = super::user_sidebar_categories::resolve_user_id(&user_id, &auth)?;
    Ok(Json(serde_json::json!({"status": "OK"})))
}

pub async fn get_user_uploads(
    auth: MmAuthUser,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let _ = super::user_sidebar_categories::resolve_user_id(&user_id, &auth)?;
    Ok(Json(vec![]))
}

pub async fn get_user_channel_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> ApiResult<Json<Vec<mm::ChannelMember>>> {
    #[derive(sqlx::FromRow)]
    struct UserChannelMemberCompatRow {
        channel_id: uuid::Uuid,
        user_id: uuid::Uuid,
        role: String,
        notify_props: serde_json::Value,
        last_viewed_at: Option<chrono::DateTime<chrono::Utc>>,
        last_update_at: chrono::DateTime<chrono::Utc>,
        msg_count: i64,
        mention_count: i64,
        mention_count_root: i64,
        urgent_mention_count: i64,
        msg_count_root: i64,
    }

    fn map_user_channel_member_row(
        row: UserChannelMemberCompatRow,
        post_priority_enabled: bool,
    ) -> mm::ChannelMember {
        let scheme_admin =
            row.role == "admin" || row.role == "team_admin" || row.role == "channel_admin";

        mm::ChannelMember {
            channel_id: encode_mm_id(row.channel_id),
            user_id: encode_mm_id(row.user_id),
            roles: crate::mattermost_compat::mappers::map_channel_role(&row.role),
            last_viewed_at: row
                .last_viewed_at
                .map(|t| t.timestamp_millis())
                .unwrap_or(0),
            msg_count: row.msg_count,
            mention_count: row.mention_count,
            mention_count_root: row.mention_count_root,
            urgent_mention_count: if post_priority_enabled {
                row.urgent_mention_count
            } else {
                0
            },
            msg_count_root: row.msg_count_root,
            notify_props: super::utils::normalize_notify_props(row.notify_props),
            last_update_at: row.last_update_at.timestamp_millis(),
            scheme_guest: false,
            scheme_user: true,
            scheme_admin,
        }
    }

    let user_id = super::user_sidebar_categories::resolve_user_id(&user_id, &auth)?;
    let rows: Vec<UserChannelMemberCompatRow> = sqlx::query_as(
        r#"
        SELECT
            cm.channel_id,
            cm.user_id,
            cm.role,
            cm.notify_props,
            cm.last_viewed_at,
            COALESCE(cm.last_update_at, cm.created_at) AS last_update_at,
            GREATEST(
                COUNT(*) FILTER (WHERE p.deleted_at IS NULL)
                - COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > COALESCE(cr.last_read_message_id, 0)
                ),
                0
            )::BIGINT AS msg_count,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
                  AND LOWER(p.message) ~ ('(^|[^a-z0-9_.-])@(' || LOWER(u.username) || '|all|channel)([^a-z0-9_.-]|$)')
            )::BIGINT AS mention_count,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
                  AND p.root_post_id IS NULL
                  AND LOWER(p.message) ~ ('(^|[^a-z0-9_.-])@(' || LOWER(u.username) || '|all|channel)([^a-z0-9_.-]|$)')
            )::BIGINT AS mention_count_root,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
                  AND LOWER(p.message) ~ ('(^|[^a-z0-9_.-])@(' || LOWER(u.username) || '|all|channel)([^a-z0-9_.-]|$)')
                  AND LOWER(p.message) ~ '(^|[^a-z0-9_.-])@here([^a-z0-9_.-]|$)'
            )::BIGINT AS urgent_mention_count,
            GREATEST(
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.root_post_id IS NULL
                )
                - COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.root_post_id IS NULL
                      AND p.seq > COALESCE(cr.last_read_message_id, 0)
                ),
                0
            )::BIGINT AS msg_count_root
        FROM channel_members cm
        JOIN users u ON u.id = cm.user_id
        LEFT JOIN channel_reads cr
               ON cr.channel_id = cm.channel_id
              AND cr.user_id = cm.user_id
        LEFT JOIN posts p
               ON p.channel_id = cm.channel_id
        WHERE cm.user_id = $1
        GROUP BY
            cm.channel_id,
            cm.user_id,
            cm.role,
            cm.notify_props,
            cm.last_viewed_at,
            cm.last_update_at,
            cm.created_at,
            u.username,
            cr.last_read_message_id
        ORDER BY cm.channel_id
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let mm_members = rows
        .into_iter()
        .map(|row| map_user_channel_member_row(row, state.config.unread.post_priority_enabled))
        .collect();

    Ok(Json(mm_members))
}
