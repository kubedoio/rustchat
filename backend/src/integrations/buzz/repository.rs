//! Persistence for Buzz bridge connections and channel mappings.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::AppError;

/// A stored connection. `private_key_encrypted` must never leave this layer
/// except through [`BuzzConnectionRecord::into_runtime`], which decrypts it
/// into a [`BuzzConnectionRuntime`](crate::integrations::buzz::connector::BuzzConnectionRuntime)
/// that exists only for the duration of a delivery cycle.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BuzzConnectionRecord {
    pub id: Uuid,
    pub name: String,
    pub relay_url: String,
    pub bridge_pubkey: String,
    pub private_key_encrypted: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl BuzzConnectionRecord {
    /// Decrypt the signing key and produce a runtime connection.
    ///
    /// The key is validated by parsing (hex or nsec) before use; a key that
    /// no longer decrypts/parses is a terminal configuration error.
    pub fn into_runtime(
        self,
        encryption_key: &str,
    ) -> Result<crate::integrations::buzz::connector::BuzzConnectionRuntime, AppError> {
        let secret =
            crate::crypto::decrypt(&self.private_key_encrypted, encryption_key).map_err(|e| {
                AppError::Internal(format!("buzz connection key decryption failed: {e}"))
            })?;
        let keys = nostr::key::Keys::parse(&secret).map_err(|_| {
            AppError::Internal("buzz connection key is not a valid secret key".to_string())
        })?;
        if keys.public_key().to_hex() != self.bridge_pubkey {
            return Err(AppError::Internal(
                "buzz connection key does not match the stored bridge pubkey".to_string(),
            ));
        }
        Ok(
            crate::integrations::buzz::connector::BuzzConnectionRuntime {
                id: self.id,
                relay_url: self.relay_url,
                keys,
            },
        )
    }
}

/// A stored channel mapping.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BuzzChannelMappingRecord {
    pub id: Uuid,
    pub connection_id: Uuid,
    pub rustchat_channel_id: Uuid,
    pub buzz_channel_id: Uuid,
    pub outbound_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating a connection. The private key is accepted in hex or
/// bech32 (`nsec…`) form, encrypted immediately, and never persisted raw.
#[derive(Debug, Clone)]
pub struct NewBuzzConnection {
    pub name: String,
    pub relay_url: String,
    pub private_key: String,
    pub enabled: bool,
}

pub struct BuzzRepository<'a> {
    pool: &'a sqlx::PgPool,
}

fn db_err(e: sqlx::Error) -> AppError {
    AppError::Internal(format!("database error: {e}"))
}

impl<'a> BuzzRepository<'a> {
    pub fn new(pool: &'a sqlx::PgPool) -> Self {
        Self { pool }
    }

