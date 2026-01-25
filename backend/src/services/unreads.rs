use crate::api::AppState;
use crate::error::ApiResult;
use deadpool_redis::redis::AsyncCommands;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow)]
pub struct ChannelUnreadOverview {
    pub channel_id: Uuid,
    pub team_id: Uuid,
    pub unread_count: i64,
}

#[derive(Debug, Serialize)]
pub struct TeamUnreadOverview {
    pub team_id: Uuid,
    pub unread_count: i64,
}

#[derive(Debug, Serialize)]
pub struct UnreadOverview {
    pub channels: Vec<ChannelUnreadOverview>,
    pub teams: Vec<TeamUnreadOverview>,
}

/// Reset unread count for a user in a channel
pub async fn mark_channel_as_read(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
    target_seq: Option<i64>,
) -> ApiResult<()> {
    let mut conn = state
        .redis
        .get()
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    let last_read_id = match target_seq {
        Some(seq) => seq,
        None => {
            // 1. Get latest message sequence from Redis, fallback to DB
            let last_msg_key = format!("rc:channel:{}:last_msg_id", channel_id);
            let last_msg_id: Option<i64> = conn.get(&last_msg_key).await?;

            match last_msg_id {
                Some(id) => id,
                None => {
                    let id: Option<i64> =
                        sqlx::query_scalar("SELECT MAX(seq) FROM posts WHERE channel_id = $1")
                            .bind(channel_id)
                            .fetch_one(&state.db)
                            .await?;
                    let id = id.unwrap_or(0);
                    // Lazily set in Redis
                    let _: () = conn.set(&last_msg_key, id).await?;
                    id
                }
            }
        }
    };

    // 2. Update Postgres channel_reads
    sqlx::query(
        r#"
        INSERT INTO channel_reads (user_id, channel_id, last_read_message_id, last_read_at)
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT (user_id, channel_id)
        DO UPDATE SET
            last_read_message_id = EXCLUDED.last_read_message_id,
            last_read_at = EXCLUDED.last_read_at
        "#,
    )
    .bind(user_id)
    .bind(channel_id)
    .bind(last_read_id)
    .execute(&state.db)
    .await?;

    // 3. Re-calculate Redis unread count for this user/channel
    let unread_key = format!("rc:unread:{}:{}", user_id, channel_id);
    let previous_unread: i64 = conn.get(&unread_key).await.unwrap_or(0);

    let db_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM posts WHERE channel_id = $1 AND seq > $2 AND deleted_at IS NULL",
    )
    .bind(channel_id)
    .bind(last_read_id)
    .fetch_one(&state.db)
    .await?;

    if db_count == 0 {
        let _: () = conn.del(&unread_key).await?;
    } else {
        let _: () = conn.set(&unread_key, db_count).await?;
    }

    // Update team unread count (approximate or precise? let's do delta)
    let team_id: Uuid = sqlx::query_scalar("SELECT team_id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_one(&state.db)
        .await?;

    let team_unread_key = format!("rc:unread_team:{}:{}", user_id, team_id);
    let delta = db_count - previous_unread;
    if delta != 0 {
        let _: () = conn.incr(&team_unread_key, delta).await.unwrap_or(());
    }

    // Broadcast update if count changed
    if delta != 0 || target_seq.is_some() {
        let broadcast = crate::realtime::WsEnvelope::event(
            crate::realtime::EventType::UnreadCountsUpdated,
            serde_json::json!({
                "channel_id": channel_id,
                "team_id": team_id,
                "unread_count": db_count
            }),
            None,
        )
        .with_broadcast(crate::realtime::WsBroadcast {
            user_id: Some(user_id),
            channel_id: None,
            team_id: None,
            exclude_user_id: None,
        });
        state.ws_hub.broadcast(broadcast).await;
    }

    Ok(())
}

