//! Agent runtime service
//!
//! Triggers agent responses when AI agents are mentioned in posts.

use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use futures_util::StreamExt;
use regex::Regex;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::models as mm;
use crate::models::{EntityType, PostResponse};
use crate::realtime::{EventType, WsBroadcast, WsEnvelope, WsHub};
use crate::repositories::agent_usage_repository::AgentUsageRepository;
use crate::repositories::{AgentRepository, PostRepository, UserRepository};
use crate::services::agent_memory::AgentMemoryService;
use crate::services::agent_rate_limiter::{AgentRateLimiter, RateLimitResult};
use crate::services::knowledge::embedder::Embedder;
use crate::services::knowledge::vector_store::VectorStore;
use crate::services::llm::{ChatMessage, CompletionRequest, LlmError, ProviderRegistry};
use crate::services::tools::executor::ToolExecutor;
use crate::services::tools::registry::ToolRegistry;

pub struct AgentRuntime {
    db: sqlx::PgPool,
    ws_hub: Arc<WsHub>,
    provider_registry: Arc<ProviderRegistry>,
    embedder: Option<Arc<dyn Embedder>>,
    vector_store: Option<Arc<dyn VectorStore>>,
    tool_registry: Option<Arc<ToolRegistry>>,
    rate_limiter: Arc<AgentRateLimiter>,
    semaphores: DashMap<Uuid, Arc<Semaphore>>,
}

impl AgentRuntime {
    pub fn new(
        db: sqlx::PgPool,
        ws_hub: Arc<WsHub>,
        provider_registry: Arc<ProviderRegistry>,
        embedder: Option<Arc<dyn Embedder>>,
        vector_store: Option<Arc<dyn VectorStore>>,
        tool_registry: Option<Arc<ToolRegistry>>,
    ) -> Self {
        let rate_limiter = Arc::new(AgentRateLimiter::new(10, 100_000));
        Self {
            db,
            ws_hub,
            provider_registry,
            embedder,
            vector_store,
            tool_registry,
            rate_limiter,
            semaphores: DashMap::new(),
        }
    }

