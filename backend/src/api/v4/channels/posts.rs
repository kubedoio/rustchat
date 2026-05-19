use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;

use super::{encode_mm_id, mm, parse_mm_or_uuid, ApiResult, AppState, MmAuthUser};
use crate::api::v4::posts::reactions_for_posts;
use crate::models::post::PostResponse;
use crate::repositories::PostRepository;
use serde_json::json;

#[derive(Deserialize)]
pub struct Pagination {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
    /// Post ID to fetch posts before (for backward pagination)
    pub before: Option<String>,
    /// Post ID to fetch posts after (for forward pagination)  
    pub after: Option<String>,
    /// Timestamp in milliseconds to fetch posts since (for incremental sync)
    pub since: Option<i64>,
}

pub async fn get_posts(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Query(pagination): Query<Pagination>,
) -> ApiResult<Json<mm::PostList>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;

    let repo = PostRepository::new(state.db.clone());

    // Check channel membership first
    repo.require_channel_membership(channel_id, auth.user_id)
        .await?;

    let per_page = pagination.per_page.unwrap_or(60).min(200) as i64;

    // Determine query type based on pagination params
    let mut posts: Vec<PostResponse> =
        if let Some(since) = pagination.since {
            // Incremental sync: get posts created or edited since timestamp
            let since_time = chrono::DateTime::from_timestamp_millis(since)
                .ok_or_else(|| crate::error::AppError::BadRequest("Invalid since timestamp".to_string()))?;

            repo.list_since(channel_id, since_time, per_page)
                .await?
                .into_iter()
                .map(Into::into)
                .collect()
        } else if let Some(before) = &pagination.before {
            // Cursor pagination: get posts before a specific post
            let before_id = parse_mm_or_uuid(before).ok_or_else(|| {
                crate::error::AppError::BadRequest("Invalid before post_id".to_string())
            })?;

            let before_time = repo.get_post_created_at(before_id).await?.ok_or_else(|| {
                crate::error::AppError::NotFound("Before post not found".to_string())
            })?;

            repo.list_before(channel_id, before_time, per_page)
                .await?
                .into_iter()
                .map(Into::into)
                .collect()
        } else if let Some(after) = &pagination.after {
            // Cursor pagination: get posts after a specific post
            let after_id = parse_mm_or_uuid(after).ok_or_else(|| {
                crate::error::AppError::BadRequest("Invalid after post_id".to_string())
            })?;

            let after_time = repo.get_post_created_at(after_id).await?.ok_or_else(|| {
                crate::error::AppError::NotFound("After post not found".to_string())
            })?;

            repo.list_after(channel_id, after_time, per_page)
                .await?
                .into_iter()
                .map(Into::into)
                .collect()
        } else {
            // Standard page-based pagination
            let page = pagination.page.unwrap_or(0);
            let offset = (page * per_page as u64) as i64;

            repo.list_by_channel(channel_id, per_page, offset)
                .await?
                .into_iter()
                .map(Into::into)
                .collect()
        };

    crate::services::posts::populate_files(&state, &mut posts).await?;

    let mut order = Vec::new();
    let mut posts_map: HashMap<String, mm::Post> = HashMap::new();
    let mut post_ids = Vec::new();
    let mut id_map = Vec::new();

    // Determine prev/next post IDs for pagination hints
    let (prev_post_id, next_post_id) = if !posts.is_empty() {
        let first_id = encode_mm_id(posts.first().unwrap().id);
        let last_id = encode_mm_id(posts.last().unwrap().id);
        // If using before/after, provide the opposite cursor
        if pagination.before.is_some() {
            (last_id, String::new())
        } else if pagination.after.is_some() {
            (String::new(), first_id)
        } else {
            (String::new(), String::new())
        }
    } else {
        (String::new(), String::new())
    };

    for p in posts {
        let id = encode_mm_id(p.id);
        post_ids.push(p.id);
        id_map.push((p.id, id.clone()));
        order.push(id.clone());
        posts_map.insert(id, p.into());
    }

    let reactions_map = reactions_for_posts(&state, &post_ids).await?;
    for (post_uuid, post_id) in id_map {
        if let Some(reactions) = reactions_map.get(&post_uuid) {
            if !reactions.is_empty() {
                if let Some(post) = posts_map.get_mut(&post_id) {
                    let mut metadata = post.metadata.take().unwrap_or_else(|| json!({}));
                    if let Some(obj) = metadata.as_object_mut() {
                        obj.insert("reactions".to_string(), json!(reactions));
                    }
                    post.metadata = Some(metadata);
                }
            }
        }
    }

    Ok(Json(mm::PostList {
        order,
        posts: posts_map,
        next_post_id,
        prev_post_id,
    }))
}

