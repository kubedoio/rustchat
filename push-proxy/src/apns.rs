//! Apple Push Notification Service (APNS) for VoIP pushes
//!
//! Handles sending VoIP push notifications to iOS devices via APNS HTTP/2 API.
//! Uses JWT-based authentication (recommended by Apple).

use serde::Serialize;
use std::path::PathBuf;
use tracing::{info, warn};

/// APNS VoIP push payload
#[derive(Debug, Serialize)]
pub struct ApnsVoipPayload {
    /// APNS topic (bundle ID + ".voip" suffix)
    pub topic: String,
    /// Device token
    pub device_token: String,
    /// Call UUID (required for CallKit)
    pub call_uuid: String,
    /// Caller name to display
    pub caller_name: String,
    /// Channel ID for navigation
    pub channel_id: String,
    /// Server URL so app knows which server
    pub server_url: String,
    /// Handle type (phone number, email, or generic)
    pub handle_type: String,
    /// Has video
    pub has_video: bool,
}

/// APNS standard notification payload
#[derive(Debug, Serialize)]
pub struct ApnsMessagePayload {
    /// APNS topic (bundle ID without ".voip")
    pub topic: String,
    /// Device token
    pub device_token: String,
    /// Notification title
    pub title: String,
    /// Notification body
    pub body: String,
    /// Channel ID for navigation
    pub channel_id: String,
    /// Post ID for navigation
    pub post_id: String,
    /// Server URL so app knows which server
    pub server_url: String,
    /// Notification type
    pub notification_type: String,
    /// Optional sub_type (e.g. "calls")
    pub sub_type: Option<String>,
    /// Optional sender name
    pub sender_name: Option<String>,
    /// Optional CRT flag
    pub is_crt_enabled: Option<bool>,
}

/// APNS configuration
#[derive(Debug, Clone)]
pub struct ApnsConfig {
    /// Path to the APNS auth key (.p8 file)
    pub key_path: PathBuf,
    /// Key ID from Apple Developer Portal
    pub key_id: String,
    /// Team ID from Apple Developer Portal
    pub team_id: String,
    /// Bundle identifier
    pub bundle_id: String,
    /// APNS server environment
    pub server: ApnsServer,
}

/// APNS server environment
#[derive(Debug, Clone, Copy)]
pub enum ApnsServer {
    /// Production server
    Production,
    /// Development/Sandbox server
    Development,
}

impl ApnsServer {
    pub fn url(&self) -> &'static str {
        match self {
            ApnsServer::Production => "https://api.push.apple.com",
            ApnsServer::Development => "https://api.development.push.apple.com",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ApnsError {
    #[error("Certificate/Key error: {0}")]
    Certificate(String),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("APNS error: {0}")]
    Apns(String),
    #[error("Invalid device token")]
    InvalidToken,
    #[error("JWT error: {0}")]
    Jwt(String),
}

/// JWT claims for APNS authentication
#[derive(Debug, Serialize)]
struct ApnsJwtClaims {
    /// Issuer (Team ID)
    iss: String,
    /// Issued at (Unix timestamp)
    iat: i64,
}

/// APNS HTTP/2 client for VoIP pushes
pub struct ApnsClient {
    http_client: reqwest::Client,
    /// APNS configuration
    pub config: ApnsConfig,
    /// JWT authentication token
    auth_token: String,
    /// Token expiration time (intentionally kept for future refresh logic)
    #[allow(dead_code)]
    token_expires_at: chrono::DateTime<chrono::Utc>,
}

impl ApnsClient {
    /// Create a new APNS client with JWT authentication
    pub async fn new(config: ApnsConfig) -> Result<Self, ApnsError> {
        // Build HTTP/2 client
        let http_client = reqwest::Client::builder()
            .http2_prior_knowledge()
            .build()
            .map_err(ApnsError::Network)?;

        // Generate initial JWT token
        let (auth_token, token_expires_at) = generate_jwt_token(&config).await?;

        info!(
            bundle_id = %config.bundle_id,
            server = ?config.server,
            "APNS client initialized for VoIP pushes"
        );

        Ok(Self {
            http_client,
            config,
            auth_token,
            token_expires_at,
        })
    }

    /// Get a valid JWT token (refreshing if necessary)
    #[allow(dead_code)]
    fn get_auth_token(&mut self) -> Result<String, ApnsError> {
        let now = chrono::Utc::now();

        // Refresh token if it expires within 5 minutes
        if now + chrono::Duration::minutes(5) > self.token_expires_at {
            // Note: In production, you'd want to handle this asynchronously
            // For now, we'll just return an error indicating token refresh is needed
            return Err(ApnsError::Jwt(
                "Token expired, client needs refresh".to_string(),
            ));
        }

        Ok(self.auth_token.clone())
    }

