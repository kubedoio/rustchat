use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;

use crate::api::v4::users::hydrate_dm_display_names_batch;
use crate::api::AppState;
use crate::mattermost_compat::{id::encode_mm_id, mappers::map_channel_role, models as mm};
use crate::models::channel::Channel;
use crate::realtime::websocket_actor::WebSocketActor;
use crate::repositories::ChannelRepository;

#[derive(serde::Serialize)]
pub(crate) struct ChannelUnreadSnapshot {
    pub(crate) channel_id: String,
    pub(crate) msg_count: i64,
    pub(crate) msg_count_root: i64,
    pub(crate) mention_count: i64,
    pub(crate) mention_count_root: i64,
    pub(crate) urgent_mention_count: i64,
    pub(crate) last_viewed_at: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct ReconnectStatusRow {
    pub(crate) id: Uuid,
    pub(crate) presence: String,
    pub(crate) presence_manual: bool,
    pub(crate) last_login_at: Option<DateTime<Utc>>,
    pub(crate) status_text: Option<String>,
    pub(crate) status_emoji: Option<String>,
    pub(crate) status_expires_at: Option<DateTime<Utc>>,
}

pub(crate) fn should_send_reconnect_snapshot(
    requested_connection_id: Option<&str>,
    sequence_number: Option<i64>,
) -> bool {
    requested_connection_id
        .map(|id| !id.trim().is_empty())
        .unwrap_or(false)
        || sequence_number.unwrap_or_default() > 0
}

pub(crate) async fn send_reconnect_snapshot_if_needed(
    state: &AppState,
    actor: &Arc<WebSocketActor>,
    user_id: Uuid,
    connection_id: &str,
    should_send: bool,
) {
    if !should_send {
        return;
    }

    match build_reconnect_snapshot(state, user_id).await {
        Ok(snapshot) => {
            let mut message = mm::WebSocketMessage {
                seq: None,
                event: "initial_load".to_string(),
                data: snapshot,
                broadcast: mm::Broadcast {
                    omit_users: None,
                    user_id: encode_mm_id(user_id),
                    channel_id: String::new(),
                    team_id: String::new(),
                },
            };

            let replay_payload = json!({
                "event": message.event.clone(),
                "data": message.data.clone(),
                "broadcast": message.broadcast.clone(),
            });
            if let Some(seq) = state
                .connection_store
                .queue_message(connection_id, replay_payload)
            {
                message.seq = Some(seq);
            }

            if let Err(err) = actor.send(message) {
                warn!(
                    user_id = %user_id,
                    connection_id = connection_id,
                    error = %err,
                    "Failed to send reconnect snapshot"
                );
            } else {
                info!(
                    user_id = %user_id,
                    connection_id = connection_id,
                    "Sent reconnect initial_load snapshot"
                );
            }
        }
        Err(err) => {
            warn!(
                user_id = %user_id,
                connection_id = connection_id,
                error = %err,
                "Failed to build reconnect snapshot"
            );
        }
    }
}

pub(crate) async fn build_reconnect_snapshot(
    state: &AppState,
    user_id: Uuid,
) -> Result<serde_json::Value, sqlx::Error> {
    let mut channels: Vec<Channel> = sqlx::query_as(
        r#"
        SELECT c.*
        FROM channels c
        JOIN channel_members cm ON cm.channel_id = c.id
        WHERE cm.user_id = $1
        ORDER BY c.updated_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let repo = ChannelRepository::new(&state.db);
    hydrate_dm_display_names_batch(&repo, &mut channels, user_id).await;

    let mm_channels: Vec<mm::Channel> = channels.iter().cloned().map(Into::into).collect();
    let channel_ids: Vec<Uuid> = channels.iter().map(|c| c.id).collect();

    let username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;

    #[allow(clippy::type_complexity)]
    let membership_rows: Vec<(
        Uuid,
        String,
        serde_json::Value,
        Option<DateTime<Utc>>,
        i64,
        i64,
        i64,
        i64,
        i64,
    )> =
        sqlx::query_as(
            r#"
            SELECT
                cm.channel_id,
                cm.role,
                cm.notify_props,
                cm.last_viewed_at,
                COUNT(*) FILTER (WHERE p.deleted_at IS NULL AND p.seq > COALESCE(cr.last_read_message_id, 0))::BIGINT AS msg_count,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > COALESCE(cr.last_read_message_id, 0)
                      AND (p.message LIKE '%@' || $2 || '%' OR p.message LIKE '%@all%' OR p.message LIKE '%@channel%')
                )::BIGINT AS mention_count,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > COALESCE(cr.last_read_message_id, 0)
                      AND p.root_post_id IS NULL
                )::BIGINT AS msg_count_root,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > COALESCE(cr.last_read_message_id, 0)
                      AND p.root_post_id IS NULL
                      AND (p.message LIKE '%@' || $2 || '%' OR p.message LIKE '%@all%' OR p.message LIKE '%@channel%')
                )::BIGINT AS mention_count_root,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > COALESCE(cr.last_read_message_id, 0)
                      AND (p.message LIKE '%@' || $2 || '%' OR p.message LIKE '%@all%' OR p.message LIKE '%@channel%')
                      AND p.message LIKE '%@here%'
                )::BIGINT AS urgent_mention_count
            FROM channel_members cm
            LEFT JOIN channel_reads cr
                ON cr.channel_id = cm.channel_id
               AND cr.user_id = cm.user_id
            LEFT JOIN posts p
                ON p.channel_id = cm.channel_id
            WHERE cm.user_id = $1
            GROUP BY cm.channel_id, cm.role, cm.notify_props, cm.last_viewed_at
            ORDER BY cm.channel_id
            "#,
        )
        .bind(user_id)
        .bind(&username)
        .fetch_all(&state.db)
        .await?;

    let channel_members: Vec<mm::ChannelMember> = membership_rows
        .iter()
        .map(
            |(
                channel_id,
                role,
                notify_props,
                last_viewed_at,
                msg_count,
                mention_count,
                msg_count_root,
                mention_count_root,
                urgent_mention_count,
            )| mm::ChannelMember {
                channel_id: encode_mm_id(*channel_id),
                user_id: encode_mm_id(user_id),
                roles: map_channel_role(role),
                last_viewed_at: last_viewed_at.map(|t| t.timestamp_millis()).unwrap_or(0),
                msg_count: *msg_count,
                mention_count: *mention_count,
                mention_count_root: *mention_count_root,
                urgent_mention_count: if state.config.unread.post_priority_enabled {
                    *urgent_mention_count
                } else {
                    0
                },
                msg_count_root: *msg_count_root,
                notify_props: normalize_notify_props_for_snapshot(notify_props.clone()),
                last_update_at: 0,
                scheme_guest: false,
                scheme_user: true,
                scheme_admin: role == "admin" || role == "team_admin" || role == "channel_admin",
            },
        )
        .collect();

    let channel_unreads: Vec<ChannelUnreadSnapshot> = membership_rows
        .iter()
        .map(
            |(
                channel_id,
                _role,
                _notify_props,
                last_viewed_at,
                msg_count,
                mention_count,
                msg_count_root,
                mention_count_root,
                urgent_mention_count,
            )| ChannelUnreadSnapshot {
                channel_id: encode_mm_id(*channel_id),
                msg_count: *msg_count,
                msg_count_root: *msg_count_root,
                mention_count: *mention_count,
                mention_count_root: *mention_count_root,
                urgent_mention_count: if state.config.unread.post_priority_enabled {
                    *urgent_mention_count
                } else {
                    0
                },
                last_viewed_at: last_viewed_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            },
        )
        .collect();

    let statuses: Vec<serde_json::Value> = if channel_ids.is_empty() {
        Vec::new()
    } else {
        let rows: Vec<ReconnectStatusRow> = sqlx::query_as(
            r#"
            SELECT DISTINCT
                u.id,
                u.presence,
                COALESCE(u.presence_manual, false) AS presence_manual,
                u.last_login_at,
                CASE WHEN u.status_expires_at IS NOT NULL AND u.status_expires_at < NOW() THEN NULL ELSE u.status_text END AS status_text,
                CASE WHEN u.status_expires_at IS NOT NULL AND u.status_expires_at < NOW() THEN NULL ELSE u.status_emoji END AS status_emoji,
                CASE WHEN u.status_expires_at IS NOT NULL AND u.status_expires_at < NOW() THEN NULL ELSE u.status_expires_at END AS status_expires_at
            FROM users u
            JOIN channel_members cm ON cm.user_id = u.id
            WHERE cm.channel_id = ANY($1)
            "#,
        )
        .bind(&channel_ids)
        .fetch_all(&state.db)
        .await?;

        rows.into_iter()
            .map(|row| {
                json!({
                    "user_id": encode_mm_id(row.id),
                    "status": if row.presence.is_empty() { "offline".to_string() } else { row.presence },
                    "manual": row.presence_manual,
                    "last_activity_at": row.last_login_at.map(|t| t.timestamp_millis()).unwrap_or(0),
                    "text": row.status_text,
                    "emoji": row.status_emoji,
                    "expires_at": row.status_expires_at.map(|t| t.timestamp_millis()),
                })
            })
            .collect()
    };

    Ok(json!({
        "channels": mm_channels,
        "channel_members": channel_members,
        "channel_unreads": channel_unreads,
        "statuses": statuses,
        "server_time": Utc::now().timestamp_millis(),
    }))
}

pub(crate) fn normalize_notify_props_for_snapshot(value: serde_json::Value) -> serde_json::Value {
    if value.is_null() {
        return json!({"desktop": "default", "mark_unread": "all"});
    }

    if let Some(obj) = value.as_object() {
        if obj.is_empty() {
            return json!({"desktop": "default", "mark_unread": "all"});
        }
    }

    value
}

#[cfg(test)]
mod tests {
    use super::should_send_reconnect_snapshot;

    #[test]
    fn reconnect_snapshot_trigger_matches_resume_signals() {
        assert!(!should_send_reconnect_snapshot(None, None));
        assert!(!should_send_reconnect_snapshot(None, Some(0)));
        assert!(!should_send_reconnect_snapshot(Some(""), Some(0)));

        assert!(should_send_reconnect_snapshot(Some("conn-1"), None));
        assert!(should_send_reconnect_snapshot(None, Some(1)));
        assert!(should_send_reconnect_snapshot(Some("conn-2"), Some(0)));
    }
}
