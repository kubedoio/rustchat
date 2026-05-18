//! Terms of Service repository for centralized query patterns

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;
use crate::models::terms::{TermsOfService, TermsStats};

/// Repository for terms of service database operations
#[derive(Debug, Clone)]
pub struct TermsRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> TermsRepository<'a> {
    /// Create a new TermsRepository instance
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Get the current active terms of service
    pub async fn get_current_terms(&self) -> ApiResult<Option<TermsOfService>> {
        let terms = sqlx::query_as::<_, TermsOfService>(
            r#"
            SELECT * FROM terms_of_service
            WHERE is_active = true
            ORDER BY effective_date DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(self.pool)
        .await?;

        Ok(terms)
    }

    /// Get terms by ID
    pub async fn get_terms_by_id(&self, id: Uuid) -> ApiResult<Option<TermsOfService>> {
        let terms = sqlx::query_as::<_, TermsOfService>(
            "SELECT * FROM terms_of_service WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(terms)
    }

    /// List all terms of service (newest first)
    pub async fn list_terms(&self) -> ApiResult<Vec<TermsOfService>> {
        let terms = sqlx::query_as::<_, TermsOfService>(
            "SELECT * FROM terms_of_service ORDER BY created_at DESC",
        )
        .fetch_all(self.pool)
        .await?;

        Ok(terms)
    }

    /// Check if a terms version already exists
    pub async fn version_exists(&self, version: &str) -> ApiResult<bool> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM terms_of_service WHERE version = $1)",
        )
        .bind(version)
        .fetch_one(self.pool)
        .await?;

        Ok(exists)
    }

    /// Create new terms of service
    pub async fn create_terms(
        &self,
        version: &str,
        title: &str,
        content: &str,
        summary: &Option<String>,
        effective_date: DateTime<Utc>,
        created_by: Uuid,
    ) -> ApiResult<TermsOfService> {
        let terms = sqlx::query_as::<_, TermsOfService>(
            r#"
            INSERT INTO terms_of_service
            (version, title, content, summary, is_active, effective_date, created_by)
            VALUES ($1, $2, $3, $4, false, $5, $6)
            RETURNING *
            "#,
        )
        .bind(version)
        .bind(title)
        .bind(content)
        .bind(summary)
        .bind(effective_date)
        .bind(created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(terms)
    }

    /// Update terms of service
    pub async fn update_terms(
        &self,
        id: Uuid,
        title: &Option<String>,
        content: &Option<String>,
        summary: &Option<String>,
        effective_date: &Option<DateTime<Utc>>,
    ) -> ApiResult<TermsOfService> {
        let updated = sqlx::query_as::<_, TermsOfService>(
            r#"
            UPDATE terms_of_service
            SET
                title = COALESCE($1, title),
                content = COALESCE($2, content),
                summary = COALESCE($3, summary),
                effective_date = COALESCE($4, effective_date)
            WHERE id = $5
            RETURNING *
            "#,
        )
        .bind(title)
        .bind(content)
        .bind(summary)
        .bind(effective_date)
        .bind(id)
        .fetch_one(self.pool)
        .await?;

        Ok(updated)
    }

    /// Delete terms of service by ID
    pub async fn delete_terms(&self, id: Uuid) -> ApiResult<()> {
        sqlx::query("DELETE FROM terms_of_service WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Get is_active status for terms
    pub async fn is_terms_active(&self, id: Uuid) -> ApiResult<Option<bool>> {
        let is_active: Option<bool> =
            sqlx::query_scalar("SELECT is_active FROM terms_of_service WHERE id = $1")
                .bind(id)
                .fetch_optional(self.pool)
                .await?;

        Ok(is_active)
    }

    /// Activate terms of service
    pub async fn activate_terms(&self, id: Uuid) -> ApiResult<Option<TermsOfService>> {
        let terms = sqlx::query_as::<_, TermsOfService>(
            r#"
            UPDATE terms_of_service
            SET is_active = true
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(terms)
    }

    /// Check if user has accepted specific terms
    pub async fn has_user_accepted(&self, user_id: Uuid, terms_id: Uuid) -> ApiResult<bool> {
        let accepted: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM user_terms_acceptance
                WHERE user_id = $1 AND terms_id = $2
            )
            "#,
        )
        .bind(user_id)
        .bind(terms_id)
        .fetch_one(self.pool)
        .await?;

        Ok(accepted)
    }

    /// Record user acceptance of terms
    pub async fn accept_terms(
        &self,
        user_id: Uuid,
        terms_id: Uuid,
        accepted_at: DateTime<Utc>,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            INSERT INTO user_terms_acceptance (user_id, terms_id, accepted_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, terms_id) DO UPDATE SET accepted_at = $3
            "#,
        )
        .bind(user_id)
        .bind(terms_id)
        .bind(accepted_at)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Get acceptance stats for specific terms
    pub async fn get_terms_stats(&self, terms_id: Uuid) -> ApiResult<TermsStats> {
        let row = sqlx::query(
            r#"
            SELECT
                (SELECT COUNT(*) FROM users WHERE deleted_at IS NULL) as total_users,
                COUNT(DISTINCT uta.user_id) as accepted_count
            FROM user_terms_acceptance uta
            WHERE uta.terms_id = $1
            "#,
        )
        .bind(terms_id)
        .fetch_one(self.pool)
        .await?;

        let total_users: i64 = row.try_get("total_users")?;
        let accepted_count: i64 = row.try_get("accepted_count")?;
        let pending_count = total_users - accepted_count;
        let acceptance_rate = if total_users > 0 {
            (accepted_count as f64 / total_users as f64) * 100.0
        } else {
            0.0
        };

        Ok(TermsStats {
            total_users,
            accepted_count,
            pending_count,
            acceptance_rate,
        })
    }

    /// Get total active user count
    pub async fn count_active_users(&self) -> ApiResult<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE deleted_at IS NULL")
            .fetch_one(self.pool)
            .await?;

        Ok(count)
    }

    /// Get count of users who accepted specific terms
    pub async fn count_accepted_users(&self, terms_id: Uuid) -> ApiResult<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT user_id) FROM user_terms_acceptance WHERE terms_id = $1",
        )
        .bind(terms_id)
        .fetch_one(self.pool)
        .await?;

        Ok(count)
    }

    /// Get list of users who have NOT accepted specific terms
    pub async fn get_pending_users(
        &self,
        terms_id: Uuid,
        limit: i64,
    ) -> ApiResult<Vec<crate::models::user::User>> {
        let users = sqlx::query_as::<_, crate::models::user::User>(
            r#"
            SELECT u.* FROM users u
            WHERE u.deleted_at IS NULL
            AND NOT EXISTS (
                SELECT 1 FROM user_terms_acceptance uta
                WHERE uta.user_id = u.id AND uta.terms_id = $1
            )
            ORDER BY u.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(terms_id)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        Ok(users)
    }
}
