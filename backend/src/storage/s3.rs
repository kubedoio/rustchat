//! S3-compatible storage client

use aws_config::Region;
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::error::SdkError;
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
    public_client: Option<Client>,
}

impl S3Client {
    /// Create a new S3 client
    pub fn new(
        endpoint: Option<String>,
        public_endpoint: Option<String>,
        bucket: String,
        access_key: Option<String>,
        secret_key: Option<String>,
        region: String,
    ) -> Self {
        let access_key_main = access_key.clone();
        let secret_key_main = secret_key.clone();
        let region_main = region.clone();

        let credentials = match (access_key_main, secret_key_main) {
            (Some(ak), Some(sk)) => Some(Credentials::new(ak, sk, None, None, "rustchat")),
            _ => None,
        };

        let mut config_builder = Config::builder()
            .region(Region::new(region_main))
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

        let public_client = public_endpoint.as_ref().map(|ep| {
            let mut public_builder = Config::builder()
                .region(Region::new(region.clone()))
                .behavior_version_latest()
                .force_path_style(true);

            if let (Some(ak), Some(sk)) = (access_key.clone(), secret_key.clone()) {
                let creds = Credentials::new(ak, sk, None, None, "rustchat");
                public_builder =
                    public_builder.credentials_provider(SharedCredentialsProvider::new(creds));
            }

            public_builder = public_builder.endpoint_url(ep);

            let public_config = public_builder.build();
            Client::from_conf(public_config)
        });

        Self {
            client,
            bucket,
            endpoint,
            public_client,
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
                AppError::ExternalService(format!("S3 upload error: {}", e))
            })?;

        Ok(())
    }

    /// Upload a file from a local path to S3
    pub async fn upload_file(
        &self,
        key: &str,
        path: &std::path::Path,
        content_type: &str,
    ) -> Result<(), AppError> {
        let body = ByteStream::from_path(path)
            .await
            .map_err(|e| AppError::ExternalService(format!("ByteStream from path error: {}", e)))?;

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
                AppError::ExternalService(format!("S3 upload error: {}", e))
            })?;

        Ok(())
    }

    /// Ensure bucket exists (create if missing) and configure CORS
    pub async fn ensure_bucket(&self) -> Result<(), AppError> {
        let result = self
            .client
            .create_bucket()
            .bucket(&self.bucket)
            .send()
            .await;

        match result {
            Ok(_) => {
                // New bucket created, configure CORS
                self.configure_cors().await?;
                Ok(())
            }
            Err(SdkError::ServiceError(service_error)) => {
                let code = service_error.err().code().unwrap_or_default();
                if code == "BucketAlreadyOwnedByYou" || code == "BucketAlreadyExists" {
                    // Bucket exists, ensure CORS is configured
                    self.configure_cors().await?;
                    Ok(())
                } else {
                    error!(error = ?service_error, bucket = %self.bucket, "S3 create bucket failed");
                    Err(AppError::Internal(format!(
                        "S3 create bucket error: {:?}",
                        service_error
                    )))
                }
            }
            Err(e) => {
                error!(error = ?e, bucket = %self.bucket, "S3 create bucket failed");
                Err(AppError::Internal(format!("S3 create bucket error: {}", e)))
            }
        }
    }

    /// Configure CORS for the bucket to allow cross-origin image loading
    async fn configure_cors(&self) -> Result<(), AppError> {
        use aws_sdk_s3::types::{CorsConfiguration, CorsRule};

        let cors_rule = CorsRule::builder()
            .allowed_origins("*") // Allow all origins - files are accessed via presigned URLs
            .allowed_methods("GET")
            .allowed_methods("HEAD")
            .allowed_headers("*")
            .max_age_seconds(3600)
            .build()
            .map_err(|e| AppError::Internal(format!("CORS rule build error: {}", e)))?;

        let cors_config = CorsConfiguration::builder()
            .cors_rules(cors_rule)
            .build()
            .map_err(|e| AppError::Internal(format!("CORS config build error: {}", e)))?;

        match self
            .client
            .put_bucket_cors()
            .bucket(&self.bucket)
            .cors_configuration(cors_config)
            .send()
            .await
        {
            Ok(_) => {
                tracing::info!(bucket = %self.bucket, "S3 CORS configuration applied");
                Ok(())
            }
            Err(e) => {
                // Log but don't fail - some S3-compatible services don't support CORS
                tracing::warn!(error = ?e, bucket = %self.bucket, "Failed to configure S3 CORS");
                Ok(())
            }
        }
    }

    /// Lightweight readiness probe for S3 dependency.
    pub async fn health_check(&self) -> bool {
        self.client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .is_ok()
    }

    /// Download a file from S3
    pub async fn download(&self, key: &str) -> Result<Vec<u8>, AppError> {
        self.download_optional(key).await?.ok_or_else(|| {
            error!(bucket = %self.bucket, key = %key, "S3 file not found");
            AppError::NotFound(format!("File not found: {}", key))
        })
    }

    /// Download a file from S3, returning None if the key doesn't exist
    pub async fn download_optional(&self, key: &str) -> Result<Option<Vec<u8>>, AppError> {
        match self.download_stream_optional(key).await? {
            Some(stream) => {
                let data = stream
                    .collect()
                    .await
                    .map_err(|e| AppError::Internal(format!("S3 read error: {}", e)))?
                    .into_bytes()
                    .to_vec();
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// Stream a file from S3 as a ByteStream.
    pub async fn download_stream(&self, key: &str) -> Result<ByteStream, AppError> {
        self.download_stream_optional(key).await?.ok_or_else(|| {
            error!(bucket = %self.bucket, key = %key, "S3 file not found");
            AppError::NotFound(format!("File not found: {}", key))
        })
    }

    /// Stream a file from S3, returning None if the key doesn't exist
    pub async fn download_stream_optional(
        &self,
        key: &str,
    ) -> Result<Option<ByteStream>, AppError> {
        let result = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await;

        match result {
            Ok(response) => Ok(Some(response.body)),
            Err(SdkError::ServiceError(service_error)) => {
                let code = service_error.err().code().unwrap_or_default();
                // Treat all common S3/S3-compatible "not found" codes as a missing key so
                // callers can fall back gracefully.
                if matches!(code, "NoSuchKey" | "NotFound" | "NoSuchObject") {
                    Ok(None)
                } else {
                    error!(error = ?service_error, bucket = %self.bucket, key = %key, "S3 download failed");
                    Err(AppError::ExternalService(format!(
                        "S3 download error: {:?}",
                        service_error
                    )))
                }
            }
            Err(e) => {
                error!(error = ?e, bucket = %self.bucket, key = %key, "S3 download failed");
                Err(AppError::ExternalService(format!(
                    "S3 download error: {}",
                    e
                )))
            }
        }
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

        let presign_client = self.public_client.as_ref().unwrap_or(&self.client);
        let presigned = presign_client
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

        let presign_client = self.public_client.as_ref().unwrap_or(&self.client);
        let presigned = presign_client
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
