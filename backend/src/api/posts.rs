//! Posts API endpoints

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

use super::AppState;
use crate::auth::policy::permissions;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::reaction::Reaction;
use crate::models::{
    ChannelMember, CreatePost, CreateReaction, Post, PostResponse, ThreadResponse, UpdatePost,
};

/// Build posts routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/channels/{channel_id}/posts",
            get(list_posts).post(create_post),
        )
        .route(
            "/posts/{id}",
            get(get_post).put(update_post).delete(delete_post),
        )
        .route("/posts/{id}/reactions", post(add_reaction))
        .route("/posts/{id}/reactions/{emoji}", delete(remove_reaction))
        .route("/posts/{id}/thread", get(get_thread))
        .route("/posts/{id}/pin", post(pin_post).delete(unpin_post))
        .route("/posts/{id}/save", post(save_post).delete(unsave_post))
        .route("/active_user/saved_posts", get(get_saved_posts))
}

#[derive(Debug, Deserialize)]
pub struct ListPostsQuery {
    pub before: Option<Uuid>,
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
    pub is_pinned: Option<bool>,
    pub q: Option<String>,
}

/// Query parameters for thread endpoint
#[derive(Debug, Deserialize)]
pub struct ThreadQuery {
    /// Cursor for pagination (post ID to start after)
    pub cursor: Option<String>,
    /// Maximum number of replies to return (1-100, default 60)
    #[serde(default = "default_thread_limit")]
    pub limit: i64,
}

fn default_thread_limit() -> i64 {
    60
}

#[derive(Debug, serde::Serialize)]
pub struct PostListResponse {
    pub messages: Vec<PostResponse>,
    pub read_state: Option<ReadState>,
}

#[derive(Debug, serde::Serialize)]
pub struct ReadState {
    pub last_read_message_id: Option<i64>,
    pub first_unread_message_id: Option<i64>,
}

/// List posts in a channel
async fn list_posts(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<ListPostsQuery>,
) -> ApiResult<Json<PostListResponse>> {
    tracing::info!(
        "list_posts: channel_id={}, user_id={}",
        channel_id,
        auth.user_id
    );
    // Check membership
    let _: ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    // Get read state
    let last_read: Option<i64> = sqlx::query_scalar(
        "SELECT last_read_message_id FROM channel_reads WHERE user_id = $1 AND channel_id = $2",
    )
    .bind(auth.user_id)
    .bind(channel_id)
    .fetch_optional(&state.db)
    .await?;

    let first_unread: Option<i64> = match last_read {
        Some(lr) => sqlx::query_scalar(
            "SELECT MIN(seq) FROM posts WHERE channel_id = $1 AND seq > $2 AND deleted_at IS NULL",
        )
        .bind(channel_id)
        .bind(lr)
        .fetch_one(&state.db)
        .await?,
        None => {
            sqlx::query_scalar(
                "SELECT MIN(seq) FROM posts WHERE channel_id = $1 AND deleted_at IS NULL",
            )
            .bind(channel_id)
            .fetch_one(&state.db)
            .await?
        }
    };

    let limit = query.limit.unwrap_or(50).min(100);

    // Build SQL query safely without format!() for dynamic parts
    // Use separate condition arrays that we track in parallel with bound values
    let mut conditions: Vec<&'static str> = Vec::new();
    let mut arg_index: usize = 2;

    if query.is_pinned.is_some() {
        conditions.push(" AND p.is_pinned = $2");
        arg_index = 3;
    }

    if query.q.is_some() {
        conditions.push(" AND p.message ILIKE $3");
        arg_index = 4;
    }

    if query.before.is_some() {
        conditions.push(" AND p.created_at < (SELECT created_at FROM posts WHERE id = $4)");
        arg_index = 5;
    } else if query.after.is_some() {
        conditions.push(" AND p.created_at > (SELECT created_at FROM posts WHERE id = $4)");
        arg_index = 5;
    }

    // Determine ORDER BY direction (validated, not user-input)
    let order_dir = if query.after.is_some() { "ASC" } else { "DESC" };
    let limit_placeholder = if arg_index == 2 {
        "$2"
    } else if arg_index == 3 {
        "$3"
    } else if arg_index == 4 {
        "$4"
    } else {
        "$5"
    };

    let sql = format!(
        r#"
        SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
               p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
               p.reply_count::int8 as reply_count,
               p.last_reply_at, p.seq,
               CASE WHEN u.deleted_at IS NOT NULL THEN 'Deleted user' ELSE u.username END as username,
               u.avatar_url,
               CASE WHEN u.deleted_at IS NOT NULL THEN 'deleted-user@local' ELSE u.email END as email
        FROM posts p
        JOIN channels c ON p.channel_id = c.id
        LEFT JOIN users u ON p.user_id = u.id
        WHERE p.channel_id = $1 AND p.deleted_at IS NULL
        AND (p.root_post_id IS NULL OR c.type IN ('direct', 'group'))
        {}
        ORDER BY p.created_at {} LIMIT {}
    "#,
        conditions.join(""),
        order_dir,
        limit_placeholder
    );

    let mut q = sqlx::query_as::<_, PostResponse>(&sql).bind(channel_id);

    if let Some(pinned) = query.is_pinned {
        q = q.bind(pinned);
    }

    if let Some(ref search_term) = query.q {
        q = q.bind(format!("%{}%", search_term));
    }

    if let Some(before) = query.before {
        q = q.bind(before);
    } else if let Some(after) = query.after {
        q = q.bind(after);
    }

    let posts: Vec<PostResponse> = q.bind(limit).fetch_all(&state.db).await?;

    let mut posts = posts;
    populate_files(&state, &mut posts).await?;
    populate_reactions(&state, &mut posts).await?;
    populate_saved_status(&state, auth.user_id, &mut posts).await?;

    Ok(Json(PostListResponse {
        messages: posts,
        read_state: Some(ReadState {
            last_read_message_id: last_read,
            first_unread_message_id: first_unread,
        }),
    }))
}

