//! Agent REST API endpoints
//!
//! Provides CRUD operations for AI agent configurations, channel assignments,
//! memory management, and testing.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::AppState;
use crate::auth::api_key::{extract_prefix, generate_api_key, hash_api_key};
use crate::auth::AuthUser;
use crate::crypto;
use crate::error::{ApiResult, AppError};
use crate::models::agent::{
    AgentChannelSettings, AgentConfigResponse, AgentMemory, AgentSummary, CreateAgentRequest,
    TestAgentRequest, TestAgentResponse, UpdateAgentRequest,
};
use crate::models::agent_feedback::*;
use crate::models::agent_usage::{AgentDailyUsage, AgentUsageSummary};
use crate::models::validate_username_token;
use crate::repositories::agent_feedback_repository::AgentFeedbackRepository;
use crate::repositories::agent_repository::AgentRepository;
use crate::repositories::agent_usage_repository::AgentUsageRepository;
use crate::repositories::PostRepository;
use crate::services::llm::{ChatMessage, CompletionRequest, LlmProvider, OpenAiProvider};

// ------------------------------------------------------------------
// Router
// ------------------------------------------------------------------

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_agents).post(create_agent))
        .route(
            "/{id}",
            get(get_agent).put(update_agent).delete(delete_agent),
        )
        .route("/{id}/regenerate-key", post(regenerate_api_key))
        .route("/{id}/channels", get(list_agent_channels))
        .route(
            "/{id}/channels/{channel_id}",
            post(add_agent_to_channel).delete(remove_agent_from_channel),
        )
        .route("/{id}/memories", get(list_memories))
        .route("/{id}/memories/{memory_id}", delete(delete_memory))
        .route("/{id}/test", post(test_agent))
        .route(
            "/posts/{post_id}/feedback",
            post(submit_feedback)
                .get(get_feedback_summary)
                .delete(delete_own_feedback),
        )
        .route("/{id}/feedback-stats", get(get_agent_feedback_stats))
        .route("/{id}/analytics", get(get_agent_analytics))
        .route(
            "/{id}/knowledge-bases",
            get(super::knowledge::list_agent_knowledge_bases)
                .post(super::knowledge::assign_kb_to_agent),
        )
        .route(
            "/{id}/knowledge-bases/{kb_id}",
            delete(super::knowledge::unassign_kb_from_agent),
        )
}

// ------------------------------------------------------------------
// Authorization helpers
// ------------------------------------------------------------------

fn require_admin(auth: &AuthUser) -> ApiResult<()> {
    if !auth.has_role("system_admin") && !auth.has_role("org_admin") {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }
    Ok(())
}

fn require_admin_or_creator(auth: &AuthUser, created_by: Uuid) -> ApiResult<()> {
    if auth.has_role("system_admin") || auth.has_role("org_admin") || auth.user_id == created_by {
        return Ok(());
    }
    Err(AppError::Forbidden(
        "Only admins or the creator can access this agent".to_string(),
    ))
}

// ------------------------------------------------------------------
// Response types
// ------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct AgentDetailResponse {
    #[serde(flatten)]
    pub config: AgentConfigResponse,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegenerateKeyResponse {
    pub api_key: String,
}

// ------------------------------------------------------------------
// Handlers
// ------------------------------------------------------------------

