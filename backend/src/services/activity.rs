//! Activity feed service

use crate::api::AppState;
use crate::error::ApiResult;
use crate::models::{
    normalize_avatar_url, Activity, ActivityFeedResponse, ActivityQuery, ActivityResponse,
    ActivityType,
};
use crate::realtime::{EventType, WsBroadcast, WsEnvelope};
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

/// Helper to truncate message text
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Internal row type for mapping joined query results
struct ActivityRow {
    id: Uuid,
    activity_type: ActivityType,
    actor_id: Uuid,
    actor_username: String,
    actor_avatar_url: Option<String>,
    channel_id: Uuid,
    channel_name: String,
    team_id: Uuid,
    team_name: String,
    post_id: Uuid,
    root_id: Option<Uuid>,
    message_text: Option<String>,
    reaction: Option<String>,
    read: bool,
    created_at: DateTime<Utc>,
}

/// Create a new activity entry
#[allow(clippy::too_many_arguments)]
pub async fn create_activity(
    state: &AppState,
    user_id: Uuid,
    activity_type: ActivityType,
    actor_id: Uuid,
    channel_id: Uuid,
    team_id: Uuid,
    post_id: Uuid,
    root_id: Option<Uuid>,
    message_text: Option<String>,
    reaction: Option<String>,
) -> ApiResult<Activity> {
    let row = sqlx::query(
        r#"
        INSERT INTO activities
            (user_id, type, actor_id, channel_id, team_id, post_id, root_id, message_text, reaction)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, user_id, type as activity_type, actor_id, channel_id, team_id,
                  post_id, root_id, message_text, reaction, read, created_at
        "#,
    )
    .bind(user_id)
    .bind(activity_type.as_str())
    .bind(actor_id)
    .bind(channel_id)
    .bind(team_id)
    .bind(post_id)
    .bind(root_id)
    .bind(message_text.map(|m| truncate_text(&m, 200)))
    .bind(reaction)
    .fetch_one(&state.db)
    .await?;
    let activity = Activity {
        id: row.get("id"),
        user_id: row.get("user_id"),
        r#type: ActivityType::parse(&row.get::<String, _>("activity_type"))
            .expect("unknown activity type"),
        actor_id: row.get("actor_id"),
        channel_id: row.get("channel_id"),
        team_id: row.get("team_id"),
        post_id: row.get("post_id"),
        root_id: row.get("root_id"),
        message_text: row.get("message_text"),
        reaction: row.get("reaction"),
        read: row.get("read"),
        created_at: row.get("created_at"),
    };

    // Fetch full ActivityResponse for broadcast (includes joined actor/channel/team data)
    let activity_response: Option<ActivityResponse> = sqlx::query(
        r#"
        SELECT
            a.id,
            a.type as activity_type,
            a.actor_id,
            u.username as actor_username,
            u.avatar_url as actor_avatar_url,
            a.channel_id,
            c.name as channel_name,
            a.team_id,
            t.name as team_name,
            a.post_id,
            a.root_id,
            a.message_text,
            a.reaction,
            a.read,
            a.created_at
        FROM activities a
        JOIN users u ON a.actor_id = u.id
        JOIN channels c ON a.channel_id = c.id
        JOIN teams t ON a.team_id = t.id
        WHERE a.id = $1
        "#,
    )
    .bind(activity.id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .map(|row: sqlx::postgres::PgRow| {
        let actor_id: Uuid = row.get("actor_id");
        ActivityResponse {
            id: row.get("id"),
            r#type: ActivityType::parse(&row.get::<String, _>("activity_type"))
                .expect("unknown activity type"),
            actor_id,
            actor_username: row.get("actor_username"),
            actor_avatar_url: normalize_avatar_url(
                actor_id,
                row.get::<Option<String>, _>("actor_avatar_url").as_deref(),
            ),
            channel_id: row.get("channel_id"),
            channel_name: row.get("channel_name"),
            team_id: row.get("team_id"),
            team_name: row.get("team_name"),
            post_id: row.get("post_id"),
            root_id: row.get("root_id"),
            message_text: row.get("message_text"),
            reaction: row.get("reaction"),
            read: row.get("read"),
            created_at: row.get("created_at"),
        }
    });

    let broadcast_data = activity_response
        .as_ref()
        .map(|r| serde_json::to_value(r).unwrap_or_default())
        .unwrap_or_else(|| {
            serde_json::json!({
                "activity_id": activity.id,
                "user_id": activity.user_id,
                "type": activity.r#type
            })
        });

    let broadcast = WsEnvelope::event(EventType::ActivityCreated, broadcast_data, None)
        .with_broadcast(WsBroadcast {
            user_id: Some(user_id),
            channel_id: None,
            team_id: None,
            exclude_user_id: None,
        });

    state.ws_hub.broadcast(broadcast).await;

    Ok(activity)
}

/// Get activity feed for a user
pub async fn get_activities(
    state: &AppState,
    user_id: Uuid,
    query: ActivityQuery,
) -> ApiResult<ActivityFeedResponse> {
    let limit = query.limit.clamp(1, 100);

    // Parse type filter into a Vec<String> for SQL ANY($n) binding
    let type_filters: Option<Vec<String>> = query
        .activity_type
        .as_ref()
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect());

    let sql = r#"
        SELECT
            a.id,
            a.type as activity_type,
            a.actor_id,
            u.username as actor_username,
            u.avatar_url as actor_avatar_url,
            a.channel_id,
            c.name as channel_name,
            a.team_id,
            t.name as team_name,
            a.post_id,
            a.root_id,
            a.message_text,
            a.reaction,
            a.read,
            a.created_at
        FROM activities a
        JOIN users u ON a.actor_id = u.id
        JOIN channels c ON a.channel_id = c.id
        JOIN teams t ON a.team_id = t.id
        WHERE a.user_id = $1
          AND ($2::uuid IS NULL OR (a.created_at, a.id) < (SELECT created_at, id FROM activities WHERE id = $2))
          AND ($3::text[] IS NULL OR a.type::text = ANY($3))
          AND (NOT $4 OR a.read = FALSE)
        ORDER BY a.created_at DESC
        LIMIT $5
    "#;

    let rows_raw = sqlx::query(sql)
        .bind(user_id)
        .bind(query.cursor)
        .bind(type_filters.as_deref())
        .bind(query.unread_only)
        .bind(limit + 1)
        .fetch_all(&state.db)
        .await?;

    let mut rows: Vec<ActivityRow> = rows_raw
        .into_iter()
        .map(|row| ActivityRow {
            id: row.get("id"),
            activity_type: ActivityType::parse(&row.get::<String, _>("activity_type"))
                .expect("unknown activity type"),
            actor_id: row.get("actor_id"),
            actor_username: row.get("actor_username"),
            actor_avatar_url: row.get("actor_avatar_url"),
            channel_id: row.get("channel_id"),
            channel_name: row.get("channel_name"),
            team_id: row.get("team_id"),
            team_name: row.get("team_name"),
            post_id: row.get("post_id"),
            root_id: row.get("root_id"),
            message_text: row.get("message_text"),
            reaction: row.get("reaction"),
            read: row.get("read"),
            created_at: row.get("created_at"),
        })
        .collect();

    for row in &mut rows {
        row.actor_avatar_url = normalize_avatar_url(row.actor_id, row.actor_avatar_url.as_deref());
    }

    // Pagination: we fetched limit+1 rows to determine if there's a next page
    let has_more = rows.len() > limit as usize;
    rows.truncate(limit as usize);

    let next_cursor = if has_more {
        rows.last().map(|r| r.id.to_string())
    } else {
        None
    };

    // Get unread count (separate query, independent of pagination, scoped to type filter)
    let unread_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activities WHERE user_id = $1 AND read = FALSE AND ($2::text[] IS NULL OR type::text = ANY($2))"
    )
    .bind(user_id)
    .bind(type_filters.as_deref())
    .fetch_one(&state.db)
    .await?;

    // Build normalised response
    let order: Vec<String> = rows.iter().map(|r| r.id.to_string()).collect();
    let activities: std::collections::HashMap<String, ActivityResponse> = rows
        .into_iter()
        .map(|r| {
            let resp = ActivityResponse {
                id: r.id,
                r#type: r.activity_type,
                actor_id: r.actor_id,
                actor_username: r.actor_username,
                actor_avatar_url: r.actor_avatar_url,
                channel_id: r.channel_id,
                channel_name: r.channel_name,
                team_id: r.team_id,
                team_name: r.team_name,
                post_id: r.post_id,
                root_id: r.root_id,
                message_text: r.message_text,
                reaction: r.reaction,
                read: r.read,
                created_at: r.created_at,
            };
            (resp.id.to_string(), resp)
        })
        .collect();

    Ok(ActivityFeedResponse {
        order,
        activities,
        unread_count,
        next_cursor,
    })
}

