//! Push Notification Service
//!
//! Handles sending push notifications to mobile devices via:
//! - Push Proxy service (recommended) which handles FCM/APNS
//! - Direct FCM (Firebase Cloud Messaging) for Android (fallback)
//! - Direct APNS (Apple Push Notification Service) for iOS (fallback)
//!
//! This service is essential for mattermost-mobile to receive:
//! - Call ringing notifications when app is in background
//! - Message notifications for mentions and direct messages

use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::api::AppState;
use crate::middleware::reliability::{send_reqwest_with_retry, RetryCondition, RetryConfig};

/// Push notification payload
#[derive(Debug, Clone, Serialize)]
pub struct PushNotification {
    /// Target device token
    pub device_token: String,
    /// Platform (ios/android)
    pub platform: String,
    /// Notification title
    pub title: String,
    /// Notification body
    pub body: String,
    /// Custom data payload
    pub data: serde_json::Value,
    /// Priority (high/normal)
    pub priority: PushPriority,
    /// Sound to play
    pub sound: Option<String>,
    /// Badge count
    pub badge: Option<i32>,
    /// Category for iOS
    pub category: Option<String>,
    /// Notification type (message, call)
    pub notification_type: NotificationType,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum NotificationType {
    Message,
    Call,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationType::Message => "message",
            NotificationType::Call => "call",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum PushPriority {
    High,
    Normal,
}

impl PushPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            PushPriority::High => "high",
            PushPriority::Normal => "normal",
        }
    }
}

/// Push Proxy payload structure
#[derive(Debug, Serialize)]
struct PushProxyPayload {
    token: String,
    title: String,
    body: String,
    /// Platform: "android" or "ios"
    platform: String,
    /// Notification type: "message" or "call"
    #[serde(rename = "type")]
    notification_type: String,
    data: PushProxyData,
}

#[derive(Debug, Serialize)]
struct PushProxyData {
    channel_id: String,
    post_id: String,
    #[serde(rename = "type")]
    notification_type: String,
    /// Sub-type for call notifications (set to "calls" for ringing)
    #[serde(skip_serializing_if = "Option::is_none")]
    sub_type: Option<String>,
    /// Push notification protocol version
    version: String,
    /// Sender user ID
    #[serde(skip_serializing_if = "Option::is_none")]
    sender_id: Option<String>,
    /// Sender display name
    #[serde(skip_serializing_if = "Option::is_none")]
    sender_name: Option<String>,
    /// CRT enabled flag
    #[serde(skip_serializing_if = "Option::is_none")]
    is_crt_enabled: Option<bool>,
    /// Server URL so the mobile app knows which server to navigate to
    #[serde(skip_serializing_if = "Option::is_none")]
    server_url: Option<String>,
    /// Call UUID for VoIP pushes (required for iOS CallKit)
    #[serde(skip_serializing_if = "Option::is_none")]
    call_uuid: Option<String>,
}

/// Send push notification to a specific device
pub async fn send_push_notification(
    state: &AppState,
    notification: PushNotification,
) -> Result<(), PushNotificationError> {
    info!(
        platform = %notification.platform,
        priority = ?notification.priority,
        token_prefix = %&notification.device_token[..20.min(notification.device_token.len())],
        "Sending push notification"
    );

    // First, try to use the push proxy service
    let proxy_url = get_push_proxy_url();
    info!(proxy_url = ?proxy_url, "Checking push proxy configuration");

    if let Some(proxy_url) = proxy_url {
        info!(%proxy_url, "Attempting to send via push proxy");
        match send_via_push_proxy(&proxy_url, &notification).await {
            Ok(_) => {
                info!("Push notification sent successfully via push proxy");
                return Ok(());
            }
            Err(PushNotificationError::InvalidToken) => {
                // Invalid token - don't try direct FCM, propagate error immediately
                // so the caller can delete the invalid device
                warn!("Push proxy reported invalid token, not falling back to direct FCM");
                return Err(PushNotificationError::InvalidToken);
            }
            Err(PushNotificationError::NotConfigured) => {
                // Proxy not configured, fall through to direct FCM
                info!("Push proxy not available, falling back to direct FCM");
            }
            Err(e) => {
                error!(error = %e, "Push proxy failed, falling back to direct FCM");
            }
        }
    } else {
        info!("RUSTCHAT_PUSH_PROXY_URL not set, checking direct FCM configuration");
    }

    // Fallback: Send directly via FCM HTTP v1 API
    send_push_notification_direct(state, notification).await
}

