//! Mattermost-compatible threads API endpoints
//! 
//! Implements:
//! - GET /users/{id}/teams/{teamId}/threads - Thread list
//! - GET /users/{id}/teams/{teamId}/threads/{threadId} - Thread detail
//! - PUT /users/{id}/teams/{teamId}/threads/{threadId}/read/{timestamp} - Mark thread read
//! - PUT /users/{id}/teams/{teamId}/threads/read - Mark all threads read
//! - GET /users/{id}/teams/{teamId}/threads/mention_counts - Thread mention counts by channel
//! - POST /users/{id}/teams/{teamId}/threads/{threadId}/set_unread/{postId} - Mark thread unread
//! - PUT/DELETE /users/{id}/teams/{teamId}/threads/{threadId}/following - Follow/unfollow

use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize};
use uuid::Uuid;

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{
    id::{encode_mm_id, parse_mm_or_uuid},
    models as mm,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users/{user_id}/threads", get(get_all_threads_internal))
        .route(
            "/users/{user_id}/teams/{team_id}/threads",
            get(get_threads_internal).put(mark_all_read_internal),
        )
        .route(
            "/users/{user_id}/teams/{team_id}/threads/read",
            put(mark_all_read_explicit),
        )
        .route(
            "/users/{user_id}/teams/{team_id}/threads/mention_counts",
            get(get_thread_mention_counts),
        )
        .route(
            "/users/{user_id}/teams/{team_id}/threads/{thread_id}",
            get(get_thread_internal),
        )
        .route(
            "/users/{user_id}/teams/{team_id}/threads/{thread_id}/read/{timestamp}",
            put(mark_thread_read_internal),
        )
        .route(
            "/users/{user_id}/teams/{team_id}/threads/{thread_id}/set_unread/{post_id}",
            post(set_thread_unread),
        )
        .route(
            "/users/{user_id}/teams/{team_id}/threads/{thread_id}/following",
            put(follow_thread_internal).delete(unfollow_thread_internal),
        )
}

// Path parameters for threads endpoints
#[derive(Deserialize)]
pub struct ThreadsPath {
    pub user_id: String,
    pub team_id: String,
}