/// Mark specific activities as read
pub async fn mark_activities_read(
    state: &AppState,
    user_id: Uuid,
    activity_ids: Vec<Uuid>,
) -> ApiResult<usize> {
    if activity_ids.is_empty() {
        return Ok(0);
    }
    let result = sqlx::query(
        "UPDATE activities SET read = TRUE WHERE user_id = $1 AND id = ANY($2) AND read = FALSE",
    )
    .bind(user_id)
    .bind(&activity_ids)
    .execute(&state.db)
    .await?;

    if result.rows_affected() > 0 {
        let broadcast = WsEnvelope::event(
            EventType::ActivityRead,
            serde_json::json!({ "activity_ids": activity_ids }),
            None,
        )
        .with_broadcast(WsBroadcast {
            user_id: Some(user_id),
            channel_id: None,
            team_id: None,
            exclude_user_id: None,
        });
        state.ws_hub.broadcast(broadcast).await;
    }

    Ok(result.rows_affected() as usize)
}

/// Mark all activities as read for a user
pub async fn mark_all_read(state: &AppState, user_id: Uuid) -> ApiResult<usize> {
    let result =
        sqlx::query("UPDATE activities SET read = TRUE WHERE user_id = $1 AND read = FALSE")
            .bind(user_id)
            .execute(&state.db)
            .await?;

    if result.rows_affected() > 0 {
        let broadcast = WsEnvelope::event(
            EventType::ActivityRead,
            serde_json::json!({ "all_read": true }),
            None,
        )
        .with_broadcast(WsBroadcast {
            user_id: Some(user_id),
            channel_id: None,
            team_id: None,
            exclude_user_id: None,
        });
        state.ws_hub.broadcast(broadcast).await;
    }

    Ok(result.rows_affected() as usize)
}

