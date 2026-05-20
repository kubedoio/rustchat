use sqlx::PgPool;
use uuid::Uuid;

use crate::models::server_config::SiteConfig;
use crate::models::{SsoConfig, User};

/// Repository for OAuth-related database operations
pub struct OAuthRepository<'a> {
    pool: &'a PgPool,
}

/// Row type for legacy provider resolution
#[derive(Debug, sqlx::FromRow)]
pub struct LegacyProviderRow {
    pub provider_key: String,
    pub provider_type: String,
    pub provider: String,
}

impl<'a> OAuthRepository<'a> {
    /// Create a new OAuthRepository instance
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// List active SSO configs for legacy provider resolution
    pub async fn list_legacy_providers(&self) -> Result<Vec<LegacyProviderRow>, sqlx::Error> {
        sqlx::query_as::<_, LegacyProviderRow>(
            r#"
            SELECT provider_key, provider_type, provider
            FROM sso_configs
            WHERE is_active = true
            ORDER BY updated_at DESC, created_at DESC
            "#,
        )
        .fetch_all(self.pool)
        .await
    }

    /// Get site configuration from server config
    pub async fn get_site_config(&self) -> Result<Option<SiteConfig>, sqlx::Error> {
        let row: Option<(sqlx::types::Json<SiteConfig>,)> =
            sqlx::query_as("SELECT site FROM server_config WHERE id = 'default'")
                .fetch_optional(self.pool)
                .await?;
        Ok(row.map(|(site,)| site.0))
    }

    /// Get authentication configuration from server config
    pub async fn get_authentication_config(
        &self,
    ) -> Result<Option<serde_json::Value>, sqlx::Error> {
        let row: Option<(serde_json::Value,)> =
            sqlx::query_as("SELECT authentication FROM server_config WHERE id = 'default'")
                .fetch_optional(self.pool)
                .await?;
        Ok(row.map(|(auth,)| auth))
    }

    /// List all active SSO configurations
    pub async fn list_active_sso_configs(&self) -> Result<Vec<SsoConfig>, sqlx::Error> {
        sqlx::query_as::<_, SsoConfig>(
            r#"
            SELECT 
                id, org_id, provider, provider_key, provider_type, display_name,
                issuer_url, client_id, client_secret_encrypted, scopes,
                idp_metadata_url, idp_entity_id, is_active, auto_provision,
                default_role, allow_domains, github_org, github_team,
                groups_claim, role_mappings, created_at, updated_at
            FROM sso_configs 
            WHERE is_active = true
            ORDER BY display_name, provider_key
            "#,
        )
        .fetch_all(self.pool)
        .await
    }

    /// Get an active SSO configuration by provider key
    pub async fn get_active_sso_config_by_provider_key(
        &self,
        provider_key: &str,
    ) -> Result<Option<SsoConfig>, sqlx::Error> {
        sqlx::query_as::<_, SsoConfig>(
            r#"
            SELECT 
                id, org_id, provider, provider_key, provider_type, display_name,
                issuer_url, client_id, client_secret_encrypted, scopes,
                idp_metadata_url, idp_entity_id, is_active, auto_provision,
                default_role, allow_domains, github_org, github_team,
                groups_claim, role_mappings, created_at, updated_at
            FROM sso_configs 
            WHERE provider_key = $1 AND is_active = true
            "#,
        )
        .bind(provider_key)
        .fetch_optional(self.pool)
        .await
    }

    /// Get a user by auth provider and external ID
    pub async fn get_user_by_auth_provider_and_id(
        &self,
        provider_key: &str,
        external_id: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE auth_provider = $1 AND auth_provider_id = $2 AND deleted_at IS NULL",
        )
        .bind(provider_key)
        .bind(external_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Get a user by email (case-insensitive)
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE LOWER(email) = LOWER($1) AND deleted_at IS NULL",
        )
        .bind(email)
        .fetch_optional(self.pool)
        .await
    }

    /// Get auth provider link for a user
    pub async fn get_user_auth_link_by_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<(Option<String>, Option<String>)>, sqlx::Error> {
        sqlx::query_as("SELECT auth_provider, auth_provider_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(self.pool)
            .await
    }

    /// Update user's last login and optionally sync role
    pub async fn update_user_login(
        &self,
        user_id: Uuid,
        should_sync_role: bool,
        role: &str,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET last_login_at = NOW(),
                updated_at = NOW(),
                role = CASE WHEN $2 THEN $3 ELSE role END
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(should_sync_role)
        .bind(role)
        .fetch_one(self.pool)
        .await
    }

    /// Update user's last login, SSO link, and optionally sync role
    pub async fn update_user_login_and_link(
        &self,
        user_id: Uuid,
        should_link: bool,
        provider_key: &str,
        external_id: Option<&str>,
        should_sync_role: bool,
        role: &str,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET last_login_at = NOW(),
                updated_at = NOW(),
                auth_provider = CASE WHEN $2 THEN $3 ELSE auth_provider END,
                auth_provider_id = CASE WHEN $2 THEN $4 ELSE auth_provider_id END,
                role = CASE WHEN $5 THEN $6 ELSE role END
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(should_link)
        .bind(provider_key)
        .bind(external_id)
        .bind(should_sync_role)
        .bind(role)
        .fetch_one(self.pool)
        .await
    }

    /// Create a new OAuth user
    #[allow(clippy::too_many_arguments)]
    pub async fn create_oauth_user(
        &self,
        username: &str,
        email: &str,
        display_name: Option<&str>,
        role: &str,
        provider_key: &str,
        external_id: Option<&str>,
        org_id: Option<Uuid>,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (
                username, email, display_name, role, 
                is_active, auth_provider, auth_provider_id, org_id
            )
            VALUES ($1, $2, $3, $4, true, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(display_name)
        .bind(role)
        .bind(provider_key)
        .bind(external_id)
        .bind(org_id)
        .fetch_one(self.pool)
        .await
    }

    /// Check if a username already exists
    pub async fn username_exists(&self, username: &str) -> Result<bool, sqlx::Error> {
        let exists: Option<(bool,)> =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
                .bind(username)
                .fetch_optional(self.pool)
                .await?;
        Ok(exists.map(|(e,)| e).unwrap_or(false))
    }

    /// Sync OIDC groups for a user into the groups table
    pub async fn sync_oidc_groups(
        &self,
        user_id: Uuid,
        auth_provider: &str,
        groups: &[String],
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        for group_name in groups {
            let group_name = group_name.trim();
            if group_name.is_empty() {
                continue;
            }

            let normalized_name = group_name.to_lowercase().replace(' ', "_");

            let group_id: Uuid = sqlx::query_scalar(
                r#"
                INSERT INTO groups (name, display_name, source, remote_id, allow_reference)
                VALUES ($1, $2, $3, $4, TRUE)
                ON CONFLICT (source, remote_id) DO UPDATE SET
                    display_name = EXCLUDED.display_name,
                    deleted_at = NULL,
                    updated_at = NOW()
                RETURNING id
                "#,
            )
            .bind(&normalized_name)
            .bind(group_name)
            .bind(format!("oidc:{}", auth_provider))
            .bind(&normalized_name)
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO group_members (group_id, user_id)
                VALUES ($1, $2)
                ON CONFLICT (group_id, user_id) DO NOTHING
                "#,
            )
            .bind(group_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        tracing::debug!(
            "Synced {} OIDC groups for user {} from provider {}",
            groups.len(),
            user_id,
            auth_provider
        );

        Ok(())
    }
}
