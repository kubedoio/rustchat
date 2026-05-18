//! Post repository for centralized query patterns
//!
//! This module centralizes common post query patterns to reduce the 20+ duplicated
//! queries previously scattered across the codebase (api/posts.rs, api/v4/posts.rs, etc.)

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{ApiResult, AppError};
use crate::models::{ChannelMember, FileInfo};

/// Post with joined user info
///
/// This struct combines post fields with user information from a JOIN query.
/// All user fields are Option<> because LEFT JOIN may return NULL if the user
/// was deleted (though this shouldn't happen in normal operation due to FK constraints).
#[derive(Debug, Clone)]
pub struct PostWithUser {
    // Post fields
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub root_post_id: Option<Uuid>,
    pub message: String,
    pub props: Option<serde_json::Value>,
    pub file_ids: Vec<Uuid>,
    pub is_pinned: bool,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub reply_count: i64,
    pub last_reply_at: Option<DateTime<Utc>>,
    pub seq: i64,
    // User fields
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
}

/// Channel unread statistics
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChannelUnreadStats {
    pub total_msg_count: i64,
    pub total_msg_count_root: i64,
    pub unread_msg_count: i64,
    pub unread_msg_count_root: i64,
    pub mention_count: i64,
    pub mention_count_root: i64,
    pub urgent_mention_count: i64,
}

/// Thread snapshot row for CRT thread responses
#[derive(sqlx::FromRow)]
pub struct ThreadSnapshotRow {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub message: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub reply_count: i64,
    pub last_reply_at: Option<chrono::DateTime<chrono::Utc>>,
    pub following: bool,
    pub last_read_at: Option<chrono::DateTime<chrono::Utc>>,
    pub mention_count: i32,
    pub unread_replies_count: i32,
}

/// Repository for post-related database operations
#[derive(Debug, Clone)]
pub struct PostRepository {
    db: PgPool,
}

impl PostRepository {
    /// Create a new PostRepository instance
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Common SELECT columns for post queries with user JOIN
    const POST_COLUMNS: &'static str = r#"
        p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
        p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
        p.reply_count::int8 as reply_count, p.last_reply_at, p.seq,
        u.username, u.avatar_url, u.email
    "#;