/// Create mention activity (convenience wrapper)
pub async fn create_mention_activity(
    state: &AppState,
    mentioned_user_id: Uuid,
    actor_id: Uuid,
    channel_id: Uuid,
    team_id: Uuid,
    post_id: Uuid,
    message: &str,
) -> ApiResult<()> {
    if mentioned_user_id == actor_id {
        return Ok(());
    }
    create_activity(
        state,
        mentioned_user_id,
        ActivityType::Mention,
        actor_id,
        channel_id,
        team_id,
        post_id,
        None,
        Some(message.to_string()),
        None,
    )
    .await?;
    Ok(())
}

/// Create reply activity (convenience wrapper)
pub async fn create_reply_activity(
    state: &AppState,
    parent_user_id: Uuid,
    actor_id: Uuid,
    channel_id: Uuid,
    team_id: Uuid,
    post_id: Uuid,
    reply_message: &str,
) -> ApiResult<()> {
    if parent_user_id == actor_id {
        return Ok(());
    }
    create_activity(
        state,
        parent_user_id,
        ActivityType::Reply,
        actor_id,
        channel_id,
        team_id,
        post_id,
        None,
        Some(reply_message.to_string()),
        None,
    )
    .await?;
    Ok(())
}

/// Create reaction activity (convenience wrapper)
pub async fn create_reaction_activity(
    state: &AppState,
    post_user_id: Uuid,
    actor_id: Uuid,
    channel_id: Uuid,
    team_id: Uuid,
    post_id: Uuid,
    emoji: &str,
) -> ApiResult<()> {
    if post_user_id == actor_id {
        return Ok(());
    }
    create_activity(
        state,
        post_user_id,
        ActivityType::Reaction,
        actor_id,
        channel_id,
        team_id,
        post_id,
        None,
        None,
        Some(emoji.to_string()),
    )
    .await?;
    Ok(())
}

/// Create thread reply activity (convenience wrapper)
#[allow(clippy::too_many_arguments)]
pub async fn create_thread_reply_activity(
    state: &AppState,
    parent_user_id: Uuid,
    actor_id: Uuid,
    channel_id: Uuid,
    team_id: Uuid,
    post_id: Uuid,
    root_id: Uuid,
    reply_message: &str,
) -> ApiResult<()> {
    if parent_user_id == actor_id {
        return Ok(());
    }
    create_activity(
        state,
        parent_user_id,
        ActivityType::ThreadReply,
        actor_id,
        channel_id,
        team_id,
        post_id,
        Some(root_id),
        Some(reply_message.to_string()),
        None,
    )
    .await?;
    Ok(())
}

/// Create DM activity (convenience wrapper)
pub async fn create_dm_activity(
    state: &AppState,
    recipient_id: Uuid,
    actor_id: Uuid,
    channel_id: Uuid,
    team_id: Uuid,
    post_id: Uuid,
    message: &str,
) -> ApiResult<()> {
    if recipient_id == actor_id {
        return Ok(());
    }
    create_activity(
        state,
        recipient_id,
        ActivityType::Dm,
        actor_id,
        channel_id,
        team_id,
        post_id,
        None,
        Some(message.to_string()),
        None,
    )
    .await?;
    Ok(())
}