#[derive(Deserialize)]
pub struct ThreadsAllPath {
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct ThreadPath {
    pub user_id: String,
    pub team_id: String,
    pub thread_id: String,
}

#[derive(Deserialize)]
pub struct ThreadReadPath {
    pub user_id: String,
    pub team_id: String,
    pub thread_id: String,
    pub timestamp: i64,
}

#[derive(Deserialize)]
pub struct ThreadSetUnreadPath {
    pub user_id: String,
    pub team_id: String,
    pub thread_id: String,
    pub post_id: String,
}

// Query parameters for thread list
#[derive(Deserialize)]
pub struct ThreadsQuery {
    #[serde(default)]
    pub deleted: bool,
    #[serde(default)]
    pub extended: bool,
    #[serde(default)]
    pub since: Option<i64>,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    #[serde(default)]
    pub page: i64,
    #[serde(default)]
    pub totals_only: bool,
    #[serde(default)]
    pub threads_only: bool,
    #[serde(default)]
    pub unread: bool,
}

fn default_per_page() -> i64 {
    25
}

// Thread membership for DB queries
#[derive(sqlx::FromRow, Debug)]
struct ThreadMembership {
    user_id: Uuid,
    post_id: Uuid,
    following: bool,
    last_read_at: Option<DateTime<Utc>>,
    mention_count: i32,
    unread_replies_count: i32,
}

// Thread row with post info
#[derive(sqlx::FromRow, Debug)]
struct ThreadRow {
    // Post fields
    id: Uuid,
    channel_id: Uuid,
    user_id: Uuid,
    message: String,
    created_at: DateTime<Utc>,
    reply_count: i64,
    last_reply_at: Option<DateTime<Utc>>,
    // Membership fields
    following: bool,
    last_read_at: Option<DateTime<Utc>>,
    mention_count: i32,
    unread_replies_count: i32,
}

/// GET /users/{user_id}/teams/{team_id}/threads
pub async fn get_threads_internal(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ThreadsPath>,
    Query(query): Query<ThreadsQuery>,
) -> ApiResult<Json<mm::ThreadResponse>> {
    let user_id = super::users::resolve_user_id(&path.user_id, &auth)?;

    let team_id = parse_mm_or_uuid(&path.team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;

    // Build query for threads the user is following
    let per_page = query.per_page.min(100);
    let offset = query.page * per_page;

    let threads: Vec<ThreadRow> = if query.unread {
        // Fetch only unread threads
        sqlx::query_as(r#"
            SELECT p.id, p.channel_id, p.user_id, p.message, p.created_at,
                   p.reply_count::int8 as reply_count, p.last_reply_at,
                   tm.following, tm.last_read_at, tm.mention_count, tm.unread_replies_count
            FROM posts p
            JOIN thread_memberships tm ON tm.post_id = p.id
            JOIN channels c ON p.channel_id = c.id
            WHERE tm.user_id = $1
              AND tm.following = true
              AND c.team_id = $2
              AND p.root_post_id IS NULL
              AND p.deleted_at IS NULL
              AND (tm.unread_replies_count > 0 OR tm.mention_count > 0)
            ORDER BY COALESCE(p.last_reply_at, p.created_at) DESC
            LIMIT $3 OFFSET $4
        "#)
        .bind(user_id)
        .bind(team_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    } else {
        // Fetch all followed threads
        sqlx::query_as(r#"
            SELECT p.id, p.channel_id, p.user_id, p.message, p.created_at,
                   p.reply_count::int8 as reply_count, p.last_reply_at,
                   tm.following, tm.last_read_at, tm.mention_count, tm.unread_replies_count
            FROM posts p
            JOIN thread_memberships tm ON tm.post_id = p.id
            JOIN channels c ON p.channel_id = c.id
            WHERE tm.user_id = $1
              AND tm.following = true
              AND c.team_id = $2
              AND p.root_post_id IS NULL
              AND p.deleted_at IS NULL
            ORDER BY COALESCE(p.last_reply_at, p.created_at) DESC
            LIMIT $3 OFFSET $4
        "#)
        .bind(user_id)
        .bind(team_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    };

    // Count totals
    let total: i64 = sqlx::query_scalar(r#"
        SELECT COUNT(*)
        FROM posts p
        JOIN thread_memberships tm ON tm.post_id = p.id
        JOIN channels c ON p.channel_id = c.id
        WHERE tm.user_id = $1
          AND tm.following = true
          AND c.team_id = $2
          AND p.root_post_id IS NULL
          AND p.deleted_at IS NULL
    "#)
    .bind(user_id)
    .bind(team_id)
    .fetch_one(&state.db)
    .await?;

    let total_unread_threads: i64 = sqlx::query_scalar(r#"
        SELECT COUNT(*)
        FROM posts p
        JOIN thread_memberships tm ON tm.post_id = p.id
        JOIN channels c ON p.channel_id = c.id
        WHERE tm.user_id = $1
          AND tm.following = true
          AND c.team_id = $2
          AND p.root_post_id IS NULL
          AND p.deleted_at IS NULL
          AND (tm.unread_replies_count > 0 OR tm.mention_count > 0)
    "#)
    .bind(user_id)
    .bind(team_id)
    .fetch_one(&state.db)
    .await?;

    let total_unread_mentions: i64 = sqlx::query_scalar(r#"
        SELECT COALESCE(SUM(tm.mention_count), 0)
        FROM thread_memberships tm
        JOIN posts p ON tm.post_id = p.id
        JOIN channels c ON p.channel_id = c.id
        WHERE tm.user_id = $1
          AND tm.following = true
          AND c.team_id = $2
          AND p.root_post_id IS NULL
          AND p.deleted_at IS NULL
    "#)
    .bind(user_id)
    .bind(team_id)
    .fetch_one(&state.db)
    .await?;

    // Map to MM format
    let mm_threads: Vec<mm::Thread> = threads
        .into_iter()
        .map(|t| {
            let unread_replies = t.unread_replies_count as i64;
            mm::Thread {
                id: encode_mm_id(t.id),
                reply_count: t.reply_count,
                last_reply_at: t.last_reply_at.map(|dt| dt.timestamp_millis()).unwrap_or(0),
                last_viewed_at: t.last_read_at.map(|dt| dt.timestamp_millis()).unwrap_or(0),
                participants: vec![], // Could be populated with thread participants
                post: mm::PostInThread {
                    id: encode_mm_id(t.id),
                    channel_id: encode_mm_id(t.channel_id),
                    user_id: encode_mm_id(t.user_id),
                    message: t.message,
                    create_at: t.created_at.timestamp_millis(),
                },
                unread_replies,
                unread_mentions: t.mention_count as i64,
                is_following: Some(t.following),
            }
        })
        .collect();

    Ok(Json(mm::ThreadResponse {
        threads: mm_threads,
        total,
        total_unread_threads,
        total_unread_mentions,
    }))
}

pub async fn get_all_threads_internal(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ThreadsAllPath>,
    Query(query): Query<ThreadsQuery>,
) -> ApiResult<Json<mm::ThreadResponse>> {
    let user_id = super::users::resolve_user_id(&path.user_id, &auth)?;

    let per_page = query.per_page.min(100);
    let offset = query.page * per_page;

    let threads: Vec<ThreadRow> = sqlx::query_as(r#"
        SELECT p.id, p.channel_id, p.user_id, p.message, p.created_at,
               p.reply_count::int8 as reply_count, p.last_reply_at,
               tm.following, tm.last_read_at, tm.mention_count, tm.unread_replies_count
        FROM posts p
        JOIN thread_memberships tm ON tm.post_id = p.id
        WHERE tm.user_id = $1
          AND tm.following = true
          AND p.root_post_id IS NULL
          AND p.deleted_at IS NULL
        ORDER BY COALESCE(p.last_reply_at, p.created_at) DESC
        LIMIT $2 OFFSET $3
    "#)
    .bind(user_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: i64 = sqlx::query_scalar(r#"
        SELECT COUNT(*)
        FROM thread_memberships tm
        JOIN posts p ON tm.post_id = p.id
        WHERE tm.user_id = $1 AND tm.following = true AND p.deleted_at IS NULL
    "#)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    let mm_threads: Vec<mm::Thread> = threads
        .into_iter()
        .map(|t| {
            mm::Thread {
                id: encode_mm_id(t.id),
                reply_count: t.reply_count,
                last_reply_at: t.last_reply_at.map(|dt| dt.timestamp_millis()).unwrap_or(0),
                last_viewed_at: t.last_read_at.map(|dt| dt.timestamp_millis()).unwrap_or(0),
                participants: vec![],
                post: mm::PostInThread {
                    id: encode_mm_id(t.id),
                    channel_id: encode_mm_id(t.channel_id),
                    user_id: encode_mm_id(t.user_id),
                    message: t.message,
                    create_at: t.created_at.timestamp_millis(),
                },
                unread_replies: t.unread_replies_count as i64,
                unread_mentions: t.mention_count as i64,
                is_following: Some(t.following),
            }
        })
        .collect();

    Ok(Json(mm::ThreadResponse {
        threads: mm_threads,
        total,
        total_unread_threads: 0,
        total_unread_mentions: 0,
    }))
}

/// GET /users/{user_id}/teams/{team_id}/threads/{thread_id}
pub async fn get_thread_internal(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ThreadPath>,
) -> ApiResult<Json<mm::Thread>> {
    let user_id = super::users::resolve_user_id(&path.user_id, &auth)?;
    let team_id = parse_mm_or_uuid(&path.team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    let thread_id = parse_mm_or_uuid(&path.thread_id)
        .ok_or_else(|| AppError::BadRequest("Invalid thread_id".to_string()))?;

    // Fetch thread info
    let thread: Option<ThreadRow> = sqlx::query_as(r#"
        SELECT p.id, p.channel_id, p.user_id, p.message, p.created_at,
               p.reply_count::int8 as reply_count, p.last_reply_at,
               COALESCE(tm.following, false) as following,
               tm.last_read_at,
               COALESCE(tm.mention_count, 0) as mention_count,
               COALESCE(tm.unread_replies_count, 0) as unread_replies_count
        FROM posts p
        JOIN channels c ON p.channel_id = c.id
        LEFT JOIN thread_memberships tm ON tm.post_id = p.id AND tm.user_id = $2
        WHERE p.id = $1
          AND c.team_id = $3
          AND p.root_post_id IS NULL
          AND p.deleted_at IS NULL
    "#)
    .bind(thread_id)
    .bind(user_id)
    .bind(team_id)
    .fetch_optional(&state.db)
    .await?;

    let t = thread.ok_or_else(|| AppError::NotFound("Thread not found".to_string()))?;

    Ok(Json(mm::Thread {
        id: encode_mm_id(t.id),
        reply_count: t.reply_count,
        last_reply_at: t.last_reply_at.map(|dt| dt.timestamp_millis()).unwrap_or(0),
        last_viewed_at: t.last_read_at.map(|dt| dt.timestamp_millis()).unwrap_or(0),
        participants: vec![],
        post: mm::PostInThread {
            id: encode_mm_id(t.id),
            channel_id: encode_mm_id(t.channel_id),
            user_id: encode_mm_id(t.user_id),
            message: t.message,
            create_at: t.created_at.timestamp_millis(),
        },
        unread_replies: t.unread_replies_count as i64,
        unread_mentions: t.mention_count as i64,
        is_following: Some(t.following),
    }))
}

/// PUT /users/{user_id}/teams/{team_id}/threads/{thread_id}/read/{timestamp}
pub async fn mark_thread_read_internal(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ThreadReadPath>,
) -> ApiResult<Json<mm::Thread>> {
    let user_id = super::users::resolve_user_id(&path.user_id, &auth)?;
    let _team_id = parse_mm_or_uuid(&path.team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    let thread_id = parse_mm_or_uuid(&path.thread_id)
        .ok_or_else(|| AppError::BadRequest("Invalid thread_id".to_string()))?;

    let read_at = DateTime::from_timestamp_millis(path.timestamp)
        .unwrap_or_else(|| Utc::now());

    // Upsert thread membership with read time
    sqlx::query(r#"
        INSERT INTO thread_memberships (user_id, post_id, last_read_at, unread_replies_count, mention_count)
        VALUES ($1, $2, $3, 0, 0)
        ON CONFLICT (user_id, post_id) DO UPDATE SET
            last_read_at = $3,
            unread_replies_count = 0,
            mention_count = 0,
            updated_at = NOW()
    "#)
    .bind(user_id)
    .bind(thread_id)
    .bind(read_at)
    .execute(&state.db)
    .await?;

    // Return updated thread
    get_thread_internal(
        State(state),
        auth,
        Path(ThreadPath {
            user_id: path.user_id,
            team_id: path.team_id,
            thread_id: path.thread_id,
        }),
    )
    .await
}

/// PUT /users/{user_id}/teams/{team_id}/threads/read
pub async fn mark_all_read_internal(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ThreadsPath>,
) -> ApiResult<Json<serde_json::Value>> {
    let user_id = super::users::resolve_user_id(&path.user_id, &auth)?;
    let team_id = parse_mm_or_uuid(&path.team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;

    // Update all thread memberships for this user/team
    sqlx::query(r#"
        UPDATE thread_memberships tm SET
            last_read_at = NOW(),
            unread_replies_count = 0,
            mention_count = 0,
            updated_at = NOW()
        FROM posts p
        JOIN channels c ON p.channel_id = c.id
        WHERE tm.post_id = p.id
          AND tm.user_id = $1
          AND c.team_id = $2
    "#)
    .bind(user_id)
    .bind(team_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

pub async fn mark_all_read_explicit(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ThreadsPath>,
) -> ApiResult<Json<serde_json::Value>> {
    mark_all_read_internal(State(state), auth, Path(path)).await
}

pub async fn get_thread_mention_counts(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ThreadsPath>,
) -> ApiResult<Json<std::collections::HashMap<String, i64>>> {
    let user_id = super::users::resolve_user_id(&path.user_id, &auth)?;
    let team_id = parse_mm_or_uuid(&path.team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;

    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        r#"
        SELECT c.id, COALESCE(SUM(tm.mention_count), 0)
        FROM thread_memberships tm
        JOIN posts p ON tm.post_id = p.id
        JOIN channels c ON p.channel_id = c.id
        WHERE tm.user_id = $1
          AND tm.following = true
          AND c.team_id = $2
          AND p.root_post_id IS NULL
          AND p.deleted_at IS NULL
        GROUP BY c.id
        "#,
    )
    .bind(user_id)
    .bind(team_id)
    .fetch_all(&state.db)
    .await?;

    let mut counts = std::collections::HashMap::new();
    for (channel_id, count) in rows {
        counts.insert(encode_mm_id(channel_id), count);
    }

    Ok(Json(counts))
}

/// PUT /users/{user_id}/teams/{team_id}/threads/{thread_id}/following
pub async fn follow_thread_internal(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ThreadPath>,
) -> ApiResult<Json<mm::Thread>> {
    let user_id = super::users::resolve_user_id(&path.user_id, &auth)?;
    let thread_id = parse_mm_or_uuid(&path.thread_id)
        .ok_or_else(|| AppError::BadRequest("Invalid thread_id".to_string()))?;

    // Upsert with following = true
    sqlx::query(r#"
        INSERT INTO thread_memberships (user_id, post_id, following)
        VALUES ($1, $2, true)
        ON CONFLICT (user_id, post_id) DO UPDATE SET
            following = true,
            updated_at = NOW()
    "#)
    .bind(user_id)
    .bind(thread_id)
    .execute(&state.db)
    .await?;

    // Return updated thread
    get_thread_internal(State(state), auth, Path(path)).await
}

/// DELETE /users/{user_id}/teams/{team_id}/threads/{thread_id}/following
pub async fn unfollow_thread_internal(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ThreadPath>,
) -> ApiResult<Json<mm::Thread>> {
    let user_id = super::users::resolve_user_id(&path.user_id, &auth)?;
    let thread_id = parse_mm_or_uuid(&path.thread_id)
        .ok_or_else(|| AppError::BadRequest("Invalid thread_id".to_string()))?;

    // Update following to false
    sqlx::query(r#"
        UPDATE thread_memberships SET
            following = false,
            updated_at = NOW()
        WHERE user_id = $1 AND post_id = $2
    "#)
    .bind(user_id)
    .bind(thread_id)
    .execute(&state.db)
    .await?;

    // Return updated thread
    get_thread_internal(State(state), auth, Path(path)).await
}

pub async fn set_thread_unread(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ThreadSetUnreadPath>,
) -> ApiResult<Json<mm::Thread>> {
    let user_id = super::users::resolve_user_id(&path.user_id, &auth)?;
    let _team_id = parse_mm_or_uuid(&path.team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;
    let thread_id = parse_mm_or_uuid(&path.thread_id)
        .ok_or_else(|| AppError::BadRequest("Invalid thread_id".to_string()))?;
    let post_id = parse_mm_or_uuid(&path.post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;

    let post_created_at: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT created_at FROM posts WHERE id = $1 AND root_post_id = $2",
    )
    .bind(post_id)
    .bind(thread_id)
    .fetch_optional(&state.db)
    .await?;

    let last_read_at = post_created_at.map(|dt| dt - Duration::milliseconds(1));

    sqlx::query(
        r#"
        INSERT INTO thread_memberships (user_id, post_id, last_read_at, unread_replies_count, mention_count)
        VALUES ($1, $2, $3, 1, 0)
        ON CONFLICT (user_id, post_id) DO UPDATE SET
            last_read_at = $3,
            unread_replies_count = GREATEST(thread_memberships.unread_replies_count, 1),
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(thread_id)
    .bind(last_read_at)
    .execute(&state.db)
    .await?;

    get_thread_internal(
        State(state),
        auth,
        Path(ThreadPath {
            user_id: path.user_id,
            team_id: path.team_id,
            thread_id: path.thread_id,
        }),
    )
    .await
}
