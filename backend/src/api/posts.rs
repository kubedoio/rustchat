//! Posts API endpoints

use axum::{
    extract::{Path, Query, State},
    routing::{get, post, delete},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;
use std::collections::HashMap;

use super::AppState;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{ChannelMember, CreatePost, CreateReaction, Post, PostResponse, Reaction, UpdatePost};

/// Build posts routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/channels/{channel_id}/posts", get(list_posts).post(create_post))
        .route("/posts/{id}", get(get_post).put(update_post).delete(delete_post))
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

/// List posts in a channel
async fn list_posts(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<ListPostsQuery>,
) -> ApiResult<Json<Vec<PostResponse>>> {
    // Check membership
    let _: ChannelMember = sqlx::query_as(
        "SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2"
    )
    .bind(channel_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    let limit = query.limit.unwrap_or(50).min(100);

    let mut sql = String::from(r#"
        SELECT p.*, u.username, u.avatar_url, u.email
        FROM posts p
        LEFT JOIN users u ON p.user_id = u.id
        WHERE p.channel_id = $1 AND p.deleted_at IS NULL AND p.root_post_id IS NULL
    "#);

    let mut arg_index = 2;
    
    if let Some(_) = query.is_pinned {
        sql.push_str(&format!(" AND p.is_pinned = ${}", arg_index));
        arg_index += 1;
    }

    if let Some(ref _q) = query.q {
        sql.push_str(&format!(" AND p.message ILIKE ${}", arg_index));
        arg_index += 1;
    }

    if query.before.is_some() {
        sql.push_str(&format!(" AND p.created_at < (SELECT created_at FROM posts WHERE id = ${})", arg_index));
        arg_index += 1;
    } else if query.after.is_some() {
        sql.push_str(&format!(" AND p.created_at > (SELECT created_at FROM posts WHERE id = ${})", arg_index));
        arg_index += 1;
    }

    sql.push_str(&format!(" ORDER BY p.created_at {} LIMIT ${}", 
        if query.after.is_some() { "ASC" } else { "DESC" },
        arg_index
    ));

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

    Ok(Json(posts))
}

/// Create a new post
async fn create_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
    Json(input): Json<CreatePost>,
) -> ApiResult<Json<PostResponse>> {
    let post = crate::services::posts::create_post(&state, auth.user_id, channel_id, input, None).await?;
    Ok(Json(post))
}


/// Get a specific post
async fn get_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Post>> {
    let post: Post = sqlx::query_as("SELECT * FROM posts WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check membership
    let _: ChannelMember = sqlx::query_as(
        "SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2"
    )
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
    let post: Post = sqlx::query_as("SELECT * FROM posts WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Only author can edit
    if post.user_id != auth.user_id && auth.role != "system_admin" {
        return Err(AppError::Forbidden("Cannot edit this post".to_string()));
    }

    let updated: Post = sqlx::query_as(
        "UPDATE posts SET message = $1, edited_at = NOW() WHERE id = $2 RETURNING *"
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
    ).with_broadcast(crate::realtime::WsBroadcast {
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
    let post: Post = sqlx::query_as("SELECT * FROM posts WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Only author or admin can delete
    if post.user_id != auth.user_id && auth.role != "system_admin" {
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
            "id": id,
            "channel_id": post.channel_id
        }),
        Some(post.channel_id),
    ).with_broadcast(crate::realtime::WsBroadcast {
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
) -> ApiResult<Json<Vec<PostResponse>>> {
    let root_post: Post = sqlx::query_as("SELECT * FROM posts WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check membership
    let _: ChannelMember = sqlx::query_as(
        "SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2"
    )
    .bind(root_post.channel_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    let replies: Vec<PostResponse> = sqlx::query_as(
        r#"
        SELECT p.*, u.username, u.avatar_url, u.email
        FROM posts p
        LEFT JOIN users u ON p.user_id = u.id
        WHERE p.root_post_id = $1 AND p.deleted_at IS NULL
        ORDER BY p.created_at
        "#
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let mut replies = replies;
    populate_files(&state, &mut replies).await?;
    populate_reactions(&state, &mut replies).await?;
    populate_saved_status(&state, auth.user_id, &mut replies).await?;

    Ok(Json(replies))
}

/// Add a reaction
async fn add_reaction(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<CreateReaction>,
) -> ApiResult<Json<Reaction>> {
    let post: Post = sqlx::query_as("SELECT * FROM posts WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check membership
    let _: ChannelMember = sqlx::query_as(
        "SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2"
    )
    .bind(post.channel_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    let reaction: Reaction = sqlx::query_as(
        r#"
        INSERT INTO reactions (post_id, user_id, emoji_name)
        VALUES ($1, $2, $3)
        ON CONFLICT (post_id, user_id, emoji_name) DO UPDATE SET created_at = NOW()
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
    ).with_broadcast(crate::realtime::WsBroadcast {
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
     let post: Option<Post> = sqlx::query_as("SELECT * FROM posts WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
        
    sqlx::query(
        "DELETE FROM reactions WHERE post_id = $1 AND user_id = $2 AND emoji_name = $3"
    )
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
        ).with_broadcast(crate::realtime::WsBroadcast {
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
    let post: Post = sqlx::query_as("SELECT * FROM posts WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check admin membership
    let member: ChannelMember = sqlx::query_as(
        "SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2"
    )
    .bind(post.channel_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    if member.role != "admin" && auth.role != "system_admin" {
        return Err(AppError::Forbidden("Only admins can pin posts".to_string()));
    }

    let pinned: Post = sqlx::query_as("UPDATE posts SET is_pinned = true WHERE id = $1 RETURNING *")
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
    ).with_broadcast(crate::realtime::WsBroadcast {
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
    let post: Post = sqlx::query_as("SELECT * FROM posts WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check admin membership
    let member: ChannelMember = sqlx::query_as(
        "SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2"
    )
    .bind(post.channel_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    if member.role != "admin" && auth.role != "system_admin" {
        return Err(AppError::Forbidden("Only admins can unpin posts".to_string()));
    }

    let unpinned: Post = sqlx::query_as("UPDATE posts SET is_pinned = false WHERE id = $1 RETURNING *")
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
    ).with_broadcast(crate::realtime::WsBroadcast {
        channel_id: Some(unpinned.channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(unpinned))
}

/// Helper to populate files for posts
async fn populate_files(
    state: &AppState,
    posts: &mut [PostResponse],
) -> ApiResult<()> {
    crate::services::posts::populate_files(state, posts).await
}

/// Save a post
async fn save_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // Verify post exists
    let _post: Post = sqlx::query_as("SELECT * FROM posts WHERE id = $1 AND deleted_at IS NULL")
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

    let _: ChannelMember = sqlx::query_as(
        "SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2"
    )
    .bind(channel_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))?;

    sqlx::query(
        "INSERT INTO saved_posts (user_id, post_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
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
    sqlx::query(
        "DELETE FROM saved_posts WHERE user_id = $1 AND post_id = $2"
    )
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
        SELECT p.*, u.username, u.avatar_url, u.email
        FROM saved_posts s
        JOIN posts p ON s.post_id = p.id
        LEFT JOIN users u ON p.user_id = u.id
        WHERE s.user_id = $1 AND p.deleted_at IS NULL
        ORDER BY s.created_at DESC
        "#
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
async fn populate_reactions(
    state: &AppState,
    posts: &mut [PostResponse],
) -> ApiResult<()> {
    if posts.is_empty() {
        return Ok(());
    }

    let post_ids: Vec<Uuid> = posts.iter().map(|p| p.id).collect();

    let reactions: Vec<Reaction> = sqlx::query_as(
        "SELECT * FROM reactions WHERE post_id = ANY($1) ORDER BY created_at"
    )
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
            let entry = aggregated.entry(r.emoji_name.clone()).or_insert_with(|| crate::models::ReactionResponse {
                emoji: r.emoji_name,
                count: 0,
                users: vec![],
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
        "SELECT post_id FROM saved_posts WHERE user_id = $1 AND post_id = ANY($2)"
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
