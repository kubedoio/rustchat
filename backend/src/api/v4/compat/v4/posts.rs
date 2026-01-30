use axum::{
    extract::{Path, State, Query},
    Json,
};
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};
use crate::services::posts::{self, PostsQuery};
use crate::models::post::Reaction;
use uuid::Uuid;
use crate::realtime::{EventType, WsBroadcast, WsEnvelope};

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

#[derive(serde::Deserialize)]
pub struct ReactionRequest {
    pub user_id: String,
    pub post_id: String,
    pub emoji_name: String,
}

pub async fn add_reaction(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<ReactionRequest>,
) -> ApiResult<Json<mm::Reaction>> {
    let input_user_id = parse_mm_or_uuid(&input.user_id)
        .ok_or_else(|| AppError::Validation("Invalid user_id".to_string()))?;
    
    if input_user_id != auth.user_id && input.user_id != "me" {
        return Err(AppError::Forbidden("Cannot react for other user".to_string()));
    }

    let post_id = parse_mm_or_uuid(&input.post_id)
        .ok_or_else(|| AppError::Validation("Invalid post_id".to_string()))?;

    let reaction: Reaction = sqlx::query_as(
        r#"
        INSERT INTO reactions (user_id, post_id, emoji_name)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, post_id, emoji_name) DO UPDATE SET emoji_name = $3
        RETURNING *
        "#
    )
    .bind(auth.user_id)
    .bind(post_id)
    .bind(&input.emoji_name)
    .fetch_one(&state.db)
    .await?;

    let channel_id: Uuid = sqlx::query_scalar("SELECT channel_id FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(&state.db)
        .await?;

    let broadcast = WsEnvelope::event(
        EventType::ReactionAdded,
        reaction.clone(),
        Some(channel_id),
    )
    .with_broadcast(WsBroadcast {
        channel_id: Some(channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(mm::Reaction {
        user_id: encode_mm_id(reaction.user_id),
        post_id: encode_mm_id(reaction.post_id),
        emoji_name: reaction.emoji_name,
        create_at: reaction.created_at.timestamp_millis(),
    }))
}

pub async fn remove_reaction(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((user_id_str, post_id_str, emoji_name)): Path<(String, String, String)>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let target_user_id = parse_mm_or_uuid(&user_id_str)
        .ok_or_else(|| AppError::Validation("Invalid user_id".to_string()))?;

    if target_user_id != auth.user_id && user_id_str != "me" {
        return Err(AppError::Forbidden("Cannot remove reaction for other user".to_string()));
    }

    let post_id = parse_mm_or_uuid(&post_id_str)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;

    let reaction: Option<Reaction> = sqlx::query_as(
        "SELECT * FROM reactions WHERE user_id = $1 AND post_id = $2 AND emoji_name = $3",
    )
    .bind(auth.user_id)
    .bind(post_id)
    .bind(&emoji_name)
    .fetch_optional(&state.db)
    .await?;

    if let Some(r) = reaction {
        sqlx::query("DELETE FROM reactions WHERE user_id = $1 AND post_id = $2 AND emoji_name = $3")
            .bind(auth.user_id)
            .bind(post_id)
            .bind(&emoji_name)
            .execute(&state.db)
            .await?;

        let channel_id: Uuid = sqlx::query_scalar("SELECT channel_id FROM posts WHERE id = $1")
            .bind(post_id)
            .fetch_one(&state.db)
            .await?;

        let broadcast = WsEnvelope::event(EventType::ReactionRemoved, r, Some(channel_id))
            .with_broadcast(WsBroadcast {
                channel_id: Some(channel_id),
                team_id: None,
                user_id: None,
                exclude_user_id: None,
            });
        state.ws_hub.broadcast(broadcast).await;
    }

    Ok(Json(serde_json::json!({"status": "OK"})))
}

pub async fn get_reactions(
    State(state): State<AppState>,
    Path(post_id_str): Path<String>,
) -> ApiResult<Json<Vec<mm::Reaction>>> {
    let post_id = parse_mm_or_uuid(&post_id_str)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;

    let reactions: Vec<Reaction> = sqlx::query_as("SELECT * FROM reactions WHERE post_id = $1")
        .bind(post_id)
        .fetch_all(&state.db)
        .await?;

    let mm_reactions = reactions.into_iter().map(|r| mm::Reaction {
        user_id: encode_mm_id(r.user_id),
        post_id: encode_mm_id(r.post_id),
        emoji_name: r.emoji_name,
        create_at: r.created_at.timestamp_millis(),
    }).collect();

    Ok(Json(mm_reactions))
}

#[derive(serde::Deserialize)]
pub struct CreateScheduledPostRequest {
    pub channel_id: String,
    pub message: String,
    #[serde(default)]
    pub root_id: String,
    #[serde(default)]
    pub props: serde_json::Value,
    #[serde(default)]
    pub file_ids: Vec<String>,
    pub scheduled_at: i64,
}



pub async fn list_scheduled_posts(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id_str): Path<String>,
) -> ApiResult<Json<Vec<mm::ScheduledPost>>> {
    let team_id = parse_mm_or_uuid(&team_id_str)
        .ok_or_else(|| AppError::Validation("Invalid team_id".to_string()))?;

    let rows: Vec<(Uuid, Uuid, Uuid, Option<Uuid>, String, serde_json::Value, Vec<Uuid>, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        r#"
        SELECT id, user_id, channel_id, root_id, message, props, file_ids, scheduled_at, created_at, updated_at
        FROM scheduled_posts
        WHERE user_id = $1 AND channel_id IN (SELECT id FROM channels WHERE team_id = $2)
        AND state = 'pending'
        "#
    )
    .bind(auth.user_id)
    .bind(team_id)
    .fetch_all(&state.db)
    .await?;

    let posts = rows.into_iter().map(|r| mm::ScheduledPost {
        id: encode_mm_id(r.0),
        user_id: encode_mm_id(r.1),
        channel_id: encode_mm_id(r.2),
        root_id: r.3.map(encode_mm_id).unwrap_or_default(),
        message: r.4,
        props: r.5,
        file_ids: r.6.into_iter().map(encode_mm_id).collect(),
        scheduled_at: r.7.timestamp_millis(),
        create_at: r.8.timestamp_millis(),
        update_at: r.9.timestamp_millis(),
    }).collect();

    Ok(Json(posts))
}

pub async fn create_scheduled_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<CreateScheduledPostRequest>,
) -> ApiResult<Json<mm::ScheduledPost>> {
    let channel_id = parse_mm_or_uuid(&input.channel_id)
        .ok_or_else(|| AppError::Validation("Invalid channel_id".to_string()))?;

    let root_id = if !input.root_id.is_empty() {
        Some(parse_mm_or_uuid(&input.root_id)
            .ok_or_else(|| AppError::Validation("Invalid root_id".to_string()))?)
    } else {
        None
    };

    let file_ids = input.file_ids.iter().filter_map(|id| parse_mm_or_uuid(id)).collect::<Vec<_>>();
    let scheduled_at = chrono::DateTime::from_timestamp_millis(input.scheduled_at)
        .ok_or_else(|| AppError::Validation("Invalid scheduled_at".to_string()))?;

    let row: (Uuid, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        r#"
        INSERT INTO scheduled_posts (user_id, channel_id, root_id, message, props, file_ids, scheduled_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, created_at, updated_at
        "#
    )
    .bind(auth.user_id)
    .bind(channel_id)
    .bind(root_id)
    .bind(&input.message)
    .bind(&input.props)
    .bind(&file_ids)
    .bind(scheduled_at)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(mm::ScheduledPost {
        id: encode_mm_id(row.0),
        user_id: encode_mm_id(auth.user_id),
        channel_id: input.channel_id,
        root_id: input.root_id,
        message: input.message,
        props: input.props,
        file_ids: input.file_ids,
        scheduled_at: input.scheduled_at,
        create_at: row.1.timestamp_millis(),
        update_at: row.2.timestamp_millis(),
    }))
}