#[derive(Deserialize)]
pub struct PinnedPostsQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

/// GET /channels/{channel_id}/pinned - Get pinned posts
pub async fn get_pinned_posts(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
    Query(query): Query<PinnedPostsQuery>,
) -> ApiResult<Json<mm::PostList>> {
    let channel_id = parse_mm_or_uuid(&channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;

    let repo = PostRepository::new(state.db.clone());

    // Verify membership
    repo.require_channel_membership(channel_id, auth.user_id)
        .await?;

    let per_page = query.per_page.unwrap_or(100).min(100) as i64;
    let page = query.page.unwrap_or(0) as i64;
    let offset = page * per_page;

    let mut posts: Vec<PostResponse> = repo
        .list_pinned_posts(channel_id, per_page, offset)
        .await?
        .into_iter()
        .map(|p| p.into())
        .collect();

    crate::services::posts::populate_files(&state, &mut posts).await?;

    let mut order = Vec::new();
    let mut posts_map: HashMap<String, mm::Post> = HashMap::new();
    let mut post_ids = Vec::new();
    let mut id_map = Vec::new();

    for p in posts {
        let id = encode_mm_id(p.id);
        post_ids.push(p.id);
        id_map.push((p.id, id.clone()));
        order.push(id.clone());
        posts_map.insert(id, p.into());
    }

    let reactions_map = reactions_for_posts(&state, &post_ids).await?;
    for (post_uuid, post_id) in id_map {
        if let Some(reactions) = reactions_map.get(&post_uuid) {
            if !reactions.is_empty() {
                if let Some(post) = posts_map.get_mut(&post_id) {
                    let mut metadata = post.metadata.take().unwrap_or_else(|| json!({}));
                    if let Some(obj) = metadata.as_object_mut() {
                        obj.insert("reactions".to_string(), json!(reactions));
                    }
                    post.metadata = Some(metadata);
                }
            }
        }
    }

    Ok(Json(mm::PostList {
        order,
        posts: posts_map,
        next_post_id: String::new(),
        prev_post_id: String::new(),
    }))
}

/// Path for pin/unpin operations
#[derive(Deserialize)]
pub struct PinPath {
    pub channel_id: String,
    pub post_id: String,
}

/// POST /channels/{channel_id}/posts/{post_id}/pin - Pin a post
pub async fn pin_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<PinPath>,
) -> ApiResult<impl IntoResponse> {
    let channel_id = parse_mm_or_uuid(&path.channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;
    let post_id = parse_mm_or_uuid(&path.post_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid post_id".to_string()))?;

    let repo = PostRepository::new(state.db.clone());

    // Verify membership
    repo.require_channel_membership(channel_id, auth.user_id)
        .await?;

    // Pin the post
    repo.pin_post(post_id).await?;

    Ok(Json(json!({"status": "OK"})))
}

/// POST /channels/{channel_id}/posts/{post_id}/unpin - Unpin a post
pub async fn unpin_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<PinPath>,
) -> ApiResult<impl IntoResponse> {
    let channel_id = parse_mm_or_uuid(&path.channel_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid channel_id".to_string()))?;
    let post_id = parse_mm_or_uuid(&path.post_id)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid post_id".to_string()))?;

    let repo = PostRepository::new(state.db.clone());

    // Verify membership
    repo.require_channel_membership(channel_id, auth.user_id)
        .await?;

    // Unpin the post
    repo.unpin_post(post_id).await?;

    Ok(Json(json!({"status": "OK"})))
}
