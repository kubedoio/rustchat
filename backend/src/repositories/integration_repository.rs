use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Bot, BotToken, IncomingWebhook, OutgoingWebhook, SlashCommand};

/// Repository for integration-related database operations
pub struct IntegrationRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> IntegrationRepository<'a> {
    /// Create a new IntegrationRepository instance
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    // ========== Incoming Webhooks ==========

    /// List incoming webhooks for a team
    pub async fn list_incoming_webhooks_by_team(
        &self,
        team_id: Uuid,
    ) -> Result<Vec<IncomingWebhook>, sqlx::Error> {
        sqlx::query_as::<_, IncomingWebhook>(
            "SELECT * FROM incoming_webhooks WHERE team_id = $1 ORDER BY created_at DESC",
        )
        .bind(team_id)
        .fetch_all(self.pool)
        .await
    }

    /// Get an incoming webhook by ID
    pub async fn get_incoming_webhook_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<IncomingWebhook>, sqlx::Error> {
        sqlx::query_as::<_, IncomingWebhook>("SELECT * FROM incoming_webhooks WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    /// Get an active incoming webhook by token
    pub async fn get_incoming_webhook_by_token(
        &self,
        token: &str,
    ) -> Result<Option<IncomingWebhook>, sqlx::Error> {
        sqlx::query_as::<_, IncomingWebhook>(
            "SELECT * FROM incoming_webhooks WHERE token = $1 AND is_active = true",
        )
        .bind(token)
        .fetch_optional(self.pool)
        .await
    }

    /// Get the creator_id of an incoming webhook
    pub async fn get_incoming_webhook_creator_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar("SELECT creator_id FROM incoming_webhooks WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    /// Create a new incoming webhook
    pub async fn create_incoming_webhook(
        &self,
        team_id: Uuid,
        channel_id: Uuid,
        creator_id: Uuid,
        display_name: Option<&str>,
        description: Option<&str>,
        token: &str,
        is_active: bool,
    ) -> Result<IncomingWebhook, sqlx::Error> {
        sqlx::query_as::<_, IncomingWebhook>(
            r#"
            INSERT INTO incoming_webhooks (team_id, channel_id, creator_id, display_name, description, token, is_active)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(team_id)
        .bind(channel_id)
        .bind(creator_id)
        .bind(display_name)
        .bind(description)
        .bind(token)
        .bind(is_active)
        .fetch_one(self.pool)
        .await
    }

    /// List incoming webhooks with pagination and optional filters
    pub async fn list_incoming_webhooks_paginated(
        &self,
        team_id: Option<Uuid>,
        creator_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<IncomingWebhook>, sqlx::Error> {
        if let Some(tid) = team_id {
            if let Some(cid) = creator_id {
                sqlx::query_as::<_, IncomingWebhook>(
                    "SELECT * FROM incoming_webhooks WHERE creator_id = $1 AND team_id = $2 LIMIT $3 OFFSET $4"
                )
                .bind(cid)
                .bind(tid)
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            } else {
                sqlx::query_as::<_, IncomingWebhook>(
                    "SELECT * FROM incoming_webhooks WHERE team_id = $1 LIMIT $2 OFFSET $3"
                )
                .bind(tid)
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            }
        } else {
            if let Some(cid) = creator_id {
                sqlx::query_as::<_, IncomingWebhook>(
                    "SELECT * FROM incoming_webhooks WHERE creator_id = $1 LIMIT $2 OFFSET $3"
                )
                .bind(cid)
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            } else {
                sqlx::query_as::<_, IncomingWebhook>(
                    "SELECT * FROM incoming_webhooks LIMIT $1 OFFSET $2"
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            }
        }
    }

    /// Update an incoming webhook
    pub async fn update_incoming_webhook(
        &self,
        id: Uuid,
        display_name: Option<&str>,
        description: Option<&str>,
    ) -> Result<IncomingWebhook, sqlx::Error> {
        sqlx::query_as::<_, IncomingWebhook>(
            r#"UPDATE incoming_webhooks SET
                display_name = COALESCE($2, display_name),
                description = COALESCE($3, description),
                updated_at = NOW()
               WHERE id = $1 RETURNING *"#,
        )
        .bind(id)
        .bind(display_name)
        .bind(description)
        .fetch_one(self.pool)
        .await
    }

    /// Delete an incoming webhook
    pub async fn delete_incoming_webhook(
        &self,
        id: Uuid,
    ) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
        sqlx::query("DELETE FROM incoming_webhooks WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await
    }

    // ========== Outgoing Webhooks ==========

    /// List outgoing webhooks for a team
    pub async fn list_outgoing_webhooks_by_team(
        &self,
        team_id: Uuid,
    ) -> Result<Vec<OutgoingWebhook>, sqlx::Error> {
        sqlx::query_as::<_, OutgoingWebhook>(
            "SELECT * FROM outgoing_webhooks WHERE team_id = $1 ORDER BY created_at DESC",
        )
        .bind(team_id)
        .fetch_all(self.pool)
        .await
    }

    /// Get an outgoing webhook by ID
    pub async fn get_outgoing_webhook_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<OutgoingWebhook>, sqlx::Error> {
        sqlx::query_as::<_, OutgoingWebhook>("SELECT * FROM outgoing_webhooks WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    /// Get the creator_id of an outgoing webhook
    pub async fn get_outgoing_webhook_creator_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar("SELECT creator_id FROM outgoing_webhooks WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    /// Create a new outgoing webhook
    pub async fn create_outgoing_webhook(
        &self,
        team_id: Uuid,
        channel_id: Option<Uuid>,
        creator_id: Uuid,
        display_name: Option<&str>,
        description: Option<&str>,
        trigger_words: &[String],
        trigger_when: &str,
        callback_urls: &[String],
        content_type: Option<&str>,
        token: &str,
        is_active: bool,
    ) -> Result<OutgoingWebhook, sqlx::Error> {
        sqlx::query_as::<_, OutgoingWebhook>(
            r#"
            INSERT INTO outgoing_webhooks 
            (team_id, channel_id, creator_id, display_name, description, trigger_words, trigger_when, callback_urls, content_type, token, is_active)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(team_id)
        .bind(channel_id)
        .bind(creator_id)
        .bind(display_name)
        .bind(description)
        .bind(trigger_words)
        .bind(trigger_when)
        .bind(callback_urls)
        .bind(content_type)
        .bind(token)
        .bind(is_active)
        .fetch_one(self.pool)
        .await
    }

    /// List outgoing webhooks with pagination and optional filters
    pub async fn list_outgoing_webhooks_paginated(
        &self,
        team_id: Option<Uuid>,
        creator_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OutgoingWebhook>, sqlx::Error> {
        if let Some(tid) = team_id {
            if let Some(cid) = creator_id {
                sqlx::query_as::<_, OutgoingWebhook>(
                    "SELECT * FROM outgoing_webhooks WHERE creator_id = $1 AND team_id = $2 LIMIT $3 OFFSET $4"
                )
                .bind(cid)
                .bind(tid)
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            } else {
                sqlx::query_as::<_, OutgoingWebhook>(
                    "SELECT * FROM outgoing_webhooks WHERE team_id = $1 LIMIT $2 OFFSET $3"
                )
                .bind(tid)
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            }
        } else {
            if let Some(cid) = creator_id {
                sqlx::query_as::<_, OutgoingWebhook>(
                    "SELECT * FROM outgoing_webhooks WHERE creator_id = $1 LIMIT $2 OFFSET $3"
                )
                .bind(cid)
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            } else {
                sqlx::query_as::<_, OutgoingWebhook>(
                    "SELECT * FROM outgoing_webhooks LIMIT $1 OFFSET $2"
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            }
        }
    }

    /// Update an outgoing webhook
    pub async fn update_outgoing_webhook(
        &self,
        id: Uuid,
        display_name: Option<&str>,
        description: Option<&str>,
        trigger_words: Option<&[String]>,
        callback_urls: Option<&[String]>,
    ) -> Result<OutgoingWebhook, sqlx::Error> {
        sqlx::query_as::<_, OutgoingWebhook>(
            r#"UPDATE outgoing_webhooks SET
                display_name = COALESCE($2, display_name),
                description = COALESCE($3, description),
                trigger_words = COALESCE($4, trigger_words),
                callback_urls = COALESCE($5, callback_urls),
                updated_at = NOW()
               WHERE id = $1 RETURNING *"#,
        )
        .bind(id)
        .bind(display_name)
        .bind(description)
        .bind(trigger_words)
        .bind(callback_urls)
        .fetch_one(self.pool)
        .await
    }

    /// Update the token of an outgoing webhook
    pub async fn update_outgoing_hook_token(
        &self,
        id: Uuid,
        token: &str,
    ) -> Result<OutgoingWebhook, sqlx::Error> {
        sqlx::query_as::<_, OutgoingWebhook>(
            "UPDATE outgoing_webhooks SET token = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(token)
        .fetch_one(self.pool)
        .await
    }

    /// Delete an outgoing webhook
    pub async fn delete_outgoing_webhook(
        &self,
        id: Uuid,
    ) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
        sqlx::query("DELETE FROM outgoing_webhooks WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await
    }

    // ========== Slash Commands ==========

    /// List slash commands for a team
    pub async fn list_slash_commands_by_team(
        &self,
        team_id: Uuid,
    ) -> Result<Vec<SlashCommand>, sqlx::Error> {
        sqlx::query_as::<_, SlashCommand>(
            "SELECT * FROM slash_commands WHERE team_id = $1 ORDER BY trigger",
        )
        .bind(team_id)
        .fetch_all(self.pool)
        .await
    }

    /// Get a slash command by ID
    pub async fn get_slash_command_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<SlashCommand>, sqlx::Error> {
        sqlx::query_as::<_, SlashCommand>("SELECT * FROM slash_commands WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    /// Get a slash command by team and trigger
    pub async fn get_slash_command_by_team_and_trigger(
        &self,
        team_id: Uuid,
        trigger: &str,
    ) -> Result<Option<SlashCommand>, sqlx::Error> {
        sqlx::query_as::<_, SlashCommand>(
            "SELECT * FROM slash_commands WHERE team_id = $1 AND trigger = $2",
        )
        .bind(team_id)
        .bind(trigger)
        .fetch_optional(self.pool)
        .await
    }

    /// Create a new slash command
    pub async fn create_slash_command(
        &self,
        team_id: Uuid,
        creator_id: Uuid,
        trigger: &str,
        url: &str,
        method: &str,
        display_name: Option<&str>,
        description: Option<&str>,
        hint: Option<&str>,
        token: &str,
    ) -> Result<SlashCommand, sqlx::Error> {
        sqlx::query_as::<_, SlashCommand>(
            r#"
            INSERT INTO slash_commands 
            (team_id, creator_id, trigger, url, method, display_name, description, hint, token)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(team_id)
        .bind(creator_id)
        .bind(trigger)
        .bind(url)
        .bind(method)
        .bind(display_name)
        .bind(description)
        .bind(hint)
        .bind(token)
        .fetch_one(self.pool)
        .await
    }

    /// Delete a slash command
    pub async fn delete_slash_command(
        &self,
        id: Uuid,
    ) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
        sqlx::query("DELETE FROM slash_commands WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await
    }

    // ========== Bots ==========

    /// List all bots
    pub async fn list_bots(&self) -> Result<Vec<Bot>, sqlx::Error> {
        sqlx::query_as::<_, Bot>("SELECT * FROM bots ORDER BY created_at DESC")
            .fetch_all(self.pool)
            .await
    }

    /// List bots by owner
    pub async fn list_bots_by_owner(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<Bot>, sqlx::Error> {
        sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE owner_id = $1 ORDER BY created_at DESC")
            .bind(owner_id)
            .fetch_all(self.pool)
            .await
    }

    /// Get a bot by ID
    pub async fn get_bot_by_id(&self, id: Uuid) -> Result<Option<Bot>, sqlx::Error> {
        sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    /// Create a bot user account
    pub async fn create_bot_user(
        &self,
        username: &str,
        email: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let row: (Uuid,) = sqlx::query_as(
            r#"
            INSERT INTO users (username, email, password_hash, is_bot, role)
            VALUES ($1, $2, 'BOT_NO_PASSWORD', true, 'member')
            RETURNING id
            "#,
        )
        .bind(username)
        .bind(email)
        .fetch_one(self.pool)
        .await?;
        Ok(row.0)
    }

    /// Create a bot record
    pub async fn create_bot(
        &self,
        user_id: Uuid,
        owner_id: Uuid,
        display_name: &str,
        description: Option<&str>,
    ) -> Result<Bot, sqlx::Error> {
        sqlx::query_as::<_, Bot>(
            r#"
            INSERT INTO bots (user_id, owner_id, display_name, description)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(owner_id)
        .bind(display_name)
        .bind(description)
        .fetch_one(self.pool)
        .await
    }

    /// Delete a bot
    pub async fn delete_bot(
        &self,
        id: Uuid,
    ) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
        sqlx::query("DELETE FROM bots WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await
    }

    // ========== Bot Tokens ==========

    /// List tokens for a bot
    pub async fn list_bot_tokens(
        &self,
        bot_id: Uuid,
    ) -> Result<Vec<BotToken>, sqlx::Error> {
        sqlx::query_as::<_, BotToken>(
            "SELECT * FROM bot_tokens WHERE bot_id = $1 ORDER BY created_at DESC",
        )
        .bind(bot_id)
        .fetch_all(self.pool)
        .await
    }

    /// Create a bot token
    pub async fn create_bot_token(
        &self,
        bot_id: Uuid,
        token: &str,
        description: Option<&str>,
    ) -> Result<BotToken, sqlx::Error> {
        sqlx::query_as::<_, BotToken>(
            r#"
            INSERT INTO bot_tokens (bot_id, token, description)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(bot_id)
        .bind(token)
        .bind(description)
        .fetch_one(self.pool)
        .await
    }

    /// Delete a bot token
    pub async fn delete_bot_token(
        &self,
        token_id: Uuid,
        bot_id: Uuid,
    ) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
        sqlx::query("DELETE FROM bot_tokens WHERE id = $1 AND bot_id = $2")
            .bind(token_id)
            .bind(bot_id)
            .execute(self.pool)
            .await
    }

    // ========== Webhook Execution ==========

    /// Create a post from an incoming webhook execution
    pub async fn create_post_from_webhook(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        message: &str,
        props: &serde_json::Value,
    ) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO posts (channel_id, user_id, message, props)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(channel_id)
        .bind(user_id)
        .bind(message)
        .bind(props)
        .execute(self.pool)
        .await
    }
}