/// List all agents (admin only)
async fn list_agents(
    auth: AuthUser,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<AgentSummary>>> {
    require_admin(&auth)?;

    let repo = AgentRepository::new(&state.db);
    let agents = repo.list_agent_summaries(None).await?;

    Ok(Json(agents))
}

/// Create a new agent (admin only)
async fn create_agent(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateAgentRequest>,
) -> ApiResult<Json<AgentConfigResponse>> {
    require_admin(&auth)?;

    // Validation
    validate_username(&req.username)?;
    validate_email(&req.email)?;
    check_duplicates(&state.db, &req.username, &req.email).await?;

    // Generate API key
    let generated_key = generate_api_key();
    let api_key_prefix = extract_prefix(&generated_key).map_err(|e| {
        tracing::error!("Failed to extract API key prefix: {}", e);
        AppError::Internal("Failed to generate API key".to_string())
    })?;
    let api_key_hash = hash_api_key(&generated_key).await.map_err(|e| {
        tracing::error!("Failed to hash API key: {}", e);
        AppError::Internal("Failed to generate API key".to_string())
    })?;

    // Encrypt API token if provided
    let api_token_encrypted = match req.api_token {
        Some(ref token) if !token.is_empty() => {
            Some(crypto::encrypt(token, &state.config.encryption_key)?)
        }
        _ => None,
    };

    let entity_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    // Insert user row for the agent
    sqlx::query(
        r#"
        INSERT INTO users (
            id, username, email, display_name,
            entity_type, api_key_hash, api_key_prefix,
            password_hash, is_bot, is_active, role, presence,
            auth_provider, auth_provider_id,
            notify_props, email_verified, email_verified_at, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4,
            'agent', $5, $6,
            NULL, TRUE, TRUE, 'member', 'offline',
            'agent', $1::text,
            '{}', TRUE, $7, $8, $9
        )
        "#,
    )
    .bind(entity_id)
    .bind(&req.username)
    .bind(&req.email)
    .bind(&req.display_name)
    .bind(&api_key_hash)
    .bind(&api_key_prefix)
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert agent user: {}", e);
        AppError::Database(e)
    })?;

    // Create agent config
    let capabilities = req.capabilities.unwrap_or_default();
    let repo = AgentRepository::new(&state.db);
    let config = repo
        .create_config(
            entity_id,
            &req.title,
            req.description.as_deref(),
            &req.system_prompt,
            &req.provider,
            &req.model,
            api_token_encrypted.as_deref(),
            req.temperature.unwrap_or(0.7),
            req.max_context_messages.unwrap_or(10),
            req.max_output_tokens.unwrap_or(1024),
            &capabilities,
            req.rag_enabled.unwrap_or(false),
            req.rag_top_k.unwrap_or(3),
            auth.user_id,
        )
        .await?;

    // Add to channels if specified
    if let Some(channel_ids) = req.channel_ids {
        for channel_id in channel_ids {
            repo.add_agent_to_channel(entity_id, channel_id, true, None, None)
                .await?;
            add_to_channel_members(&state.db, channel_id, entity_id).await?;
        }
    }

    tracing::info!(
        agent_id = %config.id,
        user_id = %entity_id,
        username = %req.username,
        admin_id = %auth.user_id,
        "Agent created successfully"
    );

    Ok(Json(config.into()))
}

