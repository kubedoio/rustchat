//! Search API endpoints

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;
use crate::api::posts::{populate_reactions, populate_saved_status};
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::PostResponse;

/// Build search routes
pub fn router() -> Router<AppState> {
    Router::new().route("/search", get(search_messages))
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub channel_id: Option<Uuid>,
}

/// Search for messages
async fn search_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<SearchQuery>,
) -> ApiResult<Json<Vec<PostResponse>>> {
    let search_term = query.q.trim();
    if search_term.is_empty() {
        return Err(AppError::Validation(
            "Search query cannot be empty".to_string(),
        ));
    }

    let search_pattern = format!("%{}%", search_term);

    let posts: Vec<PostResponse> = if let Some(channel_id) = query.channel_id {
        sqlx::query_as(
            r#"
            SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                   p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                   p.reply_count::int8 as reply_count,
                   p.last_reply_at, p.seq,
                   CASE WHEN u.deleted_at IS NOT NULL THEN 'Deleted user' ELSE u.username END as username,
                   u.avatar_url,
                   CASE WHEN u.deleted_at IS NOT NULL THEN 'deleted-user@local' ELSE u.email END as email
            FROM posts p
            JOIN users u ON u.id = p.user_id
            INNER JOIN channel_members cm ON cm.channel_id = p.channel_id AND cm.user_id = $1
            WHERE p.channel_id = $2
              AND p.deleted_at IS NULL
              AND p.message ILIKE $3
            ORDER BY p.created_at DESC
            LIMIT 50
            "#,
        )
        .bind(auth.user_id)
        .bind(channel_id)
        .bind(&search_pattern)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            r#"
            SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                   p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                   p.reply_count::int8 as reply_count,
                   p.last_reply_at, p.seq,
                   CASE WHEN u.deleted_at IS NOT NULL THEN 'Deleted user' ELSE u.username END as username,
                   u.avatar_url,
                   CASE WHEN u.deleted_at IS NOT NULL THEN 'deleted-user@local' ELSE u.email END as email
            FROM posts p
            JOIN users u ON u.id = p.user_id
            INNER JOIN channel_members cm ON cm.channel_id = p.channel_id AND cm.user_id = $1
            WHERE p.deleted_at IS NULL
              AND p.message ILIKE $2
            ORDER BY p.created_at DESC
            LIMIT 50
            "#,
        )
        .bind(auth.user_id)
        .bind(&search_pattern)
        .fetch_all(&state.db)
        .await?
    };

    let mut posts = posts;
    crate::services::posts::populate_files(&state, &mut posts).await?;
    populate_reactions(&state, &mut posts).await?;
    populate_saved_status(&state, auth.user_id, &mut posts).await?;

    Ok(Json(posts))
}
