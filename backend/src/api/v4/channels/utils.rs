use axum::body::Bytes;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use super::MmAuthUser;
use super::{encode_mm_id, mm};
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::repositories::channel_repository::{
    ChannelWithTeamDataResponse, ChannelWithTeamDataRow,
};

pub fn map_channel_with_team_data_row(row: ChannelWithTeamDataRow) -> ChannelWithTeamDataResponse {
    use crate::models::Channel;
    let team_name = row.team_name;
    let team_display_name = row
        .team_display_name
        .clone()
        .unwrap_or_else(|| team_name.clone());

    let channel = Channel {
        id: row.id,
        team_id: row.team_id,
        channel_type: row.channel_type,
        name: row.name,
        display_name: row.display_name,
        purpose: row.purpose,
        header: row.header,
        is_archived: row.is_archived,
        creator_id: row.creator_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
        deleted_at: row.deleted_at,
    };

    ChannelWithTeamDataResponse {
        channel: channel.into(),
        team_display_name,
        team_name,
        team_update_at: row.team_updated_at.timestamp_millis(),
    }
}

/// Verify that the caller can manage the channel.
///
/// Authorizes:
///   * system administrators (via the `SYSTEM_MANAGE` permission), and
///   * channel admins, team admins, or org admins of the channel's organization,
///     determined by querying `channels`, `channel_members`, `team_members`, and `users`.
///
/// The query starts from `channels` and left-joins membership rows so that team and
/// org admins who are not channel members are still recognized.
pub async fn ensure_channel_admin_or_system_manage(
    state: &AppState,
    channel_id: Uuid,
    auth: &MmAuthUser,
) -> ApiResult<()> {
    use crate::auth::policy::permissions::SYSTEM_MANAGE;
    use crate::constants::{ROLE_ADMIN, ROLE_CHANNEL_ADMIN, ROLE_TEAM_ADMIN};

    // System administrators may manage any channel.
    if auth.has_permission(&SYSTEM_MANAGE) {
        return Ok(());
    }

    #[derive(sqlx::FromRow)]
    struct ChannelRoles {
        channel_role: Option<String>,
        team_role: Option<String>,
        is_org_admin: bool,
    }

    let roles: Option<ChannelRoles> = sqlx::query_as(
        r#"
        SELECT cm.role AS channel_role,
               tm.role AS team_role,
               (u.role ~ '(^|[[:space:],])org_admin([[:space:],]|$)' AND u.org_id = t.org_id) AS is_org_admin
        FROM channels c
        JOIN teams t ON t.id = c.team_id
        JOIN users u ON u.id = $2
        LEFT JOIN channel_members cm ON cm.channel_id = c.id AND cm.user_id = u.id
        LEFT JOIN team_members tm ON tm.team_id = c.team_id AND tm.user_id = u.id
        WHERE c.id = $1
        "#,
    )
    .bind(channel_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?;

    match roles {
        Some(r) => {
            let is_channel_admin = r.channel_role.as_deref() == Some(ROLE_ADMIN)
                || r.channel_role.as_deref() == Some(ROLE_CHANNEL_ADMIN)
                || r.channel_role.as_deref() == Some(ROLE_TEAM_ADMIN);
            let is_team_admin = r.team_role.as_deref() == Some(ROLE_ADMIN)
                || r.team_role.as_deref() == Some(ROLE_TEAM_ADMIN);
            let is_org_admin = r.is_org_admin;

            if is_channel_admin || is_team_admin || is_org_admin {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Channel admin privileges required".to_string(),
                ))
            }
        }
        None => Err(AppError::Forbidden(
            "Channel admin privileges required".to_string(),
        )),
    }
}

pub fn parse_body<T: DeserializeOwned>(
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

pub async fn resolve_direct_channel_display_name(
    state: &AppState,
    channel_id: Uuid,
    viewer_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT COALESCE(NULLIF(u.display_name, ''), u.username)
        FROM channel_members cm
        JOIN users u ON u.id = cm.user_id
        WHERE cm.channel_id = $1
          AND cm.user_id <> $2
        ORDER BY u.username ASC
        LIMIT 1
        "#,
    )
    .bind(channel_id)
    .bind(viewer_id)
    .fetch_optional(&state.db)
    .await
}

