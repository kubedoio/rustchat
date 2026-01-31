use axum::{
    extract::{State, Query},
    routing::{get, post},
    Json, Router,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/hooks/incoming", get(list_incoming_hooks).post(create_incoming_hook))
        .route("/hooks/incoming/{hook_id}", get(get_incoming_hook).put(update_incoming_hook).delete(delete_incoming_hook))
        .route("/hooks/outgoing", get(list_outgoing_hooks).post(create_outgoing_hook))
        .route("/hooks/outgoing/{hook_id}", get(get_outgoing_hook).put(update_outgoing_hook).delete(delete_outgoing_hook))
        .route("/hooks/outgoing/{hook_id}/regen_token", post(regen_outgoing_hook_token))
        // Public incoming webhook endpoint (no auth required)
        .route("/hooks/{token}", post(execute_incoming_hook))
}
use axum::extract::Path;
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};
use crate::models::{IncomingWebhook, OutgoingWebhook};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct CreateIncomingRequest {
    pub channel_id: String,
    pub display_name: String,
    pub description: String,
}

#[derive(serde::Deserialize)]
pub struct CreateOutgoingRequest {
    pub team_id: String,
    pub channel_id: Option<String>,
    pub display_name: String,
    pub description: String,
    pub trigger_words: Vec<String>,
    pub trigger_when: i32,
    pub callback_urls: Vec<String>,
    pub content_type: String,
}

#[derive(serde::Deserialize)]
pub struct HooksQuery {
    pub team_id: Option<String>,
    pub channel_id: Option<String>,
    #[serde(default)]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_per_page() -> i64 {
    50
}

pub async fn create_incoming_hook(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<CreateIncomingRequest>,
) -> ApiResult<Json<mm::IncomingWebhook>> {
    let channel_id = parse_mm_or_uuid(&input.channel_id)
        .ok_or_else(|| AppError::Validation("Invalid channel_id".to_string()))?;

    // Get team_id for the channel
    let team_id: Uuid = sqlx::query_scalar("SELECT team_id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_one(&state.db)
        .await?;

    let hook: IncomingWebhook = sqlx::query_as(
        r#"
        INSERT INTO incoming_webhooks (team_id, channel_id, creator_id, display_name, description, token, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#
    )
    .bind(team_id)
    .bind(channel_id)
    .bind(auth.user_id)
    .bind(&input.display_name)
    .bind(&input.description)
    .bind(Uuid::new_v4().to_string()) // In-situ token generation
    .bind(true)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(map_incoming_hook(hook)))
}

pub async fn list_incoming_hooks(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Query(query): Query<HooksQuery>,
) -> ApiResult<Json<Vec<mm::IncomingWebhook>>> {
    let mut sql = "SELECT * FROM incoming_webhooks WHERE 1=1".to_string();
    if let Some(ref tid_str) = query.team_id {
        if let Some(tid) = parse_mm_or_uuid(tid_str) {
            sql.push_str(&format!(" AND team_id = '{}'", tid));
        }
    }
    
    sql.push_str(&format!(" LIMIT {} OFFSET {}", query.per_page, query.page * query.per_page));

    let hooks: Vec<IncomingWebhook> = sqlx::query_as(&sql)
        .fetch_all(&state.db)
        .await?;

    Ok(Json(hooks.into_iter().map(map_incoming_hook).collect()))
}

pub async fn create_outgoing_hook(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<CreateOutgoingRequest>,
) -> ApiResult<Json<mm::OutgoingWebhook>> {
    let team_id = parse_mm_or_uuid(&input.team_id)
        .ok_or_else(|| AppError::Validation("Invalid team_id".to_string()))?;

    let channel_id = input.channel_id.and_then(|id| parse_mm_or_uuid(&id));

    let hook: OutgoingWebhook = sqlx::query_as(
        r#"
        INSERT INTO outgoing_webhooks (team_id, channel_id, creator_id, display_name, description, trigger_words, trigger_when, callback_urls, content_type, token, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING *
        "#
    )
    .bind(team_id)
    .bind(channel_id)
    .bind(auth.user_id)
    .bind(&input.display_name)
    .bind(&input.description)
    .bind(&input.trigger_words)
    .bind("first_word") // Simplified mapping for trigger_when
    .bind(&input.callback_urls)
    .bind(&input.content_type)
    .bind(Uuid::new_v4().to_string())
    .bind(true)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(map_outgoing_hook(hook)))
}

pub async fn list_outgoing_hooks(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Query(query): Query<HooksQuery>,
) -> ApiResult<Json<Vec<mm::OutgoingWebhook>>> {
    let mut sql = "SELECT * FROM outgoing_webhooks WHERE 1=1".to_string();
    if let Some(ref tid_str) = query.team_id {
        if let Some(tid) = parse_mm_or_uuid(tid_str) {
            sql.push_str(&format!(" AND team_id = '{}'", tid));
        }
    }
    
    sql.push_str(&format!(" LIMIT {} OFFSET {}", query.per_page, query.page * query.per_page));

    let hooks: Vec<OutgoingWebhook> = sqlx::query_as(&sql)
        .fetch_all(&state.db)
        .await?;

    Ok(Json(hooks.into_iter().map(map_outgoing_hook).collect()))
}

fn map_incoming_hook(h: IncomingWebhook) -> mm::IncomingWebhook {
    mm::IncomingWebhook {
        id: encode_mm_id(h.id),
        create_at: h.created_at.timestamp_millis(),
        update_at: h.updated_at.timestamp_millis(),
        delete_at: 0,
        user_id: encode_mm_id(h.creator_id),
        channel_id: encode_mm_id(h.channel_id),
        team_id: encode_mm_id(h.team_id),
        display_name: h.display_name.unwrap_or_default(),
        description: h.description.unwrap_or_default(),
    }
}

fn map_outgoing_hook(h: OutgoingWebhook) -> mm::OutgoingWebhook {
    mm::OutgoingWebhook {
        id: encode_mm_id(h.id),
        create_at: h.created_at.timestamp_millis(),
        update_at: h.updated_at.timestamp_millis(),
        delete_at: 0,
        creator_id: encode_mm_id(h.creator_id),
        channel_id: h.channel_id.map(encode_mm_id).unwrap_or_default(),
        team_id: encode_mm_id(h.team_id),
        trigger_words: h.trigger_words,
        trigger_when: 0,
        callback_urls: h.callback_urls,
        display_name: h.display_name.unwrap_or_default(),
        description: h.description.unwrap_or_default(),
        content_type: h.content_type.unwrap_or_default(),
    }
}

async fn get_incoming_hook(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_hook_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn update_incoming_hook(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_hook_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn delete_incoming_hook(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_hook_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn get_outgoing_hook(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_hook_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn update_outgoing_hook(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_hook_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn delete_outgoing_hook(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_hook_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn regen_outgoing_hook_token(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
    Path(_hook_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// Public endpoint for executing incoming webhooks (no auth required)
async fn execute_incoming_hook(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(payload): Json<crate::models::WebhookPayload>,
) -> ApiResult<Json<serde_json::Value>> {
    crate::services::webhooks::execute_incoming_webhook(&state, &token, payload).await?;
    Ok(Json(serde_json::json!({"status": "OK"})))
}
