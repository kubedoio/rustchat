//! Agent runtime service
//!
//! Triggers agent responses when AI agents are mentioned in posts.

use std::sync::Arc;

use dashmap::DashMap;
use regex::Regex;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::models as mm;
use crate::models::{EntityType, PostResponse};
use crate::realtime::{EventType, WsBroadcast, WsEnvelope, WsHub};
use crate::repositories::{AgentRepository, PostRepository, UserRepository};
use crate::services::agent_memory::AgentMemoryService;
use crate::services::knowledge::embedder::Embedder;
use crate::services::knowledge::vector_store::VectorStore;
use crate::services::llm::{ChatMessage, CompletionRequest, ProviderRegistry};

pub struct AgentRuntime {
    db: sqlx::PgPool,
    ws_hub: Arc<WsHub>,
    provider_registry: Arc<ProviderRegistry>,
    embedder: Option<Arc<dyn Embedder>>,
    vector_store: Option<Arc<dyn VectorStore>>,
    semaphores: DashMap<Uuid, Arc<Semaphore>>,
}

impl AgentRuntime {
    pub fn new(
        db: sqlx::PgPool,
        ws_hub: Arc<WsHub>,
        provider_registry: Arc<ProviderRegistry>,
        embedder: Option<Arc<dyn Embedder>>,
        vector_store: Option<Arc<dyn VectorStore>>,
    ) -> Self {
        Self {
            db,
            ws_hub,
            provider_registry,
            embedder,
            vector_store,
            semaphores: DashMap::new(),
        }
    }

    #[tracing::instrument(skip(self, post), fields(post_id = %post.id, channel_id = %channel_id))]
    pub async fn handle_post_created(
        &self,
        post: &PostResponse,
        channel_id: Uuid,
    ) -> ApiResult<()> {
        let mentions = parse_mentions(&post.message);
        if mentions.is_empty() {
            return Ok(());
        }

        // Find which mentioned users are active agents
        let user_repo = UserRepository::new(&self.db);
        let users = user_repo
            .get_by_usernames(&mentions)
            .await
            .map_err(AppError::Database)?;

        let agent_user_ids: Vec<Uuid> = users
            .into_iter()
            .filter(|u| u.entity_type == EntityType::Agent && u.is_active && u.deleted_at.is_none())
            .map(|u| u.id)
            .collect();

        if agent_user_ids.is_empty() {
            return Ok(());
        }

        let agent_repo = AgentRepository::new(&self.db);

        for agent_user_id in agent_user_ids {
            // Check channel settings
            let settings = agent_repo
                .get_channel_settings(agent_user_id, channel_id)
                .await
                .map_err(AppError::Database)?;

            let is_active_in_channel = match &settings {
                Some(s) => s.is_active,
                None => false,
            };

            if !is_active_in_channel {
                continue;
            }

            // Get or create per-agent semaphore
            let semaphore = self
                .semaphores
                .entry(agent_user_id)
                .or_insert_with(|| Arc::new(Semaphore::new(1)))
                .clone();

            let db = self.db.clone();
            let ws_hub = self.ws_hub.clone();
            let provider_registry = self.provider_registry.clone();
            let embedder = self.embedder.clone();
            let vector_store = self.vector_store.clone();
            let post_clone = post.clone();

            tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            agent_id = %agent_user_id,
                            "Agent semaphore closed"
                        );
                        return;
                    }
                };

                if let Err(e) = run_agent_response(
                    db,
                    ws_hub,
                    provider_registry,
                    embedder,
                    vector_store,
                    agent_user_id,
                    channel_id,
                    &post_clone,
                )
                .await
                {
                    tracing::error!(
                        error = %e,
                        agent_id = %agent_user_id,
                        "Agent response failed"
                    );
                }
            });
        }

        Ok(())
    }
}

