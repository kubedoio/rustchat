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
use crate::constants::{DEFAULT_SEARCH_LIMIT, MAX_SEARCH_LIMIT};
use crate::error::{ApiResult, AppError};
use crate::models::reaction::Reaction;
use crate::models::{
    ChannelMember, CreatePost, CreateReaction, Post, PostResponse, ThreadResponse, UpdatePost,
};
use crate::repositories::PostRepository;

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

    let repo = PostRepository::new(state.db.clone());

    // Check membership
    let _: ChannelMember = repo
        .require_channel_membership(channel_id, auth.user_id)
        .await?;

    // Get read state
    let last_read = repo.get_channel_read(auth.user_id, channel_id).await?;
    let first_unread = repo.get_first_unread_seq(channel_id, last_read).await?;

    let limit = query.limit.unwrap_or(DEFAULT_SEARCH_LIMIT).min(MAX_SEARCH_LIMIT);

    // Build SQL query safely without format!() for dynamic parts
    // Use separate condition arrays that we track in parallel with bound values
    let mut conditions: Vec<String> = Vec::new();
    let mut arg_index: usize = 2;

    if query.is_pinned.is_some() {
        conditions.push(format!(" AND p.is_pinned = ${}", arg_index));
        arg_index += 1;
    }

    if query.q.is_some() {
        conditions.push(format!(" AND p.message ILIKE ${}", arg_index));
        arg_index += 1;
    }

    if query.before.is_some() {
        conditions.push(format!(" AND p.created_at < (SELECT created_at FROM posts WHERE id = ${})", arg_index));
        arg_index += 1;
    } else if query.after.is_some() {
        conditions.push(format!(" AND p.created_at > (SELECT created_at FROM posts WHERE id = ${})", arg_index));
        arg_index += 1;
    }

    // Determine ORDER BY direction (validated, not user-input)
    let order_dir = if query.after.is_some() { "ASC" } else { "DESC" };
    let limit_placeholder = format!("${}", arg_index);

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
    let repo = PostRepository::new(state.db.clone());

    let post = repo
        .get_post_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check membership
    let _: ChannelMember = repo
        .require_channel_membership(post.channel_id, auth.user_id)
        .await?;

    Ok(Json(post))
}

/// Update a post
async fn update_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdatePost>,
) -> ApiResult<Json<Post>> {
    let repo = PostRepository::new(state.db.clone());

    let post = repo
        .get_post_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Only author can edit
    if !auth.can_access_owned(post.user_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden("Cannot edit this post".to_string()));
    }

    if input.message != post.message {
        let post_edit_time_limit_seconds = repo.load_post_edit_time_limit_seconds().await?;
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

    let updated = repo
        .update_post_message_returning(id, &input.message)
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

/// Soft delete a post
async fn delete_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = PostRepository::new(state.db.clone());

    let post = repo
        .get_post_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Only author or admin can delete
    if !auth.can_access_owned(post.user_id, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden("Cannot delete this post".to_string()));
    }

    repo.soft_delete_post_simple(id).await?;

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

    let _: ChannelMember = PostRepository::new(state.db.clone())
        .require_channel_membership(first_post.channel_id, auth.user_id)
        .await?;

    Ok(Json(thread_response))
}

/// Add a reaction
async fn add_reaction(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<CreateReaction>,
) -> ApiResult<Json<Reaction>> {
    let repo = PostRepository::new(state.db.clone());

    let post = repo
        .get_post_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check membership
    let _: ChannelMember = repo
        .require_channel_membership(post.channel_id, auth.user_id)
        .await?;

    let reaction = repo
        .add_reaction(id, auth.user_id, &input.emoji_name)
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
    let repo = PostRepository::new(state.db.clone());

    // Get post to find channel_id for broadcast
    let post = repo.get_post_by_id(id).await?;

    repo.remove_reaction(id, auth.user_id, &emoji).await?;

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
    let repo = PostRepository::new(state.db.clone());

    let post = repo
        .get_post_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check admin membership
    let member = repo
        .require_channel_membership(post.channel_id, auth.user_id)
        .await?;

    if member.role != "admin" && !auth.has_permission(&permissions::CHANNEL_MANAGE) {
        return Err(AppError::Forbidden("Only admins can pin posts".to_string()));
    }

    let pinned = repo.pin_post_returning(id).await?;

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
    let repo = PostRepository::new(state.db.clone());

    let post = repo
        .get_post_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check admin membership
    let member = repo
        .require_channel_membership(post.channel_id, auth.user_id)
        .await?;

    if member.role != "admin" && !auth.has_permission(&permissions::CHANNEL_MANAGE) {
        return Err(AppError::Forbidden(
            "Only admins can unpin posts".to_string(),
        ));
    }

    let unpinned = repo.unpin_post_returning(id).await?;

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
    let repo = PostRepository::new(state.db.clone());

    // Verify post exists
    let _post = repo
        .get_post_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    let channel_id = repo.get_post_channel_id(id).await?;

    let _: ChannelMember = repo
        .require_channel_membership(channel_id, auth.user_id)
        .await?;

    repo.save_post(auth.user_id, id).await?;

    Ok(Json(serde_json::json!({"status": "saved"})))
}

/// Unsave a post
async fn unsave_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = PostRepository::new(state.db.clone());
    repo.unsave_post(auth.user_id, id).await?;

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

    let reactions = PostRepository::new(state.db.clone())
        .get_reactions_for_posts(&post_ids)
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

    let saved_ids = PostRepository::new(state.db.clone())
        .get_saved_post_ids(user_id, &post_ids)
        .await?;

    let saved_set: std::collections::HashSet<Uuid> = saved_ids.into_iter().collect();

    for post in posts {
        post.is_saved = saved_set.contains(&post.id);
    }

    Ok(())
}
