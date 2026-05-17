use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use deadpool_redis::redis::AsyncCommands;
use serde::Serialize;
use sqlx::FromRow;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tracing::{debug, warn};
use uuid::Uuid;

const V2_SCHEMA_VERSION: i64 = 2;

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

#[derive(Debug, Clone)]
struct ChannelUnreadV2 {
    channel_id: Uuid,
    msg_count: i64,
    msg_count_root: i64,
    mention_count: i64,
    mention_count_root: i64,
    urgent_mention_count: i64,
    last_viewed_at: i64,
    manually_unread: bool,
    version: i64,
}

#[derive(Debug, Clone)]
struct TeamUnreadV2 {
    team_id: Uuid,
    msg_count: i64,
    mention_count: i64,
    mention_count_root: i64,
    msg_count_root: i64,
    thread_count: i64,
    thread_mention_count: i64,
    thread_urgent_mention_count: i64,
    version: i64,
}

fn legacy_unread_key(user_id: Uuid, channel_id: Uuid) -> String {
    format!("rc:unread:{}:{}", user_id, channel_id)
}

fn legacy_team_unread_key(user_id: Uuid, team_id: Uuid) -> String {
    format!("rc:unread_team:{}:{}", user_id, team_id)
}

fn v2_channel_unread_key(user_id: Uuid, channel_id: Uuid) -> String {
    format!("rc:unread:v2:uc:{}:{}", user_id, channel_id)
}

fn v2_team_unread_key(user_id: Uuid, team_id: Uuid) -> String {
    format!("rc:unread:v2:ut:{}:{}", user_id, team_id)
}

fn v2_dirty_key(user_id: Uuid) -> String {
    format!("rc:unread:v2:dirty:{}", user_id)
}

fn parse_i64_opt(raw: Option<String>) -> Option<i64> {
    raw.and_then(|v| v.parse::<i64>().ok())
}

fn parse_bool_opt(raw: Option<String>) -> Option<bool> {
    raw.and_then(|v| match v.as_str() {
        "1" | "true" | "TRUE" | "True" => Some(true),
        "0" | "false" | "FALSE" | "False" => Some(false),
        _ => None,
    })
}

async fn fetch_username(state: &AppState, user_id: Uuid) -> ApiResult<String> {
    let username: Option<String> = sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?;

    Ok(username.unwrap_or_default())
}

async fn fetch_user_channels(state: &AppState, user_id: Uuid) -> ApiResult<Vec<(Uuid, Uuid)>> {
    let channels: Vec<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT cm.channel_id, c.team_id
        FROM channel_members cm
        JOIN channels c ON c.id = cm.channel_id
        WHERE cm.user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(channels)
}

async fn compute_channel_unread_from_db(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
    username: &str,
) -> ApiResult<ChannelUnreadV2> {
    #[allow(clippy::type_complexity)]
    let row: Option<(
        Option<chrono::DateTime<chrono::Utc>>,
        bool,
        i64,
        i64,
        i64,
        i64,
        i64,
    )> = sqlx::query_as(
        r#"
        SELECT
            cm.last_viewed_at,
            COALESCE(cm.manually_unread, false) AS manually_unread,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
            )::BIGINT AS msg_count,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
                  AND p.root_post_id IS NULL
            )::BIGINT AS msg_count_root,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
                  AND (p.message LIKE '%@' || $3 || '%' OR p.message LIKE '%@all%' OR p.message LIKE '%@channel%')
            )::BIGINT AS mention_count,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
                  AND p.root_post_id IS NULL
                  AND (p.message LIKE '%@' || $3 || '%' OR p.message LIKE '%@all%' OR p.message LIKE '%@channel%')
            )::BIGINT AS mention_count_root,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
                  AND (p.message LIKE '%@' || $3 || '%' OR p.message LIKE '%@all%' OR p.message LIKE '%@channel%')
                  AND p.message LIKE '%@here%'
            )::BIGINT AS urgent_mention_count
        FROM channel_members cm
        LEFT JOIN channel_reads cr
               ON cr.channel_id = cm.channel_id
              AND cr.user_id = cm.user_id
        LEFT JOIN posts p
               ON p.channel_id = cm.channel_id
        WHERE cm.user_id = $1
          AND cm.channel_id = $2
        GROUP BY cm.last_viewed_at, cm.manually_unread
        "#,
    )
    .bind(user_id)
    .bind(channel_id)
    .bind(username)
    .fetch_optional(&state.db)
    .await?;

    let Some((
        last_viewed_at,
        manually_unread,
        msg_count,
        msg_count_root,
        mention_count,
        mention_count_root,
        mut urgent_mention_count,
    )) = row
    else {
        return Ok(ChannelUnreadV2 {
            channel_id,
            msg_count: 0,
            msg_count_root: 0,
            mention_count: 0,
            mention_count_root: 0,
            urgent_mention_count: 0,
            last_viewed_at: 0,
            manually_unread: false,
            version: V2_SCHEMA_VERSION,
        });
    };

    if !state.config.unread.post_priority_enabled {
        urgent_mention_count = 0;
    }

    Ok(ChannelUnreadV2 {
        channel_id,
        msg_count,
        msg_count_root,
        mention_count,
        mention_count_root,
        urgent_mention_count,
        last_viewed_at: last_viewed_at.map(|t| t.timestamp_millis()).unwrap_or(0),
        manually_unread,
        version: V2_SCHEMA_VERSION,
    })
}

