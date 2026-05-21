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
use serde::Deserialize;

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{
    id::{encode_mm_id, parse_mm_or_uuid},
    models as mm,
};
use crate::repositories::PostRepository;

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

/// GET /users/{user_id}/teams/{team_id}/threads
pub async fn get_threads_internal(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ThreadsPath>,
    Query(query): Query<ThreadsQuery>,
) -> ApiResult<Json<mm::ThreadResponse>> {
    let user_id = super::users::resolve_user_id(&path.user_id, &auth)?;

    let team_id = parse_mm_or_uuid(&path.team_id)
        .ok_or_else(|| AppError::InvalidTeamId)?;

    let per_page = query.per_page.min(100);
    let offset = query.page * per_page;

    let repo = PostRepository::new(state.db.clone());

    let threads = repo
        .list_threads_for_user_in_team(user_id, team_id, query.unread, per_page, offset)
        .await?;

    let total = repo
        .count_threads_for_user_in_team(user_id, team_id)
        .await?;
    let total_unread_threads = repo
        .count_unread_threads_for_user_in_team(user_id, team_id)
        .await?;
    let total_unread_mentions = repo
        .sum_unread_mentions_for_user_in_team(user_id, team_id)
        .await?;

    let mm_threads: Vec<mm::Thread> = threads
        .into_iter()
        .map(|t| {
            let unread_replies = t.unread_replies_count as i64;
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

    let repo = PostRepository::new(state.db.clone());

    let threads = repo
        .list_all_threads_for_user(user_id, per_page, offset)
        .await?;

    let total = repo.count_all_threads_for_user(user_id).await?;

    let mm_threads: Vec<mm::Thread> = threads
        .into_iter()
        .map(|t| mm::Thread {
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
        .ok_or_else(|| AppError::InvalidTeamId)?;
    let thread_id = parse_mm_or_uuid(&path.thread_id)
        .ok_or_else(|| AppError::InvalidThreadId)?;

    let repo = PostRepository::new(state.db.clone());
    let thread = repo
        .get_thread_for_user_in_team(thread_id, user_id, team_id)
        .await?
        .ok_or_else(|| AppError::ThreadNotFound)?;

    Ok(Json(mm::Thread {
        id: encode_mm_id(thread.id),
        reply_count: thread.reply_count,
        last_reply_at: thread
            .last_reply_at
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(0),
        last_viewed_at: thread
            .last_read_at
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(0),
        participants: vec![],
        post: mm::PostInThread {
            id: encode_mm_id(thread.id),
            channel_id: encode_mm_id(thread.channel_id),
            user_id: encode_mm_id(thread.user_id),
            message: thread.message,
            create_at: thread.created_at.timestamp_millis(),
        },
        unread_replies: thread.unread_replies_count as i64,
        unread_mentions: thread.mention_count as i64,
        is_following: Some(thread.following),
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
        .ok_or_else(|| AppError::InvalidTeamId)?;
    let thread_id = parse_mm_or_uuid(&path.thread_id)
        .ok_or_else(|| AppError::InvalidThreadId)?;

    let read_at = DateTime::from_timestamp_millis(path.timestamp).unwrap_or_else(Utc::now);

    let repo = PostRepository::new(state.db.clone());
    repo.mark_thread_read(user_id, thread_id, read_at).await?;

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
        .ok_or_else(|| AppError::InvalidTeamId)?;

    let repo = PostRepository::new(state.db.clone());
    repo.mark_all_threads_read(user_id, team_id).await?;

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
        .ok_or_else(|| AppError::InvalidTeamId)?;

    let repo = PostRepository::new(state.db.clone());
    let rows = repo
        .get_thread_mention_counts_by_channel(user_id, team_id)
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
        .ok_or_else(|| AppError::InvalidThreadId)?;

    let repo = PostRepository::new(state.db.clone());
    repo.follow_thread(user_id, thread_id).await?;

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
        .ok_or_else(|| AppError::InvalidThreadId)?;

    let repo = PostRepository::new(state.db.clone());
    repo.unfollow_thread(user_id, thread_id).await?;

    get_thread_internal(State(state), auth, Path(path)).await
}

pub async fn set_thread_unread(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<ThreadSetUnreadPath>,
) -> ApiResult<Json<mm::Thread>> {
    let user_id = super::users::resolve_user_id(&path.user_id, &auth)?;
    let _team_id = parse_mm_or_uuid(&path.team_id)
        .ok_or_else(|| AppError::InvalidTeamId)?;
    let thread_id = parse_mm_or_uuid(&path.thread_id)
        .ok_or_else(|| AppError::InvalidThreadId)?;
    let post_id = parse_mm_or_uuid(&path.post_id)
        .ok_or_else(|| AppError::InvalidPostId)?;

    let repo = PostRepository::new(state.db.clone());
    let post_created_at = repo
        .get_post_created_at_in_thread(post_id, thread_id)
        .await?;

    let last_read_at = post_created_at.map(|dt| dt - Duration::milliseconds(1));

    repo.set_thread_unread(user_id, thread_id, last_read_at)
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
