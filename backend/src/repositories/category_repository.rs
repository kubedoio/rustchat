use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::error::ApiResult;
use crate::models::channel::ChannelType;

/// Row struct for channel_categories table (Mattermost-compat, millis timestamps).
#[derive(sqlx::FromRow, Clone)]
pub struct CategoryRow {
    pub id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    #[sqlx(rename = "type")]
    pub type_field: String,
    pub display_name: String,
    pub sorting: String,
    pub muted: bool,
    pub collapsed: bool,
    pub sort_order: i32,
    pub create_at: i64,
    pub update_at: i64,
    pub delete_at: i64,
}

/// Candidate channel for sidebar backfill.
#[derive(sqlx::FromRow, Clone, Copy)]
pub struct SidebarCandidateChannel {
    pub id: Uuid,
    #[sqlx(rename = "type")]
    pub channel_type: ChannelType,
}

pub struct CategoryRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> CategoryRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// List non-deleted categories for a user in a team.
    pub async fn list_for_user(
        &self,
        user_id: Uuid,
        team_id: Uuid,
    ) -> ApiResult<Vec<CategoryRow>> {
        let rows = sqlx::query_as::<_, CategoryRow>(
            "SELECT * FROM channel_categories WHERE user_id = $1 AND team_id = $2 AND delete_at = 0"
        )
        .bind(user_id)
        .bind(team_id)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    /// Get channel IDs assigned to a category, ordered by sort_order.
    pub async fn get_channel_ids(
        &self,
        category_id: Uuid,
    ) -> ApiResult<Vec<Uuid>> {
        let ids = sqlx::query_scalar(
            "SELECT channel_id FROM channel_category_channels WHERE category_id = $1 ORDER BY sort_order ASC"
        )
        .bind(category_id)
        .fetch_all(self.pool)
        .await?;
        Ok(ids)
    }

    /// Get channels that should appear in the sidebar for a user.
    pub async fn get_sidebar_candidate_channels(
        &self,
        user_id: Uuid,
        team_id: Uuid,
    ) -> ApiResult<Vec<SidebarCandidateChannel>> {
        let channels = sqlx::query_as::<_, SidebarCandidateChannel>(
            r#"
            SELECT c.id, c.type
            FROM channels c
            JOIN channel_members cm ON c.id = cm.channel_id
            WHERE cm.user_id = $1
              AND c.is_archived = false
              AND (
                (c.type IN ('public', 'private') AND c.team_id = $2)
                OR c.type IN ('direct', 'group')
              )
            ORDER BY COALESCE(c.display_name, c.name) ASC
            "#,
        )
        .bind(user_id)
        .bind(team_id)
        .fetch_all(self.pool)
        .await?;
        Ok(channels)
    }

    /// Get the next available sort_order for a user's categories in a team.
    pub async fn get_next_sort_order(
        &self,
        user_id: Uuid,
        team_id: Uuid,
    ) -> ApiResult<i32> {
        let next_order: i32 = sqlx::query_scalar(
            "SELECT (COALESCE(MAX(sort_order), -1) + 1)::INT FROM channel_categories WHERE user_id = $1 AND team_id = $2"
        )
        .bind(user_id)
        .bind(team_id)
        .fetch_one(self.pool)
        .await?;
        Ok(next_order)
    }

    /// Create a new category.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        id: Uuid,
        team_id: Uuid,
        user_id: Uuid,
        category_type: &str,
        display_name: &str,
        sorting: &str,
        sort_order: i32,
        now: i64,
    ) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO channel_categories (id, team_id, user_id, type, display_name, sorting, sort_order, create_at, update_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(id)
        .bind(team_id)
        .bind(user_id)
        .bind(category_type)
        .bind(display_name)
        .bind(sorting)
        .bind(sort_order)
        .bind(now)
        .bind(now)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Get a single category by ID, user, and team (non-deleted).
    pub async fn get(
        &self,
        category_id: Uuid,
        user_id: Uuid,
        team_id: Uuid,
    ) -> ApiResult<Option<CategoryRow>> {
        let row = sqlx::query_as::<_, CategoryRow>(
            "SELECT * FROM channel_categories WHERE id = $1 AND user_id = $2 AND team_id = $3 AND delete_at = 0"
        )
        .bind(category_id)
        .bind(user_id)
        .bind(team_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(row)
    }

    /// Update category fields (pool version).
    pub async fn update_fields(
        &self,
        category_id: Uuid,
        user_id: Uuid,
        team_id: Uuid,
        display_name: &str,
        sorting: &str,
        muted: bool,
        collapsed: bool,
        update_at: i64,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE channel_categories SET display_name = $1, sorting = $2, muted = $3, collapsed = $4, update_at = $5 WHERE id = $6 AND user_id = $7 AND team_id = $8"
        )
        .bind(display_name)
        .bind(sorting)
        .bind(muted)
        .bind(collapsed)
        .bind(update_at)
        .bind(category_id)
        .bind(user_id)
        .bind(team_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update category fields inside a transaction.
    pub async fn update_fields_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        category_id: Uuid,
        user_id: Uuid,
        team_id: Uuid,
        display_name: &str,
        sorting: &str,
        muted: bool,
        collapsed: bool,
        update_at: i64,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE channel_categories SET display_name = $1, sorting = $2, muted = $3, collapsed = $4, update_at = $5 WHERE id = $6 AND user_id = $7 AND team_id = $8"
        )
        .bind(display_name)
        .bind(sorting)
        .bind(muted)
        .bind(collapsed)
        .bind(update_at)
        .bind(category_id)
        .bind(user_id)
        .bind(team_id)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Update category with COALESCE semantics, returning the row.
    pub async fn update_returning(
        &self,
        category_id: Uuid,
        user_id: Uuid,
        team_id: Uuid,
        display_name: Option<&str>,
        sorting: Option<&str>,
        muted: Option<bool>,
        collapsed: Option<bool>,
        update_at: i64,
    ) -> ApiResult<CategoryRow> {
        let row = sqlx::query_as::<_, CategoryRow>(
            r#"
            UPDATE channel_categories SET
                display_name = COALESCE($4, display_name),
                sorting = COALESCE($5, sorting),
                muted = COALESCE($6, muted),
                collapsed = COALESCE($7, collapsed),
                update_at = $8
            WHERE id = $1 AND user_id = $2 AND team_id = $3 AND delete_at = 0
            RETURNING *
            "#,
        )
        .bind(category_id)
        .bind(user_id)
        .bind(team_id)
        .bind(display_name)
        .bind(sorting)
        .bind(muted)
        .bind(collapsed)
        .bind(update_at)
        .fetch_one(self.pool)
        .await?;
        Ok(row)
    }

    /// Delete all channel associations for a category (pool version).
    pub async fn delete_channel_associations(&self, category_id: Uuid) -> ApiResult<()> {
        sqlx::query("DELETE FROM channel_category_channels WHERE category_id = $1")
            .bind(category_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Delete all channel associations for a category (transaction version).
    pub async fn delete_channel_associations_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        category_id: Uuid,
    ) -> ApiResult<()> {
        sqlx::query("DELETE FROM channel_category_channels WHERE category_id = $1")
            .bind(category_id)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    /// Insert a single channel association (pool version).
    pub async fn insert_channel_association(
        &self,
        category_id: Uuid,
        channel_id: Uuid,
        sort_order: i32,
    ) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO channel_category_channels (category_id, channel_id, sort_order) VALUES ($1, $2, $3)"
        )
        .bind(category_id)
        .bind(channel_id)
        .bind(sort_order)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Insert a single channel association (transaction version).
    pub async fn insert_channel_association_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        category_id: Uuid,
        channel_id: Uuid,
        sort_order: i32,
    ) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO channel_category_channels (category_id, channel_id, sort_order) VALUES ($1, $2, $3)"
        )
        .bind(category_id)
        .bind(channel_id)
        .bind(sort_order)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Update sort_order for a single category (pool version).
    pub async fn update_sort_order(
        &self,
        category_id: Uuid,
        user_id: Uuid,
        team_id: Uuid,
        sort_order: i32,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE channel_categories SET sort_order = $1 WHERE id = $2 AND user_id = $3 AND team_id = $4"
        )
        .bind(sort_order)
        .bind(category_id)
        .bind(user_id)
        .bind(team_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update sort_order for a single category (transaction version).
    pub async fn update_sort_order_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        category_id: Uuid,
        user_id: Uuid,
        team_id: Uuid,
        sort_order: i32,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE channel_categories SET sort_order = $1 WHERE id = $2 AND user_id = $3 AND team_id = $4"
        )
        .bind(sort_order)
        .bind(category_id)
        .bind(user_id)
        .bind(team_id)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Soft-delete a category.
    pub async fn soft_delete(
        &self,
        category_id: Uuid,
        delete_at: i64,
    ) -> ApiResult<()> {
        sqlx::query("UPDATE channel_categories SET delete_at = $2 WHERE id = $1")
            .bind(category_id)
            .bind(delete_at)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Find the default "channels" category for a user in a team.
    pub async fn find_default_category(
        &self,
        user_id: Uuid,
        team_id: Uuid,
    ) -> ApiResult<Option<Uuid>> {
        let id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM channel_categories WHERE user_id = $1 AND team_id = $2 AND type = 'channels' AND delete_at = 0"
        )
        .bind(user_id)
        .bind(team_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(id)
    }

    /// Move all channels from one category to another.
    pub async fn migrate_channels_to_category(
        &self,
        from_category_id: Uuid,
        to_category_id: Uuid,
    ) -> ApiResult<()> {
        sqlx::query("UPDATE channel_category_channels SET category_id = $1 WHERE category_id = $2")
            .bind(to_category_id)
            .bind(from_category_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }
}
