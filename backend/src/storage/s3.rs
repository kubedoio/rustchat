//! S3-compatible storage client

use aws_config::Region;
use aws_sdk_s3::{
    config::{Credentials, SharedCredentialsProvider},
    presigning::PresigningConfig,
    primitives::ByteStream,
    Client, Config,
};
use std::time::Duration;
use tracing::error;

use crate::error::AppError;

/// S3 storage client
#[derive(Clone)]
pub struct S3Client {
    client: Client,
    bucket: String,
    endpoint: Option<String>,
}

impl S3Client {
    /// Create a new S3 client
    pub fn new(
        endpoint: Option<String>,
        bucket: String,
        access_key: Option<String>,
        secret_key: Option<String>,
        region: String,
    ) -> Self {
        let credentials = match (access_key, secret_key) {
            (Some(ak), Some(sk)) => Some(Credentials::new(ak, sk, None, None, "rustchat")),
            _ => None,
        };

        let mut config_builder = Config::builder()
            .region(Region::new(region))
            .behavior_version_latest()
            .force_path_style(true);

        if let Some(creds) = credentials {
            config_builder =
                config_builder.credentials_provider(SharedCredentialsProvider::new(creds));
        }

        if let Some(ref ep) = endpoint {
            config_builder = config_builder.endpoint_url(ep);
        }

        let config = config_builder.build();
        let client = Client::from_conf(config);

        Self {
            client,
            bucket,
            endpoint,
        }
    }

    /// Upload a file to S3
    pub async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> Result<(), AppError> {
        let body = ByteStream::from(data);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| {
                error!(error = ?e, bucket = %self.bucket, key = %key, "S3 upload failed");
                AppError::Internal(format!("S3 upload error: {}", e))
            })?;

        Ok(())
    }

    /// Download a file from S3
    pub async fn download(&self, key: &str) -> Result<Vec<u8>, AppError> {
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                error!(error = ?e, bucket = %self.bucket, key = %key, "S3 download failed");
                AppError::Internal(format!("S3 download error: {}", e))
            })?;

        let data = response
            .body
            .collect()
            .await
            .map_err(|e| AppError::Internal(format!("S3 read error: {}", e)))?
            .into_bytes()
            .to_vec();

        Ok(data)
    }

    /// Delete a file from S3
    pub async fn delete(&self, key: &str) -> Result<(), AppError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                error!(error = ?e, bucket = %self.bucket, key = %key, "S3 delete failed");
                AppError::Internal(format!("S3 delete error: {}", e))
            })?;

        Ok(())
    }

    /// Generate a presigned download URL
    pub async fn presigned_download_url(
        &self,
        key: &str,
        expires_in_secs: u64,
    ) -> Result<String, AppError> {
        let presigning_config = PresigningConfig::builder()
            .expires_in(Duration::from_secs(expires_in_secs))
            .build()
            .map_err(|e| AppError::Internal(format!("Presigning config error: {}", e)))?;

        let presigned = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning_config)
            .await
            .map_err(|e| {
                error!(error = ?e, bucket = %self.bucket, key = %key, "S3 presign download failed");
                AppError::Internal(format!("Presigning error: {}", e))
            })?;

        Ok(presigned.uri().to_string())
    }

    /// Generate a presigned upload URL
    pub async fn presigned_upload_url(
        &self,
        key: &str,
        content_type: &str,
        expires_in_secs: u64,
    ) -> Result<String, AppError> {
        let presigning_config = PresigningConfig::builder()
            .expires_in(Duration::from_secs(expires_in_secs))
            .build()
            .map_err(|e| AppError::Internal(format!("Presigning config error: {}", e)))?;

        let presigned = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presigning_config)
            .await
            .map_err(|e| {
                error!(error = ?e, bucket = %self.bucket, key = %key, "S3 presign upload failed");
                AppError::Internal(format!("Presigning error: {}", e))
            })?;

        Ok(presigned.uri().to_string())
    }

    /// Get the public URL for a file (if bucket is public)
    pub fn public_url(&self, key: &str) -> String {
        if let Some(ref endpoint) = self.endpoint {
            format!("{}/{}/{}", endpoint, self.bucket, key)
        } else {
            format!("https://{}.s3.amazonaws.com/{}", self.bucket, key)
        }
    }
}