async fn run_agent_response(
    db: sqlx::PgPool,
    ws_hub: Arc<WsHub>,
    provider_registry: Arc<ProviderRegistry>,
    embedder: Option<Arc<dyn Embedder>>,
    vector_store: Option<Arc<dyn VectorStore>>,
    agent_user_id: Uuid,
    channel_id: Uuid,
    trigger_post: &PostResponse,
) -> ApiResult<()> {
    let agent_repo = AgentRepository::new(&db);

    // Fetch agent config
    let config = agent_repo
        .get_config_by_user_id(agent_user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Agent config not found".to_string()))?;

    if !config.is_active {
        return Ok(());
    }

    let capabilities: crate::models::agent::AgentCapabilities =
        serde_json::from_value(config.capabilities.clone()).unwrap_or_default();

    if !capabilities.respond_to_mentions {
        return Ok(());
    }

    // Build context
    let memory_service = AgentMemoryService::new(db.clone());
    let memory_service = if let (Some(embedder), Some(vector_store)) = (&embedder, &vector_store) {
        memory_service.with_rag(embedder.clone(), vector_store.clone())
    } else {
        memory_service
    };

    let max_messages = config.max_context_messages;
    let use_rag = capabilities.use_rag;
    let query = &trigger_post.message;
    let mut messages = memory_service
        .build_context(agent_user_id, channel_id, max_messages, use_rag, query)
        .await?;

    // Add the trigger post as the last user message
    let trigger_content = format!(
        "{}: {}",
        trigger_post.username.as_deref().unwrap_or("unknown"),
        trigger_post.message
    );
    messages.push(ChatMessage::user(trigger_content, trigger_post.username.clone()));

    // Call LLM
    let provider = provider_registry
        .get(&config.provider)
        .map_err(|e| AppError::ExternalService(e.to_string()))?;

    let request = CompletionRequest {
        system_prompt: config.system_prompt.clone(),
        messages,
        model: config.model.clone(),
        temperature: config.temperature as f32,
        max_tokens: config.max_output_tokens as u32,
    };

    let response = match provider.complete(request).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!(
                error = %e,
                provider = %config.provider,
                "LLM completion failed"
            );
            return Ok(()); // Don't post if LLM fails
        }
    };

    if response.content.trim().is_empty() {
        return Ok(());
    }

    // Create post as agent user
    let post_repo = PostRepository::new(db.clone());
    let mut tx = db.begin().await.map_err(AppError::Database)?;
    let created_post = post_repo
        .create_post_in_tx(
            &mut tx,
            channel_id,
            agent_user_id,
            None, // root_post_id
            &response.content,
            serde_json::json!({
                "from_agent": true,
                "agent_id": agent_user_id.to_string(),
            }),
            &[], // file_ids
        )
        .await?;
    tx.commit().await.map_err(AppError::Database)?;

    // Build PostResponse for broadcasting
    let agent_user = UserRepository::new(&db)
        .get_by_id_unchecked(agent_user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Agent user not found".to_string()))?;

    let post_response = PostResponse {
        id: created_post.id,
        channel_id: created_post.channel_id,
        user_id: created_post.user_id,
        root_post_id: created_post.root_post_id,
        message: created_post.message,
        props: created_post.props,
        file_ids: created_post.file_ids,
        is_pinned: created_post.is_pinned,
        created_at: created_post.created_at,
        edited_at: created_post.edited_at,
        deleted_at: created_post.deleted_at,
        reply_count: created_post.reply_count,
        last_reply_at: created_post.last_reply_at,
        username: Some(agent_user.username.clone()),
        avatar_url: agent_user.avatar_url.clone(),
        email: Some(agent_user.email.clone()),
        is_bot: true,
        files: vec![],
        reactions: vec![],
        is_saved: false,
        client_msg_id: None,
        seq: created_post.seq,
    };

    // Broadcast via WebSocket
    let mm_post = mm::Post::from(post_response.clone());
    let broadcast = WsEnvelope::event(EventType::MessageCreated, mm_post, Some(channel_id))
        .with_broadcast(WsBroadcast {
            channel_id: Some(channel_id),
            team_id: None,
            user_id: None,
            exclude_user_id: None,
        });

    ws_hub.broadcast(broadcast).await;

    // Store memory
    if capabilities.use_memory {
        if let Err(e) = memory_service
            .store_turn(agent_user_id, channel_id, trigger_post.id, &response.content)
            .await
        {
            tracing::error!(error = %e, "Failed to store agent memory");
        }
    }

    Ok(())
}

/// Parse @mentions from a message, excluding code blocks and URLs.
fn parse_mentions(message: &str) -> Vec<String> {
    let mention_re = Regex::new(r"@([a-zA-Z0-9_\-\.]+)").expect("valid regex");
    let mut mentions = Vec::new();
    let mut in_code_block = false;

    for line in message.lines() {
        // Track fenced code blocks (```)
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        // Find mentions in this line, skipping inline code segments
        for mat in mention_re.find_iter(line) {
            let start = mat.start();
            let prefix = &line[..start];
            // Skip inline code: odd number of backticks before mention
            let backtick_count = prefix.matches('`').count();
            if backtick_count % 2 == 1 {
                continue;
            }

            let mention = mat.as_str()[1..].to_string(); // strip @
            if !mentions.contains(&mention) {
                mentions.push(mention);
            }
        }
    }

    mentions
}
