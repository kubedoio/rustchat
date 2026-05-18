use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Repository for upload session and file operations.
pub struct UploadRepository<'a> {
    pool: &'a PgPool,
}

#[derive(sqlx::FromRow)]
pub struct UploadSessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub channel_id: Uuid,
    pub filename: String,
    pub file_size: i64,
    pub file_offset: i64,
    pub created_at: DateTime<Utc>,
}

impl<'a> UploadRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Create a new upload session.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_session(
        &self,
        id: Uuid,
        user_id: Uuid,
        channel_id: Uuid,
        filename: &str,
        file_size: i64,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO upload_sessions (id, user_id, channel_id, filename, file_size, file_offset, created_at, expires_at)
            VALUES ($1, $2, $3, $4, $5, 0, $6, $7)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(channel_id)
        .bind(filename)
        .bind(file_size)
        .bind(created_at)
        .bind(expires_at)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Get an active upload session by ID.
    pub async fn get_session(
        &self,
        id: Uuid,
    ) -> Result<Option<UploadSessionRow>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, user_id, channel_id, filename, file_size, file_offset, created_at
            FROM upload_sessions
            WHERE id = $1 AND expires_at > NOW()
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
    }

    /// Append data to an upload session and update the offset.
    pub async fn append_data(
        &self,
        id: Uuid,
        data: &[u8],
        new_offset: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE upload_sessions
            SET file_data = COALESCE(file_data, ''::bytea) || $1,
                file_offset = $2
            WHERE id = $3
            "#,
        )
        .bind(data)
        .bind(new_offset)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Get the file data for an upload session.
    pub async fn get_file_data(&self, id: Uuid) -> Result<Option<Vec<u8>>, sqlx::Error> {
        sqlx::query_scalar("SELECT file_data FROM upload_sessions WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    /// Delete an upload session.
    pub async fn delete_session(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM upload_sessions WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Create a file record.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_file(
        &self,
        id: Uuid,
        uploader_id: Uuid,
        channel_id: Uuid,
        name: &str,
        key: &str,
        mime_type: &str,
        size: i64,
        sha256: &str,
        width: Option<i32>,
        height: Option<i32>,
        has_thumbnail: bool,
        thumbnail_key: &Option<String>,
        created_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO files (id, uploader_id, channel_id, name, key, mime_type, size, sha256, width, height, has_thumbnail, thumbnail_key, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(id)
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
        .bind(created_at)
        .execute(self.pool)
        .await?;

        Ok(())
    }
}
