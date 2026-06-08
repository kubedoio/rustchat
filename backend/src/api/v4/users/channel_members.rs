use axum::{extract::State, Json};
use uuid::Uuid;

use super::user_channels::resolve_team_id;
use super::utils::normalize_notify_props;
use super::MmAuthUser;
use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::{id::encode_mm_id, models as mm};

#[derive(sqlx::FromRow)]
struct UserChannelMemberCompatRow {
    channel_id: Uuid,
    user_id: Uuid,
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
        notify_props: normalize_notify_props(row.notify_props),
        last_update_at: row.last_update_at.timestamp_millis(),
        scheme_guest: false,
        scheme_user: true,
        scheme_admin,
    }
}

pub async fn my_team_channel_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Path(team_id): axum::extract::Path<String>,
) -> ApiResult<Json<Vec<mm::ChannelMember>>> {
    let team_id = resolve_team_id(&state, &team_id).await?;
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
        JOIN channels c ON cm.channel_id = c.id
        JOIN users u ON u.id = cm.user_id
        LEFT JOIN channel_reads cr
               ON cr.channel_id = cm.channel_id
              AND cr.user_id = cm.user_id
        LEFT JOIN posts p
               ON p.channel_id = cm.channel_id
        WHERE c.team_id = $1 AND cm.user_id = $2
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
    .bind(team_id)
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mm_members = rows
        .into_iter()
        .map(|row| map_user_channel_member_row(row, state.config.unread.post_priority_enabled))
        .collect();

    Ok(Json(mm_members))
}
