use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;
use crate::models::channel_bookmark::ChannelBookmark;

pub struct BookmarkRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> BookmarkRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// List active bookmarks for a channel, ordered by sort_order then create_at.
    pub async fn list_for_channel(&self, channel_id: Uuid) -> ApiResult<Vec<ChannelBookmark>> {
        let bookmarks = sqlx::query_as::<_, ChannelBookmark>(
            r#"
            SELECT id, channel_id, owner_id, type, display_name, link_url, file_id, emoji, sort_order, image_url, create_at, update_at, delete_at
            FROM channel_bookmarks
            WHERE channel_id = $1 AND delete_at = 0
            ORDER BY sort_order ASC, create_at ASC
            "#,
        )
        .bind(channel_id)
        .fetch_all(self.pool)
        .await?;
        Ok(bookmarks)
    }

    /// Get a single active bookmark by ID and channel.
    pub async fn get(
        &self,
        bookmark_id: Uuid,
        channel_id: Uuid,
    ) -> ApiResult<Option<ChannelBookmark>> {
        let bookmark = sqlx::query_as::<_, ChannelBookmark>(
            r#"
            SELECT id, channel_id, owner_id, type, display_name, link_url, file_id, emoji, sort_order, image_url, create_at, update_at, delete_at
            FROM channel_bookmarks
            WHERE id = $1 AND channel_id = $2 AND delete_at = 0
            "#,
        )
        .bind(bookmark_id)
        .bind(channel_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(bookmark)
    }

    /// Get the owner_id of a bookmark (for auth checks).
    pub async fn get_owner_id(
        &self,
        bookmark_id: Uuid,
        channel_id: Uuid,
    ) -> ApiResult<Option<Uuid>> {
        let owner_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT owner_id FROM channel_bookmarks WHERE id = $1 AND channel_id = $2 AND delete_at = 0"
        )
        .bind(bookmark_id)
        .bind(channel_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(owner_id)
    }

    /// Create a new bookmark.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        channel_id: Uuid,
        owner_id: Uuid,
        bookmark_type: &str,
        display_name: Option<&str>,
        link_url: Option<&str>,
        file_id: Option<Uuid>,
        emoji: Option<&str>,
        sort_order: i32,
        image_url: Option<&str>,
        now: i64,
    ) -> ApiResult<ChannelBookmark> {
        let bookmark = sqlx::query_as::<_, ChannelBookmark>(
            r#"
            INSERT INTO channel_bookmarks (channel_id, owner_id, type, display_name, link_url, file_id, emoji, sort_order, image_url, create_at, update_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10)
            RETURNING id, channel_id, owner_id, type, display_name, link_url, file_id, emoji, sort_order, image_url, create_at, update_at, delete_at
            "#,
        )
        .bind(channel_id)
        .bind(owner_id)
        .bind(bookmark_type)
        .bind(display_name)
        .bind(link_url)
        .bind(file_id)
        .bind(emoji)
        .bind(sort_order)
        .bind(image_url)
        .bind(now)
        .fetch_one(self.pool)
        .await?;
        Ok(bookmark)
    }

    /// Update a bookmark with COALESCE semantics.
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        bookmark_id: Uuid,
        channel_id: Uuid,
        display_name: Option<&str>,
        link_url: Option<&str>,
        emoji: Option<&str>,
        sort_order: Option<i32>,
        image_url: Option<&str>,
        now: i64,
    ) -> ApiResult<ChannelBookmark> {
        let bookmark = sqlx::query_as::<_, ChannelBookmark>(
            r#"
            UPDATE channel_bookmarks
            SET display_name = COALESCE($3, display_name),
                link_url = COALESCE($4, link_url),
                emoji = COALESCE($5, emoji),
                sort_order = COALESCE($6, sort_order),
                image_url = COALESCE($7, image_url),
                update_at = $8
            WHERE id = $1 AND channel_id = $2 AND delete_at = 0
            RETURNING id, channel_id, owner_id, type, display_name, link_url, file_id, emoji, sort_order, image_url, create_at, update_at, delete_at
            "#,
        )
        .bind(bookmark_id)
        .bind(channel_id)
        .bind(display_name)
        .bind(link_url)
        .bind(emoji)
        .bind(sort_order)
        .bind(image_url)
        .bind(now)
        .fetch_one(self.pool)
        .await?;
        Ok(bookmark)
    }

    /// Soft-delete a bookmark.
    pub async fn soft_delete(
        &self,
        bookmark_id: Uuid,
        channel_id: Uuid,
        delete_at: i64,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE channel_bookmarks SET delete_at = $3 WHERE id = $1 AND channel_id = $2",
        )
        .bind(bookmark_id)
        .bind(channel_id)
        .bind(delete_at)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Reorder a bookmark.
    pub async fn reorder(
        &self,
        bookmark_id: Uuid,
        channel_id: Uuid,
        sort_order: i32,
        now: i64,
    ) -> ApiResult<ChannelBookmark> {
        let bookmark = sqlx::query_as::<_, ChannelBookmark>(
            r#"
            UPDATE channel_bookmarks
            SET sort_order = $3, update_at = $4
            WHERE id = $1 AND channel_id = $2 AND delete_at = 0
            RETURNING id, channel_id, owner_id, type, display_name, link_url, file_id, emoji, sort_order, image_url, create_at, update_at, delete_at
            "#,
        )
        .bind(bookmark_id)
        .bind(channel_id)
        .bind(sort_order)
        .bind(now)
        .fetch_one(self.pool)
        .await?;
        Ok(bookmark)
    }
}