#[derive(serde::Deserialize)]
pub struct EphemeralPostRequest {
    pub user_id: String,
    pub post: CreatePostRequest,
}

pub async fn create_ephemeral_post(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<EphemeralPostRequest>,
) -> ApiResult<Json<mm::Post>> {
    let target_user_id = parse_mm_or_uuid(&input.user_id)
        .ok_or_else(|| AppError::Validation("Invalid user_id".to_string()))?;

    if target_user_id != auth.user_id && input.user_id != "me" {
        return Err(AppError::Forbidden("Cannot send ephemeral post to others".to_string()));
    }

    let channel_id = parse_mm_or_uuid(&input.post.channel_id)
        .ok_or_else(|| AppError::Validation("Invalid channel_id".to_string()))?;

    // For ephemeral posts, we broadcast a temporary event via WebSocket 
    // without full database persistence (or as a transient record).
    // In this implementation, we just mock the response to satisfy the mobile app.
    
    let post_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();
    
    let ephemeral_post = mm::Post {
        id: encode_mm_id(post_id),
        create_at: now,
        update_at: now,
        delete_at: 0,
        edit_at: 0,
        user_id: encode_mm_id(auth.user_id),
        channel_id: input.post.channel_id,
        root_id: input.post.root_id,
        original_id: "".to_string(),
        message: input.post.message,
        post_type: "ephemeral".to_string(),
        props: input.post.props,
        hashtags: "".to_string(),
        file_ids: input.post.file_ids,
        pending_post_id: input.post.pending_post_id,
        metadata: None,
    };

    // Broadcast to the user only
    let broadcast = WsEnvelope::event(
        EventType::EphemeralMessage,
        ephemeral_post.clone(),
        Some(channel_id),
    )
    .with_broadcast(WsBroadcast {
        channel_id: Some(channel_id),
        team_id: None,
        user_id: Some(auth.user_id),
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(ephemeral_post))
}

#[derive(serde::Deserialize)]
pub struct PostReminderRequest {
    pub target_at: i64,
}

pub async fn set_post_reminder(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((user_id_str, post_id_str)): Path<(String, String)>,
    Json(input): Json<PostReminderRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let target_user_id = parse_mm_or_uuid(&user_id_str)
        .ok_or_else(|| AppError::Validation("Invalid user_id".to_string()))?;

    if target_user_id != auth.user_id && user_id_str != "me" {
        return Err(AppError::Forbidden("Cannot set reminder for others".to_string()));
    }

    let post_id = parse_mm_or_uuid(&post_id_str)
        .ok_or_else(|| AppError::Validation("Invalid post_id".to_string()))?;

    let target_at = chrono::DateTime::from_timestamp_millis(input.target_at)
        .ok_or_else(|| AppError::Validation("Invalid target_at".to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO post_reminders (user_id, post_id, target_at)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, post_id) DO UPDATE SET target_at = $3
        "#
    )
    .bind(auth.user_id)
    .bind(post_id)
    .bind(target_at)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}