    /// Create a connection, encrypting the private key at rest.
    ///
    /// Returns a validation error for a malformed name/URL/key.
    pub async fn create_connection(
        &self,
        input: NewBuzzConnection,
        encryption_key: &str,
    ) -> Result<BuzzConnectionRecord, AppError> {
        let name = input.name.trim();
        if name.is_empty() || name.len() > 100 {
            return Err(AppError::Validation(
                "connection name must be 1-100 characters".to_string(),
            ));
        }
        let relay_url = input.relay_url.trim().to_string();
        if !crate::services::webhooks::is_valid_callback_url(&relay_url) {
            return Err(AppError::Validation(
                "relay URL is not an acceptable public http(s) URL".to_string(),
            ));
        }
        let keys = nostr::key::Keys::parse(input.private_key.trim()).map_err(|_| {
            AppError::Validation("private key must be hex or nsec (bech32)".to_string())
        })?;
        let encrypted = crate::crypto::encrypt(input.private_key.trim(), encryption_key)
            .map_err(|e| AppError::Internal(format!("key encryption failed: {e}")))?;

        let record = sqlx::query_as::<_, BuzzConnectionRecord>(
            r#"
            INSERT INTO buzz_connections (name, relay_url, bridge_pubkey, private_key_encrypted, enabled)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(&relay_url)
        .bind(keys.public_key().to_hex())
        .bind(encrypted)
        .bind(input.enabled)
        .fetch_one(self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                AppError::Validation("a connection with this name or relay URL already exists".to_string())
            }
            _ => db_err(e),
        })?;
        Ok(record)
    }

    /// Rotate the signing key (and derived pubkey) of a connection.
    pub async fn rotate_connection_key(
        &self,
        id: Uuid,
        private_key: &str,
        encryption_key: &str,
    ) -> Result<BuzzConnectionRecord, AppError> {
        let keys = nostr::key::Keys::parse(private_key.trim()).map_err(|_| {
            AppError::Validation("private key must be hex or nsec (bech32)".to_string())
        })?;
        let encrypted = crate::crypto::encrypt(private_key.trim(), encryption_key)
            .map_err(|e| AppError::Internal(format!("key encryption failed: {e}")))?;
        sqlx::query_as::<_, BuzzConnectionRecord>(
            r#"
            UPDATE buzz_connections
            SET private_key_encrypted = $2, bridge_pubkey = $3, updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(encrypted)
        .bind(keys.public_key().to_hex())
        .fetch_optional(self.pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| AppError::Validation("connection not found".to_string()))
    }

    pub async fn update_connection(
        &self,
        id: Uuid,
        name: Option<String>,
        relay_url: Option<String>,
        enabled: Option<bool>,
    ) -> Result<BuzzConnectionRecord, AppError> {
        if let Some(url) = &relay_url {
            if !crate::services::webhooks::is_valid_callback_url(url) {
                return Err(AppError::Validation(
                    "relay URL is not an acceptable public http(s) URL".to_string(),
                ));
            }
        }
        if let Some(n) = &name {
            let n = n.trim();
            if n.is_empty() || n.len() > 100 {
                return Err(AppError::Validation(
                    "connection name must be 1-100 characters".to_string(),
                ));
            }
        }
        sqlx::query_as::<_, BuzzConnectionRecord>(
            r#"
            UPDATE buzz_connections
            SET name = COALESCE($2, name),
                relay_url = COALESCE($3, relay_url),
                enabled = COALESCE($4, enabled),
                updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name.map(|n| n.trim().to_string()))
        .bind(relay_url.map(|u| u.trim().to_string()))
        .bind(enabled)
        .fetch_optional(self.pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| AppError::Validation("connection not found".to_string()))
    }

    /// Delete a connection. Active outbox rows are dead-lettered first so
    /// nothing is left in flight against a removed credential; delivered
    /// history is retained (connection_id set to NULL).
    pub async fn delete_connection(&self, id: Uuid) -> Result<(), AppError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::Internal(format!("key encryption failed: {e}")))?;
        sqlx::query(
            r#"
            UPDATE integration_outbox
            SET status = 'dead_letter',
                last_error = 'buzz connection deleted',
                updated_at = now()
            WHERE connection_id = $1 AND status IN ('pending', 'in_flight')
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(format!("key encryption failed: {e}")))?;
        let result = sqlx::query("DELETE FROM buzz_connections WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(format!("key encryption failed: {e}")))?;
        tx.commit()
            .await
            .map_err(|e| AppError::Internal(format!("key encryption failed: {e}")))?;
        if result.rows_affected() == 0 {
            return Err(AppError::Validation("connection not found".to_string()));
        }
        Ok(())
    }

    pub async fn get_connection(&self, id: Uuid) -> Result<Option<BuzzConnectionRecord>, AppError> {
        sqlx::query_as::<_, BuzzConnectionRecord>("SELECT * FROM buzz_connections WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
            .map_err(db_err)
    }

    pub async fn list_connections(&self) -> Result<Vec<BuzzConnectionRecord>, AppError> {
        sqlx::query_as::<_, BuzzConnectionRecord>(
            "SELECT * FROM buzz_connections ORDER BY created_at",
        )
        .fetch_all(self.pool)
        .await
        .map_err(db_err)
    }

    /// Create or replace a channel mapping (idempotent upsert).
    pub async fn upsert_mapping(
        &self,
        connection_id: Uuid,
        rustchat_channel_id: Uuid,
        buzz_channel_id: Uuid,
        outbound_enabled: bool,
    ) -> Result<BuzzChannelMappingRecord, AppError> {
        sqlx::query_as::<_, BuzzChannelMappingRecord>(
            r#"
            INSERT INTO buzz_channel_mappings
                (connection_id, rustchat_channel_id, buzz_channel_id, outbound_enabled)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (connection_id, rustchat_channel_id)
            DO UPDATE SET buzz_channel_id = $3, outbound_enabled = $4, updated_at = now()
            RETURNING *
            "#,
        )
        .bind(connection_id)
        .bind(rustchat_channel_id)
        .bind(buzz_channel_id)
        .bind(outbound_enabled)
        .fetch_one(self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db) if db.is_unique_violation() => AppError::Validation(
                "the Buzz channel is already mapped to another RustChat channel on this connection"
                    .to_string(),
            ),
            sqlx::Error::Database(db) if db.is_foreign_key_violation() => {
                AppError::Validation("connection or RustChat channel does not exist".to_string())
            }
            _ => db_err(e),
        })
    }

    pub async fn delete_mapping(
        &self,
        connection_id: Uuid,
        mapping_id: Uuid,
    ) -> Result<bool, AppError> {
        let result =
            sqlx::query("DELETE FROM buzz_channel_mappings WHERE id = $1 AND connection_id = $2")
                .bind(mapping_id)
                .bind(connection_id)
                .execute(self.pool)
                .await
                .map_err(|e| AppError::Internal(format!("key encryption failed: {e}")))?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn list_mappings(
        &self,
        connection_id: Uuid,
    ) -> Result<Vec<BuzzChannelMappingRecord>, AppError> {
        sqlx::query_as::<_, BuzzChannelMappingRecord>(
            "SELECT * FROM buzz_channel_mappings WHERE connection_id = $1 ORDER BY created_at",
        )
        .bind(connection_id)
        .fetch_all(self.pool)
        .await
        .map_err(db_err)
    }

    /// Active (enabled connection, outbound-enabled) mappings for a RustChat
    /// channel — the enqueue-time lookup on the post-creation path.
    pub async fn active_mappings_for_channel(
        &self,
        rustchat_channel_id: Uuid,
    ) -> Result<Vec<BuzzChannelMappingRecord>, AppError> {
        sqlx::query_as::<_, BuzzChannelMappingRecord>(
            r#"
            SELECT m.* FROM buzz_channel_mappings m
            JOIN buzz_connections c ON c.id = m.connection_id
            WHERE m.rustchat_channel_id = $1
              AND m.outbound_enabled
              AND c.enabled
            "#,
        )
        .bind(rustchat_channel_id)
        .fetch_all(self.pool)
        .await
        .map_err(db_err)
    }
}
