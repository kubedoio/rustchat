//! Configuration module for rustchat
//!
//! Supports loading configuration from environment variables and .env files.

use serde::Deserialize;

/// Application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Server host address
    #[serde(default = "default_host")]
    pub server_host: String,

    /// Server port
    #[serde(default = "default_port")]
    pub server_port: u16,

    /// PostgreSQL database URL
    pub database_url: String,

    /// Redis connection URL
    #[serde(default = "default_redis_url")]
    pub redis_url: String,

    /// JWT secret key
    pub jwt_secret: String,

    /// Encryption key for sensitive data
    pub encryption_key: String,

    /// JWT token expiry in hours
    #[serde(default = "default_jwt_expiry")]
    pub jwt_expiry_hours: u64,

    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// S3 endpoint URL
    #[serde(default)]
    pub s3_endpoint: Option<String>,

    /// Public S3 endpoint URL (for presigned URLs returned to clients)
    #[serde(default)]
    pub s3_public_endpoint: Option<String>,

    /// S3 bucket name
    #[serde(default = "default_s3_bucket")]
    pub s3_bucket: String,

    /// S3 access key
    #[serde(default)]
    pub s3_access_key: Option<String>,

    /// S3 secret key
    #[serde(default)]
    pub s3_secret_key: Option<String>,

    /// S3 region
    #[serde(default = "default_s3_region")]
    pub s3_region: String,

    /// Initial admin email
    #[serde(default)]
    pub admin_user: Option<String>,

    /// Initial admin password
    #[serde(default)]
    pub admin_password: Option<String>,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_redis_url() -> String {
    "redis://localhost:6379".to_string()
}

fn default_jwt_expiry() -> u64 {
    24
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_s3_bucket() -> String {
    "rustchat".to_string()
}

fn default_s3_region() -> String {
    "us-east-1".to_string()
}

impl Config {
    /// Load configuration from environment variables
    pub fn load() -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(
                config::Environment::default()
                    .prefix("RUSTCHAT")
                    .try_parsing(true),
            )
            .build()?;

        let settings: Config = config.try_deserialize()?;
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        assert_eq!(default_host(), "0.0.0.0");
        assert_eq!(default_port(), 3000);
        assert_eq!(default_log_level(), "info");
    }
}
