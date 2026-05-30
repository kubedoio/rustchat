use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use super::{
    encode_mm_id, mm, parse_mm_or_uuid, status_ok, ApiResult, AppError, AppState, MmAuthUser,
};
use crate::repositories::{ChannelRepository, PostRepository};

#[derive(Deserialize)]
pub(super) struct PostsUnreadQuery {
    #[serde(default = "default_limit")]
    limit_before: i32,
    #[serde(default = "default_limit")]
    limit_after: i32,
    #[serde(rename = "skipFetchThreads", default)]
    _skip_fetch_threads: bool,
    #[serde(rename = "collapsedThreads", default)]
    _collapsed_threads: bool,
    #[serde(rename = "collapsedThreadsExtended", default)]
    _collapsed_threads_extended: bool,
}

fn default_limit() -> i32 {
    60
}

fn clamp_unread_limits(query: &PostsUnreadQuery) -> (i64, i64) {
    (
        query.limit_before.clamp(0, 200) as i64,
        query.limit_after.clamp(1, 200) as i64,
    )
}

#[derive(Deserialize)]
pub(super) struct PostsUnreadPath {
    user_id: String,
    channel_id: String,
}

/// GET /api/v4/users/{user_id}/channels/{channel_id}/posts/unread
pub(super) async fn get_posts_around_unread(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<PostsUnreadPath>,
    Query(query): Query<PostsUnreadQuery>,
) -> ApiResult<Json<mm::PostList>> {
    let user_id = crate::api::v4::users::resolve_user_id(&path.user_id, &auth)
        .map_err(|_| AppError::Forbidden("Cannot access another user's posts".to_string()))?;

    let channel_id =
        parse_mm_or_uuid(&path.channel_id).ok_or_else(|| AppError::InvalidChannelId)?;

    let _ = ChannelRepository::new(&state.db)
        .require_member(channel_id, user_id)
        .await?;

    let (limit_before, limit_after) = clamp_unread_limits(&query);

    let last_read_seq: i64 = PostRepository::new(state.db.clone())
        .get_last_read_seq(user_id, channel_id)
        .await?
        .unwrap_or(0);

    let mut posts = PostRepository::new(state.db.clone())
        .get_posts_around_unread(channel_id, last_read_seq, limit_before, limit_after)
        .await?;

    crate::services::posts::populate_files(&state, &mut posts).await?;

    let (order, posts_map) = crate::api::v4::posts::build_mm_posts_map(&state, posts).await?;

    Ok(Json(mm::PostList {
        order,
        posts: posts_map,
        next_post_id: String::new(),
        prev_post_id: String::new(),
    }))
}

#[derive(Deserialize)]
pub(super) struct AckPath {
    user_id: String,
    post_id: String,
}

/// POST /api/v4/users/{user_id}/posts/{post_id}/ack - Acknowledge a post
pub(super) async fn save_acknowledgement_for_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<AckPath>,
) -> ApiResult<Json<mm::PostAcknowledgement>> {
    let user_id = crate::api::v4::users::resolve_user_id(&path.user_id, &auth)
        .map_err(|_| AppError::Forbidden("Cannot acknowledge for another user".to_string()))?;

    let post_id = parse_mm_or_uuid(&path.post_id).ok_or_else(|| AppError::InvalidPostId)?;

    let channel_id = PostRepository::new(state.db.clone())
        .get_post_channel_id_optional(post_id)
        .await?
        .ok_or_else(|| AppError::PostNotFound)?;

    let _ = ChannelRepository::new(&state.db)
        .require_member(channel_id, user_id)
        .await?;

    let now = chrono::Utc::now();

    PostRepository::new(state.db.clone())
        .acknowledge_post(user_id, post_id, now)
        .await?;

    Ok(Json(mm::PostAcknowledgement {
        user_id: encode_mm_id(user_id),
        post_id: encode_mm_id(post_id),
        acknowledged_at: now.timestamp_millis(),
    }))
}

/// DELETE /api/v4/users/{user_id}/posts/{post_id}/ack - Delete a post acknowledgement
pub(super) async fn delete_acknowledgement_for_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(path): Path<AckPath>,
) -> ApiResult<impl IntoResponse> {
    let user_id = crate::api::v4::users::resolve_user_id(&path.user_id, &auth).map_err(|_| {
        AppError::Forbidden("Cannot delete acknowledgement for another user".to_string())
    })?;

    let post_id = parse_mm_or_uuid(&path.post_id).ok_or_else(|| AppError::InvalidPostId)?;

    let ack_time = PostRepository::new(state.db.clone())
        .get_acknowledgement(user_id, post_id)
        .await?;

    if let Some(ack_time) = ack_time {
        let now = chrono::Utc::now();
        let five_minutes = chrono::Duration::minutes(5);
        if now - ack_time > five_minutes {
            return Err(AppError::Forbidden(
                "Cannot delete acknowledgement after 5 minutes".to_string(),
            ));
        }
    } else {
        return Err(AppError::NotFound("Acknowledgement not found".to_string()));
    }

    PostRepository::new(state.db.clone())
        .delete_acknowledgement(user_id, post_id)
        .await?;

    Ok(status_ok())
}

#[cfg(test)]
mod tests {
    use super::{clamp_unread_limits, PostsUnreadQuery};

    #[test]
    fn clamps_unread_limits() {
        let query = PostsUnreadQuery {
            limit_before: 500,
            limit_after: -10,
            _skip_fetch_threads: false,
            _collapsed_threads: false,
            _collapsed_threads_extended: false,
        };

        assert_eq!(clamp_unread_limits(&query), (200, 1));
    }
}