/// Get overview of unread counts for a user
pub async fn get_unread_overview(state: &AppState, user_id: Uuid) -> ApiResult<UnreadOverview> {
    let mut conn = state
        .redis
        .get()
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    // Get all channels user is a member of
    let channels: Vec<(Uuid, Uuid)> = sqlx::query_as("SELECT channel_id, team_id FROM channel_members JOIN channels ON channels.id = channel_members.channel_id WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(&state.db)
        .await?;

    let mut channel_overviews = Vec::new();
    let mut team_unread_map: std::collections::HashMap<Uuid, i64> =
        std::collections::HashMap::new();

    for (channel_id, team_id) in channels {
        let unread_key = format!("rc:unread:{}:{}", user_id, channel_id);
        let count: Option<i64> = conn.get(&unread_key).await?;

        let count = match count {
            Some(c) => c,
            None => {
                // Fallback to DB
                let last_read: Option<i64> = sqlx::query_scalar("SELECT last_read_message_id FROM channel_reads WHERE user_id = $1 AND channel_id = $2")
                    .bind(user_id)
                    .bind(channel_id)
                    .fetch_optional(&state.db)
                    .await?;

                let last_read = last_read.unwrap_or(0);
                let db_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE channel_id = $1 AND seq > $2 AND deleted_at IS NULL")
                    .bind(channel_id)
                    .bind(last_read)
                    .fetch_one(&state.db)
                    .await?;

                // Lazily set in Redis
                let _: () = conn.set(&unread_key, db_count).await?;
                db_count
            }
        };

        if count > 0 {
            channel_overviews.push(ChannelUnreadOverview {
                channel_id,
                team_id,
                unread_count: count,
            });
            *team_unread_map.entry(team_id).or_insert(0) += count;
        }
    }

    let team_overviews = team_unread_map
        .into_iter()
        .map(|(team_id, unread_count)| TeamUnreadOverview {
            team_id,
            unread_count,
        })
        .collect();

    Ok(UnreadOverview {
        channels: channel_overviews,
        teams: team_overviews,
    })
}

/// Increment unread counts for a new message
pub async fn increment_unreads(
    state: &AppState,
    channel_id: Uuid,
    author_id: Uuid,
    message_seq: i64,
) -> ApiResult<()> {
    let mut conn = state
        .redis
        .get()
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    // Update channel latest msg id
    let last_msg_key = format!("rc:channel:{}:last_msg_id", channel_id);
    let _: () = conn.set(last_msg_key, message_seq).await?;

    // Get team_id
    let team_id: Uuid = sqlx::query_scalar("SELECT team_id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_one(&state.db)
        .await?;

    // Increment for all members except author
    let members: Vec<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM channel_members WHERE channel_id = $1")
            .bind(channel_id)
            .fetch_all(&state.db)
            .await?;

    // Update author's read position to self-sent message
    sqlx::query(
        r#"
        INSERT INTO channel_reads (user_id, channel_id, last_read_message_id, last_read_at)
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT (user_id, channel_id)
        DO UPDATE SET
            last_read_message_id = EXCLUDED.last_read_message_id,
            last_read_at = EXCLUDED.last_read_at
        "#,
    )
    .bind(author_id)
    .bind(channel_id)
    .bind(message_seq)
    .execute(&state.db)
    .await?;

    for mid in members {
        if mid != author_id {
            let unread_key = format!("rc:unread:{}:{}", mid, channel_id);
            let team_unread_key = format!("rc:unread_team:{}:{}", mid, team_id);

            let _: () = conn.incr(&unread_key, 1).await?;
            let _: () = conn.incr(&team_unread_key, 1).await?;

            // Broadcast unread_counts_updated to the specific user
            let count: i64 = conn.get(&unread_key).await.unwrap_or(0);
            let broadcast = crate::realtime::WsEnvelope::event(
                crate::realtime::EventType::UnreadCountsUpdated,
                serde_json::json!({
                    "channel_id": channel_id,
                    "team_id": team_id,
                    "unread_count": count
                }),
                None, // No specific channel for this user-level event
            )
            .with_broadcast(crate::realtime::WsBroadcast {
                user_id: Some(mid),
                channel_id: None,
                team_id: None,
                exclude_user_id: None,
            });
            state.ws_hub.broadcast(broadcast).await;
        }
    }

    Ok(())
}

/// Mark all channels as read for a user
pub async fn mark_all_as_read(state: &AppState, user_id: Uuid) -> ApiResult<()> {
    let mut conn = state
        .redis
        .get()
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    // 1. Get all channels user is a member of
    let channel_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT channel_id FROM channel_members WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&state.db)
            .await?;

    for cid in channel_ids {
        // Find latest seq
        let last_msg_id: Option<i64> =
            sqlx::query_scalar("SELECT MAX(seq) FROM posts WHERE channel_id = $1")
                .bind(cid)
                .fetch_one(&state.db)
                .await?;

        let msg_id = last_msg_id.unwrap_or(0);

        sqlx::query(
            "INSERT INTO channel_reads (user_id, channel_id, last_read_message_id, last_read_at) VALUES ($1, $2, $3, NOW()) ON CONFLICT (user_id, channel_id) DO UPDATE SET last_read_message_id = $3, last_read_at = NOW()"
        )
        .bind(user_id)
        .bind(cid)
        .bind(msg_id)
        .execute(&state.db)
        .await?;

        // Clear Redis
        let unread_key = format!("rc:unread:{}:{}", user_id, cid);
        let _: () = conn.del(&unread_key).await.unwrap_or(());
    }

    // Clear team unreads too
    let team_keys: Vec<String> = redis::cmd("KEYS")
        .arg(format!("rc:unread_team:{}:*", user_id))
        .query_async(&mut conn)
        .await
        .unwrap_or_default();
    for k in team_keys {
        let _: () = conn.del(k).await.unwrap_or(());
    }

    Ok(())
}