async fn compute_team_unread_from_db(
    state: &AppState,
    user_id: Uuid,
    team_id: Uuid,
    username: &str,
) -> ApiResult<TeamUnreadV2> {
    let channel_rows: Vec<(serde_json::Value, i64, i64, i64, i64, i64)> = sqlx::query_as(
        r#"
        SELECT
            cm.notify_props,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
            )::BIGINT AS msg_count,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
                  AND p.root_post_id IS NULL
            )::BIGINT AS msg_count_root,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
                  AND (p.message LIKE '%@' || $3 || '%' OR p.message LIKE '%@all%' OR p.message LIKE '%@channel%')
            )::BIGINT AS mention_count,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
                  AND p.root_post_id IS NULL
                  AND (p.message LIKE '%@' || $3 || '%' OR p.message LIKE '%@all%' OR p.message LIKE '%@channel%')
            )::BIGINT AS mention_count_root,
            COUNT(*) FILTER (
                WHERE p.deleted_at IS NULL
                  AND p.seq > COALESCE(cr.last_read_message_id, 0)
                  AND (p.message LIKE '%@' || $3 || '%' OR p.message LIKE '%@all%' OR p.message LIKE '%@channel%')
                  AND p.message LIKE '%@here%'
            )::BIGINT AS urgent_mention_count
        FROM channel_members cm
        JOIN channels c ON c.id = cm.channel_id
        LEFT JOIN channel_reads cr
               ON cr.channel_id = cm.channel_id
              AND cr.user_id = cm.user_id
        LEFT JOIN posts p
               ON p.channel_id = cm.channel_id
        WHERE cm.user_id = $1
          AND c.team_id = $2
        GROUP BY cm.channel_id, cm.notify_props
        "#,
    )
    .bind(user_id)
    .bind(team_id)
    .bind(username)
    .fetch_all(&state.db)
    .await?;

    let mut msg_count = 0i64;
    let mut msg_count_root = 0i64;
    let mut mention_count = 0i64;
    let mut mention_count_root = 0i64;
    for (
        notify_props,
        channel_msg_count,
        channel_msg_count_root,
        channel_mention_count,
        channel_mention_count_root,
        channel_urgent_mention_count,
    ) in channel_rows
    {
        let mark_unread = notify_props
            .get("mark_unread")
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        if mark_unread != "mention" {
            msg_count += channel_msg_count;
            msg_count_root += channel_msg_count_root;
        }

        mention_count += channel_mention_count;
        mention_count_root += channel_mention_count_root;
        let _ = channel_urgent_mention_count;
    }

    let (thread_count, thread_mention_count, thread_urgent_mention_count): (i64, i64, i64) =
        sqlx::query_as(
            r#"
        SELECT
            COUNT(*) FILTER (
                WHERE COALESCE(tm.unread_replies_count, 0) > 0
                   OR COALESCE(tm.mention_count, 0) > 0
            )::BIGINT AS thread_count,
            COALESCE(SUM(tm.mention_count), 0)::BIGINT AS thread_mention_count,
            COALESCE(SUM((
                SELECT COUNT(*)::BIGINT
                FROM posts rp
                WHERE rp.root_post_id = tm.post_id
                  AND rp.deleted_at IS NULL
                  AND (tm.last_read_at IS NULL OR rp.created_at > tm.last_read_at)
                  AND (
                      rp.message LIKE '%@' || $3 || '%'
                      OR rp.message LIKE '%@all%'
                      OR rp.message LIKE '%@channel%'
                  )
                  AND rp.message LIKE '%@here%'
            )), 0)::BIGINT AS thread_urgent_mention_count
        FROM thread_memberships tm
        JOIN posts p ON p.id = tm.post_id
        JOIN channels c ON c.id = p.channel_id
        WHERE tm.user_id = $1
          AND tm.following = true
          AND p.root_post_id IS NULL
          AND p.deleted_at IS NULL
          AND c.team_id = $2
        "#,
        )
        .bind(user_id)
        .bind(team_id)
        .bind(username)
        .fetch_one(&state.db)
        .await
        .unwrap_or((0, 0, 0));

    Ok(TeamUnreadV2 {
        team_id,
        msg_count,
        mention_count,
        mention_count_root,
        msg_count_root,
        thread_count,
        thread_mention_count,
        thread_urgent_mention_count: if state.config.unread.post_priority_enabled {
            thread_urgent_mention_count
        } else {
            0
        },
        version: V2_SCHEMA_VERSION,
    })
}

