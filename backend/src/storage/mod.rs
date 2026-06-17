//! Storage module for rustchat
//!
//! Provides S3-compatible storage backend for files.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::error::AppError;

mod s3;

pub use s3::*;

/// A single object returned by a storage list operation.
#[derive(Debug, Clone)]
pub struct ListedObject {
    /// Object key.
    pub key: String,
    /// Last-modified timestamp, when available from the backend.
    pub last_modified: Option<DateTime<Utc>>,
}

/// Result of a paginated object listing operation.
#[derive(Debug, Clone, Default)]
pub struct ListObjectsResult {
    /// Object keys returned in this page (convenience alias for `objects.key`).
    pub keys: Vec<String>,
    /// Objects returned in this page, including metadata when available.
    pub objects: Vec<ListedObject>,
    /// Token to retrieve the next page, if any.
    pub next_continuation_token: Option<String>,
}

/// Abstraction over an object storage backend.
#[async_trait]
pub trait ObjectStorage: Send + Sync {
    /// Delete the object at `key`. Missing objects are treated as success.
    async fn delete_object(&self, key: &str) -> Result<(), AppError>;

    /// List objects in the storage backend, optionally restricted to keys that
    /// begin with `prefix`.
    async fn list_objects(
        &self,
        prefix: Option<&str>,
        continuation_token: Option<&str>,
        max_keys: i32,
    ) -> Result<ListObjectsResult, AppError>;
}