    /// Send a VoIP push notification
    ///
    /// # Important
    /// This must complete within milliseconds or iOS will terminate the app.
    pub async fn send_voip_push(&self, payload: ApnsVoipPayload) -> Result<(), ApnsError> {
        let url = format!(
            "{}/3/device/{}",
            self.config.server.url(),
            payload.device_token
        );

        // Build the APNS payload
        // This format is required for CallKit integration
        let apns_payload = serde_json::json!({
            "aps": {
                "alert": {
                    "title": payload.caller_name,
                    "body": "Incoming call"
                },
                "sound": "calls_ringtone.caf",
                "badge": 1,
                "content-available": 1,
                "mutable-content": 1
            },
            // Custom data for the app
            "data": {
                "type": "call",
                "call_uuid": payload.call_uuid,
                "caller_name": payload.caller_name,
                "channel_id": payload.channel_id,
                "server_url": payload.server_url,
                "has_video": payload.has_video,
                "is_voip": true
            }
        });

        // Send the request with JWT authentication
        let response = self
            .http_client
            .post(&url)
            .header("authorization", format!("bearer {}", self.auth_token))
            .header("apns-topic", &payload.topic)
            .header("apns-push-type", "voip")
            .header("apns-priority", "10") // Immediate delivery
            .json(&apns_payload)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            info!(
                token_prefix = %&payload.device_token[..20.min(payload.device_token.len())],
                "VoIP push sent successfully"
            );
            Ok(())
        } else {
            let body = response.text().await.unwrap_or_default();

            // Invalid token / topic-token mismatch cases from APNS.
            if status.as_u16() == 410 || (status.as_u16() == 400 && is_invalid_token_reason(&body))
            {
                warn!(status = %status, "APNS token is no longer valid");
                return Err(ApnsError::InvalidToken);
            }

            warn!(status = %status, body = %body, "APNS error");
            Err(ApnsError::Apns(format!("HTTP {}: {}", status, body)))
        }
    }

    /// Send a standard APNS push notification.
    pub async fn send_message_push(&self, payload: ApnsMessagePayload) -> Result<(), ApnsError> {
        let url = format!(
            "{}/3/device/{}",
            self.config.server.url(),
            payload.device_token
        );

        let mut apns_payload = serde_json::json!({
            "aps": {
                "alert": {
                    "title": payload.title,
                    "body": payload.body
                },
                "sound": "default",
                "badge": 1,
                "content-available": 1,
                "mutable-content": 1
            },
            "type": payload.notification_type,
            "version": "2",
            "channel_id": payload.channel_id,
            "post_id": payload.post_id,
            "server_url": payload.server_url,
        });

        if let Some(sub_type) = payload.sub_type {
            apns_payload["sub_type"] = serde_json::Value::String(sub_type);
        }
        if let Some(sender_name) = payload.sender_name {
            apns_payload["sender_name"] = serde_json::Value::String(sender_name);
        }
        if let Some(is_crt_enabled) = payload.is_crt_enabled {
            apns_payload["is_crt_enabled"] = serde_json::Value::Bool(is_crt_enabled);
        }

        let response = self
            .http_client
            .post(&url)
            .header("authorization", format!("bearer {}", self.auth_token))
            .header("apns-topic", &payload.topic)
            .header("apns-push-type", "alert")
            .header("apns-priority", "10")
            .json(&apns_payload)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            info!(
                token_prefix = %&payload.device_token[..20.min(payload.device_token.len())],
                "Standard APNS push sent successfully"
            );
            Ok(())
        } else {
            let body = response.text().await.unwrap_or_default();
            if status.as_u16() == 410 || (status.as_u16() == 400 && is_invalid_token_reason(&body))
            {
                warn!(status = %status, "APNS token is no longer valid");
                return Err(ApnsError::InvalidToken);
            }

            warn!(status = %status, body = %body, "APNS error");
            Err(ApnsError::Apns(format!("HTTP {}: {}", status, body)))
        }
    }
}

fn is_invalid_token_reason(body: &str) -> bool {
    let body_lc = body.to_lowercase();
    body_lc.contains("baddevicetoken")
        || body_lc.contains("devicetokennotfortopic")
        || body_lc.contains("unregistered")
}

/// Generate JWT token for APNS authentication
async fn generate_jwt_token(
    config: &ApnsConfig,
) -> Result<(String, chrono::DateTime<chrono::Utc>), ApnsError> {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    // Read the private key
    let key_content = tokio::fs::read_to_string(&config.key_path)
        .await
        .map_err(|e| ApnsError::Certificate(format!("Failed to read APNS key: {}", e)))?;

    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::hours(1); // Token valid for 1 hour

    let claims = ApnsJwtClaims {
        iss: config.team_id.clone(),
        iat: now.timestamp(),
    };

    let header = Header {
        alg: Algorithm::ES256,
        kid: Some(config.key_id.clone()),
        ..Default::default()
    };

    let key = EncodingKey::from_ec_pem(key_content.as_bytes())
        .map_err(|e| ApnsError::Jwt(format!("Failed to parse key: {}", e)))?;

    let token = encode(&header, &claims, &key)
        .map_err(|e| ApnsError::Jwt(format!("Failed to encode JWT: {}", e)))?;

    Ok((token, expires_at))
}

/// Parse APNS topic from bundle ID
pub fn build_voip_topic(bundle_id: &str) -> String {
    if bundle_id.ends_with(".voip") {
        bundle_id.to_string()
    } else {
        format!("{}.voip", bundle_id)
    }
}

/// Build standard APNS topic (bundle ID without ".voip" suffix).
pub fn build_alert_topic(bundle_id: &str) -> String {
    bundle_id.trim_end_matches(".voip").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_voip_topic() {
        assert_eq!(
            build_voip_topic("com.rustchat.app"),
            "com.rustchat.app.voip"
        );
        assert_eq!(
            build_voip_topic("com.rustchat.app.voip"),
            "com.rustchat.app.voip"
        );
    }

    #[test]
    fn test_build_alert_topic() {
        assert_eq!(build_alert_topic("com.rustchat.app"), "com.rustchat.app");
        assert_eq!(
            build_alert_topic("com.rustchat.app.voip"),
            "com.rustchat.app"
        );
    }
}
