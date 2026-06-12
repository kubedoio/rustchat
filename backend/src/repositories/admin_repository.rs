//! Admin repository for centralized query patterns
//!
//! This module centralizes common admin query patterns to reduce the 69+
//! inline SQL queries previously scattered across api/admin.rs.

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;
use crate::models::email::{
    EmailEvent, EmailTemplateFamily, EmailTemplateVersion, MailProviderSettings, MailProviderType,
    NotificationWorkflow, TemplateVariable, TlsMode, WorkflowPolicy,
};
use crate::models::{
    AdminChannelResponse, AdminTeamResponse, AuditLog, AuditLogQuery, Channel, ChannelType,
    CreateRetentionPolicy, CreateSsoConfig, Permission, RetentionPolicy, ServerConfig, SsoConfig,
    TeamMember, TeamMemberResponse, UpdateSsoConfig,
};

/// Repository for admin-related database operations
pub struct AdminRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> AdminRepository<'a> {
    /// Create a new AdminRepository instance
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Get the singleton server configuration
    pub async fn get_server_config(&self) -> ApiResult<ServerConfig> {
        let config: ServerConfig =
            sqlx::query_as("SELECT * FROM server_config WHERE id = 'default'")
                .fetch_one(self.pool)
                .await?;

        Ok(config)
    }

    /// List audit logs with filtering and pagination
    pub async fn list_audit_logs(
        &self,
        query: &AuditLogQuery,
        page: i64,
        per_page: i64,
    ) -> ApiResult<Vec<AuditLog>> {
        let offset = (page - 1) * per_page;

        let logs: Vec<AuditLog> = sqlx::query_as(
            r#"
            SELECT * FROM audit_logs
            WHERE ($1::VARCHAR IS NULL OR action = $1)
              AND ($2::VARCHAR IS NULL OR target_type = $2)
              AND ($3::UUID IS NULL OR actor_user_id = $3)
              AND ($4::TIMESTAMPTZ IS NULL OR created_at >= $4)
              AND ($5::TIMESTAMPTZ IS NULL OR created_at <= $5)
            ORDER BY created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(&query.action)
        .bind(&query.target_type)
        .bind(query.actor_user_id)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(per_page)
        .bind(offset)
        .fetch_all(self.pool)
        .await?;

        Ok(logs)
    }

    /// Insert a new audit log entry
    pub async fn insert_audit_log(
        &self,
        actor_user_id: Option<Uuid>,
        action: &str,
        target_type: &str,
        target_id: Option<Uuid>,
        metadata: serde_json::Value,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            INSERT INTO audit_logs (actor_user_id, action, target_type, target_id, metadata)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(actor_user_id)
        .bind(action)
        .bind(target_type)
        .bind(target_id)
        .bind(metadata)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// List all SSO configurations ordered by provider_key
    pub async fn list_sso_configs(&self) -> ApiResult<Vec<SsoConfig>> {
        let configs: Vec<SsoConfig> = sqlx::query_as(
            r#"
            SELECT 
                id, org_id, provider, provider_key, provider_type, display_name,
                issuer_url, client_id, client_secret_encrypted, scopes,
                idp_metadata_url, idp_entity_id, is_active, auto_provision,
                default_role, allow_domains, github_org, github_team,
                groups_claim, role_mappings, created_at, updated_at
            FROM sso_configs
            ORDER BY provider_key
            "#,
        )
        .fetch_all(self.pool)
        .await?;

        Ok(configs)
    }

    /// Get a single SSO configuration by ID
    pub async fn get_sso_config_by_id(&self, id: Uuid) -> ApiResult<Option<SsoConfig>> {
        let config: Option<SsoConfig> = sqlx::query_as(
            r#"
            SELECT 
                id, org_id, provider, provider_key, provider_type, display_name,
                issuer_url, client_id, client_secret_encrypted, scopes,
                idp_metadata_url, idp_entity_id, is_active, auto_provision,
                default_role, allow_domains, github_org, github_team,
                groups_claim, role_mappings, created_at, updated_at
            FROM sso_configs
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(config)
    }

    /// Get a single SSO configuration by provider key
    pub async fn get_sso_config_by_provider_key(
        &self,
        provider_key: &str,
    ) -> ApiResult<Option<SsoConfig>> {
        let config: Option<SsoConfig> = sqlx::query_as(
            r#"
            SELECT 
                id, org_id, provider, provider_key, provider_type, display_name,
                issuer_url, client_id, client_secret_encrypted, scopes,
                idp_metadata_url, idp_entity_id, is_active, auto_provision,
                default_role, allow_domains, github_org, github_team,
                groups_claim, role_mappings, created_at, updated_at
            FROM sso_configs
            WHERE provider_key = $1
            "#,
        )
        .bind(provider_key)
        .fetch_optional(self.pool)
        .await?;

        Ok(config)
    }

    /// Insert a new SSO configuration
    pub async fn insert_sso_config(
        &self,
        org_id: Option<Uuid>,
        input: &CreateSsoConfig,
        encrypted_secret: Option<String>,
        scopes: Vec<String>,
    ) -> ApiResult<SsoConfig> {
        let config: SsoConfig = sqlx::query_as(
            r#"
            INSERT INTO sso_configs (
                org_id, provider, provider_key, provider_type, display_name,
                issuer_url, client_id, client_secret_encrypted, scopes,
                is_active, auto_provision, default_role,
                allow_domains, github_org, github_team,
                groups_claim, role_mappings
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING 
                id, org_id, provider, provider_key, provider_type, display_name,
                issuer_url, client_id, client_secret_encrypted, scopes,
                idp_metadata_url, idp_entity_id, is_active, auto_provision,
                default_role, allow_domains, github_org, github_team,
                groups_claim, role_mappings, created_at, updated_at
            "#,
        )
        .bind(org_id)
        .bind(&input.provider_type)
        .bind(&input.provider_key)
        .bind(&input.provider_type)
        .bind(&input.display_name)
        .bind(&input.issuer_url)
        .bind(&input.client_id)
        .bind(&encrypted_secret)
        .bind(&scopes)
        .bind(input.is_active.unwrap_or(true))
        .bind(input.auto_provision.unwrap_or(true))
        .bind(input.default_role.as_ref().unwrap_or(&"member".to_string()))
        .bind(&input.allow_domains)
        .bind(&input.github_org)
        .bind(&input.github_team)
        .bind(&input.groups_claim)
        .bind(&input.role_mappings)
        .fetch_one(self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") {
                crate::error::AppError::Validation(format!(
                    "Provider key '{}' already exists",
                    input.provider_key
                ))
            } else {
                crate::error::AppError::Internal(format!("Failed to create SSO config: {}", e))
            }
        })?;

        Ok(config)
    }

    /// Update an existing SSO configuration
    pub async fn update_sso_config(
        &self,
        id: Uuid,
        input: &UpdateSsoConfig,
        encrypted_secret: Option<String>,
    ) -> ApiResult<SsoConfig> {
        let config: SsoConfig = sqlx::query_as(
            r#"
            UPDATE sso_configs SET
                provider_key = COALESCE($1, provider_key),
                display_name = COALESCE($2, display_name),
                issuer_url = COALESCE($3, issuer_url),
                client_id = COALESCE($4, client_id),
                client_secret_encrypted = COALESCE($5, client_secret_encrypted),
                scopes = COALESCE($6, scopes),
                is_active = COALESCE($7, is_active),
                auto_provision = COALESCE($8, auto_provision),
                default_role = COALESCE($9, default_role),
                allow_domains = COALESCE($10, allow_domains),
                github_org = COALESCE($11, github_org),
                github_team = COALESCE($12, github_team),
                groups_claim = COALESCE($13, groups_claim),
                role_mappings = COALESCE($14, role_mappings),
                updated_at = NOW()
            WHERE id = $15
            RETURNING 
                id, org_id, provider, provider_key, provider_type, display_name,
                issuer_url, client_id, client_secret_encrypted, scopes,
                idp_metadata_url, idp_entity_id, is_active, auto_provision,
                default_role, allow_domains, github_org, github_team,
                groups_claim, role_mappings, created_at, updated_at
            "#,
        )
        .bind(&input.provider_key)
        .bind(&input.display_name)
        .bind(&input.issuer_url)
        .bind(&input.client_id)
        .bind(&encrypted_secret)
        .bind(&input.scopes)
        .bind(input.is_active)
        .bind(input.auto_provision)
        .bind(&input.default_role)
        .bind(&input.allow_domains)
        .bind(&input.github_org)
        .bind(&input.github_team)
        .bind(&input.groups_claim)
        .bind(&input.role_mappings)
        .bind(id)
        .fetch_one(self.pool)
        .await
        .map_err(|e| {
            crate::error::AppError::Internal(format!("Failed to update SSO config: {}", e))
        })?;

        Ok(config)
    }

    /// Delete an SSO configuration
    pub async fn delete_sso_config(&self, id: Uuid) -> ApiResult<bool> {
        let result = sqlx::query("DELETE FROM sso_configs WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ============ Retention Policies ============

    /// List retention policies
    pub async fn list_retention_policies(
        &self,
        org_id: Option<Uuid>,
        is_global_admin: bool,
    ) -> ApiResult<Vec<RetentionPolicy>> {
        let policies: Vec<RetentionPolicy> = if let Some(org_id) = org_id {
            sqlx::query_as(
                "SELECT * FROM retention_policies WHERE org_id = $1 ORDER BY created_at DESC",
            )
            .bind(org_id)
            .fetch_all(self.pool)
            .await?
        } else if is_global_admin {
            sqlx::query_as("SELECT * FROM retention_policies ORDER BY created_at DESC")
                .fetch_all(self.pool)
                .await?
        } else {
            vec![]
        };

        Ok(policies)
    }

    /// Insert a retention policy
    pub async fn insert_retention_policy(
        &self,
        input: &CreateRetentionPolicy,
    ) -> ApiResult<RetentionPolicy> {
        let policy: RetentionPolicy = sqlx::query_as(
            r#"
            INSERT INTO retention_policies (org_id, team_id, channel_id, retention_days, delete_files)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(input.org_id)
        .bind(input.team_id)
        .bind(input.channel_id)
        .bind(input.retention_days)
        .bind(input.delete_files)
        .fetch_one(self.pool)
        .await?;

        Ok(policy)
    }

    /// Get a retention policy by ID
    pub async fn get_retention_policy(&self, id: Uuid) -> ApiResult<Option<RetentionPolicy>> {
        let policy: Option<RetentionPolicy> =
            sqlx::query_as("SELECT * FROM retention_policies WHERE id = $1")
                .bind(id)
                .fetch_optional(self.pool)
                .await?;

        Ok(policy)
    }

    /// Delete a retention policy
    pub async fn delete_retention_policy(&self, id: Uuid) -> ApiResult<()> {
        sqlx::query("DELETE FROM retention_policies WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    // ============ Permissions ============

    /// List all permissions
    pub async fn list_permissions(&self) -> ApiResult<Vec<Permission>> {
        let permissions: Vec<Permission> =
            sqlx::query_as("SELECT * FROM permissions ORDER BY category, id")
                .fetch_all(self.pool)
                .await?;

        Ok(permissions)
    }

    /// Get permission IDs for a role
    pub async fn get_role_permissions(&self, role: &str) -> ApiResult<Vec<String>> {
        let permissions: Vec<(String,)> =
            sqlx::query_as("SELECT permission_id FROM role_permissions WHERE role = $1")
                .bind(role)
                .fetch_all(self.pool)
                .await?;

        Ok(permissions.into_iter().map(|p| p.0).collect())
    }

    /// Set permissions for a role (replaces existing)
    pub async fn set_role_permissions(
        &self,
        role: &str,
        permission_ids: &[String],
    ) -> ApiResult<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM role_permissions WHERE role = $1")
            .bind(role)
            .execute(&mut *tx)
            .await?;

        for permission_id in permission_ids {
            sqlx::query("INSERT INTO role_permissions (role, permission_id) VALUES ($1, $2)")
                .bind(role)
                .bind(permission_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    // ============ Users ============

    /// List users with filtering and pagination
    pub async fn list_users(
        &self,
        status: Option<bool>,
        role: Option<&str>,
        search: Option<&str>,
        include_deleted: bool,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<crate::models::User>> {
        let users: Vec<crate::models::User> = sqlx::query_as(
            r#"
            SELECT * FROM users
            WHERE ($1::BOOL IS NULL OR is_active = $1)
              AND ($2::VARCHAR IS NULL OR role = $2)
              AND ($3::VARCHAR IS NULL OR username ILIKE '%' || $3 || '%' OR email ILIKE '%' || $3 || '%')
              AND ($4::BOOL = TRUE OR deleted_at IS NULL)
            ORDER BY created_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(status)
        .bind(role)
        .bind(search)
        .bind(include_deleted)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await?;

        Ok(users)
    }

    /// Count users with filtering
    pub async fn count_users(
        &self,
        status: Option<bool>,
        role: Option<&str>,
        search: Option<&str>,
        include_deleted: bool,
    ) -> ApiResult<i64> {
        let total: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM users
            WHERE ($1::BOOL IS NULL OR is_active = $1)
              AND ($2::VARCHAR IS NULL OR role = $2)
              AND ($3::VARCHAR IS NULL OR username ILIKE '%' || $3 || '%' OR email ILIKE '%' || $3 || '%')
              AND ($4::BOOL = TRUE OR deleted_at IS NULL)
            "#,
        )
        .bind(status)
        .bind(role)
        .bind(search)
        .bind(include_deleted)
        .fetch_one(self.pool)
        .await?;

        Ok(total.0)
    }

    /// Insert a new user
    pub async fn insert_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
        role: &str,
        display_name: Option<&str>,
    ) -> ApiResult<crate::models::User> {
        let user: crate::models::User = sqlx::query_as(
            r#"
            INSERT INTO users (username, email, password_hash, role, display_name, is_active)
            VALUES ($1, $2, $3, $4, $5, true)
            RETURNING *
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(role)
        .bind(display_name)
        .fetch_one(self.pool)
        .await?;

        Ok(user)
    }

    /// Update a user
    pub async fn update_user(
        &self,
        id: Uuid,
        role: Option<&str>,
        display_name: Option<&str>,
    ) -> ApiResult<crate::models::User> {
        let user: crate::models::User = sqlx::query_as(
            r#"
            UPDATE users SET
                role = COALESCE($1, role),
                display_name = COALESCE($2, display_name),
                updated_at = NOW()
            WHERE id = $3
            RETURNING *
            "#,
        )
        .bind(role)
        .bind(display_name)
        .bind(id)
        .fetch_one(self.pool)
        .await?;

        Ok(user)
    }

    /// Deactivate a user
    pub async fn deactivate_user(&self, id: Uuid) -> ApiResult<()> {
        sqlx::query("UPDATE users SET is_active = false, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Reactivate a user
    pub async fn reactivate_user(&self, id: Uuid) -> ApiResult<()> {
        sqlx::query("UPDATE users SET is_active = true, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Get a user by ID
    pub async fn get_user_by_id(&self, id: Uuid) -> ApiResult<Option<crate::models::User>> {
        let user: Option<crate::models::User> = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(user)
    }

    /// Count active system admins
    pub async fn count_system_admins(&self) -> ApiResult<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE role = 'system_admin' AND deleted_at IS NULL",
        )
        .fetch_one(self.pool)
        .await?;

        Ok(count)
    }

    /// Soft delete a user
    pub async fn soft_delete_user(
        &self,
        id: Uuid,
        deleted_by: Uuid,
        reason: Option<&str>,
    ) -> ApiResult<crate::models::User> {
        let mut tx = self.pool.begin().await?;

        let deleted_user: crate::models::User = sqlx::query_as(
            r#"
            UPDATE users
            SET is_active = false,
                deleted_at = NOW(),
                deleted_by = $2,
                delete_reason = $3,
                updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(deleted_by)
        .bind(reason)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM upload_sessions WHERE user_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(deleted_user)
    }

    /// Count posts by a user
    pub async fn count_posts_by_user(&self, id: Uuid) -> ApiResult<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE user_id = $1")
            .bind(id)
            .fetch_one(self.pool)
            .await?;

        Ok(count)
    }

    /// Permanently wipe a soft-deleted user and related data
    pub async fn wipe_user(&self, id: Uuid) -> ApiResult<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM user_preferences WHERE user_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM channel_members WHERE user_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM team_members WHERE user_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM reactions WHERE user_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM saved_posts WHERE user_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM upload_sessions WHERE user_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    // ============ Email / Notification ============

    /// List mail providers, optionally filtered by tenant
    pub async fn list_mail_providers(
        &self,
        tenant_id: Option<Uuid>,
    ) -> ApiResult<Vec<MailProviderSettings>> {
        let providers: Vec<MailProviderSettings> = if let Some(tenant_id) = tenant_id {
            sqlx::query_as(
                r#"
                SELECT * FROM mail_provider_settings
                WHERE tenant_id = $1
                ORDER BY is_default DESC, created_at ASC
                "#,
            )
            .bind(tenant_id)
            .fetch_all(self.pool)
            .await?
        } else {
            sqlx::query_as(
                r#"
                SELECT * FROM mail_provider_settings
                ORDER BY is_default DESC, created_at ASC
                "#,
            )
            .fetch_all(self.pool)
            .await?
        };

        Ok(providers)
    }

    /// Get a mail provider by ID
    pub async fn get_mail_provider(&self, id: Uuid) -> ApiResult<Option<MailProviderSettings>> {
        let provider: Option<MailProviderSettings> =
            sqlx::query_as("SELECT * FROM mail_provider_settings WHERE id = $1")
                .bind(id)
                .fetch_optional(self.pool)
                .await?;

        Ok(provider)
    }

    /// Create a new mail provider
    #[allow(clippy::too_many_arguments)]
    pub async fn create_mail_provider(
        &self,
        tenant_id: Option<Uuid>,
        provider_type: MailProviderType,
        host: &str,
        port: i32,
        username: &str,
        password_encrypted: &str,
        tls_mode: TlsMode,
        skip_cert_verify: bool,
        from_address: &str,
        from_name: &str,
        reply_to: Option<&str>,
        max_emails_per_minute: i32,
        max_emails_per_hour: i32,
        enabled: bool,
        is_default: bool,
        created_by: Option<Uuid>,
    ) -> ApiResult<MailProviderSettings> {
        let provider: MailProviderSettings = sqlx::query_as(
            r#"
            INSERT INTO mail_provider_settings (
                tenant_id, provider_type, host, port, username, password_encrypted,
                tls_mode, skip_cert_verify, from_address, from_name, reply_to,
                max_emails_per_minute, max_emails_per_hour, enabled, is_default, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(provider_type)
        .bind(host)
        .bind(port)
        .bind(username)
        .bind(password_encrypted)
        .bind(tls_mode)
        .bind(skip_cert_verify)
        .bind(from_address)
        .bind(from_name)
        .bind(reply_to)
        .bind(max_emails_per_minute)
        .bind(max_emails_per_hour)
        .bind(enabled)
        .bind(is_default)
        .bind(created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(provider)
    }

    /// Update a mail provider
    #[allow(clippy::too_many_arguments)]
    pub async fn update_mail_provider(
        &self,
        id: Uuid,
        provider_type: Option<MailProviderType>,
        host: Option<&str>,
        port: Option<i32>,
        username: Option<&str>,
        password_encrypted: Option<&str>,
        tls_mode: Option<TlsMode>,
        skip_cert_verify: Option<bool>,
        from_address: Option<&str>,
        from_name: Option<&str>,
        reply_to: Option<&str>,
        max_emails_per_minute: Option<i32>,
        max_emails_per_hour: Option<i32>,
        enabled: Option<bool>,
        is_default: Option<bool>,
    ) -> ApiResult<MailProviderSettings> {
        let provider: MailProviderSettings = sqlx::query_as(
            r#"
            UPDATE mail_provider_settings SET
                provider_type = COALESCE($2, provider_type),
                host = COALESCE($3, host),
                port = COALESCE($4, port),
                username = COALESCE($5, username),
                password_encrypted = COALESCE($6, password_encrypted),
                tls_mode = COALESCE($7, tls_mode),
                skip_cert_verify = COALESCE($8, skip_cert_verify),
                from_address = COALESCE($9, from_address),
                from_name = COALESCE($10, from_name),
                reply_to = COALESCE($11, reply_to),
                max_emails_per_minute = COALESCE($12, max_emails_per_minute),
                max_emails_per_hour = COALESCE($13, max_emails_per_hour),
                enabled = COALESCE($14, enabled),
                is_default = COALESCE($15, is_default),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(provider_type)
        .bind(host)
        .bind(port)
        .bind(username)
        .bind(password_encrypted)
        .bind(tls_mode)
        .bind(skip_cert_verify)
        .bind(from_address)
        .bind(from_name)
        .bind(reply_to)
        .bind(max_emails_per_minute)
        .bind(max_emails_per_hour)
        .bind(enabled)
        .bind(is_default)
        .fetch_one(self.pool)
        .await?;

        Ok(provider)
    }

    /// Delete a mail provider
    pub async fn delete_mail_provider(&self, id: Uuid) -> ApiResult<bool> {
        let result = sqlx::query("DELETE FROM mail_provider_settings WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Clear default flag from all mail providers for a given tenant
    pub async fn clear_default_mail_providers(&self, tenant_id: Option<Uuid>) -> ApiResult<()> {
        sqlx::query(
            "UPDATE mail_provider_settings SET is_default = false WHERE is_default = true AND tenant_id IS NOT DISTINCT FROM $1"
        )
        .bind(tenant_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Set a mail provider as default
    pub async fn set_default_mail_provider(&self, id: Uuid) -> ApiResult<MailProviderSettings> {
        let provider: MailProviderSettings = sqlx::query_as(
            "UPDATE mail_provider_settings SET is_default = true WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(self.pool)
        .await?;

        Ok(provider)
    }

    /// Get the default enabled mail provider
    pub async fn get_default_mail_provider(&self) -> ApiResult<Option<MailProviderSettings>> {
        let provider: Option<MailProviderSettings> = sqlx::query_as(
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

    /// List notification workflows visible to an org
    pub async fn list_notification_workflows(
        &self,
        org_id: Uuid,
    ) -> ApiResult<Vec<NotificationWorkflow>> {
        let workflows: Vec<NotificationWorkflow> = sqlx::query_as(
            r#"
            SELECT * FROM notification_workflows
            WHERE tenant_id IS NULL OR tenant_id = $1
            ORDER BY category, workflow_key
            "#,
        )
        .bind(org_id)
        .fetch_all(self.pool)
        .await?;

        Ok(workflows)
    }

    /// Get a notification workflow by ID
    pub async fn get_notification_workflow(
        &self,
        id: Uuid,
    ) -> ApiResult<Option<NotificationWorkflow>> {
        let workflow: Option<NotificationWorkflow> =
            sqlx::query_as("SELECT * FROM notification_workflows WHERE id = $1")
                .bind(id)
                .fetch_optional(self.pool)
                .await?;

        Ok(workflow)
    }

    /// Update a notification workflow
    pub async fn update_notification_workflow(
        &self,
        id: Uuid,
        enabled: Option<bool>,
        default_locale: Option<&str>,
        selected_template_family_id: Option<Uuid>,
        policy_json: Option<sqlx::types::Json<WorkflowPolicy>>,
    ) -> ApiResult<NotificationWorkflow> {
        let workflow: NotificationWorkflow = sqlx::query_as(
            r#"
            UPDATE notification_workflows SET
                enabled = COALESCE($2, enabled),
                default_locale = COALESCE($3, default_locale),
                selected_template_family_id = COALESCE($4, selected_template_family_id),
                policy_json = COALESCE($5, policy_json),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(enabled)
        .bind(default_locale)
        .bind(selected_template_family_id)
        .bind(policy_json)
        .fetch_one(self.pool)
        .await?;

        Ok(workflow)
    }

    /// List email template families visible to an org
    pub async fn list_email_template_families(
        &self,
        org_id: Uuid,
    ) -> ApiResult<Vec<EmailTemplateFamily>> {
        let families: Vec<EmailTemplateFamily> = sqlx::query_as(
            r#"
            SELECT * FROM email_template_families
            WHERE tenant_id IS NULL OR tenant_id = $1
            ORDER BY key
            "#,
        )
        .bind(org_id)
        .fetch_all(self.pool)
        .await?;

        Ok(families)
    }

    /// Get an email template family by ID
    pub async fn get_email_template_family(
        &self,
        id: Uuid,
    ) -> ApiResult<Option<EmailTemplateFamily>> {
        let family: Option<EmailTemplateFamily> =
            sqlx::query_as("SELECT * FROM email_template_families WHERE id = $1")
                .bind(id)
                .fetch_optional(self.pool)
                .await?;

        Ok(family)
    }

    /// Create an email template family
    pub async fn create_email_template_family(
        &self,
        tenant_id: Option<Uuid>,
        key: &str,
        name: &str,
        description: Option<&str>,
        workflow_key: Option<&str>,
        created_by: Option<Uuid>,
    ) -> ApiResult<EmailTemplateFamily> {
        let family: EmailTemplateFamily = sqlx::query_as(
            r#"
            INSERT INTO email_template_families (tenant_id, key, name, description, workflow_key, is_system, created_by)
            VALUES ($1, $2, $3, $4, $5, false, $6)
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(key)
        .bind(name)
        .bind(description)
        .bind(workflow_key)
        .bind(created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(family)
    }

    /// Update an email template family (system families are excluded)
    pub async fn update_email_template_family(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
    ) -> ApiResult<Option<EmailTemplateFamily>> {
        let family: Option<EmailTemplateFamily> = sqlx::query_as(
            r#"
            UPDATE email_template_families SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                updated_at = NOW()
            WHERE id = $1 AND is_system = false
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .fetch_optional(self.pool)
        .await?;

        Ok(family)
    }

    /// Delete an email template family (system families are excluded)
    pub async fn delete_email_template_family(&self, id: Uuid) -> ApiResult<bool> {
        let result =
            sqlx::query("DELETE FROM email_template_families WHERE id = $1 AND is_system = false")
                .bind(id)
                .execute(self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List email template versions for a family
    pub async fn list_email_template_versions(
        &self,
        family_id: Uuid,
    ) -> ApiResult<Vec<EmailTemplateVersion>> {
        let versions: Vec<EmailTemplateVersion> = sqlx::query_as(
            r#"
            SELECT * FROM email_template_versions
            WHERE family_id = $1
            ORDER BY locale, version DESC
            "#,
        )
        .bind(family_id)
        .fetch_all(self.pool)
        .await?;

        Ok(versions)
    }

    /// Get an email template version by ID
    pub async fn get_email_template_version(
        &self,
        id: Uuid,
    ) -> ApiResult<Option<EmailTemplateVersion>> {
        let version: Option<EmailTemplateVersion> =
            sqlx::query_as("SELECT * FROM email_template_versions WHERE id = $1")
                .bind(id)
                .fetch_optional(self.pool)
                .await?;

        Ok(version)
    }

    /// Create a new email template version (auto-increments version number)
    #[allow(clippy::too_many_arguments)]
    pub async fn create_email_template_version(
        &self,
        family_id: Uuid,
        locale: &str,
        subject: &str,
        body_text: &str,
        body_html: &str,
        variables: Vec<TemplateVariable>,
        is_compiled_from_mjml: bool,
        mjml_source: Option<&str>,
        created_by: Option<Uuid>,
    ) -> ApiResult<EmailTemplateVersion> {
        let max_version: Option<i32> = sqlx::query_scalar(
            "SELECT MAX(version) FROM email_template_versions WHERE family_id = $1 AND locale = $2",
        )
        .bind(family_id)
        .bind(locale)
        .fetch_one(self.pool)
        .await?;

        let version = max_version.unwrap_or(0) + 1;

        let new_version: EmailTemplateVersion = sqlx::query_as(
            r#"
            INSERT INTO email_template_versions (
                family_id, version, status, locale, subject, body_text, body_html,
                variables_schema_json, is_compiled_from_mjml, mjml_source, created_by
            )
            VALUES ($1, $2, 'draft', $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(family_id)
        .bind(version)
        .bind(locale)
        .bind(subject)
        .bind(body_text)
        .bind(body_html)
        .bind(sqlx::types::Json(variables))
        .bind(is_compiled_from_mjml)
        .bind(mjml_source)
        .bind(created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(new_version)
    }

    /// Update an email template version
    pub async fn update_email_template_version(
        &self,
        id: Uuid,
        subject: Option<&str>,
        body_text: Option<&str>,
        body_html: Option<&str>,
        variables_json: Option<sqlx::types::Json<Vec<TemplateVariable>>>,
        mjml_source: Option<&str>,
    ) -> ApiResult<EmailTemplateVersion> {
        let version: EmailTemplateVersion = sqlx::query_as(
            r#"
            UPDATE email_template_versions SET
                subject = COALESCE($2, subject),
                body_text = COALESCE($3, body_text),
                body_html = COALESCE($4, body_html),
                variables_schema_json = COALESCE($5, variables_schema_json),
                mjml_source = COALESCE($6, mjml_source)
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(subject)
        .bind(body_text)
        .bind(body_html)
        .bind(variables_json)
        .bind(mjml_source)
        .fetch_one(self.pool)
        .await?;

        Ok(version)
    }

    /// Publish a draft email template version
    pub async fn publish_email_template_version(
        &self,
        id: Uuid,
        published_by: Uuid,
    ) -> ApiResult<Option<EmailTemplateVersion>> {
        let version: Option<EmailTemplateVersion> = sqlx::query_as(
            r#"
            UPDATE email_template_versions SET
                status = 'published',
                published_at = NOW(),
                published_by = $2
            WHERE id = $1 AND status = 'draft'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(published_by)
        .fetch_optional(self.pool)
        .await?;

        Ok(version)
    }

    /// Cancel a queued outbox entry
    pub async fn cancel_outbox_entry(&self, id: Uuid) -> ApiResult<bool> {
        let result = sqlx::query(
            "UPDATE email_outbox SET status = 'cancelled' WHERE id = $1 AND status = 'queued'",
        )
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Retry a failed outbox entry
    pub async fn retry_outbox_entry(&self, id: Uuid) -> ApiResult<bool> {
        let result = sqlx::query(
            r#"
            UPDATE email_outbox SET 
                status = 'queued',
                attempt_count = 0,
                next_attempt_at = NULL,
                last_error_category = NULL,
                last_error_message = NULL
            WHERE id = $1 AND status = 'failed'
            "#,
        )
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List email events with filtering and pagination
    pub async fn list_email_events(
        &self,
        outbox_id: Option<Uuid>,
        workflow_key: Option<&str>,
        event_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<EmailEvent>> {
        let events: Vec<EmailEvent> = sqlx::query_as(
            r#"
            SELECT * FROM email_events
            WHERE ($1::uuid IS NULL OR outbox_id = $1)
              AND ($2::varchar IS NULL OR workflow_key = $2)
              AND ($3::varchar IS NULL OR event_type = $3)
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(outbox_id)
        .bind(workflow_key)
        .bind(event_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await?;

        Ok(events)
    }

    // ============ Teams & Channels ============

    /// List teams with optional search and pagination
    pub async fn list_teams(
        &self,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<AdminTeamResponse>> {
        let teams: Vec<AdminTeamResponse> = sqlx::query_as(
            r#"
            SELECT t.*, 
                   (SELECT COUNT(*) FROM team_members WHERE team_id = t.id) as members_count,
                   (SELECT COUNT(*) FROM channels WHERE team_id = t.id) as channels_count
            FROM teams t
            WHERE ($1::VARCHAR IS NULL OR t.name ILIKE '%' || $1 || '%' OR t.display_name ILIKE '%' || $1 || '%')
            ORDER BY t.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(search)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await?;

        Ok(teams)
    }

    /// Count all teams
    pub async fn count_teams(&self) -> ApiResult<i64> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM teams")
            .fetch_one(self.pool)
            .await?;

        Ok(total.0)
    }

    /// Get a single team by ID with member/channel counts
    pub async fn get_team_by_id(&self, id: Uuid) -> ApiResult<Option<AdminTeamResponse>> {
        let team: Option<AdminTeamResponse> = sqlx::query_as(
            r#"
            SELECT t.*, 
                   (SELECT COUNT(*) FROM team_members WHERE team_id = t.id) as members_count,
                   (SELECT COUNT(*) FROM channels WHERE team_id = t.id) as channels_count
            FROM teams t
            WHERE t.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(team)
    }

    /// Update a team's display_name and/or description
    pub async fn update_team(
        &self,
        id: Uuid,
        display_name: Option<&str>,
        description: Option<&str>,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            UPDATE teams 
            SET display_name = COALESCE($2, display_name),
                description = COALESCE($3, description)
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(display_name)
        .bind(description)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Delete a team by ID
    pub async fn delete_team(&self, id: Uuid) -> ApiResult<()> {
        sqlx::query("DELETE FROM teams WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// List members of a team with user details
    pub async fn list_team_members(&self, team_id: Uuid) -> ApiResult<Vec<TeamMemberResponse>> {
        let members: Vec<TeamMemberResponse> = sqlx::query_as(
            r#"
            SELECT tm.team_id, tm.user_id, tm.role, tm.created_at,
                   u.username, u.display_name, u.avatar_url,
                   COALESCE(tm.presence, 'offline') as presence
            FROM team_members tm
            JOIN users u ON tm.user_id = u.id
            WHERE tm.team_id = $1
            ORDER BY u.username
            "#,
        )
        .bind(team_id)
        .fetch_all(self.pool)
        .await?;

        Ok(members)
    }

    /// Add a member to a team
    pub async fn add_team_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: Option<&str>,
    ) -> ApiResult<TeamMember> {
        let member: TeamMember = sqlx::query_as(
            r#"
            INSERT INTO team_members (team_id, user_id, role)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .bind(role.unwrap_or("member"))
        .fetch_one(self.pool)
        .await?;

        Ok(member)
    }

    /// Remove a member from a team
    pub async fn remove_team_member(&self, team_id: Uuid, user_id: Uuid) -> ApiResult<()> {
        sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// List channels with optional team filter and search
    pub async fn list_channels(
        &self,
        team_id: Option<Uuid>,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<AdminChannelResponse>> {
        let channels: Vec<AdminChannelResponse> = sqlx::query_as(
            r#"
            SELECT c.*, 
                   (SELECT COUNT(*) FROM channel_members WHERE channel_id = c.id) as members_count
            FROM channels c
            WHERE ($1::UUID IS NULL OR c.team_id = $1)
              AND ($2::VARCHAR IS NULL OR c.name ILIKE '%' || $2 || '%' OR c.display_name ILIKE '%' || $2 || '%')
            ORDER BY c.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(team_id)
        .bind(search)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await?;

        Ok(channels)
    }

    /// Count channels with optional team filter
    pub async fn count_channels(&self, team_id: Option<Uuid>) -> ApiResult<i64> {
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM channels WHERE ($1::UUID IS NULL OR team_id = $1)",
        )
        .bind(team_id)
        .fetch_one(self.pool)
        .await?;

        Ok(total.0)
    }

    /// Create a new channel
    pub async fn create_channel(
        &self,
        team_id: Uuid,
        name: &str,
        display_name: Option<&str>,
        purpose: Option<&str>,
        channel_type: ChannelType,
        creator_id: Uuid,
    ) -> ApiResult<Channel> {
        let channel: Channel = sqlx::query_as(
            r#"
            INSERT INTO channels (team_id, name, display_name, purpose, type, creator_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(team_id)
        .bind(name)
        .bind(display_name)
        .bind(purpose)
        .bind(channel_type)
        .bind(creator_id)
        .fetch_one(self.pool)
        .await?;

        Ok(channel)
    }

    /// Update a channel's display_name
    pub async fn update_channel_display_name(&self, id: Uuid, display_name: &str) -> ApiResult<()> {
        sqlx::query("UPDATE channels SET display_name = $1 WHERE id = $2")
            .bind(display_name)
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Update a channel's purpose
    pub async fn update_channel_purpose(&self, id: Uuid, purpose: &str) -> ApiResult<()> {
        sqlx::query("UPDATE channels SET purpose = $1 WHERE id = $2")
            .bind(purpose)
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Update a channel's header
    pub async fn update_channel_header(&self, id: Uuid, header: &str) -> ApiResult<()> {
        sqlx::query("UPDATE channels SET header = $1 WHERE id = $2")
            .bind(header)
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Get a channel by ID
    pub async fn get_channel_by_id(&self, id: Uuid) -> ApiResult<Option<Channel>> {
        let channel: Option<Channel> = sqlx::query_as("SELECT * FROM channels WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(channel)
    }

    /// Delete a channel by ID
    pub async fn delete_channel(&self, id: Uuid) -> ApiResult<()> {
        sqlx::query("DELETE FROM channels WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Check if a user has the system_manage permission.
    pub async fn has_system_manage_permission(&self, user_id: Uuid) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM users u
                JOIN roles r ON u.role = r.name
                WHERE u.id = $1 AND r.permissions @> ARRAY['system_manage']
            )
            "#,
        )
        .bind(user_id)
        .fetch_one(self.pool)
        .await
    }
}
