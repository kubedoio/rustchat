use axum::{
    extract::{Path, State, Query},
    Json,
};
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};
use crate::services::posts::{self, PostsQuery};

#[derive(serde::Deserialize)]
pub struct CreatePostRequest {
    pub channel_id: String,
    pub message: String,
    #[serde(default)]
    pub root_id: String,
    #[serde(default)]
    pub file_ids: Vec<String>,
    #[serde(default)]
    pub props: serde_json::Value,
    #[serde(default)]
    pub pending_post_id: String,
}

#[derive(serde::Deserialize)]
pub struct GetPostsQuery {
    #[serde(default)]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    #[serde(default)]
    pub since: Option<i64>,
    #[serde(default)]
    pub before: Option<String>,
    #[serde(default)]
    pub after: Option<String>,
}

fn default_per_page() -> i64 {
    60
}

pub async fn create_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<CreatePostRequest>,
) -> ApiResult<Json<mm::Post>> {
    let channel_id = parse_mm_or_uuid(&input.channel_id)
        .ok_or_else(|| AppError::Validation("Invalid channel_id".to_string()))?;

    let root_post_id = if !input.root_id.is_empty() {
        Some(
            parse_mm_or_uuid(&input.root_id)
                .ok_or_else(|| AppError::Validation("Invalid root_id".to_string()))?,
        )
    } else {
        None
    };

    let file_ids = input
        .file_ids
        .iter()
        .filter_map(|id| parse_mm_or_uuid(id))
        .collect();

    let create_payload = crate::models::CreatePost {
        message: input.message,
        root_post_id,
        props: Some(input.props),
        file_ids,
    };

    let client_msg_id = if !input.pending_post_id.is_empty() {
        Some(input.pending_post_id)
    } else {
        None
    };

    let post_resp = posts::create_post(
        &state,
        auth.user_id,
        channel_id,
        create_payload,
        client_msg_id,
    )
    .await?;

    Ok(Json(post_resp.into()))
}

pub async fn get_posts_for_channel(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(channel_id_str): Path<String>,
    Query(query): Query<GetPostsQuery>,
) -> ApiResult<Json<mm::PostList>> {
    let channel_id = parse_mm_or_uuid(&channel_id_str)
        .ok_or_else(|| AppError::Validation("Invalid channel_id".to_string()))?;

    let service_query = PostsQuery {
        page: query.page,
        per_page: query.per_page,
        since: query.since,
        before: query.before.and_then(|id| parse_mm_or_uuid(&id)),
        after: query.after.and_then(|id| parse_mm_or_uuid(&id)),
    };

    let (posts, _total) = posts::get_posts(
        &state,
        channel_id,
        service_query,
    )
    .await?;

    let mut mm_posts = std::collections::HashMap::new();
    let mut order = Vec::new();

    for p in posts {
        let id = encode_mm_id(p.id);
        order.push(id.clone());
        mm_posts.insert(id, p.into());
    }

    Ok(Json(mm::PostList {
        order,
        posts: mm_posts,
        next_post_id: "".to_string(),
        prev_post_id: "".to_string(),
    }))
}

pub async fn get_post(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(post_id_str): Path<String>,
) -> ApiResult<Json<mm::Post>> {
    let post_id = parse_mm_or_uuid(&post_id_str)
        .ok_or_else(|| AppError::Validation("Invalid post_id".to_string()))?;

    let post = posts::get_post_by_id(&state, post_id).await?;
    Ok(Json(post.into()))
}