async fn read_v2_channel_hash(
    conn: &mut deadpool_redis::Connection,
    user_id: Uuid,
    channel_id: Uuid,
) -> Result<Option<ChannelUnreadV2>, redis::RedisError> {
    let key = v2_channel_unread_key(user_id, channel_id);
    let values: Vec<Option<String>> = redis::cmd("HMGET")
        .arg(&key)
        .arg(
            &[
                "msg_count",
                "msg_count_root",
                "mention_count",
                "mention_count_root",
                "urgent_mention_count",
                "last_viewed_at",
                "manually_unread",
                "version",
            ][..],
        )
        .query_async(conn)
        .await?;

    if values.iter().all(|v| v.is_none()) {
        return Ok(None);
    }

    let mut iter = values.into_iter();
    let msg_count = parse_i64_opt(iter.next().flatten()).unwrap_or(0);
    let msg_count_root = parse_i64_opt(iter.next().flatten()).unwrap_or(0);
    let mention_count = parse_i64_opt(iter.next().flatten()).unwrap_or(0);
    let mention_count_root = parse_i64_opt(iter.next().flatten()).unwrap_or(0);
    let urgent_mention_count = parse_i64_opt(iter.next().flatten()).unwrap_or(0);
    let last_viewed_at = parse_i64_opt(iter.next().flatten()).unwrap_or(0);
    let manually_unread = parse_bool_opt(iter.next().flatten()).unwrap_or(false);
    let version = parse_i64_opt(iter.next().flatten()).unwrap_or(V2_SCHEMA_VERSION);

    Ok(Some(ChannelUnreadV2 {
        channel_id,
        msg_count,
        msg_count_root,
        mention_count,
        mention_count_root,
        urgent_mention_count,
        last_viewed_at,
        manually_unread,
        version,
    }))
}

async fn write_v2_channel_hash(
    conn: &mut deadpool_redis::Connection,
    user_id: Uuid,
    channel: &ChannelUnreadV2,
) -> Result<(), redis::RedisError> {
    let key = v2_channel_unread_key(user_id, channel.channel_id);
    redis::cmd("HSET")
        .arg(&key)
        .arg("msg_count")
        .arg(channel.msg_count)
        .arg("msg_count_root")
        .arg(channel.msg_count_root)
        .arg("mention_count")
        .arg(channel.mention_count)
        .arg("mention_count_root")
        .arg(channel.mention_count_root)
        .arg("urgent_mention_count")
        .arg(channel.urgent_mention_count)
        .arg("last_viewed_at")
        .arg(channel.last_viewed_at)
        .arg("manually_unread")
        .arg(if channel.manually_unread { 1 } else { 0 })
        .arg("version")
        .arg(channel.version)
        .query_async(conn)
        .await
}

