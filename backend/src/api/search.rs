//! Search API endpoints

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppState;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::Post;

/// Build search routes
pub fn router() -> Router<AppState> {
    Router::new().route("/search", get(search_messages))
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub channel_id: Option<Uuid>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub posts: Vec<Post>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

/// Full-text search for messages
async fn search_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<SearchQuery>,
) -> ApiResult<Json<SearchResult>> {
    if query.q.trim().is_empty() {
        return Err(AppError::Validation(
            "Search query cannot be empty".to_string(),
        ));
    }

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    // Search across accessible channels using regular sqlx::query_as
    let posts: Vec<Post> = if let Some(channel_id) = query.channel_id {
        sqlx::query_as(
            r#"
            SELECT p.* FROM posts p
            INNER JOIN channel_members cm ON cm.channel_id = p.channel_id AND cm.user_id = $1
            WHERE p.channel_id = $2
              AND p.deleted_at IS NULL
              AND to_tsvector('english', p.message) @@ plainto_tsquery('english', $3)
            ORDER BY ts_rank(to_tsvector('english', p.message), plainto_tsquery('english', $3)) DESC, p.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(auth.user_id)
        .bind(channel_id)
        .bind(&query.q)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            r#"
            SELECT p.* FROM posts p
            INNER JOIN channel_members cm ON cm.channel_id = p.channel_id AND cm.user_id = $1
            WHERE p.deleted_at IS NULL
              AND to_tsvector('english', p.message) @@ plainto_tsquery('english', $2)
            ORDER BY ts_rank(to_tsvector('english', p.message), plainto_tsquery('english', $2)) DESC, p.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(auth.user_id)
        .bind(&query.q)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    };

    let total = posts.len() as i64;

    Ok(Json(SearchResult {
        posts,
        total,
        page,
        per_page,
    }))
}
