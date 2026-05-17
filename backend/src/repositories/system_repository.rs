use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;
use crate::models::email::MailProviderSettings;
use crate::models::ServerConfig;

pub struct SystemRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> SystemRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Get the default server configuration.
    pub async fn get_server_config(&self) -> ApiResult<ServerConfig> {
        let config = sqlx::query_as::<_, ServerConfig>(
            "SELECT * FROM server_config WHERE id = 'default'"
        )
        .fetch_one(self.pool)
        .await?;
        Ok(config)
    }

    /// Get the default enabled mail provider settings.
    pub async fn get_default_mail_provider(&self) -> ApiResult<Option<MailProviderSettings>> {
        let provider = sqlx::query_as::<_, MailProviderSettings>(
            r#"
            SELECT * FROM mail_provider_settings
            WHERE enabled = true AND is_default = true
            ORDER BY tenant_id NULLS LAST
            LIMIT 1
            "#,
        )
        .fetch_optional(self.pool)
        .await?;
        Ok(provider)
    }

    /// Get FCM credentials from the default server config.
    pub async fn get_fcm_credentials(&self) -> ApiResult<Option<(String, String)>> {
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT fcm_project_id, fcm_access_token FROM server_config WHERE id = 'default'",
        )
        .fetch_optional(self.pool)
        .await?;
        Ok(row)
    }

    /// Get user device tokens and platforms for a user.
    pub async fn get_user_devices(
        &self,
        user_id: Uuid,
    ) -> ApiResult<Vec<(Option<String>, Option<String>)>> {
        let rows = sqlx::query_as(
            "SELECT token, platform FROM user_devices WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    /// Update the site name in the default server config.
    pub async fn update_site_name(
        &self,
        site_name: &str,
        updated_by: Uuid,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE server_config SET site = jsonb_set(site, '{site_name}', $1, true), updated_at = NOW(), updated_by = $2 WHERE id = 'default'"
        )
        .bind(serde_json::json!(site_name))
        .bind(updated_by)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update the team default channels in the default server config.
    pub async fn update_team_default_channels(
        &self,
        channels: &serde_json::Value,
        updated_by: Uuid,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE server_config SET experimental = jsonb_set(experimental, '{team_default_channels}', $1, true), updated_at = NOW(), updated_by = $2 WHERE id = 'default'"
        )
        .bind(serde_json::json!(channels))
        .bind(updated_by)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update the SMTP host in the default server config.
    pub async fn update_smtp_host(
        &self,
        host: &str,
        updated_by: Uuid,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE server_config SET email = jsonb_set(email, '{smtp_host}', $1, true), updated_at = NOW(), updated_by = $2 WHERE id = 'default'"
        )
        .bind(serde_json::json!(host))
        .bind(updated_by)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update the SMTP port in the default server config.
    pub async fn update_smtp_port(
        &self,
        port: &str,
        updated_by: Uuid,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE server_config SET email = jsonb_set(email, '{smtp_port}', $1, true), updated_at = NOW(), updated_by = $2 WHERE id = 'default'"
        )
        .bind(serde_json::json!(port))
        .bind(updated_by)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update the from address in the default server config.
    pub async fn update_from_address(
        &self,
        from: &str,
        updated_by: Uuid,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE server_config SET email = jsonb_set(email, '{from_address}', $1, true), updated_at = NOW(), updated_by = $2 WHERE id = 'default'"
        )
        .bind(serde_json::json!(from))
        .bind(updated_by)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update the webhooks enabled flag in the default server config.
    pub async fn update_enable_webhooks(
        &self,
        enabled: bool,
        updated_by: Uuid,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE server_config SET integrations = jsonb_set(integrations, '{enable_webhooks}', $1, true), updated_at = NOW(), updated_by = $2 WHERE id = 'default'"
        )
        .bind(serde_json::json!(enabled))
        .bind(updated_by)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update the slash commands enabled flag in the default server config.
    pub async fn update_enable_slash_commands(
        &self,
        enabled: bool,
        updated_by: Uuid,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE server_config SET integrations = jsonb_set(integrations, '{enable_slash_commands}', $1, true), updated_at = NOW(), updated_by = $2 WHERE id = 'default'"
        )
        .bind(serde_json::json!(enabled))
        .bind(updated_by)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update the message retention days in the default server config.
    pub async fn update_message_retention_days(
        &self,
        days: i64,
        updated_by: Uuid,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE server_config SET compliance = jsonb_set(compliance, '{message_retention_days}', $1, true), updated_at = NOW(), updated_by = $2 WHERE id = 'default'"
        )
        .bind(serde_json::json!(days))
        .bind(updated_by)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update the file retention days in the default server config.
    pub async fn update_file_retention_days(
        &self,
        days: i64,
        updated_by: Uuid,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE server_config SET compliance = jsonb_set(compliance, '{file_retention_days}', $1, true), updated_at = NOW(), updated_by = $2 WHERE id = 'default'"
        )
        .bind(serde_json::json!(days))
        .bind(updated_by)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Get the site URL from the default server config.
    pub async fn get_site_url(&self) -> Result<Option<String>, sqlx::Error> {
        let url: Option<String> = sqlx::query_scalar(
            "SELECT site->>'site_url' FROM server_config WHERE id = 'default'",
        )
        .fetch_optional(self.pool)
        .await?;

        Ok(url.filter(|u| !u.is_empty()))
    }
}