async fn write_v2_team_hash(
    conn: &mut deadpool_redis::Connection,
    user_id: Uuid,
    team: &TeamUnreadV2,
) -> Result<(), redis::RedisError> {
    let key = v2_team_unread_key(user_id, team.team_id);
    redis::cmd("HSET")
        .arg(&key)
        .arg("msg_count")
        .arg(team.msg_count)
        .arg("mention_count")
        .arg(team.mention_count)
        .arg("mention_count_root")
        .arg(team.mention_count_root)
        .arg("msg_count_root")
        .arg(team.msg_count_root)
        .arg("thread_count")
        .arg(team.thread_count)
        .arg("thread_mention_count")
        .arg(team.thread_mention_count)
        .arg("thread_urgent_mention_count")
        .arg(team.thread_urgent_mention_count)
        .arg("version")
        .arg(team.version)
        .query_async(conn)
        .await
}

async fn write_legacy_channel_count(
    conn: &mut deadpool_redis::Connection,
    user_id: Uuid,
    channel_id: Uuid,
    count: i64,
) -> Result<(), redis::RedisError> {
    let key = legacy_unread_key(user_id, channel_id);
    if count <= 0 {
        let _: () = conn.del(key).await?;
    } else {
        let _: () = conn.set(key, count).await?;
    }
    Ok(())
}

async fn write_legacy_team_count(
    conn: &mut deadpool_redis::Connection,
    user_id: Uuid,
    team_id: Uuid,
    count: i64,
) -> Result<(), redis::RedisError> {
    let key = legacy_team_unread_key(user_id, team_id);
    if count <= 0 {
        let _: () = conn.del(key).await?;
    } else {
        let _: () = conn.set(key, count).await?;
    }
    Ok(())
}

async fn mark_dirty_with_conn(
    conn: &mut deadpool_redis::Connection,
    user_id: Uuid,
    marker: &str,
) -> Result<(), redis::RedisError> {
    let key = v2_dirty_key(user_id);
    let _: () = conn.sadd(key, marker).await?;
    Ok(())
}

async fn mark_dirty_best_effort(state: &AppState, user_id: Uuid, marker: &str) {
    if let Ok(mut conn) = state.redis.get().await {
        let _ = mark_dirty_with_conn(&mut conn, user_id, marker).await;
    }
}

async fn cache_channel_unread_best_effort(
    state: &AppState,
    user_id: Uuid,
    channel: &ChannelUnreadV2,
    team: &TeamUnreadV2,
) {
    let redis_result = async {
        let mut conn = state
            .redis
            .get()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        write_legacy_channel_count(&mut conn, user_id, channel.channel_id, channel.msg_count)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        write_legacy_team_count(&mut conn, user_id, team.team_id, team.msg_count)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        write_v2_channel_hash(&mut conn, user_id, channel)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        write_v2_team_hash(&mut conn, user_id, team)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok::<(), AppError>(())
    }
    .await;

    if let Err(err) = redis_result {
        warn!(user_id = %user_id, channel_id = %channel.channel_id, error = %err, "Unread cache write failed; marking dirty");
        mark_dirty_best_effort(state, user_id, &format!("channel:{}", channel.channel_id)).await;
    }
}

