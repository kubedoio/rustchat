//! Post querying and rendering: pagination, single-post fetch, thread fetch,
//! file population and avatar normalization.

use std::collections::HashMap;
use uuid::Uuid;

use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::models::{normalize_avatar_url, FileUploadResponse, PostResponse};
use crate::repositories::PostRepository;

#[derive(Debug, Default)]
pub struct PostsQuery {
    pub page: i64,
    pub per_page: i64,
    pub since: Option<i64>,
    pub before: Option<Uuid>,
    pub after: Option<Uuid>,
}

/// Helper to populate files for posts
/// Uses authenticated API endpoints instead of presigned S3 URLs
/// This ensures files remain accessible after re-login and require authentication
pub async fn populate_files(state: &AppState, posts: &mut [PostResponse]) -> ApiResult<()> {
    use crate::mattermost_compat::id::encode_mm_id;

    normalize_post_avatar_urls(posts);

    // 1. Collect all file IDs
    let all_file_ids: Vec<Uuid> = posts.iter().flat_map(|p| p.file_ids.clone()).collect();

    if all_file_ids.is_empty() {
        return Ok(());
    }

    // 2. Fetch file infos
    let files = PostRepository::new(state.db.clone())
        .get_post_files(&all_file_ids)
        .await?;

    // 3. Generate authenticated API URLs (not presigned S3 URLs)
    // These URLs require authentication and don't expire
    let mut file_map = HashMap::new();
    for file in files {
        let mm_file_id = encode_mm_id(file.id);

        // Use authenticated API endpoints instead of presigned S3 URLs
        // This ensures:
        // 1. Files require authentication to access
        // 2. URLs don't expire after logout/login
        // 3. Original filenames are preserved in Content-Disposition header
        let url = format!("/api/v4/files/{}", mm_file_id);
        let thumbnail_url = if file.has_thumbnail {
            Some(format!("/api/v4/files/{}/thumbnail", mm_file_id))
        } else {
            None
        };

        file_map.insert(
            file.id,
            FileUploadResponse {
                id: file.id,
                name: file.name,
                mime_type: file.mime_type,
                size: file.size,
                width: file.width.unwrap_or(0),
                height: file.height.unwrap_or(0),
                url,
                thumbnail_url,
            },
        );
    }

    for post in posts {
        post.files.clear();
        for file_id in &post.file_ids {
            if let Some(file_resp) = file_map.get(file_id) {
                post.files.push(file_resp.clone());
            }
        }
    }

    Ok(())
}

pub fn normalize_post_avatar_urls(posts: &mut [PostResponse]) {
    for post in posts {
        post.avatar_url = normalize_avatar_url(post.user_id, post.avatar_url.as_deref());
    }
}

/// Get posts for a channel with various pagination options
pub async fn get_posts(
    state: &AppState,
    channel_id: Uuid,
    query: PostsQuery,
) -> ApiResult<(Vec<PostResponse>, i64)> {
    let per_page = if query.per_page > 0 {
        query.per_page
    } else {
        60
    }
    .min(200);
    let offset = query.page * per_page;

    let posts: Vec<PostResponse> = if let Some(since) = query.since {
        let since_time =
            chrono::DateTime::from_timestamp_millis(since).unwrap_or_else(chrono::Utc::now);

        PostRepository::new(state.db.clone())
            .list_since_including_edited(channel_id, since_time, per_page)
            .await?
    } else if let Some(before_id) = query.before {
        let before_time = PostRepository::new(state.db.clone())
            .get_created_at(before_id)
            .await?;

        let before_time = before_time.ok_or_else(|| AppError::BeforePostNotFound)?;

        PostRepository::new(state.db.clone())
            .list_before(channel_id, before_time, per_page)
            .await?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>()
    } else if let Some(after_id) = query.after {
        let after_time = PostRepository::new(state.db.clone())
            .get_created_at(after_id)
            .await?;

        let after_time = after_time.ok_or_else(|| AppError::AfterPostNotFound)?;

        PostRepository::new(state.db.clone())
            .list_after(channel_id, after_time, per_page)
            .await?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>()
    } else {
        PostRepository::new(state.db.clone())
            .list_by_channel(channel_id, per_page, offset)
            .await?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>()
    };

    let total = PostRepository::new(state.db.clone())
        .count_posts_in_channel(channel_id)
        .await?;

    let mut posts = posts;
    normalize_post_avatar_urls(&mut posts);
    if !posts.is_empty() {
        populate_files(state, &mut posts).await?;
    }

    Ok((posts, total))
}

pub async fn get_post_by_id(state: &AppState, post_id: Uuid) -> ApiResult<PostResponse> {
    let post = PostRepository::new(state.db.clone())
        .find_by_id_include_deleted(post_id)
        .await?
        .ok_or_else(|| AppError::PostNotFound)?;

    let mut post = post;
    post.avatar_url = normalize_avatar_url(post.user_id, post.avatar_url.as_deref());
    populate_files(state, std::slice::from_mut(&mut post)).await?;

    Ok(post)
}

/// Query parameters for thread fetching
#[derive(Debug, Default)]
pub struct ThreadQuery {
    pub cursor: Option<Uuid>,
    pub limit: i64,
}

/// Get thread with parent post and replies
pub async fn get_thread(
    state: &AppState,
    post_id: Uuid,
    cursor: Option<Uuid>,
    limit: i64,
) -> ApiResult<crate::models::ThreadResponse> {
    let limit = limit.clamp(1, 100);

    // Fetch parent post with user info
    let parent = PostRepository::new(state.db.clone())
        .find_by_id_strict(post_id)
        .await?;

    let mut parent = parent.ok_or_else(|| AppError::PostNotFound)?;
    parent.avatar_url = normalize_avatar_url(parent.user_id, parent.avatar_url.as_deref());

    // Fetch replies
    let replies = PostRepository::new(state.db.clone())
        .get_thread_replies_with_cursor(post_id, cursor, limit + 1)
        .await?;

    // Determine pagination
    let has_more = replies.len() > limit as usize;
    let mut replies: Vec<PostResponse> = replies.into_iter().take(limit as usize).collect();
    normalize_post_avatar_urls(&mut replies);

    let next_cursor = if has_more {
        replies.last().map(|r| r.id.to_string())
    } else {
        None
    };

    // Build response
    let mut order = vec![parent.id.to_string()];
    let mut posts_map = std::collections::HashMap::new();
    posts_map.insert(parent.id.to_string(), parent);

    for reply in replies {
        order.push(reply.id.to_string());
        posts_map.insert(reply.id.to_string(), reply);
    }

    Ok(crate::models::ThreadResponse {
        order,
        posts: posts_map,
        next_cursor,
    })
}
