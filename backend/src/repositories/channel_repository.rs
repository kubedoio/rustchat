use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{ApiResult, AppError};
use crate::models::{Channel, ChannelMember, ChannelType};

fn escape_like_pattern(term: &str) -> String {
    term.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Row returned from channel list queries that join team data
#[derive(Debug, sqlx::FromRow)]
pub struct ChannelWithTeamDataRow {
    pub id: Uuid,
    pub team_id: Uuid,
    #[sqlx(rename = "type")]
    pub channel_type: ChannelType,
    pub name: String,
    pub display_name: Option<String>,
    pub purpose: Option<String>,
    pub header: Option<String>,
    pub is_archived: bool,
    pub creator_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub team_display_name: Option<String>,
    pub team_name: String,
    pub team_updated_at: DateTime<Utc>,
}

/// API response that flattens a channel with its team data
#[derive(Debug, serde::Serialize)]
pub struct ChannelWithTeamDataResponse {
    #[serde(flatten)]
    pub channel: crate::mattermost_compat::models::Channel,
    pub team_display_name: String,
    pub team_name: String,
    pub team_update_at: i64,
}

/// Row returned from channel bookmark queries
#[derive(sqlx::FromRow)]
pub struct BookmarkRow {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub channel_id: Uuid,
    pub owner_id: Uuid,
    pub file_id: Option<Uuid>,
    pub display_name: String,
    pub sort_order: i64,
    pub link_url: Option<String>,
    pub image_url: Option<String>,
    pub emoji: Option<String>,
    pub bookmark_type: String,
    pub original_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
}

/// Row returned from channel group queries
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChannelGroupRow {
    pub id: Uuid,
    pub name: Option<String>,
    pub display_name: String,
    pub description: String,
    pub source: String,
    pub remote_id: Option<String>,
    pub allow_reference: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub scheme_admin: bool,
    pub has_syncables: bool,
    pub member_count: i64,
}

pub struct ChannelRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> ChannelRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Get a channel by ID that has not been soft-deleted
    pub async fn get_by_id(&self, id: Uuid) -> Result<Channel, sqlx::Error> {
        sqlx::query_as::<_, Channel>("SELECT * FROM channels WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_one(self.pool)
            .await
    }

    /// Get a channel by ID (optional, not found returns None instead of error).
    pub async fn get_by_id_optional(&self, id: Uuid) -> Result<Option<Channel>, sqlx::Error> {
        sqlx::query_as::<_, Channel>("SELECT * FROM channels WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    /// Get channel creator ID
    pub async fn get_creator_id(&self, channel_id: Uuid) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar("SELECT creator_id FROM channels WHERE id = $1")
            .bind(channel_id)
            .fetch_optional(self.pool)
            .await
    }

    /// Get team_id for a channel
    pub async fn get_team_id(&self, channel_id: Uuid) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar("SELECT team_id FROM channels WHERE id = $1")
            .bind(channel_id)
            .fetch_optional(self.pool)
            .await
    }

    /// Get channel name by ID
    pub async fn get_name(&self, channel_id: Uuid) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT name FROM channels WHERE id = $1")
            .bind(channel_id)
            .fetch_optional(self.pool)
            .await
    }

    /// Find a channel by team_id and name
    pub async fn find_by_team_and_name(
        &self,
        team_id: Uuid,
        name: &str,
    ) -> Result<Option<Channel>, sqlx::Error> {
        sqlx::query_as::<_, Channel>(
            "SELECT * FROM channels WHERE team_id = $1 AND name = $2 AND deleted_at IS NULL",
        )
        .bind(team_id)
        .bind(name)
        .fetch_optional(self.pool)
        .await
    }

    /// Get a user's role in a channel
    pub async fn get_member_role(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT role FROM channel_members WHERE channel_id = $1 AND user_id = $2",
        )
        .bind(channel_id)
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
    }

    /// List channels a user is a member of within a team
    pub async fn list_for_user(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        include_archived: bool,
    ) -> Result<Vec<Channel>, sqlx::Error> {
        if include_archived {
            sqlx::query_as(
                r#"
                SELECT c.* FROM channels c
                INNER JOIN channel_members cm ON cm.channel_id = c.id
                WHERE c.team_id = $1 AND cm.user_id = $2 AND c.deleted_at IS NULL
                ORDER BY c.name
                "#,
            )
            .bind(team_id)
            .bind(user_id)
            .fetch_all(self.pool)
            .await
        } else {
            sqlx::query_as(
                r#"
                SELECT c.* FROM channels c
                INNER JOIN channel_members cm ON cm.channel_id = c.id
                WHERE c.team_id = $1 AND cm.user_id = $2 AND c.is_archived = false AND c.deleted_at IS NULL
                ORDER BY c.name
                "#
            )
            .bind(team_id)
            .bind(user_id)
            .fetch_all(self.pool)
            .await
        }
    }

    /// List public/private channels in a team that a user is NOT a member of
    pub async fn list_joinable(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Channel>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT c.* FROM channels c
            WHERE c.team_id = $1 
            AND c.type IN ('public', 'private')
            AND c.is_archived = false
            AND c.deleted_at IS NULL
            AND c.id NOT IN (
                SELECT channel_id FROM channel_members WHERE user_id = $2
            )
            ORDER BY c.name
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_all(self.pool)
        .await
    }

    /// Find an existing DM channel between two users in a team
    pub async fn find_dm_channel(
        &self,
        team_id: Uuid,
        name_a: &str,
        name_b: &str,
    ) -> Result<Option<Channel>, sqlx::Error> {
        sqlx::query_as::<_, Channel>(
            r#"
            SELECT *
            FROM channels
            WHERE team_id = $1
              AND type = 'direct'::channel_type
              AND (name = $2 OR name = $3)
              AND deleted_at IS NULL
            ORDER BY created_at ASC
            LIMIT 1
            "#,
        )
        .bind(team_id)
        .bind(name_a)
        .bind(name_b)
        .fetch_optional(self.pool)
        .await
    }

    /// Check if a user is a member of a team
    pub async fn is_team_member(&self, team_id: Uuid, user_id: Uuid) -> ApiResult<bool> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2)",
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_one(self.pool)
        .await?;
        Ok(exists)
    }

    /// Add a member to a channel
    pub async fn add_member(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING"
        )
        .bind(channel_id)
        .bind(user_id)
        .bind(role)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Verify user is a member of a channel and return the membership row
    pub async fn require_member(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
    ) -> Result<ChannelMember, AppError> {
        sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(user_id)
            .fetch_optional(self.pool)
            .await?
            .ok_or_else(|| AppError::NotAMember)
    }

    /// Update a channel with optional fields (COALESCE pattern)
    pub async fn update(
        &self,
        id: Uuid,
        name: Option<&str>,
        display_name: Option<&str>,
        purpose: Option<&str>,
        header: Option<&str>,
    ) -> ApiResult<Channel> {
        let result = sqlx::query_as(
            r#"
            UPDATE channels SET
                name = COALESCE($2, name),
                display_name = COALESCE($3, display_name),
                purpose = COALESCE($4, purpose),
                header = COALESCE($5, header),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(display_name)
        .bind(purpose)
        .bind(header)
        .fetch_one(self.pool)
        .await;

        match result {
            Ok(channel) => Ok(channel),
            Err(sqlx::Error::RowNotFound) => Err(AppError::ChannelNotFound),
            Err(e) => {
                if let Some(db_err) = e.as_database_error() {
                    if let Some(constraint) = db_err.constraint() {
                        if constraint.contains("channels_name_team_unique") {
                            return Err(AppError::Conflict(
                                "Channel name already exists in this team".to_string(),
                            ));
                        }
                    }
                }
                Err(AppError::Database(e))
            }
        }
    }

    /// Soft delete a channel
    pub async fn soft_delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE channels SET deleted_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// List channel members with user details
    pub async fn list_members(&self, channel_id: Uuid) -> Result<Vec<ChannelMember>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT cm.*, u.username, u.display_name, u.avatar_url, u.presence
            FROM channel_members cm
            INNER JOIN users u ON cm.user_id = u.id
            WHERE cm.channel_id = $1
            ORDER BY u.username ASC
            "#,
        )
        .bind(channel_id)
        .fetch_all(self.pool)
        .await
    }

    /// Upsert a channel member (insert or update role) and return the row
    pub async fn upsert_member(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<ChannelMember, sqlx::Error> {
        sqlx::query_as(
            r#"
            INSERT INTO channel_members (channel_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (channel_id, user_id) DO UPDATE SET role = $3
            RETURNING *
            "#,
        )
        .bind(channel_id)
        .bind(user_id)
        .bind(role)
        .fetch_one(self.pool)
        .await
    }

    /// Update a channel member's role.
    pub async fn update_member_role(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE channel_members SET role = $1 WHERE channel_id = $2 AND user_id = $3")
            .bind(role)
            .bind(channel_id)
            .bind(user_id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    /// Update a channel member's notify_props.
    pub async fn update_member_notify_props(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        notify_props: &serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE channel_members SET notify_props = $1 WHERE channel_id = $2 AND user_id = $3",
        )
        .bind(notify_props)
        .bind(channel_id)
        .bind(user_id)
        .execute(self.pool)
        .await
        .map(|_| ())
    }

    /// Remove a member from a channel
    pub async fn remove_member(&self, channel_id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // ========================================================================
    // Bookmark methods
    // ========================================================================

    /// Check if a user is a member of a channel
    pub async fn is_channel_member(&self, channel_id: Uuid, user_id: Uuid) -> ApiResult<bool> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM channel_members WHERE channel_id = $1 AND user_id = $2)",
        )
        .bind(channel_id)
        .bind(user_id)
        .fetch_one(self.pool)
        .await?;
        Ok(exists)
    }

    /// List bookmarks for a channel, optionally filtered by since timestamp
    pub async fn list_channel_bookmarks(
        &self,
        channel_id: Uuid,
        since: i64,
    ) -> ApiResult<Vec<BookmarkRow>> {
        let bookmarks = sqlx::query_as::<_, BookmarkRow>(
            r#"
            SELECT id, created_at, updated_at, deleted_at, channel_id, owner_id, file_id,
                   display_name, sort_order, link_url, image_url, emoji, bookmark_type,
                   original_id, parent_id
            FROM channel_bookmarks
            WHERE channel_id = $1
              AND ($2 <= 0 OR updated_at >= to_timestamp($2::double precision / 1000.0))
              AND deleted_at IS NULL
            ORDER BY sort_order ASC, created_at ASC
            "#,
        )
        .bind(channel_id)
        .bind(since)
        .fetch_all(self.pool)
        .await?;
        Ok(bookmarks)
    }

    /// Get the maximum sort order for bookmarks in a channel
    pub async fn get_max_bookmark_sort_order(&self, channel_id: Uuid) -> ApiResult<Option<i64>> {
        let max_order: Option<i64> = sqlx::query_scalar(
            "SELECT MAX(sort_order) FROM channel_bookmarks WHERE channel_id = $1",
        )
        .bind(channel_id)
        .fetch_one(self.pool)
        .await?;
        Ok(max_order)
    }

    /// Create a new channel bookmark
    #[allow(clippy::too_many_arguments)]
    pub async fn create_channel_bookmark(
        &self,
        channel_id: Uuid,
        owner_id: Uuid,
        file_id: Option<Uuid>,
        display_name: &str,
        sort_order: i64,
        link_url: Option<&str>,
        image_url: Option<&str>,
        emoji: Option<&str>,
        bookmark_type: &str,
        now: DateTime<Utc>,
    ) -> ApiResult<BookmarkRow> {
        let bookmark = sqlx::query_as::<_, BookmarkRow>(
            r#"
            INSERT INTO channel_bookmarks (
                channel_id, owner_id, file_id, display_name, sort_order,
                link_url, image_url, emoji, bookmark_type, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10)
            RETURNING id, created_at, updated_at, deleted_at, channel_id, owner_id, file_id,
                      display_name, sort_order, link_url, image_url, emoji, bookmark_type,
                      original_id, parent_id
            "#,
        )
        .bind(channel_id)
        .bind(owner_id)
        .bind(file_id)
        .bind(display_name)
        .bind(sort_order)
        .bind(link_url)
        .bind(image_url)
        .bind(emoji)
        .bind(bookmark_type)
        .bind(now)
        .fetch_one(self.pool)
        .await?;
        Ok(bookmark)
    }

    /// Update a channel bookmark
    #[allow(clippy::too_many_arguments)]
    pub async fn update_channel_bookmark(
        &self,
        bookmark_id: Uuid,
        channel_id: Uuid,
        display_name: Option<&str>,
        link_url: Option<&str>,
        image_url: Option<&str>,
        emoji: Option<&str>,
        file_id: Option<Uuid>,
        sort_order: Option<i64>,
    ) -> ApiResult<Option<BookmarkRow>> {
        let bookmark = sqlx::query_as::<_, BookmarkRow>(
            r#"
            UPDATE channel_bookmarks SET
                display_name = COALESCE($3, display_name),
                link_url = COALESCE($4, link_url),
                image_url = COALESCE($5, image_url),
                emoji = COALESCE($6, emoji),
                file_id = COALESCE($7, file_id),
                sort_order = COALESCE($8, sort_order),
                updated_at = NOW()
            WHERE id = $1 AND channel_id = $2 AND deleted_at IS NULL
            RETURNING id, created_at, updated_at, deleted_at, channel_id, owner_id, file_id,
                      display_name, sort_order, link_url, image_url, emoji, bookmark_type,
                      original_id, parent_id
            "#,
        )
        .bind(bookmark_id)
        .bind(channel_id)
        .bind(display_name)
        .bind(link_url)
        .bind(image_url)
        .bind(emoji)
        .bind(file_id)
        .bind(sort_order)
        .fetch_optional(self.pool)
        .await?;
        Ok(bookmark)
    }

    /// Update the sort order of a bookmark
    pub async fn update_bookmark_sort_order(
        &self,
        bookmark_id: Uuid,
        channel_id: Uuid,
        new_order: i64,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE channel_bookmarks SET sort_order = $3, updated_at = NOW() WHERE id = $1 AND channel_id = $2"
        )
        .bind(bookmark_id)
        .bind(channel_id)
        .bind(new_order)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Soft delete a channel bookmark
    pub async fn soft_delete_channel_bookmark(
        &self,
        bookmark_id: Uuid,
        channel_id: Uuid,
    ) -> ApiResult<Option<BookmarkRow>> {
        let bookmark = sqlx::query_as::<_, BookmarkRow>(
            r#"
            UPDATE channel_bookmarks SET deleted_at = NOW(), updated_at = NOW()
            WHERE id = $1 AND channel_id = $2 AND deleted_at IS NULL
            RETURNING id, created_at, updated_at, deleted_at, channel_id, owner_id, file_id,
                      display_name, sort_order, link_url, image_url, emoji, bookmark_type,
                      original_id, parent_id
            "#,
        )
        .bind(bookmark_id)
        .bind(channel_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(bookmark)
    }

    // ========================================================================
    // Channel group methods
    // ========================================================================

    /// List all channels with optional filters and pagination (includes team data)
    pub async fn get_all_channels(
        &self,
        include_deleted: bool,
        exclude_default_channels: bool,
        not_associated_group_id: Option<Uuid>,
        per_page: i64,
        offset: i64,
    ) -> ApiResult<Vec<ChannelWithTeamDataRow>> {
        let rows = sqlx::query_as::<_, ChannelWithTeamDataRow>(
            r#"
            SELECT
                c.id,
                c.team_id,
                c.type,
                c.name,
                c.display_name,
                c.purpose,
                c.header,
                c.is_archived,
                c.creator_id,
                c.created_at,
                c.updated_at,
                c.deleted_at,
                t.display_name AS team_display_name,
                t.name AS team_name,
                t.updated_at AS team_updated_at
            FROM channels c
            JOIN teams t ON t.id = c.team_id
            WHERE
                ($1::bool OR (c.is_archived = false AND c.deleted_at IS NULL))
                AND (NOT $2::bool OR c.name NOT IN ('town-square', 'off-topic'))
                AND (
                    $3::uuid IS NULL
                    OR NOT EXISTS (
                        SELECT 1
                        FROM group_syncables gs
                        WHERE gs.syncable_type = 'channel'
                          AND gs.group_id = $3
                          AND gs.syncable_id = c.id
                    )
                )
            ORDER BY c.created_at ASC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(include_deleted)
        .bind(exclude_default_channels)
        .bind(not_associated_group_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    /// Count all channels with optional filters
    pub async fn count_all_channels(
        &self,
        include_deleted: bool,
        exclude_default_channels: bool,
        not_associated_group_id: Option<Uuid>,
    ) -> ApiResult<i64> {
        let total_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::BIGINT
            FROM channels c
            WHERE
                ($1::bool OR (c.is_archived = false AND c.deleted_at IS NULL))
                AND (NOT $2::bool OR c.name NOT IN ('town-square', 'off-topic'))
                AND (
                    $3::uuid IS NULL
                    OR NOT EXISTS (
                        SELECT 1
                        FROM group_syncables gs
                        WHERE gs.syncable_type = 'channel'
                          AND gs.group_id = $3
                          AND gs.syncable_id = c.id
                    )
                )
            "#,
        )
        .bind(include_deleted)
        .bind(exclude_default_channels)
        .bind(not_associated_group_id)
        .fetch_one(self.pool)
        .await?;
        Ok(total_count)
    }

    /// Create a new channel
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        team_id: Uuid,
        channel_type: &str,
        name: &str,
        display_name: &str,
        purpose: &str,
        header: &str,
        creator_id: Uuid,
    ) -> ApiResult<Channel> {
        let channel: Channel = sqlx::query_as(
            r#"
            INSERT INTO channels (team_id, type, name, display_name, purpose, header, creator_id)
            VALUES ($1, $2::channel_type, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(team_id)
        .bind(channel_type)
        .bind(name)
        .bind(display_name)
        .bind(purpose)
        .bind(header)
        .bind(creator_id)
        .fetch_one(self.pool)
        .await?;
        Ok(channel)
    }

    /// Update channel privacy (type)
    pub async fn update_privacy(&self, id: Uuid, channel_type: &str) -> ApiResult<Channel> {
        let channel: Channel = sqlx::query_as(
            r#"UPDATE channels SET type = $2::channel_type, updated_at = NOW() WHERE id = $1 RETURNING *"#,
        )
        .bind(id)
        .bind(channel_type)
        .fetch_one(self.pool)
        .await?;
        Ok(channel)
    }

    /// Restore a soft-deleted channel
    pub async fn restore(&self, id: Uuid) -> ApiResult<Channel> {
        let channel: Channel = sqlx::query_as(
            r#"UPDATE channels SET deleted_at = NULL, updated_at = NOW() WHERE id = $1 RETURNING *"#,
        )
        .bind(id)
        .fetch_one(self.pool)
        .await?;
        Ok(channel)
    }

    /// Move a channel to another team
    pub async fn move_to_team(&self, id: Uuid, new_team_id: Uuid) -> ApiResult<Channel> {
        let channel: Channel = sqlx::query_as(
            r#"UPDATE channels SET team_id = $2, updated_at = NOW() WHERE id = $1 RETURNING *"#,
        )
        .bind(id)
        .bind(new_team_id)
        .fetch_one(self.pool)
        .await?;
        Ok(channel)
    }

    /// Get channel type and team_id
    pub async fn get_channel_type_and_team(
        &self,
        channel_id: Uuid,
    ) -> ApiResult<Option<(Option<Uuid>, String)>> {
        let row: Option<(Option<Uuid>, String)> =
            sqlx::query_as("SELECT team_id, type::text FROM channels WHERE id = $1")
                .bind(channel_id)
                .fetch_optional(self.pool)
                .await?;
        Ok(row)
    }

    /// List groups associated with a channel
    pub async fn list_channel_groups(
        &self,
        channel_id: Uuid,
        filter_allow_reference: bool,
        search_term: &str,
    ) -> ApiResult<Vec<ChannelGroupRow>> {
        let rows = sqlx::query_as::<_, ChannelGroupRow>(
            r#"
            SELECT
                g.id,
                g.name,
                g.display_name,
                g.description,
                g.source,
                g.remote_id,
                g.allow_reference,
                g.created_at,
                g.updated_at,
                g.deleted_at,
                gs.scheme_admin,
                EXISTS(
                    SELECT 1
                    FROM group_syncables gs2
                    WHERE gs2.group_id = g.id
                      AND gs2.delete_at IS NULL
                ) AS has_syncables,
                (
                    SELECT COUNT(*)
                    FROM group_members gm
                    WHERE gm.group_id = g.id
                ) AS member_count
            FROM groups g
            JOIN group_syncables gs
              ON gs.group_id = g.id
             AND gs.syncable_type = 'channel'
             AND gs.syncable_id = $1
             AND gs.delete_at IS NULL
            WHERE g.deleted_at IS NULL
              AND ($2 = FALSE OR g.allow_reference = TRUE)
              AND (
                    $3 = ''
                    OR LOWER(COALESCE(g.name, '')) LIKE $4
                    OR LOWER(g.display_name) LIKE $4
              )
            ORDER BY g.display_name ASC
            "#,
        )
        .bind(channel_id)
        .bind(filter_allow_reference)
        .bind(search_term)
        .bind(format!(
            "%{}%",
            escape_like_pattern(&search_term.to_lowercase())
        ))
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    /// Mark a channel as read for a user (updates channel_members and channel_reads).
    pub async fn mark_channel_read(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE channel_members SET last_viewed_at = NOW(), manually_unread = false, last_update_at = NOW() WHERE channel_id = $1 AND user_id = $2",
        )
        .bind(channel_id)
        .bind(user_id)
        .execute(self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO channel_reads (user_id, channel_id, last_read_message_id, last_read_at)
            VALUES ($1, $2, (SELECT MAX(seq) FROM posts WHERE channel_id = $2), NOW())
            ON CONFLICT (user_id, channel_id)
            DO UPDATE SET last_read_message_id = EXCLUDED.last_read_message_id, last_read_at = NOW()
            "#,
        )
        .bind(user_id)
        .bind(channel_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Mark a channel as unread for a user.
    pub async fn mark_channel_unread(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        mark_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE channel_members SET last_viewed_at = $3, manually_unread = true, last_update_at = NOW() WHERE channel_id = $1 AND user_id = $2",
        )
        .bind(channel_id)
        .bind(user_id)
        .bind(mark_time)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Find an existing direct channel between users in a team.
    pub async fn find_direct_channel(
        &self,
        team_id: Uuid,
        canonical_name: &str,
        legacy_name: &str,
    ) -> Result<Option<Channel>, sqlx::Error> {
        sqlx::query_as::<_, Channel>(
            r#"
            SELECT *
            FROM channels
            WHERE team_id = $1
              AND type = 'direct'::channel_type
              AND (name = $2 OR name = $3)
            ORDER BY created_at ASC
            LIMIT 1
            "#,
        )
        .bind(team_id)
        .bind(canonical_name)
        .bind(legacy_name)
        .fetch_optional(self.pool)
        .await
    }

    /// Create a direct channel.
    pub async fn create_direct_channel(
        &self,
        team_id: Uuid,
        name: &str,
        display_name: &str,
        creator_id: Uuid,
    ) -> Result<Channel, sqlx::Error> {
        sqlx::query_as::<_, Channel>(
            r#"
            INSERT INTO channels (team_id, type, name, display_name, purpose, header, creator_id)
            VALUES ($1, 'direct', $2, $3, '', '', $4)
            ON CONFLICT (team_id, name) DO UPDATE SET
                name = EXCLUDED.name,
                display_name = CASE
                    WHEN channels.display_name IS NULL OR channels.display_name = '' THEN EXCLUDED.display_name
                    ELSE channels.display_name
                END
            RETURNING *
            "#,
        )
        .bind(team_id)
        .bind(name)
        .bind(display_name)
        .bind(creator_id)
        .fetch_one(self.pool)
        .await
    }

    /// Create a group channel.
    pub async fn create_group_channel(
        &self,
        team_id: Uuid,
        name: &str,
        display_name: &str,
        creator_id: Uuid,
    ) -> Result<Channel, sqlx::Error> {
        sqlx::query_as::<_, Channel>(
            r#"
            INSERT INTO channels (team_id, type, name, display_name, purpose, header, creator_id)
            VALUES ($1, 'group', $2, $3, '', '', $4)
            ON CONFLICT (team_id, name) DO UPDATE SET name = EXCLUDED.name
            RETURNING *
            "#,
        )
        .bind(team_id)
        .bind(name)
        .bind(display_name)
        .bind(creator_id)
        .fetch_one(self.pool)
        .await
    }

    /// Get the display name of the other participant in a direct channel.
    pub async fn get_dm_display_name(
        &self,
        channel_id: Uuid,
        viewer_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT COALESCE(NULLIF(u.display_name, ''), u.username)
            FROM channel_members cm
            JOIN users u ON u.id = cm.user_id
            WHERE cm.channel_id = $1
              AND cm.user_id <> $2
            ORDER BY u.username ASC
            LIMIT 1
            "#,
        )
        .bind(channel_id)
        .bind(viewer_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Batch-get display names for multiple direct channels.
    pub async fn get_dm_display_names(
        &self,
        channel_ids: &[Uuid],
        viewer_id: Uuid,
    ) -> Result<HashMap<Uuid, String>, sqlx::Error> {
        if channel_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            r#"
            SELECT cm.channel_id, COALESCE(NULLIF(u.display_name, ''), u.username)
            FROM channel_members cm
            JOIN users u ON u.id = cm.user_id
            WHERE cm.channel_id = ANY($1)
              AND cm.user_id <> $2
            "#,
        )
        .bind(channel_ids)
        .bind(viewer_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().collect())
    }

    /// Update channel_reads for a user, setting last_read to the latest post seq.
    pub async fn update_channel_reads_to_latest(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO channel_reads (user_id, channel_id, last_read_message_id, last_read_at)
            VALUES ($1, $2, (SELECT MAX(seq) FROM posts WHERE channel_id = $2), NOW())
            ON CONFLICT (user_id, channel_id)
            DO UPDATE SET last_read_message_id = EXCLUDED.last_read_message_id, last_read_at = NOW()
            "#,
        )
        .bind(user_id)
        .bind(channel_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Count members in a channel.
    pub async fn count_members(&self, channel_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM channel_members WHERE channel_id = $1")
            .bind(channel_id)
            .fetch_one(self.pool)
            .await
    }

    /// List channels for a user in a team with optional deleted-filter.
    pub async fn list_team_channels_for_user(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        include_deleted: bool,
        last_delete_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<Channel>, sqlx::Error> {
        if include_deleted {
            sqlx::query_as(
                r#"
                SELECT c.* FROM channels c
                JOIN channel_members cm ON c.id = cm.channel_id
                WHERE c.team_id = $1 AND cm.user_id = $2
                "#,
            )
            .bind(team_id)
            .bind(user_id)
            .fetch_all(self.pool)
            .await
        } else if let Some(ts) = last_delete_at {
            sqlx::query_as(
                r#"
                SELECT c.* FROM channels c
                JOIN channel_members cm ON c.id = cm.channel_id
                WHERE c.team_id = $1 AND cm.user_id = $2
                  AND (c.deleted_at IS NULL OR c.deleted_at >= $3)
                "#,
            )
            .bind(team_id)
            .bind(user_id)
            .bind(ts)
            .fetch_all(self.pool)
            .await
        } else {
            sqlx::query_as(
                r#"
                SELECT c.* FROM channels c
                JOIN channel_members cm ON c.id = cm.channel_id
                WHERE c.team_id = $1 AND cm.user_id = $2 AND c.deleted_at IS NULL
                "#,
            )
            .bind(team_id)
            .bind(user_id)
            .fetch_all(self.pool)
            .await
        }
    }

    /// List all channels a user is a member of, optionally filtered by update time.
    pub async fn list_user_channels(
        &self,
        user_id: Uuid,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<Channel>, sqlx::Error> {
        if let Some(ts) = since {
            sqlx::query_as(
                r#"
                SELECT c.* FROM channels c
                JOIN channel_members cm ON c.id = cm.channel_id
                WHERE cm.user_id = $1 AND c.updated_at >= $2
                "#,
            )
            .bind(user_id)
            .bind(ts)
            .fetch_all(self.pool)
            .await
        } else {
            sqlx::query_as(
                r#"
                SELECT c.* FROM channels c
                JOIN channel_members cm ON c.id = cm.channel_id
                WHERE cm.user_id = $1
                "#,
            )
            .bind(user_id)
            .fetch_all(self.pool)
            .await
        }
    }

    /// List public/private channels in a team that a user is NOT a member of (paginated).
    pub async fn list_not_member_channels(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Channel>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT c.*
            FROM channels c
            WHERE c.team_id = $1
              AND c.is_archived = false
              AND c.type IN ('public', 'private')
              AND NOT EXISTS (
                  SELECT 1 FROM channel_members cm
                  WHERE cm.channel_id = c.id AND cm.user_id = $2
              )
            ORDER BY COALESCE(c.display_name, c.name) ASC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
    }

    /// List channel IDs visible to a user in a team (public + member-only).
    pub async fn list_team_channel_ids(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT DISTINCT c.id
            FROM channels c
            LEFT JOIN channel_members cm ON c.id = cm.channel_id AND cm.user_id = $2
            WHERE c.team_id = $1 AND (c.type = 'public' OR cm.user_id IS NOT NULL)
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_all(self.pool)
        .await
    }

    /// List private channels for a user in a team.
    pub async fn list_team_private_channels(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Channel>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT c.*
            FROM channels c
            JOIN channel_members cm ON c.id = cm.channel_id
            WHERE c.team_id = $1 AND c.type = 'private' AND cm.user_id = $2
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_all(self.pool)
        .await
    }

    /// List deleted/archived channels visible to a user in a team.
    pub async fn list_team_deleted_channels(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Channel>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT c.* FROM channels c
            WHERE c.team_id = $1 AND c.is_archived = true
              AND (
                c.type != 'private'
                OR EXISTS (
                    SELECT 1 FROM channel_members cm
                    WHERE cm.channel_id = c.id AND cm.user_id = $2
                )
              )
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_all(self.pool)
        .await
    }

    /// Get a channel by team ID and name.
    pub async fn get_channel_by_name(
        &self,
        team_id: Uuid,
        name: &str,
    ) -> Result<Option<Channel>, sqlx::Error> {
        sqlx::query_as::<_, Channel>("SELECT * FROM channels WHERE team_id = $1 AND name = $2")
            .bind(team_id)
            .bind(name)
            .fetch_optional(self.pool)
            .await
    }

    /// Get member IDs in a channel from a list of user IDs
    pub async fn get_member_ids_by_user_ids(
        &self,
        channel_id: Uuid,
        user_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>(
            "SELECT user_id FROM channel_members WHERE channel_id = $1 AND user_id = ANY($2)",
        )
        .bind(channel_id)
        .bind(user_ids)
        .fetch_all(self.pool)
        .await
    }

    /// Get channel info for push notifications
    pub async fn get_channel_push_info(
        &self,
        channel_id: Uuid,
    ) -> Result<Option<(String, String, String)>, sqlx::Error> {
        sqlx::query_as::<_, (String, String, String)>(
            "SELECT c.name, c.display_name, c.type::text as channel_type FROM channels c WHERE c.id = $1",
        )
        .bind(channel_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Get the other participant in a DM channel
    pub async fn get_other_dm_participant(
        &self,
        channel_id: Uuid,
        exclude_user_id: Uuid,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>(
            "SELECT user_id FROM channel_members WHERE channel_id = $1 AND user_id != $2",
        )
        .bind(channel_id)
        .bind(exclude_user_id)
        .fetch_all(self.pool)
        .await
    }

    /// Get member IDs by usernames in a channel
    pub async fn get_member_ids_by_usernames(
        &self,
        channel_id: Uuid,
        usernames: &[&str],
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>(
            "SELECT cm.user_id FROM channel_members cm JOIN users u ON cm.user_id = u.id WHERE cm.channel_id = $1 AND u.username = ANY($2)",
        )
        .bind(channel_id)
        .bind(usernames)
        .fetch_all(self.pool)
        .await
    }

    /// Ensure a user is a member of a channel (insert if missing)
    pub async fn ensure_membership(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            INSERT INTO channel_members (channel_id, user_id, role)
            VALUES ($1, $2, 'member')
            ON CONFLICT (channel_id, user_id) DO NOTHING
            RETURNING user_id
            "#,
        )
        .bind(channel_id)
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
    }
}
