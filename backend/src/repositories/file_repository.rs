use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;
use crate::models::FileInfo;

pub struct FileRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> FileRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Get a file by ID.
    pub async fn get_by_id(&self, file_id: Uuid) -> ApiResult<Option<FileInfo>> {
        let file = sqlx::query_as::<_, FileInfo>("SELECT * FROM files WHERE id = $1")
            .bind(file_id)
            .fetch_optional(self.pool)
            .await?;
        Ok(file)
    }

    /// Get multiple files by their IDs.
    pub async fn get_by_ids(&self, file_ids: &[Uuid]) -> ApiResult<Vec<FileInfo>> {
        if file_ids.is_empty() {
            return Ok(vec![]);
        }
        let files = sqlx::query_as::<_, FileInfo>(
            "SELECT * FROM files WHERE id = ANY($1)"
        )
        .bind(file_ids)
        .fetch_all(self.pool)
        .await?;
        Ok(files)
    }

    /// Create a file record with full metadata (used by v4/files upload).
    #[allow(clippy::too_many_arguments)]
    pub async fn create_full(
        &self,
        file_id: Uuid,
        uploader_id: Uuid,
        channel_id: Option<Uuid>,
        name: &str,
        key: &str,
        mime_type: &str,
        size: i64,
        sha256: &str,
        width: Option<i32>,
        height: Option<i32>,
        has_thumbnail: bool,
        thumbnail_key: Option<&str>,
    ) -> ApiResult<FileInfo> {
        let file = sqlx::query_as::<_, FileInfo>(
            r#"
            INSERT INTO files (id, uploader_id, channel_id, name, key, mime_type, size, sha256, width, height, has_thumbnail, thumbnail_key)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(file_id)
        .bind(uploader_id)
        .bind(channel_id)
        .bind(name)
        .bind(key)
        .bind(mime_type)
        .bind(size)
        .bind(sha256)
        .bind(width)
        .bind(height)
        .bind(has_thumbnail)
        .bind(thumbnail_key)
        .fetch_one(self.pool)
        .await?;
        Ok(file)
    }

    /// Create a file record with minimal metadata (used by api/files upload).
    #[allow(clippy::too_many_arguments)]
    pub async fn create_simple(
        &self,
        file_id: Uuid,
        uploader_id: Uuid,
        channel_id: Option<Uuid>,
        name: &str,
        key: &str,
        mime_type: &str,
        size: i64,
        sha256: &str,
    ) -> ApiResult<FileInfo> {
        let file = sqlx::query_as::<_, FileInfo>(
            r#"
            INSERT INTO files (id, uploader_id, channel_id, name, key, mime_type, size, sha256)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(file_id)
        .bind(uploader_id)
        .bind(channel_id)
        .bind(name)
        .bind(key)
        .bind(mime_type)
        .bind(size)
        .bind(sha256)
        .fetch_one(self.pool)
        .await?;
        Ok(file)
    }

    /// Update image dimensions and thumbnail info for a file.
    pub async fn update_dimensions(
        &self,
        file_id: Uuid,
        width: Option<i32>,
        height: Option<i32>,
        has_thumbnail: bool,
        thumbnail_key: Option<&str>,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE files SET width = $1, height = $2, has_thumbnail = $3, thumbnail_key = $4 WHERE id = $5"
        )
        .bind(width)
        .bind(height)
        .bind(has_thumbnail)
        .bind(thumbnail_key)
        .bind(file_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Delete a file record.
    pub async fn delete(&self, file_id: Uuid) -> ApiResult<()> {
        sqlx::query("DELETE FROM files WHERE id = $1")
            .bind(file_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Search files globally for a user.
    pub async fn search(&self, user_id: Uuid, pattern: &str) -> ApiResult<Vec<FileInfo>> {
        let files = sqlx::query_as::<_, FileInfo>(
            r#"
            SELECT f.* FROM files f
            JOIN channel_members cm ON f.channel_id = cm.channel_id
            WHERE cm.user_id = $1 AND f.name ILIKE $2
            ORDER BY f.created_at DESC
            LIMIT 100
            "#,
        )
        .bind(user_id)
        .bind(pattern)
        .fetch_all(self.pool)
        .await?;
        Ok(files)
    }

    /// Search files within a team for a user.
    pub async fn search_for_team(
        &self,
        user_id: Uuid,
        team_id: Uuid,
        pattern: &str,
    ) -> ApiResult<Vec<FileInfo>> {
        let files = sqlx::query_as::<_, FileInfo>(
            r#"
            SELECT f.* FROM files f
            JOIN channels c ON f.channel_id = c.id
            JOIN channel_members cm ON c.id = cm.channel_id
            WHERE cm.user_id = $1 AND c.team_id = $2 AND f.name ILIKE $3
            ORDER BY f.created_at DESC
            LIMIT 100
            "#,
        )
        .bind(user_id)
        .bind(team_id)
        .bind(pattern)
        .fetch_all(self.pool)
        .await?;
        Ok(files)
    }
}