    #[tracing::instrument(skip(self, post), fields(post_id = %post.id, channel_id = %channel_id))]
    pub async fn handle_post_created(
        &self,
        post: &PostResponse,
        channel_id: Uuid,
    ) -> ApiResult<()> {
        let agent_repo = AgentRepository::new(&self.db);
        let mut agent_user_ids: Vec<Uuid> = Vec::new();

        // 1. Respond to mentions
        let mentions = parse_mentions(&post.message);
        if !mentions.is_empty() {
            let user_repo = UserRepository::new(&self.db);
            let users = user_repo
                .get_by_usernames(&mentions)
                .await
                .map_err(AppError::Database)?;

            agent_user_ids.extend(
                users
                    .into_iter()
                    .filter(|u| {
                        u.entity_type == EntityType::Agent && u.is_active && u.deleted_at.is_none()
                    })
                    .map(|u| u.id),
            );
        }

        // 2. Respond to all messages (agents with respond_to_all in this channel)
        let channel_agents = agent_repo
            .list_channel_agents(channel_id)
            .await
            .map_err(AppError::Database)?;
        for config in channel_agents {
            let caps: crate::models::agent::AgentCapabilities =
                serde_json::from_value(config.capabilities).unwrap_or_default();
            if caps.respond_to_all {
                agent_user_ids.push(config.user_id);
            }
        }

        // Deduplicate
        agent_user_ids.sort_unstable();
        agent_user_ids.dedup();

        if agent_user_ids.is_empty() {
            return Ok(());
        }

        for agent_user_id in agent_user_ids {
            // Prevent agent from triggering itself (infinite loop)
            if post.user_id == agent_user_id {
                tracing::debug!(agent_id = %agent_user_id, "Skipping self-triggered agent response");
                continue;
            }

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

            // Check rate limits
            match self.rate_limiter.check_request(agent_user_id) {
                RateLimitResult::Allowed => {}
                RateLimitResult::Throttled { retry_after_secs } => {
                    tracing::warn!(
                        agent_id = %agent_user_id,
                        retry_after = retry_after_secs,
                        "Agent rate limited (requests)"
                    );
                    continue;
                }
            }

            match self.rate_limiter.check_tokens(agent_user_id) {
                RateLimitResult::Allowed => {}
                RateLimitResult::Throttled { retry_after_secs } => {
                    tracing::warn!(
                        agent_id = %agent_user_id,
                        retry_after = retry_after_secs,
                        "Agent rate limited (tokens)"
                    );
                    continue;
                }
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
            let tool_registry = self.tool_registry.clone();
            let rate_limiter = self.rate_limiter.clone();
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
                    tool_registry,
                    rate_limiter,
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

#[allow(clippy::too_many_arguments)]
async fn run_agent_response(
    db: sqlx::PgPool,
    ws_hub: Arc<WsHub>,
    provider_registry: Arc<ProviderRegistry>,
    embedder: Option<Arc<dyn Embedder>>,
    vector_store: Option<Arc<dyn VectorStore>>,
    tool_registry: Option<Arc<ToolRegistry>>,
    rate_limiter: Arc<AgentRateLimiter>,
    agent_user_id: Uuid,
    channel_id: Uuid,
    trigger_post: &PostResponse,
) -> ApiResult<()> {
    let result = async {
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
        let memory_service =
            if let (Some(embedder), Some(vector_store)) = (&embedder, &vector_store) {
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

        // Add the trigger post as the last user message (if not already present)
        let trigger_already_last = messages
            .last()
            .map(|m| {
                m.role == crate::services::llm::MessageRole::User
                    && m.content.contains(&trigger_post.message)
            })
            .unwrap_or(false);

        if !trigger_already_last {
            let trigger_content = format!(
                "{}: {}",
                trigger_post.username.as_deref().unwrap_or("unknown"),
                trigger_post.message
            );
            messages.push(ChatMessage::user(
                trigger_content,
                trigger_post.username.clone(),
            ));
        }

        // Estimate input tokens before messages are consumed
        let tokens_input = estimate_tokens(&messages) as i32;

        // Call LLM
        let start_time = Instant::now();
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

        let stream_post_id = Uuid::new_v4();
        let response_text;

        // Determine whether to use tool calling
        let has_tools = tool_registry
            .as_ref()
            .map(|r| !r.is_empty())
            .unwrap_or(false);

        if has_tools {
            // Tool calling requires the full response to parse <tool_call> blocks,
            // so we use the non-streaming complete() path.
            let executor = ToolExecutor::new(tool_registry.clone().unwrap());
            response_text = executor
                .execute_with_tools(provider.as_ref(), request)
                .await
                .map_err(|e| AppError::ExternalService(e.to_string()))?;
        } else {
            // Try streaming first
            match provider.complete_stream(request.clone()).await {
                Ok(mut stream) => {
                    let mut accumulated = String::new();
                    let mut last_broadcast = Instant::now();

                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(chunk) => {
                                accumulated.push_str(&chunk);

                                // Debounce: broadcast every 100ms or every 5 tokens
                                if last_broadcast.elapsed().as_millis() > 100 {
                                    let event = WsEnvelope {
                                        msg_type: "event".to_string(),
                                        event: "agent_stream_chunk".to_string(),
                                        seq: None,
                                        channel_id: Some(channel_id),
                                        data: serde_json::json!({
                                            "content": accumulated.clone(),
                                            "agent_id": agent_user_id,
                                            "post_id": stream_post_id,
                                        }),
                                        broadcast: Some(WsBroadcast {
                                            channel_id: Some(channel_id),
                                            team_id: None,
                                            user_id: None,
                                            exclude_user_id: Some(agent_user_id),
                                        }),
                                    };
                                    ws_hub.broadcast(event).await;
                                    last_broadcast = Instant::now();
                                }
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "Stream chunk error");
                                // Send error event
                                let event = WsEnvelope {
                                    msg_type: "event".to_string(),
                                    event: "agent_stream_error".to_string(),
                                    seq: None,
                                    channel_id: Some(channel_id),
                                    data: serde_json::json!({
                                        "message": format!("Streaming error: {}", e),
                                        "agent_id": agent_user_id,
                                        "post_id": stream_post_id,
                                    }),
                                    broadcast: Some(WsBroadcast {
                                        channel_id: Some(channel_id),
                                        team_id: None,
                                        user_id: None,
                                        exclude_user_id: Some(agent_user_id),
                                    }),
                                };
                                ws_hub.broadcast(event).await;
                                return Err(e.into());
                            }
                        }
                    }

                    // Stream complete — broadcast final accumulated content before complete event
                    if !accumulated.is_empty() {
                        let event = WsEnvelope {
                            msg_type: "event".to_string(),
                            event: "agent_stream_chunk".to_string(),
                            seq: None,
                            channel_id: Some(channel_id),
                            data: serde_json::json!({
                                "content": accumulated.clone(),
                                "agent_id": agent_user_id,
                                "post_id": stream_post_id,
                            }),
                            broadcast: Some(WsBroadcast {
                                channel_id: Some(channel_id),
                                team_id: None,
                                user_id: None,
                                exclude_user_id: Some(agent_user_id),
                            }),
                        };
                        ws_hub.broadcast(event).await;
                    }

                    response_text = accumulated;

                    // Send complete event
                    let event = WsEnvelope {
                        msg_type: "event".to_string(),
                        event: "agent_stream_complete".to_string(),
                        seq: None,
                        channel_id: Some(channel_id),
                        data: serde_json::json!({
                            "agent_id": agent_user_id,
                            "post_id": stream_post_id,
                        }),
                        broadcast: Some(WsBroadcast {
                            channel_id: Some(channel_id),
                            team_id: None,
                            user_id: None,
                            exclude_user_id: Some(agent_user_id),
                        }),
                    };
                    ws_hub.broadcast(event).await;
                }
                Err(LlmError::Config(_)) => {
                    // Streaming not supported, fall back to complete
                    let completion = provider.complete(request).await?;
                    response_text = completion.content;
                }
                Err(e) => return Err(e.into()),
            }
        }

        // Log usage
        let latency_ms = start_time.elapsed().as_millis() as i32;
        let tokens_output =
            estimate_tokens(&[ChatMessage::assistant(response_text.clone())]) as i32;

        let usage_repo = AgentUsageRepository::new(&db);
        if let Err(e) = usage_repo
            .log_usage(
                agent_user_id,
                channel_id,
                "mention",
                tokens_input,
                tokens_output,
                latency_ms,
                &config.model,
            )
            .await
        {
            tracing::error!(error = %e, "Failed to log agent usage");
        }

        // Record token usage for rate limiting
        let total_tokens = (tokens_input + tokens_output) as u32;
        rate_limiter.record_tokens(agent_user_id, total_tokens);

        if response_text.trim().is_empty() {
            return Ok(());
        }

        // Create post as agent user, reusing the stream placeholder ID so the
        // frontend can replace the temporary streaming message with the real one.
        let post_repo = PostRepository::new(db.clone());
        let mut tx = db.begin().await.map_err(AppError::Database)?;
        let created_post = post_repo
            .create_post_in_tx_with_id(
                &mut tx,
                stream_post_id,
                channel_id,
                agent_user_id,
                None, // root_post_id
                &response_text,
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
                .store_turn(agent_user_id, channel_id, trigger_post.id, &response_text)
                .await
            {
                tracing::error!(error = %e, "Failed to store agent memory");
            }
        }

        Ok(())
    }
    .await;

    if let Err(ref e) = result {
        tracing::error!(error = %e, agent_id = %agent_user_id, "Agent response failed");
        let event = WsEnvelope {
            msg_type: "event".to_string(),
            event: "agent_error".to_string(),
            seq: None,
            channel_id: Some(channel_id),
            data: serde_json::json!({
                "message": "Agent is temporarily unavailable. Please try again later.",
                "agent_id": agent_user_id,
            }),
            broadcast: Some(WsBroadcast {
                channel_id: Some(channel_id),
                team_id: None,
                user_id: None,
                exclude_user_id: Some(agent_user_id),
            }),
        };
        ws_hub.broadcast(event).await;
    }

    result
}

fn estimate_tokens(messages: &[ChatMessage]) -> usize {
    // Naive estimate: ~4 chars per token
    messages.iter().map(|m| m.content.len() / 4).sum()
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
