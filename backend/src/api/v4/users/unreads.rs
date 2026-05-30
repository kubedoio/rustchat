use axum::extract::Query;
use axum::{extract::State, Json};
use regex::escape as escape_regex;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

use super::user_channels::resolve_team_id;
use super::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{
    id::{encode_mm_id, parse_mm_or_uuid},
    models as mm,
};

#[derive(Deserialize, Default)]
pub struct TeamsUnreadQuery {
    #[serde(default)]
    pub exclude_team: Option<String>,
    #[serde(default)]
    pub include_collapsed_threads: Option<bool>,
}

#[derive(Deserialize, Default)]
pub struct TeamUnreadQuery {
    #[serde(default)]
    pub include_collapsed_threads: Option<bool>,
}

#[derive(sqlx::FromRow)]
struct TeamUnreadChannelRow {
    team_id: Uuid,
    notify_props: serde_json::Value,
    msg_count: i64,
    mention_count: i64,
    msg_count_root: i64,
    mention_count_root: i64,
    urgent_mention_count: i64,
}

#[derive(sqlx::FromRow)]
struct TeamUnreadThreadRow {
    team_id: Uuid,
    thread_count: i64,
    thread_mention_count: i64,
    thread_urgent_mention_count: i64,
}

fn mention_pattern(username: &str) -> String {
    format!(
        r"(^|[^a-z0-9_.-])@({}|all|channel)([^a-z0-9_.-]|$)",
        escape_regex(&username.to_ascii_lowercase())
    )
}

fn here_pattern() -> &'static str {
    r"(^|[^a-z0-9_.-])@here([^a-z0-9_.-]|$)"
}

pub fn parse_optional_team_query(exclude_team: &Option<String>) -> ApiResult<Option<Uuid>> {
    let Some(raw_team) = exclude_team else {
        return Ok(None);
    };

    if raw_team.is_empty() {
        return Ok(None);
    }

    parse_mm_or_uuid(raw_team)
        .ok_or_else(|| AppError::BadRequest("Invalid exclude_team".to_string()))
        .map(Some)
}

pub async fn compute_team_unreads(
    state: &AppState,
    user_id: Uuid,
    exclude_team: Option<Uuid>,
    only_team: Option<Uuid>,
    include_collapsed_threads: bool,
) -> ApiResult<Vec<mm::TeamUnread>> {
    let username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;
    let mention_pattern = mention_pattern(&username);

    let mut rows: Vec<TeamUnreadChannelRow> = sqlx::query_as(
        r#"
        SELECT
            c.team_id,
            cm.notify_props,
            COALESCE(un.msg_count, 0) AS msg_count,
            COALESCE(un.mention_count, 0) AS mention_count,
            COALESCE(un.msg_count_root, 0) AS msg_count_root,
            COALESCE(un.mention_count_root, 0) AS mention_count_root,
            COALESCE(un.urgent_mention_count, 0) AS urgent_mention_count
        FROM channel_members cm
        JOIN channels c ON c.id = cm.channel_id
        LEFT JOIN channel_reads cr
            ON cr.channel_id = cm.channel_id
           AND cr.user_id = cm.user_id
        LEFT JOIN LATERAL (
            SELECT
                COUNT(*) FILTER (WHERE p.deleted_at IS NULL AND p.seq > COALESCE(cr.last_read_message_id, 0)) AS msg_count,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > COALESCE(cr.last_read_message_id, 0)
                      AND LOWER(p.message) ~ $2
                ) AS mention_count,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > COALESCE(cr.last_read_message_id, 0)
                      AND p.root_post_id IS NULL
                ) AS msg_count_root,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > COALESCE(cr.last_read_message_id, 0)
                      AND p.root_post_id IS NULL
                      AND LOWER(p.message) ~ $2
                ) AS mention_count_root,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > COALESCE(cr.last_read_message_id, 0)
                      AND LOWER(p.message) ~ $2
                      AND LOWER(p.message) ~ $5
                ) AS urgent_mention_count
            FROM posts p
            WHERE p.channel_id = cm.channel_id
        ) un ON TRUE
        WHERE cm.user_id = $1
          AND ($3::UUID IS NULL OR c.team_id <> $3)
          AND ($4::UUID IS NULL OR c.team_id = $4)
        "#,
    )
    .bind(user_id)
    .bind(&mention_pattern)
    .bind(exclude_team)
    .bind(only_team)
    .bind(here_pattern())
    .fetch_all(&state.db)
    .await?;

    if !state.config.unread.post_priority_enabled {
        for row in &mut rows {
            row.urgent_mention_count = 0;
        }
    }

    let mut aggregated: HashMap<Uuid, mm::TeamUnread> = HashMap::new();
    for row in rows {
        let team_entry = aggregated
            .entry(row.team_id)
            .or_insert_with(|| mm::TeamUnread {
                team_id: encode_mm_id(row.team_id),
                msg_count: 0,
                mention_count: 0,
                mention_count_root: 0,
                msg_count_root: 0,
                thread_count: 0,
                thread_mention_count: 0,
                thread_urgent_mention_count: 0,
            });

        let mark_unread = row
            .notify_props
            .get("mark_unread")
            .and_then(|value| value.as_str())
            .unwrap_or("all");
        if mark_unread != "mention" {
            team_entry.msg_count += row.msg_count;
            team_entry.msg_count_root += row.msg_count_root;
        }

        team_entry.mention_count += row.mention_count;
        team_entry.mention_count_root += row.mention_count_root;
    }

    if include_collapsed_threads && state.config.unread.collapsed_threads_enabled {
        let thread_rows: Vec<TeamUnreadThreadRow> = sqlx::query_as(
            r#"
            SELECT
                c.team_id,
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
                      AND LOWER(rp.message) ~ $4
                      AND LOWER(rp.message) ~ $5
                )), 0)::BIGINT AS thread_urgent_mention_count
            FROM thread_memberships tm
            JOIN posts p ON p.id = tm.post_id
            JOIN channels c ON c.id = p.channel_id
            WHERE tm.user_id = $1
              AND tm.following = true
              AND p.root_post_id IS NULL
              AND p.deleted_at IS NULL
              AND ($2::UUID IS NULL OR c.team_id <> $2)
              AND ($3::UUID IS NULL OR c.team_id = $3)
            GROUP BY c.team_id
            "#,
        )
        .bind(user_id)
        .bind(exclude_team)
        .bind(only_team)
        .bind(&mention_pattern)
        .bind(here_pattern())
        .fetch_all(&state.db)
        .await?;

        for thread_row in thread_rows {
            let entry = aggregated
                .entry(thread_row.team_id)
                .or_insert_with(|| mm::TeamUnread {
                    team_id: encode_mm_id(thread_row.team_id),
                    msg_count: 0,
                    mention_count: 0,
                    mention_count_root: 0,
                    msg_count_root: 0,
                    thread_count: 0,
                    thread_mention_count: 0,
                    thread_urgent_mention_count: 0,
                });
            entry.thread_count = thread_row.thread_count;
            entry.thread_mention_count = thread_row.thread_mention_count;
            entry.thread_urgent_mention_count = if state.config.unread.post_priority_enabled {
                thread_row.thread_urgent_mention_count
            } else {
                0
            };
        }
    }

    let mut values: Vec<mm::TeamUnread> = aggregated.into_values().collect();
    values.sort_by(|a, b| a.team_id.cmp(&b.team_id));
    Ok(values)
}