#[derive(sqlx::FromRow, Clone)]
pub struct ChannelMemberCompatRow {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub notify_props: serde_json::Value,
    pub last_viewed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_update_at: chrono::DateTime<chrono::Utc>,
    pub msg_count: i64,
    pub mention_count: i64,
    pub mention_count_root: i64,
    pub urgent_mention_count: i64,
    pub msg_count_root: i64,
}

pub fn row_to_mm_channel_member(
    row: ChannelMemberCompatRow,
    post_priority_enabled: bool,
) -> mm::ChannelMember {
    use crate::mattermost_compat::mappers;
    let scheme_admin =
        row.role == "admin" || row.role == "team_admin" || row.role == "channel_admin";
    mm::ChannelMember {
        channel_id: encode_mm_id(row.channel_id),
        user_id: encode_mm_id(row.user_id),
        roles: mappers::map_channel_role(&row.role),
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
        notify_props: super::helpers::normalize_notify_props(row.notify_props),
        last_update_at: row.last_update_at.timestamp_millis(),
        scheme_guest: false,
        scheme_user: true,
        scheme_admin,
    }
}

pub async fn fetch_channel_member_compat_rows(
    state: &AppState,
    channel_id: Uuid,
    user_ids: Option<&[Uuid]>,
) -> ApiResult<Vec<ChannelMemberCompatRow>> {
    if let Some(ids) = user_ids {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let rows = sqlx::query_as(
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
                      AND LOWER(p.message) ~ ('(^|[^a-z0-9_.-])@(' || REGEXP_REPLACE(LOWER(u.username), '([^a-zA-Z0-9_])', '\\\\\\1', 'g') || '|all|channel)([^a-z0-9_.-]|$)')
                )::BIGINT AS mention_count,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > COALESCE(cr.last_read_message_id, 0)
                      AND p.root_post_id IS NULL
                      AND LOWER(p.message) ~ ('(^|[^a-z0-9_.-])@(' || REGEXP_REPLACE(LOWER(u.username), '([^a-zA-Z0-9_])', '\\\\\\1', 'g') || '|all|channel)([^a-z0-9_.-]|$)')
                )::BIGINT AS mention_count_root,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > COALESCE(cr.last_read_message_id, 0)
                      AND LOWER(p.message) ~ ('(^|[^a-z0-9_.-])@(' || REGEXP_REPLACE(LOWER(u.username), '([^a-zA-Z0-9_])', '\\\\\\1', 'g') || '|all|channel)([^a-z0-9_.-]|$)')
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
            WHERE cm.channel_id = $1
              AND cm.user_id = ANY($2)
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
            ORDER BY cm.user_id
            "#,
        )
        .bind(channel_id)
        .bind(ids)
        .fetch_all(&state.db)
        .await?;

        return Ok(rows);
    }

    let rows = sqlx::query_as(
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
                  AND LOWER(p.message) ~ ('(^|[^a-z0-9_.-])@(' || REGEXP_REPLACE(LOWER(u.username), '([^a-zA-Z0-9_])', '\\\\\\1', 'g') || '|all|channel)([^a-z0-9_.-]|$)')
            )::BIGINT AS mention_count,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
                  AND p.root_post_id IS NULL
                  AND LOWER(p.message) ~ ('(^|[^a-z0-9_.-])@(' || REGEXP_REPLACE(LOWER(u.username), '([^a-zA-Z0-9_])', '\\\\\\1', 'g') || '|all|channel)([^a-z0-9_.-]|$)')
            )::BIGINT AS mention_count_root,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
                  AND LOWER(p.message) ~ ('(^|[^a-z0-9_.-])@(' || REGEXP_REPLACE(LOWER(u.username), '([^a-zA-Z0-9_])', '\\\\\\1', 'g') || '|all|channel)([^a-z0-9_.-]|$)')
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
        WHERE cm.channel_id = $1
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
        ORDER BY cm.user_id
        "#,
    )
    .bind(channel_id)
    .fetch_all(&state.db)
    .await?;

    Ok(rows)
}