/// Get push proxy URL from environment
fn get_push_proxy_url() -> Option<String> {
    std::env::var("RUSTCHAT_PUSH_PROXY_URL").ok()
}

fn outbound_retry_config() -> RetryConfig {
    RetryConfig {
        max_attempts: 3,
        initial_delay: Duration::from_millis(150),
        max_delay: Duration::from_secs(2),
        backoff_multiplier: 2.0,
        retry_if: RetryCondition::Default,
    }
}

/// Send notification via push proxy service
async fn send_via_push_proxy(
    proxy_url: &str,
    notification: &PushNotification,
) -> Result<(), PushNotificationError> {
    let url = format!("{}/send", proxy_url.trim_end_matches('/'));

    // Extract channel_id and type from data payload
    let channel_id = notification
        .data
        .get("channel_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let notification_type = notification
        .data
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("message")
        .to_string();

    let post_id = notification
        .data
        .get("call_id")
        .or_else(|| notification.data.get("post_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Extract additional fields for mobile app compatibility
    let sub_type = notification
        .data
        .get("sub_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let sender_id = notification
        .data
        .get("sender_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let sender_name = notification
        .data
        .get("sender_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let is_crt_enabled = notification
        .data
        .get("is_crt_enabled")
        .and_then(|v| v.as_bool());

    let server_url = notification
        .data
        .get("server_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Generate call UUID for VoIP pushes (required for iOS CallKit)
    let call_uuid = if notification_type == "call" || sub_type.as_deref() == Some("calls") {
        Some(uuid::Uuid::new_v4().to_string())
    } else {
        None
    };

    let payload = PushProxyPayload {
        token: notification.device_token.clone(),
        title: notification.title.clone(),
        body: notification.body.clone(),
        platform: notification.platform.clone(),
        notification_type: notification.notification_type.as_str().to_string(),
        data: PushProxyData {
            channel_id,
            post_id,
            notification_type,
            sub_type,
            version: "2".to_string(),
            sender_id,
            sender_name,
            is_crt_enabled,
            server_url,
            call_uuid,
        },
    };

    let body = serde_json::to_vec(&payload).map_err(|e| {
        PushNotificationError::ProxyError(format!("JSON serialization error: {}", e))
    })?;
    let body_hash = hex::encode(sha2::Sha256::digest(&body));

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let nonce = uuid::Uuid::new_v4().to_string();

    let sig_input = format!("{}:{}:{}", timestamp, nonce, body_hash);

    let client = reqwest::Client::new();
    let mut request = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body);

    if let Ok(auth_key) = std::env::var("PUSH_PROXY_AUTH_KEY") {
        if !auth_key.is_empty() {
            let mut mac = Hmac::<Sha256>::new_from_slice(auth_key.as_bytes()).map_err(|e| {
                PushNotificationError::ProxyError(format!("Invalid push proxy auth key: {}", e))
            })?;
            mac.update(sig_input.as_bytes());
            let signature = hex::encode(mac.finalize().into_bytes());

            request = request
                .header("x-push-proxy-signature", signature)
                .header("x-push-proxy-timestamp", timestamp.to_string())
                .header("x-push-proxy-nonce", nonce);
        }
    }
    let retry_config = outbound_retry_config();
    let response = send_reqwest_with_retry(
        request,
        &retry_config,
        |e| {
            if e.is_connect() || e.is_timeout() {
                PushNotificationError::NotConfigured
            } else {
                PushNotificationError::NetworkError(format!("Push proxy connection failed: {}", e))
            }
        },
        || PushNotificationError::NetworkError("Push proxy request clone failed".to_string()),
    )
    .await?;

    let status = response.status();

    if status.is_success() {
        info!("Successfully sent notification via push proxy");
        Ok(())
    } else {
        let body = response.text().await.unwrap_or_default();
        let status_code = status.as_u16();

        // Check for invalid token errors from FCM/APNS via proxy.
        // We treat these as permanent token failures so callers can remove stale devices.
        let body_lc = body.to_lowercase();
        if status_code == 410
            || (status_code == 400 && body.contains("INVALID_ARGUMENT"))
            || body_lc.contains("invalid token")
            || body_lc.contains("baddevicetoken")
            || body_lc.contains("devicetokennotfortopic")
            || body_lc.contains("token unregistered")
        {
            warn!(status = %status_code, body = %body, "FCM returned invalid token error");
            return Err(PushNotificationError::InvalidToken);
        }

        error!(
            status = %status,
            body = %body,
            "Push proxy returned error"
        );
        Err(PushNotificationError::ProxyError(format!(
            "HTTP {}: {}",
            status, body
        )))
    }
}

/// Send push notification directly via FCM (fallback)
async fn send_push_notification_direct(
    state: &AppState,
    notification: PushNotification,
) -> Result<(), PushNotificationError> {
    // Check if FCM is configured
    let fcm_config = match get_fcm_config(state).await {
        Some(config) => config,
        None => {
            warn!("Push notifications not configured (neither push proxy nor direct FCM)");
            return Err(PushNotificationError::NotConfigured);
        }
    };

    // Build FCM message
    let fcm_message = build_fcm_message(&notification);

    // Send via FCM HTTP v1 API
    let result = send_fcm_message(&fcm_config, &fcm_message).await;

    match &result {
        Ok(_) => {
            debug!("Push notification sent successfully via direct FCM");
        }
        Err(e) => {
            error!(error = %e, "Failed to send push notification via direct FCM");
        }
    }

    result
}

/// Delete a device registration when its token is invalid
async fn delete_invalid_device(state: &AppState, user_id: Uuid, device_token: &str) {
    match sqlx::query("DELETE FROM user_devices WHERE user_id = $1 AND token = $2")
        .bind(user_id)
        .bind(device_token)
        .execute(&state.db)
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                info!(
                    user_id = %user_id,
                    token_prefix = %&device_token[..20.min(device_token.len())],
                    "Deleted invalid device registration"
                );
            }
        }
        Err(e) => {
            error!(user_id = %user_id, error = %e, "Failed to delete invalid device");
        }
    }
}

/// Send push notification to multiple devices for a user
pub async fn send_push_to_user(
    state: &AppState,
    user_id: Uuid,
    title: String,
    body: String,
    data: serde_json::Value,
    priority: PushPriority,
) -> Result<usize, PushNotificationError> {
    info!(user_id = %user_id, title = %title, "send_push_to_user called");

    // Get user's devices
    let devices = get_user_devices(state, user_id).await?;

    if devices.is_empty() {
        info!(user_id = %user_id, "No devices found for user, skipping push notification");
        return Ok(0);
    }

    info!(user_id = %user_id, device_count = devices.len(), "Found devices for user");

    let mut sent_count = 0;

    for device in &devices {
        // Determine notification type from data payload
        let notification_type = data
            .get("sub_type")
            .and_then(|v| v.as_str())
            .map(|s| {
                if s == "calls" {
                    NotificationType::Call
                } else {
                    NotificationType::Message
                }
            })
            .unwrap_or(NotificationType::Message);

        let notification = PushNotification {
            device_token: device.token.clone(),
            platform: device.platform.clone(),
            title: title.clone(),
            body: body.clone(),
            data: data.clone(),
            priority,
            sound: Some("default".to_string()),
            badge: None,
            category: None,
            notification_type,
        };

        match send_push_notification(state, notification).await {
            Ok(_) => sent_count += 1,
            Err(PushNotificationError::NotConfigured) => {
                warn!(
                    user_id = %user_id,
                    device_platform = %device.platform,
                    token_prefix = %&device.token[..20.min(device.token.len())],
                    sent_count = sent_count,
                    "Push notifications not configured; stopping send loop and returning partial result"
                );
                return Ok(sent_count);
            }
            Err(PushNotificationError::InvalidToken) => {
                // Token is invalid - delete it from database
                warn!(
                    user_id = %user_id,
                    token_prefix = %&device.token[..20.min(device.token.len())],
                    "Device token is invalid, deleting registration"
                );
                delete_invalid_device(state, user_id, &device.token).await;
            }
            Err(e) => {
                error!(user_id = %user_id, error = %e, "Failed to send push to device");
            }
        }
    }

    info!(
        user_id = %user_id,
        device_count = devices.len(),
        sent_count = sent_count,
        "Sent push notifications to user devices"
    );

    Ok(sent_count)
}

/// Get the site URL from server config
async fn get_site_url(state: &AppState) -> String {
    let result: Option<(serde_json::Value,)> =
        sqlx::query_as("SELECT site FROM server_config WHERE id = 'default'")
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten();

    result
        .and_then(|(site,)| {
            site.get("site_url")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
        })
        .unwrap_or_default()
}

/// Send call ringing notification to a user
pub async fn send_call_ringing_notification(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
    call_id: Uuid,
    caller_name: String,
) -> Result<usize, PushNotificationError> {
    let title = format!("Incoming call from {}", caller_name);
    let body = "Tap to answer".to_string();

    // Get the call's thread_id (which is the post_id of the call post)
    let thread_id = state.call_state_manager.get_thread_id(call_id).await;
    let post_id = thread_id
        .map(crate::mattermost_compat::id::encode_mm_id)
        .unwrap_or_else(|| call_id.to_string());

    // Get server URL for the mobile app to identify which server
    let server_url = get_site_url(state).await;

    // Mattermost mobile expects:
    // - type: "message" (not "call_ringing")
    // - sub_type: "calls" (this triggers the ringing UI)
    // - channel_id, post_id (thread_id/post_id of the call post for navigation)
    // - sender_name (for display)
    // - server_url (so the app knows which server to navigate to)
    //
    // IMPORTANT: channel_id and post_id must be Mattermost-encoded to match the mobile app's database
    let data = serde_json::json!({
        "type": "message",
        "sub_type": "calls",
        "version": "2",
        "channel_id": crate::mattermost_compat::id::encode_mm_id(channel_id),
        "post_id": post_id,
        "call_id": call_id.to_string(),
        "sender_name": caller_name,
        "channel_name": "Call",
        "server_url": server_url,
    });

    send_push_to_user(
        state,
        user_id,
        title,
        body,
        data,
        PushPriority::High, // High priority for calls
    )
    .await
}

/// Send message notification to a user
pub async fn send_message_notification(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
    channel_name: String,
    sender_name: String,
    message: String,
    is_dm: bool,
) -> Result<usize, PushNotificationError> {
    let (title, body) = if is_dm {
        (sender_name.clone(), message)
    } else {
        (format!("{} in {}", sender_name, channel_name), message)
    };

    // Generate a post_id for navigation (mobile requires this field)
    let post_id = uuid::Uuid::new_v4().to_string();

    // Get server URL for the mobile app to identify which server
    let server_url = get_site_url(state).await;

    // IMPORTANT: channel_id must be Mattermost-encoded to match the mobile app's database
    let data = serde_json::json!({
        "type": "message",
        "version": "2",
        "channel_id": crate::mattermost_compat::id::encode_mm_id(channel_id),
        "post_id": post_id,
        "channel_name": channel_name,
        "sender_name": sender_name,
        "server_url": server_url,
    });

    send_push_to_user(state, user_id, title, body, data, PushPriority::Normal).await
}

/// Device info from database
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub token: String,
    pub platform: String,
}

/// Get all devices for a user
async fn get_user_devices(
    state: &AppState,
    user_id: Uuid,
) -> Result<Vec<DeviceInfo>, PushNotificationError> {
    let devices: Vec<(String, String)> = sqlx::query_as(
        "SELECT token, platform FROM user_devices WHERE user_id = $1 AND token IS NOT NULL",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| PushNotificationError::DatabaseError(e.to_string()))?;

    Ok(devices
        .into_iter()
        .map(|(token, platform)| DeviceInfo { token, platform })
        .collect())
}

// ============ Direct FCM Implementation (Fallback) ============

/// FCM message structure (HTTP v1 API)
#[derive(Debug, Serialize)]
struct FcmMessage {
    message: FcmMessageBody,
}

#[derive(Debug, Serialize)]
struct FcmMessageBody {
    token: String,
    notification: Option<FcmNotification>,
    data: Option<HashMap<String, String>>,
    android: Option<FcmAndroidConfig>,
    apns: Option<FcmApnsConfig>,
}

#[derive(Debug, Serialize)]
struct FcmNotification {
    title: String,
    body: String,
}

#[derive(Debug, Serialize)]
struct FcmAndroidConfig {
    priority: String,
    notification: FcmAndroidNotification,
}

#[derive(Debug, Serialize)]
struct FcmAndroidNotification {
    channel_id: String,
    sound: String,
    priority: String,
}

#[derive(Debug, Serialize)]
struct FcmApnsConfig {
    headers: HashMap<String, String>,
    payload: FcmApnsPayload,
}

#[derive(Debug, Serialize)]
struct FcmApnsPayload {
    aps: FcmAps,
}

#[derive(Debug, Serialize)]
struct FcmAps {
    alert: FcmAlert,
    badge: i32,
    sound: String,
    #[serde(rename = "content-available")]
    content_available: i32,
    #[serde(rename = "mutable-content")]
    mutable_content: i32,
}

#[derive(Debug, Serialize)]
struct FcmAlert {
    title: String,
    body: String,
}

/// FCM configuration
#[derive(Debug, Clone)]
struct FcmConfig {
    project_id: String,
    access_token: String,
}

/// Get FCM configuration from database or environment (fallback)
async fn get_fcm_config(state: &AppState) -> Option<FcmConfig> {
    // Try to get from database first
    let config: Option<(String, String)> = sqlx::query_as(
        "SELECT fcm_project_id, fcm_access_token FROM server_config WHERE id = 'default'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    if let Some((project_id, access_token)) = config {
        if !project_id.is_empty() && !access_token.is_empty() {
            return Some(FcmConfig {
                project_id,
                access_token,
            });
        }
    }

    // Fall back to environment variables
    let project_id = std::env::var("FCM_PROJECT_ID").ok()?;
    let access_token = std::env::var("FCM_ACCESS_TOKEN").ok()?;

    if project_id.is_empty() || access_token.is_empty() {
        return None;
    }

    Some(FcmConfig {
        project_id,
        access_token,
    })
}

/// Build FCM message from notification
fn build_fcm_message(notification: &PushNotification) -> FcmMessage {
    let is_high_priority = matches!(notification.priority, PushPriority::High);
    let is_call_notification = matches!(notification.notification_type, NotificationType::Call);

    let mut data_map = HashMap::new();
    if let serde_json::Value::Object(map) = &notification.data {
        for (key, value) in map {
            if let Some(str_val) = value.as_str() {
                data_map.insert(key.clone(), str_val.to_string());
            } else {
                data_map.insert(key.clone(), value.to_string());
            }
        }
    }

    // For call notifications, add a marker for the mobile app
    if is_call_notification {
        data_map.insert("is_call".to_string(), "true".to_string());
    }

    let android_config = if is_high_priority {
        Some(FcmAndroidConfig {
            priority: "high".to_string(),
            notification: FcmAndroidNotification {
                // Use the mobile app's existing "channel_01" (High Importance, IMPORTANCE_HIGH)
                // NOTE: Previously used "calls" which does NOT exist in the mobile app,
                // causing Android to fall back to a silent notification on locked screens.
                channel_id: "channel_01".to_string(),
                sound: if is_call_notification {
                    "default".to_string()
                } else {
                    notification
                        .sound
                        .clone()
                        .unwrap_or_else(|| "default".to_string())
                },
                priority: "high".to_string(),
            },
        })
    } else {
        Some(FcmAndroidConfig {
            priority: "normal".to_string(),
            notification: FcmAndroidNotification {
                channel_id: "channel_01".to_string(),
                sound: notification
                    .sound
                    .clone()
                    .unwrap_or_else(|| "default".to_string()),
                priority: "default".to_string(),
            },
        })
    };

    let mut apns_headers = HashMap::new();
    apns_headers.insert(
        "apns-priority".to_string(),
        if is_high_priority {
            "10".to_string()
        } else {
            "5".to_string()
        },
    );

    // For call notifications, use a specific sound and category
    let sound = if is_call_notification {
        "calls_ringtone.caf".to_string() // Custom ringtone for calls
    } else {
        notification
            .sound
            .clone()
            .unwrap_or_else(|| "default".to_string())
    };

    let apns_config = FcmApnsConfig {
        headers: apns_headers,
        payload: FcmApnsPayload {
            aps: FcmAps {
                alert: FcmAlert {
                    title: notification.title.clone(),
                    body: notification.body.clone(),
                },
                badge: notification.badge.unwrap_or(1),
                sound,
                content_available: 1,
                mutable_content: 1,
            },
        },
    };

    FcmMessage {
        message: FcmMessageBody {
            token: notification.device_token.clone(),
            notification: Some(FcmNotification {
                title: notification.title.clone(),
                body: notification.body.clone(),
            }),
            data: if data_map.is_empty() {
                None
            } else {
                Some(data_map)
            },
            android: android_config,
            apns: Some(apns_config),
        },
    }
}

/// Send message via FCM HTTP v1 API
async fn send_fcm_message(
    config: &FcmConfig,
    message: &FcmMessage,
) -> Result<(), PushNotificationError> {
    let url = format!(
        "https://fcm.googleapis.com/v1/projects/{}/messages:send",
        config.project_id
    );

    let client = reqwest::Client::new();
    let retry_config = outbound_retry_config();
    let response = send_reqwest_with_retry(
        client
            .post(&url)
            .header("Authorization", format!("Bearer {}", config.access_token))
            .header("Content-Type", "application/json")
            .json(message),
        &retry_config,
        |e| PushNotificationError::NetworkError(e.to_string()),
        || PushNotificationError::NetworkError("FCM request clone failed".to_string()),
    )
    .await?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string());

    if status.is_success() {
        debug!(response = %response_text, "FCM message sent successfully");
        Ok(())
    } else {
        let status_code = status.as_u16();

        // Check for invalid token errors
        // 404 = not found, 400 with INVALID_ARGUMENT = bad token
        if status_code == 404 || (status_code == 400 && response_text.contains("INVALID_ARGUMENT"))
        {
            warn!(status = %status_code, "FCM returned invalid token error");
            return Err(PushNotificationError::InvalidToken);
        }

        error!(
            status = %status,
            response = %response_text,
            "FCM API error"
        );
        Err(PushNotificationError::FcmError(format!(
            "HTTP {}: {}",
            status, response_text
        )))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PushNotificationError {
    #[error("Push notifications not configured")]
    NotConfigured,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("FCM API error: {0}")]
    FcmError(String),

    #[error("Push proxy error: {0}")]
    ProxyError(String),

    #[error("Invalid device token")]
    InvalidToken,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_fcm_message() {
        let notification = PushNotification {
            device_token: "test_token".to_string(),
            platform: "android".to_string(),
            title: "Test Title".to_string(),
            body: "Test Body".to_string(),
            data: serde_json::json!({"key": "value"}),
            priority: PushPriority::High,
            sound: Some("default".to_string()),
            badge: Some(1),
            category: None,
            notification_type: NotificationType::Message,
        };

        let fcm_message = build_fcm_message(&notification);
        assert_eq!(fcm_message.message.token, "test_token");
        assert!(fcm_message.message.android.is_some());
    }
}
