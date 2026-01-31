//! Webhook delivery service
//!
//! Handles incoming webhook execution (POST to create message) and
//! outgoing webhook triggers (send HTTP request on new posts).

use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::models::{IncomingWebhook, OutgoingWebhook, WebhookPayload, OutgoingWebhookPayload};
use crate::services::posts;
use uuid::Uuid;

/// Execute an incoming webhook - creates a post in the target channel
pub async fn execute_incoming_webhook(
    state: &AppState,
    token: &str,
    payload: WebhookPayload,
) -> ApiResult<()> {
    // 1. Find the webhook by token
    let hook: IncomingWebhook = sqlx::query_as(
        "SELECT * FROM incoming_webhooks WHERE token = $1 AND is_active = true"
    )
    .bind(token)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Webhook not found or inactive".to_string()))?;

    // 2. Get the bot user or creator as the poster
    let poster_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM users WHERE is_bot = true LIMIT 1"
    )
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(hook.creator_id);

    // 3. Build props with override info
    let mut props = payload.props.as_object().cloned().unwrap_or_default();
    if let Some(username) = &payload.username {
        props.insert("override_username".to_string(), serde_json::json!(username));
    }
    if let Some(icon) = &payload.icon_url {
        props.insert("override_icon_url".to_string(), serde_json::json!(icon));
    }
    props.insert("from_webhook".to_string(), serde_json::json!(true));
    props.insert("webhook_display_name".to_string(), serde_json::json!(hook.display_name));

    // 4. Create the post
    let input = crate::models::CreatePost {
        message: payload.text,
        root_post_id: None,
        file_ids: vec![],
        props: Some(serde_json::Value::Object(props)),
    };

    posts::create_post(state, poster_id, hook.channel_id, input, None).await?;

    Ok(())
}

/// Check for outgoing webhook triggers and execute them
pub async fn check_outgoing_triggers(
    state: &AppState,
    channel_id: Uuid,
    team_id: Uuid,
    user_id: Uuid,
    username: &str,
    channel_name: &str,
    message: &str,
) -> ApiResult<()> {
    // 1. Get words from message
    let first_word = message.split_whitespace().next().unwrap_or("");
    let message_lower = message.to_lowercase();

    // 2. Find matching outgoing webhooks
    let hooks: Vec<OutgoingWebhook> = sqlx::query_as(
        r#"
        SELECT * FROM outgoing_webhooks 
        WHERE is_active = true 
          AND team_id = $1
          AND (channel_id IS NULL OR channel_id = $2)
        "#
    )
    .bind(team_id)
    .bind(channel_id)
    .fetch_all(&state.db)
    .await?;

    for hook in hooks {
        let matched_word = hook.trigger_words.iter().find(|tw| {
            let tw_lower = tw.to_lowercase();
            match hook.trigger_when.as_str() {
                "first_word" => first_word.to_lowercase() == tw_lower,
                _ => message_lower.contains(&tw_lower), // "any" match
            }
        });

        if let Some(trigger_word) = matched_word {
            // Build payload
            let payload = OutgoingWebhookPayload {
                token: hook.token.clone(),
                team_id,
                channel_id,
                channel_name: channel_name.to_string(),
                user_id,
                user_name: username.to_string(),
                text: message.to_string(),
                trigger_word: trigger_word.clone(),
            };

            // Spawn async task to call each callback URL
            for url in &hook.callback_urls {
                let url = url.clone();
                let payload = payload.clone();
                let content_type = hook.content_type.clone().unwrap_or_else(|| "application/json".to_string());
                
                tokio::spawn(async move {
                    let client = reqwest::Client::new();
                    let result = client
                        .post(&url)
                        .header("Content-Type", &content_type)
                        .json(&payload)
                        .timeout(std::time::Duration::from_secs(30))
                        .send()
                        .await;
                    
                    if let Err(e) = result {
                        tracing::warn!("Outgoing webhook to {} failed: {}", url, e);
                    }
                });
            }
            
            // Only trigger once per message
            break;
        }
    }

    Ok(())
}