/// Get agent config by ID (admin or creator)
async fn get_agent(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AgentDetailResponse>> {
    let repo = AgentRepository::new(&state.db);
    let config = repo
        .get_config_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    require_admin_or_creator(&auth, config.created_by)?;

    // Join with users table for details
    let row: (String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT username, display_name, avatar_url FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(config.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AppError::NotFound("Agent user not found".to_string()))?;

    Ok(Json(AgentDetailResponse {
        config: config.into(),
        username: row.0,
        display_name: row.1,
        avatar_url: row.2,
    }))
}

/// Update agent config (admin or creator)
async fn update_agent(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAgentRequest>,
) -> ApiResult<Json<AgentConfigResponse>> {
    let repo = AgentRepository::new(&state.db);
    let existing = repo
        .get_config_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    require_admin_or_creator(&auth, existing.created_by)?;

    // Encrypt API token if provided
    let encrypted_token_string;
    let api_token_encrypted: Option<Option<&str>> = match req.api_token {
        Some(ref token) if !token.is_empty() => {
            encrypted_token_string = crypto::encrypt(token, &state.config.encryption_key)?;
            Some(Some(&encrypted_token_string))
        }
        Some(_) => Some(None), // explicitly clear
        None => None,          // don't update
    };

    let capabilities_ref = req.capabilities.as_ref();

    let updated = repo
        .update_config(
            id,
            req.title.as_deref(),
            req.description.as_deref().map(Some),
            req.system_prompt.as_deref(),
            req.provider.as_deref(),
            req.model.as_deref(),
            api_token_encrypted,
            req.temperature,
            req.max_context_messages,
            req.max_output_tokens,
            capabilities_ref,
            req.rag_enabled,
            req.rag_top_k,
            req.is_active,
        )
        .await?;

    Ok(Json(updated.into()))
}

/// Soft-delete an agent (admin only)
async fn delete_agent(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let repo = AgentRepository::new(&state.db);
    let config = repo
        .get_config_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    repo.delete_agent(config.user_id, auth.user_id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Regenerate API key for an agent (admin only)
async fn regenerate_api_key(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RegenerateKeyResponse>> {
    require_admin(&auth)?;

    let repo = AgentRepository::new(&state.db);
    let config = repo
        .get_config_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    let generated_key = generate_api_key();
    let api_key_prefix = extract_prefix(&generated_key).map_err(|e| {
        tracing::error!("Failed to extract API key prefix: {}", e);
        AppError::Internal("Failed to generate API key".to_string())
    })?;
    let api_key_hash = hash_api_key(&generated_key).await.map_err(|e| {
        tracing::error!("Failed to hash API key: {}", e);
        AppError::Internal("Failed to generate API key".to_string())
    })?;

    sqlx::query(
        "UPDATE users SET api_key_hash = $1, api_key_prefix = $2, updated_at = NOW() WHERE id = $3",
    )
    .bind(&api_key_hash)
    .bind(&api_key_prefix)
    .bind(config.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(RegenerateKeyResponse {
        api_key: generated_key,
    }))
}

/// List channels an agent is assigned to
async fn list_agent_channels(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<AgentChannelSettings>>> {
    let repo = AgentRepository::new(&state.db);
    let config = repo
        .get_config_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    require_admin_or_creator(&auth, config.created_by)?;

    let channels = repo.list_agent_channels(config.user_id).await?;
    Ok(Json(channels))
}

/// Add an agent to a channel
async fn add_agent_to_channel(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, channel_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<AgentChannelSettings>> {
    require_admin(&auth)?;

    let repo = AgentRepository::new(&state.db);
    let config = repo
        .get_config_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    let settings = repo
        .add_agent_to_channel(config.user_id, channel_id, true, None, None)
        .await?;
    add_to_channel_members(&state.db, channel_id, config.user_id).await?;

    Ok(Json(settings))
}

/// Remove an agent from a channel
async fn remove_agent_from_channel(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, channel_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;

    let repo = AgentRepository::new(&state.db);
    let config = repo
        .get_config_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    repo.remove_agent_from_channel(config.user_id, channel_id)
        .await?;

    sqlx::query("DELETE FROM channel_members WHERE channel_id = $1 AND user_id = $2")
        .bind(channel_id)
        .bind(config.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// List memories for an agent (admin or creator)
async fn list_memories(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<AgentMemory>>> {
    let repo = AgentRepository::new(&state.db);
    let config = repo
        .get_config_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    require_admin_or_creator(&auth, config.created_by)?;

    let memories: Vec<AgentMemory> = sqlx::query_as(
        r#"
        SELECT * FROM agent_memories
        WHERE agent_id = $1
        ORDER BY importance_score DESC, created_at DESC
        LIMIT 100
        "#,
    )
    .bind(config.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(memories))
}

/// Delete a specific memory entry
async fn delete_memory(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, memory_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = AgentRepository::new(&state.db);
    let config = repo
        .get_config_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    require_admin_or_creator(&auth, config.created_by)?;

    repo.delete_memory(memory_id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Test an agent with a sample message (admin only)
async fn test_agent(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<TestAgentRequest>,
) -> ApiResult<Json<TestAgentResponse>> {
    require_admin(&auth)?;

    let repo = AgentRepository::new(&state.db);
    let config = repo
        .get_config_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    // Decrypt API token
    let api_token = match config.api_token_encrypted {
        Some(ref encrypted) => crypto::decrypt(encrypted, &state.config.encryption_key)?,
        None => {
            return Err(AppError::BadRequest(
                "Agent does not have an API token configured".to_string(),
            ));
        }
    };

    // Build LLM provider (OpenAI-compatible)
    let provider = OpenAiProvider::new(&api_token)
        .map_err(|e| AppError::Internal(format!("Failed to initialize LLM provider: {}", e)))?;

    // Cap test calls to prevent runaway token spend
    let test_max_tokens = std::cmp::min(config.max_output_tokens as u32, 500);

    let completion_req = CompletionRequest {
        system_prompt: config.system_prompt,
        messages: vec![ChatMessage::user(req.message, None)],
        model: config.model,
        temperature: config.temperature as f32,
        max_tokens: test_max_tokens,
    };

    let response = provider
        .complete(completion_req)
        .await
        .map_err(|e| AppError::ExternalService(format!("LLM request failed: {}", e)))?;

    Ok(Json(TestAgentResponse {
        response: response.content,
        provider: response.provider,
        model: response.model,
        latency_ms: response.latency_ms,
    }))
}

async fn submit_feedback(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    Json(req): Json<CreateFeedbackRequest>,
) -> ApiResult<(StatusCode, Json<AgentMessageFeedback>)> {
    if req.feedback_type != "positive" && req.feedback_type != "negative" {
        return Err(AppError::BadRequest(
            "feedback_type must be 'positive' or 'negative'".to_string(),
        ));
    }

    // Load post and verify membership + agent origin
    let post_repo = PostRepository::new(state.db.clone());
    let post = post_repo
        .get_post_by_id(post_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;
    post_repo
        .require_channel_membership(post.channel_id, auth.user_id)
        .await?;
    let is_agent_post = post
        .props
        .get("from_agent")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !is_agent_post {
        return Err(AppError::BadRequest(
            "Feedback is only allowed on agent posts".to_string(),
        ));
    }

    let repo = AgentFeedbackRepository::new(&state.db);
    let feedback = repo
        .create_feedback(
            post_id,
            auth.user_id,
            &req.feedback_type,
            req.comment.as_deref(),
        )
        .await
        .map_err(AppError::Database)?;

    Ok((StatusCode::CREATED, Json(feedback)))
}

async fn get_feedback_summary(
    auth: AuthUser,
    Path(post_id): Path<Uuid>,
    State(state): State<AppState>,
) -> ApiResult<Json<FeedbackSummary>> {
    // Load post and verify membership
    let post_repo = PostRepository::new(state.db.clone());
    let post = post_repo
        .get_post_by_id(post_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;
    post_repo
        .require_channel_membership(post.channel_id, auth.user_id)
        .await?;

    let repo = AgentFeedbackRepository::new(&state.db);
    let summary = repo
        .get_feedback_summary(post_id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(summary))
}

async fn delete_own_feedback(
    auth: AuthUser,
    Path(post_id): Path<Uuid>,
    State(state): State<AppState>,
) -> ApiResult<StatusCode> {
    let repo = AgentFeedbackRepository::new(&state.db);
    repo.delete_feedback(post_id, auth.user_id)
        .await
        .map_err(AppError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn get_agent_feedback_stats(
    auth: AuthUser,
    Path(agent_id): Path<Uuid>,
    State(state): State<AppState>,
) -> ApiResult<Json<AgentFeedbackStats>> {
    require_admin(&auth)?;

    let repo = AgentFeedbackRepository::new(&state.db);
    let stats = repo
        .get_agent_feedback_stats(agent_id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(stats))
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AgentAnalyticsResponse {
    pub summary: AgentUsageSummary,
    pub daily_usage: Vec<AgentDailyUsage>,
    pub feedback_stats: AgentFeedbackStats,
}

async fn get_agent_analytics(
    auth: AuthUser,
    Path(agent_id): Path<Uuid>,
    Query(params): Query<AnalyticsQuery>,
    State(state): State<AppState>,
) -> ApiResult<Json<AgentAnalyticsResponse>> {
    require_admin(&auth)?;

    let days = params.days.unwrap_or(7);
    let since = Utc::now() - Duration::days(days);

    let usage_repo = AgentUsageRepository::new(&state.db);
    let feedback_repo = AgentFeedbackRepository::new(&state.db);

    let summary = usage_repo
        .get_summary(agent_id, since)
        .await
        .map_err(AppError::Database)?;
    let daily = usage_repo
        .get_daily_usage(agent_id, since)
        .await
        .map_err(AppError::Database)?;
    let feedback = feedback_repo
        .get_agent_feedback_stats(agent_id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(AgentAnalyticsResponse {
        summary,
        daily_usage: daily,
        feedback_stats: feedback,
    }))
}

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

async fn add_to_channel_members(db: &PgPool, channel_id: Uuid, user_id: Uuid) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, 'member') ON CONFLICT DO NOTHING"
    )
    .bind(channel_id)
    .bind(user_id)
    .execute(db)
    .await?;
    Ok(())
}

fn validate_username(username: &str) -> ApiResult<()> {
    validate_username_token(username).map_err(|message| AppError::BadRequest(message.to_string()))
}

fn validate_email(email: &str) -> ApiResult<()> {
    if email.is_empty() {
        return Err(AppError::BadRequest("Email cannot be empty".to_string()));
    }
    if !email.contains('@') {
        return Err(AppError::BadRequest("Invalid email format".to_string()));
    }
    if email.len() > 255 {
        return Err(AppError::BadRequest(
            "Email cannot exceed 255 characters".to_string(),
        ));
    }
    Ok(())
}

async fn check_duplicates(db: &PgPool, username: &str, email: &str) -> ApiResult<()> {
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT username FROM users WHERE username = $1 OR email = $2 LIMIT 1")
            .bind(username)
            .bind(email)
            .fetch_optional(db)
            .await?;

    if let Some((existing_username,)) = existing {
        if existing_username == username {
            return Err(AppError::Conflict("Username already exists".to_string()));
        } else {
            return Err(AppError::Conflict("Email already exists".to_string()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn router_builds_with_axum_v08_route_syntax() {
        let _ = super::router();
    }
}
