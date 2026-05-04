//! TURN credential generation for Mattermost Calls
//!
//! Supports both static credentials (from environment variables) and REST API style ephemeral credentials.
//! Static credentials are used by default if TURN_SERVER_USERNAME and TURN_SERVER_CREDENTIAL are provided.
#![allow(dead_code)]

use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::time::{SystemTime, UNIX_EPOCH};

/// TURN credentials for a user
#[derive(Debug, Clone)]
pub struct TurnCredentials {
    pub username: String,
    pub credential: String,
}

/// TURN server configuration
#[derive(Debug, Clone)]
pub struct TurnServerConfig {
    pub enabled: bool,
    pub url: String,
    pub username: String,
    pub credential: String,
}

/// Generates TURN credentials
///
/// Supports two modes:
/// 1. Static credentials: Uses pre-configured username/password from env vars
/// 2. REST API style: Generates ephemeral credentials with HMAC-SHA1
pub struct TurnCredentialGenerator {
    config: TurnServerConfig,
    use_static_credentials: bool,
    ttl_minutes: u64,
}

impl TurnCredentialGenerator {
    /// Create a new credential generator with static credentials
    pub fn with_static_credentials(config: TurnServerConfig) -> Self {
        Self {
            config: config.clone(),
            use_static_credentials: !config.username.is_empty() && !config.credential.is_empty(),
            ttl_minutes: 1440, // Default 24 hours
        }
    }

    /// Create a new credential generator with REST API style ephemeral credentials
    pub fn with_rest_api(secret: String, ttl_minutes: u64) -> Self {
        Self {
            config: TurnServerConfig {
                enabled: true,
                url: String::new(),
                username: secret,
                credential: String::new(),
            },
            use_static_credentials: false,
            ttl_minutes,
        }
    }

    /// Generate TURN credentials for a user
    ///
    /// If static credentials are configured, returns those.
    /// Otherwise, generates ephemeral credentials using REST API style.
    pub fn generate_credentials(&self, user_id: &str) -> TurnCredentials {
        if self.use_static_credentials {
            // Return static credentials from configuration
            TurnCredentials {
                username: self.config.username.clone(),
                credential: self.config.credential.clone(),
            }
        } else {
            // Generate ephemeral credentials
            self.generate_ephemeral_credentials(user_id)
        }
    }

    /// Generate ephemeral TURN credentials using REST API style
    ///
    /// Uses the TURN REST API authentication mechanism:
    /// - Username: "{timestamp}:{user_id}"
    /// - Credential: base64(HMAC-SHA1(secret, username))
    fn generate_ephemeral_credentials(&self, user_id: &str) -> TurnCredentials {
        // Calculate expiration timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expiration = now + (self.ttl_minutes * 60);

        // Username format: "{expiration}:{user_id}"
        let username = format!("{}:{}", expiration, user_id);

        // Generate HMAC-SHA1 using the secret as the key
        let credential = self.hmac_sha1(&self.config.username, &username);

        TurnCredentials {
            username,
            credential: base64_encode(&credential),
        }
    }

    /// Validate ephemeral credentials
    pub fn validate_ephemeral_credentials(&self, username: &str, credential: &str) -> bool {
        // Parse username to extract expiration and user_id
        let parts: Vec<&str> = username.split(':').collect();
        if parts.len() != 2 {
            return false;
        }

        let expiration = match parts[0].parse::<u64>() {
            Ok(ts) => ts,
            Err(_) => return false,
        };

        // Check if expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now > expiration {
            return false; // Credentials expired
        }

        // Validate HMAC
        let expected_credential = self.hmac_sha1(&self.config.username, username);
        let expected_encoded = base64_encode(&expected_credential);

        credential == expected_encoded
    }

    /// Get the TURN server URL
    pub fn get_turn_url(&self) -> Option<String> {
        if self.config.enabled && !self.config.url.is_empty() {
            Some(self.config.url.clone())
        } else {
            None
        }
    }

    /// Check if using static credentials
    pub fn is_using_static_credentials(&self) -> bool {
        self.use_static_credentials
    }

    /// Calculate HMAC-SHA1
    fn hmac_sha1(&self, key: &str, message: &str) -> Vec<u8> {
        type HmacSha1 = Hmac<Sha1>;

        let mut mac =
            HmacSha1::new_from_slice(key.as_bytes()).expect("HMAC can take key of any size");
        mac.update(message.as_bytes());

        mac.finalize().into_bytes().to_vec()
    }
}

/// Simple base64 encoding (using standard base64 alphabet)
fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_credentials() {
        let config = TurnServerConfig {
            enabled: true,
            url: "turn:turn.example.com:3478".to_string(),
            username: "test-user".to_string(),
            credential: "test-password".to_string(),
        };

        let generator = TurnCredentialGenerator::with_static_credentials(config);
        let creds = generator.generate_credentials("user-123");

        // Should return static credentials
        assert_eq!(creds.username, "test-user");
        assert_eq!(creds.credential, "test-password");
        assert!(generator.is_using_static_credentials());
    }

    #[test]
    fn test_ephemeral_credentials() {
        let generator = TurnCredentialGenerator::with_rest_api(
            "test-secret".to_string(),
            1440, // 24 hours
        );

        let creds = generator.generate_credentials("user-123");

        // Username should contain expiration and user_id
        assert!(creds.username.contains("user-123"));
        assert!(creds.username.contains(':'));

        // Credential should be valid base64
        assert!(!creds.credential.is_empty());

        // Should NOT be using static credentials
        assert!(!generator.is_using_static_credentials());

        // Validation should succeed
        assert!(generator.validate_ephemeral_credentials(&creds.username, &creds.credential));
    }

    #[test]
    fn test_invalid_credential() {
        let generator = TurnCredentialGenerator::with_rest_api("test-secret".to_string(), 1440);

        let creds = generator.generate_credentials("user-123");

        // Wrong credential should fail
        assert!(!generator.validate_ephemeral_credentials(&creds.username, "invalid-credential"));
    }
}