/// Create a new post
async fn create_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
    Json(input): Json<CreatePost>,
) -> ApiResult<Json<PostResponse>> {
    let post = crate::services::posts::create_post(
        &state,
        auth.user_id,
        channel_id,
        input.clone(),
        input.client_msg_id,
    )
    .await?;
    Ok(Json(post))
}

/// Get a specific post
async fn get_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Post>> {
    let post: Post = sqlx::query_as(
        r#"
        SELECT id, channel_id, user_id, root_post_id, message, props, file_ids,
               is_pinned, created_at, edited_at, deleted_at,
               reply_count::int8 as reply_count,
               last_reply_at, seq
        FROM posts WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check membership
    let _: ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(post.channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    Ok(Json(post))
}

/// Update a post
async fn update_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdatePost>,
) -> ApiResult<Json<Post>> {
    let post: Post = sqlx::query_as(
        r#"
        SELECT id, channel_id, user_id, root_post_id, message, props, file_ids,
               is_pinned, created_at, edited_at, deleted_at,
               reply_count::int8 as reply_count,
               last_reply_at, seq
        FROM posts WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Only author can edit
    if !auth.can_access_owned(post.user_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden("Cannot edit this post".to_string()));
    }

    if input.message != post.message {
        let post_edit_time_limit_seconds = load_post_edit_time_limit_seconds(&state).await?;
        if post_edit_time_limit_seconds == 0 {
            return Err(AppError::BadRequest(
                "Message editing is disabled by server policy".to_string(),
            ));
        }
        if post_edit_time_limit_seconds > 0 {
            let post_age_seconds = Utc::now()
                .signed_duration_since(post.created_at)
                .num_seconds();
            if post_age_seconds >= post_edit_time_limit_seconds {
                return Err(AppError::BadRequest(format!(
                    "Message edit window expired after {} seconds",
                    post_edit_time_limit_seconds
                )));
            }
        }
    }

    let updated: Post = sqlx::query_as(
        r#"
        UPDATE posts SET message = $1, edited_at = NOW() WHERE id = $2
        RETURNING id, channel_id, user_id, root_post_id, message, props, file_ids,
                  is_pinned, created_at, edited_at, deleted_at,
                  reply_count::int8 as reply_count,
                  last_reply_at, seq
        "#,
    )
    .bind(&input.message)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    // Broadcast update
    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::MessageUpdated,
        serde_json::json!({
            "id": updated.id,
            "channel_id": updated.channel_id,
            "message": updated.message,
            "edited_at": updated.edited_at
        }),
        Some(updated.channel_id),
    )
    .with_broadcast(crate::realtime::WsBroadcast {
        channel_id: Some(updated.channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(updated))
}

async fn load_post_edit_time_limit_seconds(state: &AppState) -> ApiResult<i64> {
    let limit = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE((site->>'post_edit_time_limit_seconds')::bigint, -1) FROM server_config WHERE id = 'default'",
    )
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(-1);
    Ok(limit)
}

/// Soft delete a post
async fn delete_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let post: Post = sqlx::query_as(
        r#"
        SELECT id, channel_id, user_id, root_post_id, message, props, file_ids,
               is_pinned, created_at, edited_at, deleted_at,
               reply_count::int8 as reply_count,
               last_reply_at, seq
        FROM posts WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Only author or admin can delete
    if !auth.can_access_owned(post.user_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden("Cannot delete this post".to_string()));
    }

    sqlx::query("UPDATE posts SET deleted_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    // Broadcast deletion
    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::MessageDeleted,
        serde_json::json!({
            "post_id": id,
            "channel_id": post.channel_id
        }),
        Some(post.channel_id),
    )
    .with_broadcast(crate::realtime::WsBroadcast {
        channel_id: Some(post.channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

/// Get thread replies
async fn get_thread(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<ThreadQuery>,
) -> ApiResult<Json<ThreadResponse>> {
    use crate::mattermost_compat::id::parse_mm_or_uuid;

    // Parse cursor if provided
    let cursor = match query.cursor {
        Some(cursor_str) => Some(
            parse_mm_or_uuid(&cursor_str)
                .ok_or_else(|| AppError::BadRequest("Invalid cursor".to_string()))?,
        ),
        None => None,
    };

    // Call the service method
    let thread_response =
        crate::services::posts::get_thread(&state, id, cursor, query.limit).await?;

    // Check channel membership permission using the first post
    let first_post = thread_response
        .posts
        .values()
        .next()
        .ok_or_else(|| AppError::NotFound("Thread not found".to_string()))?;

    let _: ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(first_post.channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    Ok(Json(thread_response))
}

/// Add a reaction
async fn add_reaction(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<CreateReaction>,
) -> ApiResult<Json<Reaction>> {
    let post: Post = sqlx::query_as(
        r#"
        SELECT id, channel_id, user_id, root_post_id, message, props, file_ids,
               is_pinned, created_at, edited_at, deleted_at,
               reply_count::int8 as reply_count,
               last_reply_at, seq
        FROM posts WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check membership
    let _: ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(post.channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    let reaction: Reaction = sqlx::query_as(
        r#"
        INSERT INTO reactions (post_id, user_id, emoji_name)
        VALUES ($1, $2, $3)
        ON CONFLICT (post_id, user_id, emoji_name) DO UPDATE SET create_at = extract(epoch from now()) * 1000
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&input.emoji_name)
    .fetch_one(&state.db)
    .await?;

    // Broadcast reaction
    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::ReactionAdded,
        reaction.clone(),
        Some(post.channel_id),
    )
    .with_broadcast(crate::realtime::WsBroadcast {
        channel_id: Some(post.channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(reaction))
}

/// Remove a reaction
async fn remove_reaction(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, emoji)): Path<(Uuid, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    // Get post to find channel_id for broadcast
    let post: Option<Post> = sqlx::query_as(
        r#"
        SELECT id, channel_id, user_id, root_post_id, message, props, file_ids,
               is_pinned, created_at, edited_at, deleted_at,
               reply_count::int8 as reply_count,
               last_reply_at, seq
        FROM posts WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    sqlx::query("DELETE FROM reactions WHERE post_id = $1 AND user_id = $2 AND emoji_name = $3")
        .bind(id)
        .bind(auth.user_id)
        .bind(&emoji)
        .execute(&state.db)
        .await?;

    if let Some(p) = post {
        let broadcast = crate::realtime::WsEnvelope::event(
            crate::realtime::EventType::ReactionRemoved,
            serde_json::json!({
                "post_id": id,
                "user_id": auth.user_id,
                "emoji_name": emoji
            }),
            Some(p.channel_id),
        )
        .with_broadcast(crate::realtime::WsBroadcast {
            channel_id: Some(p.channel_id),
            team_id: None,
            user_id: None,
            exclude_user_id: None,
        });
        state.ws_hub.broadcast(broadcast).await;
    }

    Ok(Json(serde_json::json!({"status": "removed"})))
}

/// Pin a post
async fn pin_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Post>> {
    let post: Post = sqlx::query_as(
        r#"
        SELECT id, channel_id, user_id, root_post_id, message, props, file_ids,
               is_pinned, created_at, edited_at, deleted_at,
               reply_count::int8 as reply_count,
               last_reply_at, seq
        FROM posts WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check admin membership
    let member: ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(post.channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    if member.role != "admin" && !auth.has_permission(&permissions::CHANNEL_MANAGE) {
        return Err(AppError::Forbidden("Only admins can pin posts".to_string()));
    }

    let pinned: Post = sqlx::query_as(
        r#"
        UPDATE posts SET is_pinned = true WHERE id = $1
        RETURNING id, channel_id, user_id, root_post_id, message, props, file_ids,
                  is_pinned, created_at, edited_at, deleted_at,
                  reply_count::int8 as reply_count,
                  last_reply_at, seq
        "#,
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    // Broadcast pin change
    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::MessageUpdated,
        serde_json::json!({
            "id": pinned.id,
            "channel_id": pinned.channel_id,
            "is_pinned": true
        }),
        Some(pinned.channel_id),
    )
    .with_broadcast(crate::realtime::WsBroadcast {
        channel_id: Some(pinned.channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(pinned))
}

/// Unpin a post
async fn unpin_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Post>> {
    let post: Post = sqlx::query_as(
        r#"
        SELECT id, channel_id, user_id, root_post_id, message, props, file_ids,
               is_pinned, created_at, edited_at, deleted_at,
               reply_count::int8 as reply_count,
               last_reply_at, seq
        FROM posts WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check admin membership
    let member: ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(post.channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    if member.role != "admin" && !auth.has_permission(&permissions::CHANNEL_MANAGE) {
        return Err(AppError::Forbidden(
            "Only admins can unpin posts".to_string(),
        ));
    }

    let unpinned: Post = sqlx::query_as(
        r#"
        UPDATE posts SET is_pinned = false WHERE id = $1
        RETURNING id, channel_id, user_id, root_post_id, message, props, file_ids,
                  is_pinned, created_at, edited_at, deleted_at,
                  reply_count::int8 as reply_count,
                  last_reply_at, seq
        "#,
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    // Broadcast pin change
    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::MessageUpdated,
        serde_json::json!({
            "id": unpinned.id,
            "channel_id": unpinned.channel_id,
            "is_pinned": false
        }),
        Some(unpinned.channel_id),
    )
    .with_broadcast(crate::realtime::WsBroadcast {
        channel_id: Some(unpinned.channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(unpinned))
}

/// Helper to populate files for posts
async fn populate_files(state: &AppState, posts: &mut [PostResponse]) -> ApiResult<()> {
    crate::services::posts::populate_files(state, posts).await
}

/// Save a post
async fn save_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // Verify post exists
    let _post: Post = sqlx::query_as(
        r#"
        SELECT id, channel_id, user_id, root_post_id, message, props, file_ids,
               is_pinned, created_at, edited_at, deleted_at,
               reply_count::int8 as reply_count,
               last_reply_at, seq
        FROM posts WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check if membership is needed? Usually saving implies you can see it.
    // If list_posts filters by membership, saving implies you have access.
    // We can skip explicit membership check here if we assume valid post ID is sufficient.
    // Let's add a quick membership check.
    let channel_id: Uuid = sqlx::query_scalar("SELECT channel_id FROM posts WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    let _: ChannelMember =
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    sqlx::query(
        "INSERT INTO saved_posts (user_id, post_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(auth.user_id)
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({"status": "saved"})))
}

/// Unsave a post
async fn unsave_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    sqlx::query("DELETE FROM saved_posts WHERE user_id = $1 AND post_id = $2")
        .bind(auth.user_id)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "unsaved"})))
}

/// Get saved posts for current user
async fn get_saved_posts(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<PostResponse>>> {
    let posts: Vec<PostResponse> = sqlx::query_as(
        r#"
        SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
               p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
               p.reply_count::int8 as reply_count,
               p.last_reply_at, p.seq,
               CASE WHEN u.deleted_at IS NOT NULL THEN 'Deleted user' ELSE u.username END as username,
               u.avatar_url,
               CASE WHEN u.deleted_at IS NOT NULL THEN 'deleted-user@local' ELSE u.email END as email
        FROM saved_posts s
        JOIN posts p ON s.post_id = p.id
        LEFT JOIN users u ON p.user_id = u.id
        WHERE s.user_id = $1 AND p.deleted_at IS NULL
        ORDER BY s.created_at DESC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mut posts = posts;
    populate_files(&state, &mut posts).await?;
    populate_reactions(&state, &mut posts).await?;
    // For saved posts view, they are all obviously saved.
    for post in &mut posts {
        post.is_saved = true;
    }

    Ok(Json(posts))
}

/// Helper to populate reactions status
async fn populate_reactions(state: &AppState, posts: &mut [PostResponse]) -> ApiResult<()> {
    if posts.is_empty() {
        return Ok(());
    }

    let post_ids: Vec<Uuid> = posts.iter().map(|p| p.id).collect();

    let reactions: Vec<Reaction> =
        sqlx::query_as("SELECT * FROM reactions WHERE post_id = ANY($1) ORDER BY create_at")
            .bind(&post_ids)
            .fetch_all(&state.db)
            .await?;

    let mut reaction_map: HashMap<Uuid, Vec<Reaction>> = HashMap::new();
    for r in reactions {
        reaction_map.entry(r.post_id).or_default().push(r);
    }

    for post in posts {
        let post_reactions = reaction_map.remove(&post.id).unwrap_or_default();
        let mut aggregated: HashMap<String, crate::models::ReactionResponse> = HashMap::new();

        for r in post_reactions {
            let entry = aggregated.entry(r.emoji_name.clone()).or_insert_with(|| {
                crate::models::ReactionResponse {
                    emoji: r.emoji_name,
                    count: 0,
                    users: vec![],
                }
            });
            entry.count += 1;
            entry.users.push(r.user_id);
        }

        post.reactions = aggregated.into_values().collect();
    }

    Ok(())
}

/// Helper to populate is_saved status
async fn populate_saved_status(
    state: &AppState,
    user_id: Uuid,
    posts: &mut [PostResponse],
) -> ApiResult<()> {
    if posts.is_empty() {
        return Ok(());
    }

    let post_ids: Vec<Uuid> = posts.iter().map(|p| p.id).collect();

    let saved_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT post_id FROM saved_posts WHERE user_id = $1 AND post_id = ANY($2)",
    )
    .bind(user_id)
    .bind(&post_ids)
    .fetch_all(&state.db)
    .await?;

    let saved_set: std::collections::HashSet<Uuid> = saved_ids.into_iter().collect();

    for post in posts {
        post.is_saved = saved_set.contains(&post.id);
    }

    Ok(())
}