    /// Get a single post by ID with user info
    ///
    /// Returns None if the post doesn't exist or has been soft-deleted.
    ///
    /// # Example
    /// ```rust,ignore
    /// let post_repo = PostRepository::new(pool);
    /// if let Some(post) = post_repo.find_by_id(post_id).await? {
    ///     println!("Found post by {}", post.username.unwrap_or_default());
    /// }
    /// ```
    pub async fn find_by_id(&self, post_id: Uuid) -> ApiResult<Option<PostWithUser>> {
        let post = sqlx::query_as::<_, PostWithUserRow>(&format!(
            r#"
                SELECT {}
                FROM posts p
                LEFT JOIN users u ON p.user_id = u.id
                WHERE p.id = $1 AND p.deleted_at IS NULL
                "#,
            Self::POST_COLUMNS
        ))
        .bind(post_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(post.map(Into::into))
    }

    /// List posts in a channel with pagination
    ///
    /// Returns posts ordered by creation time (newest first).
    /// This is the common pattern used for channel message history.
    ///
    /// # Arguments
    /// * `channel_id` - The channel to query
    /// * `limit` - Maximum number of posts to return
    /// * `offset` - Number of posts to skip (for pagination)
    pub async fn list_by_channel(
        &self,
        channel_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<PostWithUser>> {
        let rows = sqlx::query_as::<_, PostWithUserRow>(&format!(
            r#"
                SELECT {}
                FROM posts p
                LEFT JOIN users u ON p.user_id = u.id
                WHERE p.channel_id = $1
                  AND p.deleted_at IS NULL
                ORDER BY p.created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            Self::POST_COLUMNS
        ))
        .bind(channel_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Get thread replies
    ///
    /// Returns all non-deleted replies to a root post, ordered chronologically.
    /// The root post itself is NOT included in the results.
    ///
    /// # Arguments
    /// * `root_post_id` - The ID of the parent/root post
    pub async fn get_thread_replies(
        &self,
        root_post_id: Uuid,
        limit: i64,
    ) -> ApiResult<Vec<PostWithUser>> {
        let limit = if limit <= 0 { 200 } else { limit.min(500) };
        let rows = sqlx::query_as::<_, PostWithUserRow>(&format!(
            r#"
                SELECT {}
                FROM posts p
                LEFT JOIN users u ON p.user_id = u.id
                WHERE p.root_post_id = $1
                  AND p.deleted_at IS NULL
                ORDER BY p.created_at ASC
                LIMIT $2
                "#,
            Self::POST_COLUMNS
        ))
        .bind(root_post_id)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// List posts since a timestamp (for sync)
    ///
    /// Returns all posts in a channel created or updated after the given timestamp.
    /// Used by mobile clients and sync endpoints.
    ///
    /// # Arguments
    /// * `channel_id` - The channel to query
    /// * `since` - Return posts with created_at > this timestamp
    pub async fn list_since(
        &self,
        channel_id: Uuid,
        since: DateTime<Utc>,
        limit: i64,
    ) -> ApiResult<Vec<PostWithUser>> {
        let limit = if limit <= 0 { 500 } else { limit.min(1000) };
        let rows = sqlx::query_as::<_, PostWithUserRow>(&format!(
            r#"
                SELECT {}
                FROM posts p
                LEFT JOIN users u ON p.user_id = u.id
                WHERE p.channel_id = $1
                  AND p.deleted_at IS NULL
                  AND p.created_at > $2
                ORDER BY p.created_at ASC
                LIMIT $3
                "#,
            Self::POST_COLUMNS
        ))
        .bind(channel_id)
        .bind(since)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// List posts before a timestamp with pagination
    ///
    /// Used for infinite scroll "load older messages" functionality.
    /// Returns posts with created_at < before, ordered newest first.
    pub async fn list_before(
        &self,
        channel_id: Uuid,
        before: DateTime<Utc>,
        limit: i64,
    ) -> ApiResult<Vec<PostWithUser>> {
        let rows = sqlx::query_as::<_, PostWithUserRow>(&format!(
            r#"
                SELECT {}
                FROM posts p
                LEFT JOIN users u ON p.user_id = u.id
                WHERE p.channel_id = $1
                  AND p.deleted_at IS NULL
                  AND p.created_at < $2
                ORDER BY p.created_at DESC
                LIMIT $3
                "#,
            Self::POST_COLUMNS
        ))
        .bind(channel_id)
        .bind(before)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// List posts after a timestamp with pagination
    ///
    /// Used for "load newer messages" functionality.
    /// Returns posts with created_at > after, ordered oldest first.
    pub async fn list_after(
        &self,
        channel_id: Uuid,
        after: DateTime<Utc>,
        limit: i64,
    ) -> ApiResult<Vec<PostWithUser>> {
        let rows = sqlx::query_as::<_, PostWithUserRow>(&format!(
            r#"
                SELECT {}
                FROM posts p
                LEFT JOIN users u ON p.user_id = u.id
                WHERE p.channel_id = $1
                  AND p.deleted_at IS NULL
                  AND p.created_at > $2
                ORDER BY p.created_at ASC
                LIMIT $3
                "#,
            Self::POST_COLUMNS
        ))
        .bind(channel_id)
        .bind(after)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Require channel membership for a user
    ///
    /// Returns the ChannelMember if found, otherwise Forbidden.
    pub async fn require_channel_membership(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
    ) -> ApiResult<ChannelMember> {
        let membership: Option<ChannelMember> =
            sqlx::query_as("SELECT * FROM channel_members WHERE channel_id = $1 AND user_id = $2")
                .bind(channel_id)
                .bind(user_id)
                .fetch_optional(&self.db)
                .await?;

        membership.ok_or_else(|| AppError::Forbidden("Not a member of this channel".to_string()))
    }

    /// Get the channel_id for a post
    pub async fn get_post_channel_id(&self, post_id: Uuid) -> ApiResult<Uuid> {
        let channel_id: Uuid =
            sqlx::query_scalar("SELECT channel_id FROM posts WHERE id = $1")
                .bind(post_id)
                .fetch_one(&self.db)
                .await?;
        Ok(channel_id)
    }

    /// Get basic post info (user_id, channel_id, created_at, message)
    pub async fn get_post_basic(
        &self,
        post_id: Uuid,
    ) -> ApiResult<Option<(Uuid, Uuid, chrono::DateTime<chrono::Utc>, String)>> {
        let result = sqlx::query_as(
            "SELECT user_id, channel_id, created_at, message FROM posts WHERE id = $1",
        )
        .bind(post_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(result)
    }

    /// Pin a post
    pub async fn pin_post(&self, post_id: Uuid) -> ApiResult<()> {
        sqlx::query("UPDATE posts SET is_pinned = true WHERE id = $1")
            .bind(post_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// Unpin a post
    pub async fn unpin_post(&self, post_id: Uuid) -> ApiResult<()> {
        sqlx::query("UPDATE posts SET is_pinned = false WHERE id = $1")
            .bind(post_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// Soft delete a post and return the deleted post with user info
    pub async fn soft_delete_post(&self, post_id: Uuid) -> ApiResult<PostWithUser> {
        let row = sqlx::query_as::<_, PostWithUserRow>(
            r#"
            WITH updated_post AS (
                UPDATE posts SET deleted_at = NOW() WHERE id = $1
                RETURNING *
            )
            SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                   p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                   p.reply_count::int8 as reply_count,
                   p.last_reply_at, p.seq,
                   u.username, u.avatar_url, u.email
            FROM updated_post p
            LEFT JOIN users u ON p.user_id = u.id
            "#,
        )
        .bind(post_id)
        .fetch_one(&self.db)
        .await?;

        Ok(row.into())
    }

    /// Update post message and return the updated post with user info
    pub async fn update_post_message(
        &self,
        post_id: Uuid,
        message: String,
    ) -> ApiResult<PostWithUser> {
        let row = sqlx::query_as::<_, PostWithUserRow>(
            r#"
            WITH updated_post AS (
                UPDATE posts SET message = $1, edited_at = NOW()
                WHERE id = $2
                RETURNING *
            )
            SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                   p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                   p.reply_count::int8 as reply_count,
                   p.last_reply_at, p.seq,
                   u.username, u.avatar_url, u.email
            FROM updated_post p
            LEFT JOIN users u ON p.user_id = u.id
            "#,
        )
        .bind(message)
        .bind(post_id)
        .fetch_one(&self.db)
        .await?;

        Ok(row.into())
    }

    /// Get posts by IDs with visibility check for a user
    pub async fn get_posts_by_ids_for_user(
        &self,
        post_ids: &[Uuid],
        user_id: Uuid,
    ) -> ApiResult<Vec<PostWithUser>> {
        let rows = sqlx::query_as::<_, PostWithUserRow>(&format!(
            r#"
            SELECT {}
            FROM posts p
            LEFT JOIN users u ON p.user_id = u.id
            JOIN channel_members cm ON cm.channel_id = p.channel_id AND cm.user_id = $2
            WHERE p.id = ANY($1) AND p.deleted_at IS NULL
            "#,
            Self::POST_COLUMNS
        ))
        .bind(post_ids)
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Get visible post IDs for a user
    pub async fn get_visible_post_ids(
        &self,
        post_ids: &[Uuid],
        user_id: Uuid,
    ) -> ApiResult<Vec<Uuid>> {
        let ids = sqlx::query_scalar(
            r#"
            SELECT p.id
            FROM posts p
            JOIN channel_members cm ON cm.channel_id = p.channel_id AND cm.user_id = $2
            WHERE p.id = ANY($1) AND p.deleted_at IS NULL
            "#,
        )
        .bind(post_ids)
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(ids)
    }

    /// Get file info for a list of file IDs
    pub async fn get_post_files(&self, file_ids: &[Uuid]) -> ApiResult<Vec<FileInfo>> {
        let files = sqlx::query_as("SELECT * FROM files WHERE id = ANY($1)")
            .bind(file_ids)
            .fetch_all(&self.db)
            .await?;
        Ok(files)
    }

    /// Load the post edit time limit from server config
    pub async fn load_post_edit_time_limit_seconds(&self) -> ApiResult<i64> {
        let limit = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE((site->>'post_edit_time_limit_seconds')::bigint, -1) FROM server_config WHERE id = 'default'",
        )
        .fetch_optional(&self.db)
        .await?
        .unwrap_or(-1);
        Ok(limit)
    }

    /// Get flagged/saved posts for a user
    pub async fn get_flagged_posts(&self, user_id: Uuid) -> ApiResult<Vec<PostWithUser>> {
        let rows = sqlx::query_as::<_, PostWithUserRow>(&format!(
            r#"
            SELECT {}
            FROM saved_posts s
            JOIN posts p ON s.post_id = p.id
            LEFT JOIN users u ON p.user_id = u.id
            WHERE s.user_id = $1 AND p.deleted_at IS NULL
            ORDER BY s.created_at DESC
            "#,
            Self::POST_COLUMNS
        ))
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// List scheduled posts for a user in a team
    #[allow(clippy::type_complexity)]
    pub async fn list_scheduled_posts(
        &self,
        user_id: Uuid,
        team_id: Uuid,
    ) -> ApiResult<
        Vec<(
            Uuid,
            Uuid,
            Uuid,
            Option<Uuid>,
            String,
            serde_json::Value,
            Vec<Uuid>,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        )>,
    > {
        let rows = sqlx::query_as(
            r#"
            SELECT id, user_id, channel_id, root_id, message, props, file_ids, scheduled_at, created_at, updated_at
            FROM scheduled_posts
            WHERE user_id = $1 AND channel_id IN (SELECT id FROM channels WHERE team_id = $2)
            AND state = 'pending'
            "#,
        )
        .bind(user_id)
        .bind(team_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rows)
    }

    /// Create a scheduled post
    pub async fn create_scheduled_post(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        root_id: Option<Uuid>,
        message: &str,
        props: &serde_json::Value,
        file_ids: &[Uuid],
        scheduled_at: chrono::DateTime<chrono::Utc>,
    ) -> ApiResult<(
        Uuid,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
    )> {
        let row = sqlx::query_as(
            r#"
            INSERT INTO scheduled_posts (user_id, channel_id, root_id, message, props, file_ids, scheduled_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(channel_id)
        .bind(root_id)
        .bind(message)
        .bind(props)
        .bind(file_ids)
        .bind(scheduled_at)
        .fetch_one(&self.db)
        .await?;

        Ok(row)
    }

    /// Update a scheduled post
    pub async fn update_scheduled_post(
        &self,
        scheduled_id: Uuid,
        user_id: Uuid,
        channel_id: Uuid,
        root_id: Option<Uuid>,
        message: &str,
        props: &serde_json::Value,
        file_ids: &[Uuid],
        scheduled_at: chrono::DateTime<chrono::Utc>,
    ) -> ApiResult<Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>> {
        let row = sqlx::query_as(
            r#"
            UPDATE scheduled_posts
            SET channel_id = $1,
                root_id = $2,
                message = $3,
                props = $4,
                file_ids = $5,
                scheduled_at = $6,
                updated_at = NOW()
            WHERE id = $7 AND user_id = $8
            RETURNING created_at, updated_at
            "#,
        )
        .bind(channel_id)
        .bind(root_id)
        .bind(message)
        .bind(props)
        .bind(file_ids)
        .bind(scheduled_at)
        .bind(scheduled_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row)
    }

    /// Delete a scheduled post
    #[allow(clippy::type_complexity)]
    pub async fn delete_scheduled_post(
        &self,
        scheduled_id: Uuid,
        user_id: Uuid,
    ) -> ApiResult<
        Option<(
            Uuid,
            Uuid,
            String,
            String,
            serde_json::Value,
            Vec<Uuid>,
            i64,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        )>,
    > {
        let row = sqlx::query_as(
            r#"
            DELETE FROM scheduled_posts
            WHERE id = $1 AND user_id = $2 AND processed_at = 0
            RETURNING channel_id, user_id, root_id::text, message, props, file_ids, scheduled_at, create_at, update_at
            "#,
        )
        .bind(scheduled_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row)
    }

    /// Set a post reminder for a user
    pub async fn set_post_reminder(
        &self,
        user_id: Uuid,
        post_id: Uuid,
        target_at: chrono::DateTime<chrono::Utc>,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            INSERT INTO post_reminders (user_id, post_id, target_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, post_id) DO UPDATE SET target_at = $3
            "#,
        )
        .bind(user_id)
        .bind(post_id)
        .bind(target_at)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// Check if collapsed reply threads (CRT) is enabled for a user
    pub async fn is_crt_enabled_for_user(
        &self,
        user_id: Uuid,
        collapsed_threads_enabled: bool,
    ) -> ApiResult<bool> {
        if !collapsed_threads_enabled {
            return Ok(false);
        }

        let pref_value: Option<String> = sqlx::query_scalar(
            r#"
            SELECT value
            FROM mattermost_preferences
            WHERE user_id = $1
              AND category = 'display_settings'
              AND name = 'collapsed_reply_threads'
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        let enabled = pref_value
            .as_deref()
            .map(|v| {
                let normalized = v.trim().to_ascii_lowercase();
                normalized == "on" || normalized == "true" || normalized == "1"
            })
            .unwrap_or(true);

        Ok(enabled)
    }

    /// Get username for a user ID
    pub async fn get_username(&self, user_id: Uuid) -> ApiResult<String> {
        let username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.db)
            .await?;
        Ok(username)
    }

    /// Get post with channel info
    pub async fn get_post_with_channel(
        &self,
        post_id: Uuid,
    ) -> ApiResult<Option<(Uuid, Uuid, i64, Option<Uuid>, chrono::DateTime<chrono::Utc>)>> {
        let result = sqlx::query_as(
            r#"
            SELECT p.channel_id, c.team_id, p.seq, p.root_post_id, p.created_at
            FROM posts p
            JOIN channels c ON p.channel_id = c.id
            WHERE p.id = $1 AND p.deleted_at IS NULL
            "#,
        )
        .bind(post_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(result)
    }

    /// Compute channel unread statistics from a last read post ID
    pub async fn compute_channel_unread(
        &self,
        channel_id: Uuid,
        last_read_id: i64,
        username: &str,
    ) -> ApiResult<ChannelUnreadStats> {
        let stats = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE p.deleted_at IS NULL)::BIGINT AS total_msg_count,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.root_post_id IS NULL
                )::BIGINT AS total_msg_count_root,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > $2
                )::BIGINT AS unread_msg_count,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > $2
                      AND p.root_post_id IS NULL
                )::BIGINT AS unread_msg_count_root,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > $2
                      AND (
                          p.message LIKE '%@' || $3 || '%'
                          OR p.message LIKE '%@all%'
                          OR p.message LIKE '%@channel%'
                      )
                )::BIGINT AS mention_count,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > $2
                      AND p.root_post_id IS NULL
                      AND (
                          p.message LIKE '%@' || $3 || '%'
                          OR p.message LIKE '%@all%'
                          OR p.message LIKE '%@channel%'
                      )
                )::BIGINT AS mention_count_root,
                COUNT(*) FILTER (
                    WHERE p.deleted_at IS NULL
                      AND p.seq > $2
                      AND (
                          p.message LIKE '%@' || $3 || '%'
                          OR p.message LIKE '%@all%'
                          OR p.message LIKE '%@channel%'
                      )
                      AND p.message LIKE '%@here%'
                )::BIGINT AS urgent_mention_count
            FROM posts p
            WHERE p.channel_id = $1
            "#,
        )
        .bind(channel_id)
        .bind(last_read_id)
        .bind(username)
        .fetch_one(&self.db)
        .await?;

        Ok(stats)
    }

    /// Get thread unread counts for a thread root
    pub async fn get_thread_unread_counts(
        &self,
        thread_root_id: Uuid,
        username: &str,
        mark_view_at: chrono::DateTime<chrono::Utc>,
    ) -> ApiResult<(i64, i64)> {
        let counts = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE p.deleted_at IS NULL AND p.created_at > $3)::BIGINT AS unread_replies_count,
                COUNT(*) FILTER (WHERE p.deleted_at IS NULL AND p.created_at > $3 AND (p.message LIKE '%@' || $2 || '%' OR p.message LIKE '%@all%' OR p.message LIKE '%@channel%'))::BIGINT AS mention_count
            FROM posts p
            WHERE p.root_post_id = $1
            "#,
        )
        .bind(thread_root_id)
        .bind(username)
        .bind(mark_view_at)
        .fetch_one(&self.db)
        .await?;

        Ok(counts)
    }

    /// Upsert channel read state
    pub async fn upsert_channel_read(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        last_read_id: i64,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            INSERT INTO channel_reads (user_id, channel_id, last_read_message_id, last_read_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (user_id, channel_id)
            DO UPDATE SET last_read_message_id = $3, last_read_at = NOW()
            "#,
        )
        .bind(user_id)
        .bind(channel_id)
        .bind(last_read_id)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// Upsert thread membership
    pub async fn upsert_thread_membership(
        &self,
        user_id: Uuid,
        thread_root_id: Uuid,
        mark_view_at: chrono::DateTime<chrono::Utc>,
        mention_count: i32,
        unread_replies_count: i32,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            INSERT INTO thread_memberships (user_id, post_id, following, last_read_at, mention_count, unread_replies_count, updated_at)
            VALUES ($1, $2, true, $3, $4, $5, NOW())
            ON CONFLICT (user_id, post_id)
            DO UPDATE SET following = true, last_read_at = $3, mention_count = $4, unread_replies_count = $5, updated_at = NOW()
            "#,
        )
        .bind(user_id)
        .bind(thread_root_id)
        .bind(mark_view_at)
        .bind(mention_count)
        .bind(unread_replies_count)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// Update channel member unread state
    pub async fn update_channel_member_unread(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        mark_view_at: chrono::DateTime<chrono::Utc>,
        msg_count: i64,
        mention_count: i64,
        msg_count_root: i64,
        mention_count_root: i64,
        urgent_mention_count: i64,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            UPDATE channel_members
            SET last_viewed_at = $3,
                manually_unread = true,
                msg_count = $4,
                mention_count = $5,
                msg_count_root = $6,
                mention_count_root = $7,
                urgent_mention_count = $8,
                last_update_at = NOW()
            WHERE channel_id = $1 AND user_id = $2
            "#,
        )
        .bind(channel_id)
        .bind(user_id)
        .bind(mark_view_at)
        .bind(msg_count)
        .bind(mention_count)
        .bind(msg_count_root)
        .bind(mention_count_root)
        .bind(urgent_mention_count)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// Fetch thread snapshot for a user
    pub async fn fetch_thread_snapshot(
        &self,
        thread_root_id: Uuid,
        user_id: Uuid,
    ) -> ApiResult<Option<ThreadSnapshotRow>> {
        let row = sqlx::query_as(
            r#"
            SELECT
                p.id,
                p.channel_id,
                p.user_id,
                p.message,
                p.created_at,
                p.reply_count::int8 AS reply_count,
                p.last_reply_at,
                COALESCE(tm.following, false) AS following,
                tm.last_read_at,
                COALESCE(tm.mention_count, 0)::int4 AS mention_count,
                COALESCE(tm.unread_replies_count, 0)::int4 AS unread_replies_count
            FROM posts p
            LEFT JOIN thread_memberships tm
                   ON tm.post_id = p.id
                  AND tm.user_id = $2
            WHERE p.id = $1
              AND p.root_post_id IS NULL
              AND p.deleted_at IS NULL
            "#,
        )
        .bind(thread_root_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row)
    }

    // ========================================================================
    // Thread queries
    // ========================================================================

    /// List threads for a user in a team
    pub async fn list_threads_for_user_in_team(
        &self,
        user_id: Uuid,
        team_id: Uuid,
        unread_only: bool,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<ThreadSnapshotRow>> {
        let rows = if unread_only {
            sqlx::query_as(
                r#"
                SELECT p.id, p.channel_id, p.user_id, p.message, p.created_at,
                       p.reply_count::int8 as reply_count, p.last_reply_at,
                       tm.following, tm.last_read_at, tm.mention_count, tm.unread_replies_count
                FROM posts p
                JOIN thread_memberships tm ON tm.post_id = p.id
                JOIN channels c ON p.channel_id = c.id
                WHERE tm.user_id = $1
                  AND tm.following = true
                  AND c.team_id = $2
                  AND p.root_post_id IS NULL
                  AND p.deleted_at IS NULL
                  AND (tm.unread_replies_count > 0 OR tm.mention_count > 0)
                ORDER BY COALESCE(p.last_reply_at, p.created_at) DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(user_id)
            .bind(team_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.db)
            .await?
        } else {
            sqlx::query_as(
                r#"
                SELECT p.id, p.channel_id, p.user_id, p.message, p.created_at,
                       p.reply_count::int8 as reply_count, p.last_reply_at,
                       tm.following, tm.last_read_at, tm.mention_count, tm.unread_replies_count
                FROM posts p
                JOIN thread_memberships tm ON tm.post_id = p.id
                JOIN channels c ON p.channel_id = c.id
                WHERE tm.user_id = $1
                  AND tm.following = true
                  AND c.team_id = $2
                  AND p.root_post_id IS NULL
                  AND p.deleted_at IS NULL
                ORDER BY COALESCE(p.last_reply_at, p.created_at) DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(user_id)
            .bind(team_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.db)
            .await?
        };
        Ok(rows)
    }

    /// Count total followed threads for a user in a team
    pub async fn count_threads_for_user_in_team(
        &self,
        user_id: Uuid,
        team_id: Uuid,
    ) -> ApiResult<i64> {
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM posts p
            JOIN thread_memberships tm ON tm.post_id = p.id
            JOIN channels c ON p.channel_id = c.id
            WHERE tm.user_id = $1
              AND tm.following = true
              AND c.team_id = $2
              AND p.root_post_id IS NULL
              AND p.deleted_at IS NULL
            "#,
        )
        .bind(user_id)
        .bind(team_id)
        .fetch_one(&self.db)
        .await?;
        Ok(total)
    }

    /// Count unread followed threads for a user in a team
    pub async fn count_unread_threads_for_user_in_team(
        &self,
        user_id: Uuid,
        team_id: Uuid,
    ) -> ApiResult<i64> {
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM posts p
            JOIN thread_memberships tm ON tm.post_id = p.id
            JOIN channels c ON p.channel_id = c.id
            WHERE tm.user_id = $1
              AND tm.following = true
              AND c.team_id = $2
              AND p.root_post_id IS NULL
              AND p.deleted_at IS NULL
              AND (tm.unread_replies_count > 0 OR tm.mention_count > 0)
            "#,
        )
        .bind(user_id)
        .bind(team_id)
        .fetch_one(&self.db)
        .await?;
        Ok(total)
    }

    /// Sum unread mentions across all followed threads for a user in a team
    pub async fn sum_unread_mentions_for_user_in_team(
        &self,
        user_id: Uuid,
        team_id: Uuid,
    ) -> ApiResult<i64> {
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(tm.mention_count), 0)
            FROM thread_memberships tm
            JOIN posts p ON tm.post_id = p.id
            JOIN channels c ON p.channel_id = c.id
            WHERE tm.user_id = $1
              AND tm.following = true
              AND c.team_id = $2
              AND p.root_post_id IS NULL
              AND p.deleted_at IS NULL
            "#,
        )
        .bind(user_id)
        .bind(team_id)
        .fetch_one(&self.db)
        .await?;
        Ok(total)
    }

    /// List all followed threads for a user (no team filter)
    pub async fn list_all_threads_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<ThreadSnapshotRow>> {
        let rows = sqlx::query_as(
            r#"
            SELECT p.id, p.channel_id, p.user_id, p.message, p.created_at,
                   p.reply_count::int8 as reply_count, p.last_reply_at,
                   tm.following, tm.last_read_at, tm.mention_count, tm.unread_replies_count
            FROM posts p
            JOIN thread_memberships tm ON tm.post_id = p.id
            WHERE tm.user_id = $1
              AND tm.following = true
              AND p.root_post_id IS NULL
              AND p.deleted_at IS NULL
            ORDER BY COALESCE(p.last_reply_at, p.created_at) DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;
        Ok(rows)
    }

    /// Count all followed threads for a user (no team filter)
    pub async fn count_all_threads_for_user(&self, user_id: Uuid) -> ApiResult<i64> {
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM thread_memberships tm
            JOIN posts p ON tm.post_id = p.id
            WHERE tm.user_id = $1 AND tm.following = true AND p.deleted_at IS NULL
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;
        Ok(total)
    }

    /// Get a single thread by ID for a user in a team
    pub async fn get_thread_for_user_in_team(
        &self,
        thread_id: Uuid,
        user_id: Uuid,
        team_id: Uuid,
    ) -> ApiResult<Option<ThreadSnapshotRow>> {
        let row = sqlx::query_as(
            r#"
            SELECT p.id, p.channel_id, p.user_id, p.message, p.created_at,
                   p.reply_count::int8 as reply_count, p.last_reply_at,
                   COALESCE(tm.following, false) as following,
                   tm.last_read_at,
                   COALESCE(tm.mention_count, 0) as mention_count,
                   COALESCE(tm.unread_replies_count, 0) as unread_replies_count
            FROM posts p
            JOIN channels c ON p.channel_id = c.id
            LEFT JOIN thread_memberships tm ON tm.post_id = p.id AND tm.user_id = $2
            WHERE p.id = $1
              AND c.team_id = $3
              AND p.root_post_id IS NULL
              AND p.deleted_at IS NULL
            "#,
        )
        .bind(thread_id)
        .bind(user_id)
        .bind(team_id)
        .fetch_optional(&self.db)
        .await?;
        Ok(row)
    }

    /// Mark a thread as read for a user
    pub async fn mark_thread_read(
        &self,
        user_id: Uuid,
        thread_id: Uuid,
        read_at: DateTime<Utc>,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            INSERT INTO thread_memberships (user_id, post_id, last_read_at, unread_replies_count, mention_count)
            VALUES ($1, $2, $3, 0, 0)
            ON CONFLICT (user_id, post_id) DO UPDATE SET
                last_read_at = $3,
                unread_replies_count = 0,
                mention_count = 0,
                updated_at = NOW()
            "#,
        )
        .bind(user_id)
        .bind(thread_id)
        .bind(read_at)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// Mark all threads as read for a user in a team
    pub async fn mark_all_threads_read(
        &self,
        user_id: Uuid,
        team_id: Uuid,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            UPDATE thread_memberships tm SET
                last_read_at = NOW(),
                unread_replies_count = 0,
                mention_count = 0,
                updated_at = NOW()
            FROM posts p
            JOIN channels c ON p.channel_id = c.id
            WHERE tm.post_id = p.id
              AND tm.user_id = $1
              AND c.team_id = $2
            "#,
        )
        .bind(user_id)
        .bind(team_id)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// Get thread mention counts grouped by channel for a user in a team
    pub async fn get_thread_mention_counts_by_channel(
        &self,
        user_id: Uuid,
        team_id: Uuid,
    ) -> ApiResult<Vec<(Uuid, i64)>> {
        let rows = sqlx::query_as(
            r#"
            SELECT c.id, COALESCE(SUM(tm.mention_count), 0)
            FROM thread_memberships tm
            JOIN posts p ON tm.post_id = p.id
            JOIN channels c ON p.channel_id = c.id
            WHERE tm.user_id = $1
              AND tm.following = true
              AND c.team_id = $2
              AND p.root_post_id IS NULL
              AND p.deleted_at IS NULL
            GROUP BY c.id
            "#,
        )
        .bind(user_id)
        .bind(team_id)
        .fetch_all(&self.db)
        .await?;
        Ok(rows)
    }

    /// Follow a thread
    pub async fn follow_thread(&self, user_id: Uuid, thread_id: Uuid) -> ApiResult<()> {
        sqlx::query(
            r#"
            INSERT INTO thread_memberships (user_id, post_id, following)
            VALUES ($1, $2, true)
            ON CONFLICT (user_id, post_id) DO UPDATE SET
                following = true,
                updated_at = NOW()
            "#,
        )
        .bind(user_id)
        .bind(thread_id)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// Unfollow a thread
    pub async fn unfollow_thread(&self, user_id: Uuid, thread_id: Uuid) -> ApiResult<()> {
        sqlx::query(
            r#"
            UPDATE thread_memberships SET
                following = false,
                updated_at = NOW()
            WHERE user_id = $1 AND post_id = $2
            "#,
        )
        .bind(user_id)
        .bind(thread_id)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// Get the created_at timestamp of a post
    pub async fn get_post_created_at(&self, post_id: Uuid) -> ApiResult<Option<DateTime<Utc>>> {
        let created_at = sqlx::query_scalar("SELECT created_at FROM posts WHERE id = $1")
            .bind(post_id)
            .fetch_optional(&self.db)
            .await?;
        Ok(created_at)
    }

    /// Get the created_at timestamp of a post within a thread
    pub async fn get_post_created_at_in_thread(
        &self,
        post_id: Uuid,
        thread_id: Uuid,
    ) -> ApiResult<Option<DateTime<Utc>>> {
        let created_at = sqlx::query_scalar(
            "SELECT created_at FROM posts WHERE id = $1 AND root_post_id = $2",
        )
        .bind(post_id)
        .bind(thread_id)
        .fetch_optional(&self.db)
        .await?;
        Ok(created_at)
    }

    /// List pinned posts in a channel with pagination
    pub async fn list_pinned_posts(
        &self,
        channel_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> ApiResult<Vec<PostWithUser>> {
        let rows = sqlx::query_as::<_, PostWithUserRow>(&format!(
            r#"
                SELECT {}
                FROM posts p
                LEFT JOIN users u ON p.user_id = u.id
                WHERE p.channel_id = $1
                  AND p.is_pinned = true
                  AND p.deleted_at IS NULL
                ORDER BY p.created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            Self::POST_COLUMNS
        ))
        .bind(channel_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Get the oldest post timestamp in a channel
    pub async fn get_oldest_post_time(&self, channel_id: Uuid) -> ApiResult<Option<DateTime<Utc>>> {
        let time = sqlx::query_scalar(
            "SELECT MIN(created_at) FROM posts WHERE channel_id = $1 AND deleted_at IS NULL",
        )
        .bind(channel_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(time)
    }

    /// Reset channel read to zero for a user
    pub async fn reset_channel_read(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        mark_time: DateTime<Utc>,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE channel_reads SET last_read_message_id = 0, last_read_at = $3 WHERE channel_id = $1 AND user_id = $2",
        )
        .bind(channel_id)
        .bind(user_id)
        .bind(mark_time)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Compute channel unread counts (msg_count, mention_count, mention_count_root, urgent_mention_count, msg_count_root)
    pub async fn compute_channel_unread_counts(
        &self,
        channel_id: Uuid,
        last_read_message_id: i64,
        username: &str,
        post_priority_enabled: bool,
    ) -> ApiResult<(i64, i64, i64, i64, i64)> {
        let row: (i64, i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE p.deleted_at IS NULL AND p.seq > $2)::BIGINT AS msg_count,
                COUNT(*) FILTER (WHERE p.deleted_at IS NULL AND p.seq > $2 AND (p.message LIKE '%@' || $3 || '%' OR p.message LIKE '%@all%' OR p.message LIKE '%@channel%'))::BIGINT AS mention_count,
                COUNT(*) FILTER (WHERE p.deleted_at IS NULL AND p.seq > $2 AND p.root_post_id IS NULL AND (p.message LIKE '%@' || $3 || '%' OR p.message LIKE '%@all%' OR p.message LIKE '%@channel%'))::BIGINT AS mention_count_root,
                COUNT(*) FILTER (WHERE p.deleted_at IS NULL AND p.seq > $2 AND (p.message LIKE '%@' || $3 || '%' OR p.message LIKE '%@all%' OR p.message LIKE '%@channel%') AND p.message LIKE '%@here%')::BIGINT AS urgent_mention_count,
                COUNT(*) FILTER (WHERE p.deleted_at IS NULL AND p.seq > $2 AND p.root_post_id IS NULL)::BIGINT AS msg_count_root
            FROM posts p
            WHERE p.channel_id = $1
            "#,
        )
        .bind(channel_id)
        .bind(last_read_message_id)
        .bind(username)
        .fetch_one(&self.db)
        .await?;

        let urgent = if post_priority_enabled { row.3 } else { 0 };
        Ok((row.0, row.1, row.2, urgent, row.4))
    }

    /// Set a thread as unread for a user
    pub async fn set_thread_unread(
        &self,
        user_id: Uuid,
        thread_id: Uuid,
        last_read_at: Option<DateTime<Utc>>,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            INSERT INTO thread_memberships (user_id, post_id, last_read_at, unread_replies_count, mention_count)
            VALUES ($1, $2, $3, 1, 0)
            ON CONFLICT (user_id, post_id) DO UPDATE SET
                last_read_at = $3,
                unread_replies_count = GREATEST(thread_memberships.unread_replies_count, 1),
                updated_at = NOW()
            "#,
        )
        .bind(user_id)
        .bind(thread_id)
        .bind(last_read_at)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    // ========================================================================
    // Methods for api/posts.rs refactor (returning Post model from crate::models::Post)
    // ========================================================================

    /// Get a post by ID (Post model, no user join)
    pub async fn get_post_by_id(&self, post_id: Uuid) -> ApiResult<Option<crate::models::Post>> {
        let post = sqlx::query_as::<_, crate::models::Post>(
            r#"
            SELECT id, channel_id, user_id, root_post_id, message, props, file_ids,
                   is_pinned, created_at, edited_at, deleted_at,
                   reply_count::int8 as reply_count,
                   last_reply_at, seq
            FROM posts WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(post_id)
        .fetch_optional(&self.db)
        .await?;
        Ok(post)
    }

    /// Pin a post and return the updated Post
    pub async fn pin_post_returning(&self, post_id: Uuid) -> ApiResult<crate::models::Post> {
        let post = sqlx::query_as::<_, crate::models::Post>(
            r#"
            UPDATE posts SET is_pinned = true WHERE id = $1
            RETURNING id, channel_id, user_id, root_post_id, message, props, file_ids,
                      is_pinned, created_at, edited_at, deleted_at,
                      reply_count::int8 as reply_count,
                      last_reply_at, seq
            "#,
        )
        .bind(post_id)
        .fetch_one(&self.db)
        .await?;
        Ok(post)
    }

    /// Unpin a post and return the updated Post
    pub async fn unpin_post_returning(&self, post_id: Uuid) -> ApiResult<crate::models::Post> {
        let post = sqlx::query_as::<_, crate::models::Post>(
            r#"
            UPDATE posts SET is_pinned = false WHERE id = $1
            RETURNING id, channel_id, user_id, root_post_id, message, props, file_ids,
                      is_pinned, created_at, edited_at, deleted_at,
                      reply_count::int8 as reply_count,
                      last_reply_at, seq
            "#,
        )
        .bind(post_id)
        .fetch_one(&self.db)
        .await?;
        Ok(post)
    }

    /// Soft delete a post (no returning)
    pub async fn soft_delete_post_simple(&self, post_id: Uuid) -> ApiResult<()> {
        sqlx::query("UPDATE posts SET deleted_at = NOW() WHERE id = $1")
            .bind(post_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// Update post message and return updated Post
    pub async fn update_post_message_returning(
        &self,
        post_id: Uuid,
        message: &str,
    ) -> ApiResult<crate::models::Post> {
        let post = sqlx::query_as::<_, crate::models::Post>(
            r#"
            UPDATE posts SET message = $1, edited_at = NOW() WHERE id = $2
            RETURNING id, channel_id, user_id, root_post_id, message, props, file_ids,
                      is_pinned, created_at, edited_at, deleted_at,
                      reply_count::int8 as reply_count,
                      last_reply_at, seq
            "#,
        )
        .bind(message)
        .bind(post_id)
        .fetch_one(&self.db)
        .await?;
        Ok(post)
    }

    // ========================================================================
    // Channel read queries
    // ========================================================================

    /// Get last read message ID for a channel
    pub async fn get_channel_read(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
    ) -> ApiResult<Option<i64>> {
        let last_read: Option<i64> = sqlx::query_scalar(
            "SELECT last_read_message_id FROM channel_reads WHERE user_id = $1 AND channel_id = $2",
        )
        .bind(user_id)
        .bind(channel_id)
        .fetch_optional(&self.db)
        .await?;
        Ok(last_read)
    }

    /// Get first unread seq in a channel (optionally after a specific seq)
    pub async fn get_first_unread_seq(
        &self,
        channel_id: Uuid,
        after_seq: Option<i64>,
    ) -> ApiResult<Option<i64>> {
        let seq = match after_seq {
            Some(lr) => sqlx::query_scalar(
                "SELECT MIN(seq) FROM posts WHERE channel_id = $1 AND seq > $2 AND deleted_at IS NULL",
            )
            .bind(channel_id)
            .bind(lr)
            .fetch_one(&self.db)
            .await?,
            None => sqlx::query_scalar(
                "SELECT MIN(seq) FROM posts WHERE channel_id = $1 AND deleted_at IS NULL",
            )
            .bind(channel_id)
            .fetch_one(&self.db)
            .await?,
        };
        Ok(seq)
    }

    // ========================================================================
    // Saved post queries
    // ========================================================================

    /// Save a post for a user
    pub async fn save_post(&self, user_id: Uuid, post_id: Uuid) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO saved_posts (user_id, post_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(post_id)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// Unsave a post for a user
    pub async fn unsave_post(&self, user_id: Uuid, post_id: Uuid) -> ApiResult<()> {
        sqlx::query("DELETE FROM saved_posts WHERE user_id = $1 AND post_id = $2")
            .bind(user_id)
            .bind(post_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// Get saved post IDs for a user from a list of post IDs
    pub async fn get_saved_post_ids(
        &self,
        user_id: Uuid,
        post_ids: &[Uuid],
    ) -> ApiResult<Vec<Uuid>> {
        let saved_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT post_id FROM saved_posts WHERE user_id = $1 AND post_id = ANY($2)",
        )
        .bind(user_id)
        .bind(post_ids)
        .fetch_all(&self.db)
        .await?;
        Ok(saved_ids)
    }

    // ========================================================================
    // Reaction queries
    // ========================================================================

    /// Add or update a reaction
    pub async fn add_reaction(
        &self,
        post_id: Uuid,
        user_id: Uuid,
        emoji_name: &str,
    ) -> ApiResult<crate::models::reaction::Reaction> {
        let reaction = sqlx::query_as::<_, crate::models::reaction::Reaction>(
            r#"
            INSERT INTO reactions (post_id, user_id, emoji_name)
            VALUES ($1, $2, $3)
            ON CONFLICT (post_id, user_id, emoji_name) DO UPDATE SET create_at = extract(epoch from now()) * 1000
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(user_id)
        .bind(emoji_name)
        .fetch_one(&self.db)
        .await?;
        Ok(reaction)
    }

    /// Remove a reaction
    pub async fn remove_reaction(
        &self,
        post_id: Uuid,
        user_id: Uuid,
        emoji_name: &str,
    ) -> ApiResult<()> {
        sqlx::query(
            "DELETE FROM reactions WHERE post_id = $1 AND user_id = $2 AND emoji_name = $3",
        )
        .bind(post_id)
        .bind(user_id)
        .bind(emoji_name)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// Get reactions for a list of post IDs
    pub async fn get_reactions_for_posts(
        &self,
        post_ids: &[Uuid],
    ) -> ApiResult<Vec<crate::models::reaction::Reaction>> {
        let reactions = sqlx::query_as::<_, crate::models::reaction::Reaction>(
            "SELECT * FROM reactions WHERE post_id = ANY($1) ORDER BY create_at",
        )
        .bind(post_ids)
        .fetch_all(&self.db)
        .await?;
        Ok(reactions)
    }

    /// Check if a user is a member of the channel containing a post.
    pub async fn check_channel_membership(
        &self,
        post_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM posts p
                JOIN channel_members cm ON cm.channel_id = p.channel_id
                WHERE p.id = $1 AND cm.user_id = $2
            )
            "#,
        )
        .bind(post_id)
        .bind(user_id)
        .fetch_one(&self.db)
        .await
    }

    /// Check if a custom emoji exists and is not deleted.
    pub async fn custom_emoji_exists(&self, name: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM custom_emojis WHERE name = $1 AND delete_at IS NULL)",
        )
        .bind(name)
        .fetch_one(&self.db)
        .await
    }

    /// Get a specific reaction by user, post, and emoji name.
    pub async fn get_reaction(
        &self,
        user_id: Uuid,
        post_id: Uuid,
        emoji_name: &str,
    ) -> Result<Option<crate::models::reaction::Reaction>, sqlx::Error> {
        sqlx::query_as::<_, crate::models::reaction::Reaction>(
            "SELECT * FROM reactions WHERE user_id = $1 AND post_id = $2 AND emoji_name = $3",
        )
        .bind(user_id)
        .bind(post_id)
        .bind(emoji_name)
        .fetch_optional(&self.db)
        .await
    }

    /// Get the post author user_id and team_id for a post.
    pub async fn get_post_author_and_team(
        &self,
        post_id: Uuid,
    ) -> Result<Option<(Uuid, Uuid)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT p.user_id, c.team_id FROM posts p JOIN channels c ON p.channel_id = c.id WHERE p.id = $1"
        )
        .bind(post_id)
        .fetch_optional(&self.db)
        .await
    }

    /// Get reactions for a single post with channel_id.
    pub async fn get_reactions_with_channel_for_post(
        &self,
        post_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, String, i64, Uuid)>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT r.user_id, r.post_id, r.emoji_name, r.create_at, p.channel_id
            FROM reactions r
            JOIN posts p ON p.id = r.post_id
            WHERE r.post_id = $1
            "#,
        )
        .bind(post_id)
        .fetch_all(&self.db)
        .await
    }

    /// Get reactions for multiple posts with channel_id.
    pub async fn get_reactions_with_channel_for_posts(
        &self,
        post_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, Uuid, String, i64, Uuid)>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT r.post_id, r.user_id, r.emoji_name, r.create_at, p.channel_id
            FROM reactions r
            JOIN posts p ON p.id = r.post_id
            WHERE r.post_id = ANY($1)
            "#,
        )
        .bind(post_ids)
        .fetch_all(&self.db)
        .await
    }

    /// Get the channel_id for a post, checking it's not deleted.
    pub async fn get_post_channel_id_optional(
        &self,
        post_id: Uuid,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT channel_id FROM posts WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(post_id)
        .fetch_optional(&self.db)
        .await
    }

    /// Get the last read message sequence for a user in a channel.
    pub async fn get_last_read_seq(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
    ) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT last_read_message_id FROM channel_reads WHERE user_id = $1 AND channel_id = $2",
        )
        .bind(user_id)
        .bind(channel_id)
        .fetch_optional(&self.db)
        .await
    }

    /// Get posts around an unread sequence in a channel.
    pub async fn get_posts_around_unread(
        &self,
        channel_id: Uuid,
        last_read_seq: i64,
        limit_before: i64,
        limit_after: i64,
    ) -> Result<Vec<crate::models::post::PostResponse>, sqlx::Error> {
        let rows: Vec<PostWithUserRow> = sqlx::query_as(
            r#"
            (
                SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                       p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                       p.reply_count::int8 as reply_count,
                       p.last_reply_at, p.seq,
                       u.username, u.avatar_url, u.email
                FROM posts p
                LEFT JOIN users u ON p.user_id = u.id
                WHERE p.channel_id = $1 AND p.seq <= $2 AND p.deleted_at IS NULL
                ORDER BY p.seq DESC
                LIMIT $3
            )
            UNION ALL
            (
                SELECT p.id, p.channel_id, p.user_id, p.root_post_id, p.message, p.props, p.file_ids,
                       p.is_pinned, p.created_at, p.edited_at, p.deleted_at,
                       p.reply_count::int8 as reply_count,
                       p.last_reply_at, p.seq,
                       u.username, u.avatar_url, u.email
                FROM posts p
                LEFT JOIN users u ON p.user_id = u.id
                WHERE p.channel_id = $1 AND p.seq > $2 AND p.deleted_at IS NULL
                ORDER BY p.seq ASC
                LIMIT $4
            )
            ORDER BY seq DESC
            "#,
        )
        .bind(channel_id)
        .bind(last_read_seq)
        .bind(limit_before)
        .bind(limit_after)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(|r| PostWithUser::from(r).into()).collect())
    }

    /// Acknowledge a post (upsert).
    pub async fn acknowledge_post(
        &self,
        user_id: Uuid,
        post_id: Uuid,
        acknowledged_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO post_acknowledgements (user_id, post_id, acknowledged_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, post_id) DO UPDATE SET acknowledged_at = $3
            "#,
        )
        .bind(user_id)
        .bind(post_id)
        .bind(acknowledged_at)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get an acknowledgement time for a post.
    pub async fn get_acknowledgement(
        &self,
        user_id: Uuid,
        post_id: Uuid,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT acknowledged_at FROM post_acknowledgements WHERE user_id = $1 AND post_id = $2",
        )
        .bind(user_id)
        .bind(post_id)
        .fetch_optional(&self.db)
        .await
    }

    /// Delete a post acknowledgement.
    pub async fn delete_acknowledgement(
        &self,
        user_id: Uuid,
        post_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM post_acknowledgements WHERE user_id = $1 AND post_id = $2")
            .bind(user_id)
            .bind(post_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }
}

/// Internal row type for SQLx mapping
///
/// This struct maps directly to the SQL query results. We use this as an
/// intermediate to handle the Option<> wrapping that comes from LEFT JOIN.
#[derive(Debug, Clone, sqlx::FromRow)]
struct PostWithUserRow {
    // Post fields
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub root_post_id: Option<Uuid>,
    pub message: String,
    pub props: Option<serde_json::Value>,
    pub file_ids: Vec<Uuid>,
    pub is_pinned: bool,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub reply_count: i64,
    pub last_reply_at: Option<DateTime<Utc>>,
    pub seq: i64,
    // User fields (Option because of LEFT JOIN)
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
}

impl From<PostWithUserRow> for PostWithUser {
    fn from(row: PostWithUserRow) -> Self {
        Self {
            id: row.id,
            channel_id: row.channel_id,
            user_id: row.user_id,
            root_post_id: row.root_post_id,
            message: row.message,
            props: row.props,
            file_ids: row.file_ids,
            is_pinned: row.is_pinned,
            created_at: row.created_at,
            edited_at: row.edited_at,
            deleted_at: row.deleted_at,
            reply_count: row.reply_count,
            last_reply_at: row.last_reply_at,
            seq: row.seq,
            username: row.username,
            avatar_url: row.avatar_url,
            email: row.email,
        }
    }
}

impl From<PostWithUser> for crate::models::post::PostResponse {
    fn from(p: PostWithUser) -> Self {
        Self {
            id: p.id,
            channel_id: p.channel_id,
            user_id: p.user_id,
            root_post_id: p.root_post_id,
            message: p.message,
            props: p.props.unwrap_or_default(),
            file_ids: p.file_ids,
            is_pinned: p.is_pinned,
            created_at: p.created_at,
            edited_at: p.edited_at,
            deleted_at: p.deleted_at,
            reply_count: p.reply_count,
            last_reply_at: p.last_reply_at,
            username: p.username,
            avatar_url: p.avatar_url,
            email: p.email,
            files: vec![],
            reactions: vec![],
            is_saved: false,
            client_msg_id: None,
            seq: p.seq,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These are compile-time tests to ensure the SQL queries are valid.
    // Full integration tests would require a database connection.

    #[test]
    fn test_post_columns_format() {
        // Verify the SQL column string is valid
        let columns = PostRepository::POST_COLUMNS;
        assert!(columns.contains("p.id"));
        assert!(columns.contains("p.channel_id"));
        assert!(columns.contains("p.user_id"));
        assert!(columns.contains("p.reply_count::int8"));
        assert!(columns.contains("u.username"));
        assert!(columns.contains("u.avatar_url"));
        assert!(columns.contains("u.email"));
    }
}