pub async fn my_teams_unread(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Query(query): Query<TeamsUnreadQuery>,
) -> ApiResult<Json<Vec<mm::TeamUnread>>> {
    let exclude_team = parse_optional_team_query(&query.exclude_team)?;
    let include_collapsed_threads = query.include_collapsed_threads.unwrap_or(false);
    let unread = compute_team_unreads(
        &state,
        auth.user_id,
        exclude_team,
        None,
        include_collapsed_threads,
    )
    .await?;
    Ok(Json(unread))
}

pub async fn get_user_teams_unread(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    Query(query): Query<TeamsUnreadQuery>,
) -> ApiResult<Json<Vec<mm::TeamUnread>>> {
    let user_id = super::user_sidebar_categories::resolve_user_id(&user_id, &auth)?;
    let exclude_team = parse_optional_team_query(&query.exclude_team)?;
    let include_collapsed_threads = query.include_collapsed_threads.unwrap_or(false);
    let unread = compute_team_unreads(
        &state,
        user_id,
        exclude_team,
        None,
        include_collapsed_threads,
    )
    .await?;
    Ok(Json(unread))
}

pub async fn get_user_team_unread(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Path((user_id, team_id)): axum::extract::Path<(String, String)>,
    Query(query): Query<TeamUnreadQuery>,
) -> ApiResult<Json<mm::TeamUnread>> {
    let user_id = super::user_sidebar_categories::resolve_user_id(&user_id, &auth)?;
    let team_id = resolve_team_id(&state, &team_id).await?;
    let include_collapsed_threads = query.include_collapsed_threads.unwrap_or(false);

    let mut unread = compute_team_unreads(
        &state,
        user_id,
        None,
        Some(team_id),
        include_collapsed_threads,
    )
    .await?;

    let fallback = mm::TeamUnread {
        team_id: encode_mm_id(team_id),
        msg_count: 0,
        mention_count: 0,
        mention_count_root: 0,
        msg_count_root: 0,
        thread_count: 0,
        thread_mention_count: 0,
        thread_urgent_mention_count: 0,
    };

    Ok(Json(unread.pop().unwrap_or(fallback)))
}