async fn sync_user_unread_cache(state: &AppState, user_id: Uuid) -> ApiResult<()> {
    let username = fetch_username(state, user_id).await?;
    let channels = fetch_user_channels(state, user_id).await?;
    let mut teams: HashSet<Uuid> = HashSet::new();

    let mut conn = state
        .redis
        .get()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    for (channel_id, team_id) in &channels {
        teams.insert(*team_id);

        let channel =
            compute_channel_unread_from_db(state, user_id, *channel_id, &username).await?;
        write_legacy_channel_count(&mut conn, user_id, *channel_id, channel.msg_count)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        write_v2_channel_hash(&mut conn, user_id, &channel)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    for team_id in teams {
        let team = compute_team_unread_from_db(state, user_id, team_id, &username).await?;
        write_legacy_team_count(&mut conn, user_id, team_id, team.msg_count)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        write_v2_team_hash(&mut conn, user_id, &team)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let _: () = conn.del(v2_dirty_key(user_id)).await.unwrap_or(());
    Ok(())
}

pub async fn reconcile_dirty_users_once(state: &AppState) -> ApiResult<usize> {
    let mut conn = state
        .redis
        .get()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let mut cursor = 0u64;
    let mut user_ids: HashSet<Uuid> = HashSet::new();

    loop {
        let (next, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg("rc:unread:v2:dirty:*")
            .arg("COUNT")
            .arg(200)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        for key in keys {
            if let Some(user_raw) = key.rsplit(':').next() {
                if let Ok(user_id) = Uuid::parse_str(user_raw) {
                    user_ids.insert(user_id);
                }
            }
        }

        if next == 0 {
            break;
        }
        cursor = next;
    }

    drop(conn);

    let mut reconciled = 0usize;
    for user_id in user_ids {
        match sync_user_unread_cache(state, user_id).await {
            Ok(()) => reconciled += 1,
            Err(err) => {
                warn!(user_id = %user_id, error = %err, "Failed to reconcile unread cache for user")
            }
        }
    }

    Ok(reconciled)
}

pub async fn run_unread_v2_reconciler(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        if !state.config.unread.unread_v2_enabled {
            continue;
        }

        match reconcile_dirty_users_once(&state).await {
            Ok(count) if count > 0 => {
                debug!(reconciled_users = count, "Unread v2 reconciler completed")
            }
            Ok(_) => {}
            Err(err) => warn!(error = %err, "Unread v2 reconciler failed"),
        }
    }
}

/// Reset unread count for a user in a channel
pub async fn mark_channel_as_read(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
    target_seq: Option<i64>,
) -> ApiResult<()> {
    let last_read_id = match target_seq {
        Some(seq) => seq,
        None => {
            let id: Option<i64> = sqlx::query_scalar(
                "SELECT MAX(seq) FROM posts WHERE channel_id = $1 AND deleted_at IS NULL",
            )
            .bind(channel_id)
            .fetch_one(&state.db)
            .await?;
            id.unwrap_or(0)
        }
    };

    // DB is source of truth.
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

    let team_id: Uuid = sqlx::query_scalar("SELECT team_id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_one(&state.db)
        .await?;

    let username = fetch_username(state, user_id).await?;
    let channel = compute_channel_unread_from_db(state, user_id, channel_id, &username).await?;
    let team = compute_team_unread_from_db(state, user_id, team_id, &username).await?;

    cache_channel_unread_best_effort(state, user_id, &channel, &team).await;

    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::UnreadCountsUpdated,
        serde_json::json!({
            "channel_id": channel_id,
            "team_id": team_id,
            "unread_count": channel.msg_count
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

    Ok(())
}

/// Get overview of unread counts for a user
pub async fn get_unread_overview(state: &AppState, user_id: Uuid) -> ApiResult<UnreadOverview> {
    let channels = fetch_user_channels(state, user_id).await?;
    let username = fetch_username(state, user_id).await?;

    let mut channel_overviews = Vec::new();
    let mut team_unread_map: HashMap<Uuid, i64> = HashMap::new();

    let mut conn = state
        .redis
        .get()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    for (channel_id, team_id) in channels {
        let legacy_key = legacy_unread_key(user_id, channel_id);
        let legacy_count: Option<i64> = conn.get(&legacy_key).await.unwrap_or(None);

        let mut tuple_from_v2 = None;
        if state.config.unread.unread_v2_enabled {
            tuple_from_v2 = read_v2_channel_hash(&mut conn, user_id, channel_id)
                .await
                .unwrap_or(None);
        }

        let count = if state.config.unread.unread_v2_enabled {
            if let Some(tuple) = tuple_from_v2 {
                let mut needs_refresh = tuple.version != V2_SCHEMA_VERSION;
                if needs_refresh {
                    warn!(
                        user_id = %user_id,
                        channel_id = %channel_id,
                        seen_version = tuple.version,
                        expected_version = V2_SCHEMA_VERSION,
                        "Unread cache version mismatch detected"
                    );
                }

                if let Some(legacy) = legacy_count {
                    if legacy != tuple.msg_count {
                        needs_refresh = true;
                        warn!(
                            user_id = %user_id,
                            channel_id = %channel_id,
                            legacy = legacy,
                            v2 = tuple.msg_count,
                            "Unread cache mismatch detected (v1 vs v2)"
                        );
                    }
                }

                if needs_refresh {
                    let recomputed =
                        compute_channel_unread_from_db(state, user_id, channel_id, &username)
                            .await?;
                    let team =
                        compute_team_unread_from_db(state, user_id, team_id, &username).await?;

                    let write_result = async {
                        write_legacy_channel_count(
                            &mut conn,
                            user_id,
                            channel_id,
                            recomputed.msg_count,
                        )
                        .await?;
                        write_legacy_team_count(&mut conn, user_id, team_id, team.msg_count)
                            .await?;
                        write_v2_channel_hash(&mut conn, user_id, &recomputed).await?;
                        write_v2_team_hash(&mut conn, user_id, &team).await?;
                        Ok::<(), redis::RedisError>(())
                    }
                    .await;

                    if let Err(err) = write_result {
                        warn!(
                            user_id = %user_id,
                            channel_id = %channel_id,
                            error = %err,
                            "Failed to refresh unread cache from DB"
                        );
                        let _ = mark_dirty_with_conn(
                            &mut conn,
                            user_id,
                            &format!("channel:{}", channel_id),
                        )
                        .await;
                    }

                    recomputed.msg_count
                } else {
                    tuple.msg_count
                }
            } else {
                let tuple =
                    compute_channel_unread_from_db(state, user_id, channel_id, &username).await?;
                let team = compute_team_unread_from_db(state, user_id, team_id, &username).await?;

                let _ = write_legacy_channel_count(&mut conn, user_id, channel_id, tuple.msg_count)
                    .await;
                let _ = write_legacy_team_count(&mut conn, user_id, team_id, team.msg_count).await;
                let _ = write_v2_channel_hash(&mut conn, user_id, &tuple).await;
                let _ = write_v2_team_hash(&mut conn, user_id, &team).await;

                tuple.msg_count
            }
        } else if let Some(legacy) = legacy_count {
            legacy
        } else {
            let tuple =
                compute_channel_unread_from_db(state, user_id, channel_id, &username).await?;
            let team = compute_team_unread_from_db(state, user_id, team_id, &username).await?;

            let _ =
                write_legacy_channel_count(&mut conn, user_id, channel_id, tuple.msg_count).await;
            let _ = write_legacy_team_count(&mut conn, user_id, team_id, team.msg_count).await;
            let _ = write_v2_channel_hash(&mut conn, user_id, &tuple).await;
            let _ = write_v2_team_hash(&mut conn, user_id, &team).await;

            tuple.msg_count
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

    let mut team_overviews: Vec<TeamUnreadOverview> = team_unread_map
        .into_iter()
        .map(|(team_id, unread_count)| TeamUnreadOverview {
            team_id,
            unread_count,
        })
        .collect();
    team_overviews.sort_by_key(|a| a.team_id);

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
    let team_id: Uuid = sqlx::query_scalar("SELECT team_id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_one(&state.db)
        .await?;

    // Update author's read position to self-sent message in DB.
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

    let members: Vec<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM channel_members WHERE channel_id = $1")
            .bind(channel_id)
            .fetch_all(&state.db)
            .await?;

    let mut conn = match state.redis.get().await {
        Ok(conn) => conn,
        Err(err) => {
            warn!(channel_id = %channel_id, error = %err, "Redis unavailable during increment_unreads; marking members dirty");
            for mid in members {
                if mid != author_id {
                    mark_dirty_best_effort(state, mid, &format!("channel:{}", channel_id)).await;
                }
            }
            return Ok(());
        }
    };

    let last_msg_key = format!("rc:channel:{}:last_msg_id", channel_id);
    let _: () = conn.set(last_msg_key, message_seq).await.unwrap_or(());

    let non_author_members: Vec<Uuid> =
        members.into_iter().filter(|m| *m != author_id).collect();

    if non_author_members.is_empty() {
        return Ok(());
    }

    // Batch all per-member Redis writes and the final GET for each
    // unread count into a single pipeline.
    let mut pipe = redis::pipe();
    for mid in &non_author_members {
        let unread_key = legacy_unread_key(*mid, channel_id);
        let team_unread_key = legacy_team_unread_key(*mid, team_id);
        let channel_hash_key = v2_channel_unread_key(*mid, channel_id);
        let team_hash_key = v2_team_unread_key(*mid, team_id);
        let dirty_key = v2_dirty_key(*mid);
        let marker = format!("channel:{}", channel_id);

        pipe.cmd("INCR").arg(&unread_key).arg(1).ignore();
        pipe.cmd("INCR").arg(&team_unread_key).arg(1).ignore();
        pipe.cmd("HINCRBY")
            .arg(&channel_hash_key)
            .arg("msg_count")
            .arg(1)
            .ignore();
        pipe.cmd("HINCRBY")
            .arg(&channel_hash_key)
            .arg("msg_count_root")
            .arg(1)
            .ignore();
        pipe.cmd("HSET")
            .arg(&channel_hash_key)
            .arg("version")
            .arg(V2_SCHEMA_VERSION)
            .ignore();
        pipe.cmd("HINCRBY")
            .arg(&team_hash_key)
            .arg("msg_count")
            .arg(1)
            .ignore();
        pipe.cmd("HINCRBY")
            .arg(&team_hash_key)
            .arg("msg_count_root")
            .arg(1)
            .ignore();
        pipe.cmd("HSET")
            .arg(&team_hash_key)
            .arg("version")
            .arg(V2_SCHEMA_VERSION)
            .ignore();
        pipe.cmd("SADD").arg(&dirty_key).arg(&marker).ignore();
        pipe.cmd("GET").arg(&unread_key);
    }

    let counts: Vec<Option<i64>> =
        pipe.query_async(&mut conn).await.unwrap_or_default();

    for (mid, count) in non_author_members.iter().zip(counts.iter()) {
        let broadcast = crate::realtime::WsEnvelope::event(
            crate::realtime::EventType::UnreadCountsUpdated,
            serde_json::json!({
                "channel_id": channel_id,
                "team_id": team_id,
                "unread_count": count.unwrap_or(0)
            }),
            None,
        )
        .with_broadcast(crate::realtime::WsBroadcast {
            user_id: Some(*mid),
            channel_id: None,
            team_id: None,
            exclude_user_id: None,
        });
        state.ws_hub.broadcast(broadcast).await;
    }

    Ok(())
}

/// Mark all channels as read for a user
pub async fn mark_all_as_read(state: &AppState, user_id: Uuid) -> ApiResult<()> {
    let channel_rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT cm.channel_id, c.team_id
        FROM channel_members cm
        JOIN channels c ON c.id = cm.channel_id
        WHERE cm.user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    for (channel_id, _team_id) in &channel_rows {
        let last_msg_id: Option<i64> =
            sqlx::query_scalar("SELECT MAX(seq) FROM posts WHERE channel_id = $1")
                .bind(channel_id)
                .fetch_one(&state.db)
                .await?;

        let msg_id = last_msg_id.unwrap_or(0);

        sqlx::query(
            "INSERT INTO channel_reads (user_id, channel_id, last_read_message_id, last_read_at) VALUES ($1, $2, $3, NOW()) ON CONFLICT (user_id, channel_id) DO UPDATE SET last_read_message_id = $3, last_read_at = NOW()",
        )
        .bind(user_id)
        .bind(channel_id)
        .bind(msg_id)
        .execute(&state.db)
        .await?;
    }

    let mut conn = match state.redis.get().await {
        Ok(conn) => conn,
        Err(err) => {
            warn!(user_id = %user_id, error = %err, "Redis unavailable during mark_all_as_read");
            return Ok(());
        }
    };

    let mut team_ids: HashSet<Uuid> = HashSet::new();
    for (channel_id, team_id) in &channel_rows {
        team_ids.insert(*team_id);

        let _: () = conn
            .del(legacy_unread_key(user_id, *channel_id))
            .await
            .unwrap_or(());
        let _: () = conn
            .del(v2_channel_unread_key(user_id, *channel_id))
            .await
            .unwrap_or(());
    }

    for team_id in team_ids {
        let _: () = conn
            .del(legacy_team_unread_key(user_id, team_id))
            .await
            .unwrap_or(());
        let _: () = conn
            .del(v2_team_unread_key(user_id, team_id))
            .await
            .unwrap_or(());
    }

    let _: () = conn.del(v2_dirty_key(user_id)).await.unwrap_or(());

    Ok(())
}
