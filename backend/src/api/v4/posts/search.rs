use axum::{
    body::Bytes,
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

use super::{
    mm, parse_body, parse_mm_or_uuid, ApiResult, AppError,
    AppState, MmAuthUser,
};

#[derive(Deserialize)]
struct SearchPostsRequest {
    terms: String,
    #[serde(rename = "is_or_search", default)]
    _is_or_search: bool,
    #[serde(rename = "time_zone_offset", default)]
    _time_zone_offset: i32,
    #[serde(rename = "include_deleted_channels", default)]
    _include_deleted_channels: bool,
    #[serde(default)]
    page: i32,
    #[serde(default = "default_per_page")]
    per_page: i32,
}

fn default_per_page() -> i32 {
    60
}

fn compute_limit_and_offset(page: i32, per_page: i32) -> (i64, i64) {
    let limit = per_page.clamp(1, 200) as i64;
    let offset = (page.max(0) as i64) * limit;
    (limit, offset)
}

fn build_search_terms(terms: &str) -> String {
    format!("%{}%", terms.replace('%', "\\%").replace('_', "\\_"))
}

/// POST /api/v4/teams/{team_id}/posts/search - Search posts in team
pub(super) async fn search_team_posts(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::PostListWithSearchMatches>> {
    let team_id = parse_mm_or_uuid(&team_id)
        .ok_or_else(|| AppError::BadRequest("Invalid team_id".to_string()))?;

    let input: SearchPostsRequest = parse_body(&headers, &body, "Invalid search body")?;

    let _: crate::models::TeamMember =
        sqlx::query_as("SELECT * FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this team".to_string()))?;

    let (limit, offset) = compute_limit_and_offset(input.page, input.per_page);
    let search_terms = build_search_terms(&input.terms);

    let posts: Vec<crate::models::post::PostResponse> = sqlx::query_as(
        r#"
        SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
               p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
               p.reply_count::int8 as reply_count,
               p.last_reply_at, p.seq,
               u.username, u.avatar_url, u.email
        FROM posts p
        LEFT JOIN users u ON p.user_id = u.id
        JOIN channels c ON p.channel_id = c.id
        JOIN channel_members cm ON cm.channel_id = c.id AND cm.user_id = $4
        WHERE c.team_id = $1
          AND p.message ILIKE $2
          AND p.deleted_at IS NULL
        ORDER BY p.created_at DESC
        LIMIT $3 OFFSET $5
        "#,
    )
    .bind(team_id)
    .bind(&search_terms)
    .bind(limit)
    .bind(auth.user_id)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let mut posts = posts;
    if !posts.is_empty() {
        let _ = crate::services::posts::populate_files(&state, &mut posts).await;
    }

    let (order, posts_map) = crate::api::v4::posts::build_mm_posts_map(&state, posts).await?;
    let mut matches_map = std::collections::HashMap::new();
    for id in &order {
        matches_map.insert(id.clone(), Vec::new());
    }

    Ok(Json(mm::PostListWithSearchMatches {
        order,
        posts: posts_map,
        matches: Some(matches_map),
        next_post_id: String::new(),
        prev_post_id: String::new(),
    }))
}

/// POST /api/v4/posts/search - Search posts across all teams
pub(super) async fn search_posts_all_teams(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<Json<mm::PostListWithSearchMatches>> {
    let input: SearchPostsRequest = parse_body(&headers, &body, "Invalid search body")?;

    let (limit, offset) = compute_limit_and_offset(input.page, input.per_page);
    let search_terms = build_search_terms(&input.terms);

    let posts: Vec<crate::models::post::PostResponse> = sqlx::query_as(
        r#"
        SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
               p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
               p.reply_count::int8 as reply_count,
               p.last_reply_at, p.seq,
               u.username, u.avatar_url, u.email
        FROM posts p
        LEFT JOIN users u ON p.user_id = u.id
        JOIN channel_members cm ON cm.channel_id = p.channel_id AND cm.user_id = $2
        WHERE p.message ILIKE $1
          AND p.deleted_at IS NULL
        ORDER BY p.created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(&search_terms)
    .bind(auth.user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let mut posts = posts;
    if !posts.is_empty() {
        let _ = crate::services::posts::populate_files(&state, &mut posts).await;
    }

    let (order, posts_map) = crate::api::v4::posts::build_mm_posts_map(&state, posts).await?;
    let mut matches_map = std::collections::HashMap::new();
    for id in &order {
        matches_map.insert(id.clone(), Vec::new());
    }

    Ok(Json(mm::PostListWithSearchMatches {
        order,
        posts: posts_map,
        matches: Some(matches_map),
        next_post_id: String::new(),
        prev_post_id: String::new(),
    }))
}

#[cfg(test)]
mod tests {
    use super::{build_search_terms, compute_limit_and_offset};

    #[test]
    fn clamps_search_pagination() {
        assert_eq!(compute_limit_and_offset(-1, 999), (200, 0));
        assert_eq!(compute_limit_and_offset(2, 0), (1, 2));
    }

    #[test]
    fn escapes_like_wildcards() {
        let terms = build_search_terms("foo%bar_baz");
        assert_eq!(terms, "%foo\\%bar\\_baz%");
    }
}
