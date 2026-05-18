use sqlx::PgPool;
use uuid::Uuid;

/// Repository for custom emoji operations.
pub struct EmojiRepository<'a> {
    pool: &'a PgPool,
}

#[derive(sqlx::FromRow)]
pub struct DbEmoji {
    pub id: Uuid,
    pub name: String,
    pub creator_id: Uuid,
    pub create_at: i64,
    pub update_at: i64,
    pub delete_at: i64,
}

impl<'a> EmojiRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// List all active custom emojis.
    pub async fn list(&self) -> Result<Vec<DbEmoji>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, name, creator_id,
                (extract(epoch from create_at)*1000)::bigint as create_at,
                (extract(epoch from update_at)*1000)::bigint as update_at,
                COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at
            FROM custom_emojis WHERE delete_at IS NULL
            "#,
        )
        .fetch_all(self.pool)
        .await
    }

    /// Search emojis by name (case-insensitive).
    pub async fn search(&self, term: &str) -> Result<Vec<DbEmoji>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, name, creator_id,
                (extract(epoch from create_at)*1000)::bigint as create_at,
                (extract(epoch from update_at)*1000)::bigint as update_at,
                COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at
            FROM custom_emojis
            WHERE name ILIKE $1 AND delete_at IS NULL
            "#,
        )
        .bind(term)
        .fetch_all(self.pool)
        .await
    }

    /// Get an active emoji by ID.
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<DbEmoji>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, name, creator_id,
                (extract(epoch from create_at)*1000)::bigint as create_at,
                (extract(epoch from update_at)*1000)::bigint as update_at,
                COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at
            FROM custom_emojis WHERE id = $1 AND delete_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
    }

    /// Get an active emoji by name.
    pub async fn get_by_name(&self, name: &str) -> Result<Option<DbEmoji>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, name, creator_id,
                (extract(epoch from create_at)*1000)::bigint as create_at,
                (extract(epoch from update_at)*1000)::bigint as update_at,
                COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at
            FROM custom_emojis WHERE name = $1 AND delete_at IS NULL
            "#,
        )
        .bind(name)
        .fetch_optional(self.pool)
        .await
    }

    /// Get the image URL for an emoji.
    pub async fn get_image_url(&self, id: Uuid) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT image_url FROM custom_emojis WHERE id = $1 AND delete_at IS NULL",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
    }

    /// Check if an active emoji with the given name exists.
    pub async fn exists(&self, name: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM custom_emojis WHERE name = $1 AND delete_at IS NULL)",
        )
        .bind(name)
        .fetch_one(self.pool)
        .await
    }

    /// Create a new custom emoji.
    pub async fn create(
        &self,
        id: Uuid,
        name: &str,
        creator_id: Uuid,
        image_url: &str,
    ) -> Result<DbEmoji, sqlx::Error> {
        sqlx::query_as(
            r#"
            INSERT INTO custom_emojis (id, name, creator_id, image_url)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, creator_id,
                (extract(epoch from create_at)*1000)::bigint as create_at,
                (extract(epoch from update_at)*1000)::bigint as update_at,
                COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(creator_id)
        .bind(image_url)
        .fetch_one(self.pool)
        .await
    }

    /// Soft delete an emoji.
    pub async fn soft_delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE custom_emojis SET delete_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Get multiple emojis by name.
    pub async fn get_by_names(&self, names: &[String]) -> Result<Vec<DbEmoji>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, name, creator_id,
                (extract(epoch from create_at)*1000)::bigint as create_at,
                (extract(epoch from update_at)*1000)::bigint as update_at,
                COALESCE((extract(epoch from delete_at)*1000)::bigint, 0) as delete_at
            FROM custom_emojis
            WHERE name = ANY($1) AND delete_at IS NULL
            "#,
        )
        .bind(names)
        .fetch_all(self.pool)
        .await
    }
}
