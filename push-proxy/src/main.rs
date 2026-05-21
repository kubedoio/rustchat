//! RustChat Push Proxy Server
//!
//! This service relays push notifications to Firebase Cloud Messaging (FCM) for Android
//! and Apple Push Notification Service (APNS) for iOS VoIP pushes.
//!
//! ## Environment Variables
//!
//! ### Firebase (Android)
//! - `FIREBASE_PROJECT_ID` - Firebase project ID
//! - `GOOGLE_APPLICATION_CREDENTIALS` - Path to service account JSON key
//!
//! ### APNS (iOS VoIP)
//! - `APNS_KEY_PATH` - Path to APNS auth key (.p8)
//! - `APNS_KEY_ID` - Key ID from Apple Developer
//! - `APNS_TEAM_ID` - Team ID from Apple Developer
//! - `APNS_BUNDLE_ID` - iOS bundle identifier (e.g., com.rustchat.app)
//! - `APNS_USE_PRODUCTION` - Use production APNS server (default: false for development)
//!
//! ### General
//! - `RUSTCHAT_PUSH_PORT` - Server port (default: 3000)
//! - `RUST_LOG` - Logging level

mod apns;
mod fcm;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use bytes::Bytes;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use apns::{ApnsClient, ApnsConfig, ApnsServer};
use fcm::FcmClient;

/// Push notification request from RustChat backend
#[derive(Debug, Deserialize)]
struct PushRequest {
    /// Device token (FCM token for Android, APNS token for iOS)
    token: String,
    /// Notification title
    title: String,
    /// Notification body
    body: String,
    /// Platform: "android" or "ios"
    #[serde(default = "default_platform")]
    platform: String,
    /// Notification type: "message" or "call"
    #[serde(rename = "type", default = "default_notification_type")]
    notification_type: String,
    /// Data payload
    data: PushData,
}

fn default_platform() -> String {
    "android".to_string()
}

fn default_notification_type() -> String {
    "message".to_string()
}

#[derive(Debug, Deserialize)]
struct PushData {
    channel_id: String,
    post_id: String,
    #[serde(rename = "type")]
    data_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sub_type: Option<String>, // "calls" for call notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sender_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sender_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_crt_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    server_url: Option<String>,
    /// Call UUID for VoIP pushes
    #[serde(skip_serializing_if = "Option::is_none")]
    call_uuid: Option<String>,
}

/// Push response
#[derive(Debug, Serialize)]
struct PushResponse {
    success: bool,
    message: String,
}

struct AppState {
    fcm_client: Option<FcmClient>,
    apns_client: Option<ApnsClient>,
    auth_key: Option<String>,
    seen_nonces: Mutex<HashMap<String, Instant>>,
}

fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/send", post(send_notification))
        .route("/health", get(health_check))
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "push_proxy=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting RustChat Push Proxy");

    // Initialize FCM client (Android)
    let fcm_client = init_fcm_client().await?;

    // Initialize APNS client (iOS VoIP)
    let apns_client = init_apns_client().await?;

    let auth_key = std::env::var("PUSH_PROXY_AUTH_KEY")
        .ok()
        .filter(|s| !s.is_empty());

    let state = Arc::new(AppState {
        fcm_client,
        apns_client,
        auth_key,
        seen_nonces: Mutex::new(HashMap::new()),
    });

    let app = app(state);

    // Run server
    let port = std::env::var("RUSTCHAT_PUSH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

/// Initialize FCM client if Firebase credentials are available
async fn init_fcm_client() -> anyhow::Result<Option<FcmClient>> {
    let project_id = match std::env::var("FIREBASE_PROJECT_ID") {
        Ok(id) => id,
        Err(_) => {
            info!("FIREBASE_PROJECT_ID not set, FCM support disabled");
            return Ok(None);
        }
    };

    let key_path = match std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        Ok(path) => std::path::PathBuf::from(path),
        Err(_) => {
            info!("GOOGLE_APPLICATION_CREDENTIALS not set, FCM support disabled");
            return Ok(None);
        }
    };

    info!("Initializing FCM client for project: {}", project_id);
    let client = FcmClient::new(project_id, key_path).await?;
    info!("FCM client initialized successfully");
    Ok(Some(client))
}

/// Initialize APNS client if VoIP credentials are available
async fn init_apns_client() -> anyhow::Result<Option<ApnsClient>> {
    let key_path = match std::env::var("APNS_KEY_PATH") {
        Ok(path) => std::path::PathBuf::from(path),
        Err(_) => {
            info!("APNS_KEY_PATH not set, APNS support disabled");
            return Ok(None);
        }
    };

    let key_id = match std::env::var("APNS_KEY_ID") {
        Ok(id) => id,
        Err(_) => {
            info!("APNS_KEY_ID not set, APNS support disabled");
            return Ok(None);
        }
    };

    let team_id = match std::env::var("APNS_TEAM_ID") {
        Ok(id) => id,
        Err(_) => {
            info!("APNS_TEAM_ID not set, APNS support disabled");
            return Ok(None);
        }
    };

    let bundle_id = match std::env::var("APNS_BUNDLE_ID") {
        Ok(id) => id,
        Err(_) => {
            info!("APNS_BUNDLE_ID not set, APNS support disabled");
            return Ok(None);
        }
    };

    let use_production = std::env::var("APNS_USE_PRODUCTION")
        .ok()
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let server = if use_production {
        ApnsServer::Production
    } else {
        ApnsServer::Development
    };

    info!(
        bundle_id = %bundle_id,
        key_id = %key_id,
        server = ?server,
        "Initializing APNS client for VoIP pushes"
    );

    let config = ApnsConfig {
        key_path,
        key_id,
        team_id,
        bundle_id,
        server,
    };

    let client = ApnsClient::new(config).await?;
    info!("APNS client initialized successfully");
    Ok(Some(client))
}

use axum::routing::get;

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "rustchat-push-proxy"
    }))
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

fn validate_hmac(
    auth_key: &str,
    headers: &HeaderMap,
    body_bytes: &Bytes,
    seen_nonces: &mut HashMap<String, Instant>,
    now_secs: i64,
    now_instant: Instant,
) -> Result<(), (StatusCode, Json<PushResponse>)> {
    let signature = headers
        .get("x-push-proxy-signature")
        .and_then(|v| v.to_str().ok());
    let timestamp = headers
        .get("x-push-proxy-timestamp")
        .and_then(|v| v.to_str().ok());
    let nonce = headers
        .get("x-push-proxy-nonce")
        .and_then(|v| v.to_str().ok());

    if signature.is_none() || timestamp.is_none() || nonce.is_none() {
        warn!("Rejecting push request: missing signature headers");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(PushResponse {
                success: false,
                message: "Unauthorized".to_string(),
            }),
        ));
    }

    let signature = signature.unwrap();
    let timestamp = timestamp.unwrap();
    let nonce = nonce.unwrap();

    // 1. Timestamp within 5 minutes
    let ts: i64 = match timestamp.parse() {
        Ok(t) => t,
        Err(_) => {
            warn!("Rejecting push request: invalid timestamp");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(PushResponse {
                    success: false,
                    message: "Unauthorized".to_string(),
                }),
            ));
        }
    };
    if (now_secs - ts).abs() > 300 {
        warn!("Rejecting push request: timestamp too old");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(PushResponse {
                success: false,
                message: "Unauthorized".to_string(),
            }),
        ));
    }

    // 2. Nonce deduplication (5-minute TTL)
    seen_nonces.retain(|_, &mut inst| now_instant.duration_since(inst) < Duration::from_secs(300));
    if seen_nonces.contains_key(nonce) {
        warn!("Rejecting push request: nonce already seen");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(PushResponse {
                success: false,
                message: "Unauthorized".to_string(),
            }),
        ));
    }
    seen_nonces.insert(nonce.to_string(), now_instant);

    // 3. HMAC signature verification
    let body_hash = {
        let mut hasher = sha2::Sha256::new();
        hasher.update(body_bytes);
        hex::encode(hasher.finalize())
    };
    let expected_sig_input = format!("{}:{}:{}", timestamp, nonce, body_hash);
    let mut mac =
        Hmac::<Sha256>::new_from_slice(auth_key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(expected_sig_input.as_bytes());
    let expected_signature = hex::encode(mac.finalize().into_bytes());

    if !constant_time_eq(&expected_signature, signature) {
        warn!("Rejecting push request: invalid signature");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(PushResponse {
                success: false,
                message: "Unauthorized".to_string(),
            }),
        ));
    }

    Ok(())
}

async fn send_notification(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body_bytes: Bytes,
) -> Result<StatusCode, (StatusCode, Json<PushResponse>)> {
    // Validate HMAC-SHA256 request signing if configured
    if let Some(secret) = state.auth_key.as_ref() {
        let mut nonces = state.seen_nonces.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let now_inst = Instant::now();
        validate_hmac(secret, &headers, &body_bytes, &mut nonces, now, now_inst)?;
    }

    let payload: PushRequest = match serde_json::from_slice(&body_bytes) {
        Ok(p) => p,
        Err(e) => {
            warn!("Invalid JSON body: {}", e);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(PushResponse {
                    success: false,
                    message: "Invalid request body".to_string(),
                }),
            ));
        }
    };

    let platform = payload.platform.to_lowercase();
    let is_call =
        payload.data.sub_type.as_deref() == Some("calls") || payload.notification_type == "call";

    info!(
        platform = %platform,
        is_call = is_call,
        token_prefix = %&payload.token[..20.min(payload.token.len())],
        title = %payload.title,
        "Received push notification request"
    );

    match platform.as_str() {
        "ios" => {
            info!("Routing to iOS handler");
            if is_call {
                send_voip_push(&state, &payload).await
            } else {
                send_ios_message_push(&state, &payload).await
            }
        }
        "android" => {
            info!("Routing to Android/FCM handler");
            send_fcm_push(&state, &payload).await
        }
        _ => {
            warn!(platform = %platform, "Unknown platform, defaulting to FCM");
            send_fcm_push(&state, &payload).await
        }
    }
}

/// Send standard iOS push via APNS (non-VoIP).
async fn send_ios_message_push(
    state: &AppState,
    payload: &PushRequest,
) -> Result<StatusCode, (StatusCode, Json<PushResponse>)> {
    let apns_client = match &state.apns_client {
        Some(client) => client,
        None => {
            warn!("APNS client not configured for iOS message pushes, falling back to FCM");
            return send_fcm_push(state, payload).await;
        }
    };

    let message_payload = apns::ApnsMessagePayload {
        topic: apns::build_alert_topic(&apns_client.config.bundle_id),
        device_token: payload.token.clone(),
        title: payload.title.clone(),
        body: payload.body.clone(),
        channel_id: payload.data.channel_id.clone(),
        post_id: payload.data.post_id.clone(),
        server_url: payload.data.server_url.clone().unwrap_or_default(),
        notification_type: payload.data.data_type.clone(),
        sub_type: payload.data.sub_type.clone(),
        sender_name: payload.data.sender_name.clone(),
        is_crt_enabled: payload.data.is_crt_enabled,
    };

    match apns_client.send_message_push(message_payload).await {
        Ok(_) => {
            info!("Standard iOS push sent successfully via APNS");
            Ok(StatusCode::OK)
        }
        Err(apns::ApnsError::InvalidToken) => {
            warn!("APNS token is invalid (device unregistered)");
            Err((
                StatusCode::GONE,
                Json(PushResponse {
                    success: false,
                    message: "Token unregistered".to_string(),
                }),
            ))
        }
        Err(e) => {
            error!("Failed to send standard iOS push: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PushResponse {
                    success: false,
                    message: format!("APNS error: {}", e),
                }),
            ))
        }
    }
}

/// Send VoIP push via APNS
async fn send_voip_push(
    state: &AppState,
    payload: &PushRequest,
) -> Result<StatusCode, (StatusCode, Json<PushResponse>)> {
    let apns_client = match &state.apns_client {
        Some(client) => client,
        None => {
            warn!("APNS client not configured, cannot send VoIP push");
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(PushResponse {
                    success: false,
                    message: "APNS not configured".to_string(),
                }),
            ));
        }
    };

    // Generate a call UUID if not provided
    let call_uuid = payload
        .data
        .call_uuid
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let voip_payload = apns::ApnsVoipPayload {
        topic: apns::build_voip_topic(&apns_client.config.bundle_id),
        device_token: payload.token.clone(),
        call_uuid,
        caller_name: payload
            .data
            .sender_name
            .clone()
            .unwrap_or_else(|| payload.title.clone()),
        channel_id: payload.data.channel_id.clone(),
        server_url: payload.data.server_url.clone().unwrap_or_default(),
        handle_type: "generic".to_string(),
        has_video: false,
    };

    match apns_client.send_voip_push(voip_payload).await {
        Ok(_) => {
            info!("VoIP push sent successfully via APNS");
            Ok(StatusCode::OK)
        }
        Err(apns::ApnsError::InvalidToken) => {
            warn!("APNS token is invalid (device unregistered)");
            Err((
                StatusCode::GONE,
                Json(PushResponse {
                    success: false,
                    message: "Token unregistered".to_string(),
                }),
            ))
        }
        Err(e) => {
            error!("Failed to send VoIP push: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PushResponse {
                    success: false,
                    message: format!("APNS error: {}", e),
                }),
            ))
        }
    }
}

/// Send push via FCM (Android or iOS fallback)
async fn send_fcm_push(
    state: &AppState,
    payload: &PushRequest,
) -> Result<StatusCode, (StatusCode, Json<PushResponse>)> {
    info!("Starting FCM push send");

    let fcm_client = match &state.fcm_client {
        Some(client) => client,
        None => {
            warn!("FCM client not configured, cannot send push");
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(PushResponse {
                    success: false,
                    message: "FCM not configured".to_string(),
                }),
            ));
        }
    };

    info!("FCM client is available, building payload");

    // Convert to FCM payload format
    let fcm_payload = fcm::PushPayload {
        token: payload.token.clone(),
        title: payload.title.clone(),
        body: payload.body.clone(),
        data: fcm::PushData {
            channel_id: payload.data.channel_id.clone(),
            post_id: payload.data.post_id.clone(),
            r#type: payload.data.data_type.clone(),
            sub_type: payload.data.sub_type.clone(),
            version: payload.data.version.clone(),
            sender_id: payload.data.sender_id.clone(),
            sender_name: payload.data.sender_name.clone(),
            is_crt_enabled: payload.data.is_crt_enabled,
            server_url: payload.data.server_url.clone(),
            call_uuid: payload.data.call_uuid.clone(),
        },
    };

    info!("FCM payload built, sending to FCM client");

    match fcm_client.send(fcm_payload).await {
        Ok(_) => {
            info!("Push sent successfully via FCM");
            Ok(StatusCode::OK)
        }
        Err(fcm::FcmError::Api(ref s)) if s.contains("UNREGISTERED") => {
            warn!("FCM token is unregistered");
            Err((
                StatusCode::GONE,
                Json(PushResponse {
                    success: false,
                    message: "Token unregistered".to_string(),
                }),
            ))
        }
        Err(fcm::FcmError::Api(ref s)) if s.contains("SENDER_ID_MISMATCH") => {
            warn!(
                "FCM token Sender ID mismatch - token was registered with a different Firebase project"
            );
            Err((
                StatusCode::GONE,
                Json(PushResponse {
                    success: false,
                    message: "Token Sender ID mismatch - token needs to be refreshed".to_string(),
                }),
            ))
        }
        Err(fcm::FcmError::Api(ref s)) if s.contains("INVALID_ARGUMENT") => {
            warn!("FCM token is invalid");
            Err((
                StatusCode::BAD_REQUEST,
                Json(PushResponse {
                    success: false,
                    message: "Invalid token".to_string(),
                }),
            ))
        }
        Err(e) => {
            error!("Failed to send FCM push: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PushResponse {
                    success: false,
                    message: format!("FCM error: {}", e),
                }),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use bytes::Bytes;
    use http_body_util::BodyExt;
    use std::collections::HashMap;
    use std::time::Instant;
    use tower::ServiceExt;

    const TEST_AUTH_KEY: &str = "test-secret-key";

    fn test_state_without_auth() -> Arc<AppState> {
        Arc::new(AppState {
            fcm_client: None,
            apns_client: None,
            auth_key: None,
            seen_nonces: Mutex::new(HashMap::new()),
        })
    }

    fn generate_signature(auth_key: &str, timestamp: i64, nonce: &str, body: &Bytes) -> String {
        let body_hash = {
            let mut hasher = sha2::Sha256::new();
            hasher.update(body);
            hex::encode(hasher.finalize())
        };
        let expected_sig_input = format!("{}:{}:{}", timestamp, nonce, body_hash);
        let mut mac = Hmac::<Sha256>::new_from_slice(auth_key.as_bytes()).unwrap();
        mac.update(expected_sig_input.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    fn make_headers(timestamp: i64, nonce: &str, signature: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-push-proxy-signature", signature.parse().unwrap());
        headers.insert(
            "x-push-proxy-timestamp",
            timestamp.to_string().parse().unwrap(),
        );
        headers.insert("x-push-proxy-nonce", nonce.parse().unwrap());
        headers
    }

    #[tokio::test]
    async fn hmac_valid_request() {
        let body = Bytes::from(
            r#"{"token":"t","title":"T","body":"B","platform":"android","type":"message","data":{"channel_id":"c","post_id":"p","type":"message"}}"#,
        );
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let nonce = "nonce-1";
        let signature = generate_signature(TEST_AUTH_KEY, timestamp, nonce, &body);
        let headers = make_headers(timestamp, nonce, &signature);
        let mut nonces = HashMap::new();
        let now = Instant::now();

        let result = validate_hmac(TEST_AUTH_KEY, &headers, &body, &mut nonces, timestamp, now);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn hmac_expired_timestamp() {
        let body = Bytes::from(
            r#"{"token":"t","title":"T","body":"B","platform":"android","type":"message","data":{"channel_id":"c","post_id":"p","type":"message"}}"#,
        );
        let timestamp = 1000i64;
        let nonce = "nonce-expired";
        let signature = generate_signature(TEST_AUTH_KEY, timestamp, nonce, &body);
        let headers = make_headers(timestamp, nonce, &signature);
        let mut nonces = HashMap::new();
        let now = Instant::now();
        let now_secs = 10000i64;

        let result = validate_hmac(TEST_AUTH_KEY, &headers, &body, &mut nonces, now_secs, now);
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn hmac_invalid_signature() {
        let body = Bytes::from(
            r#"{"token":"t","title":"T","body":"B","platform":"android","type":"message","data":{"channel_id":"c","post_id":"p","type":"message"}}"#,
        );
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let nonce = "nonce-invalid";
        let headers = make_headers(timestamp, nonce, "invalid-signature");
        let mut nonces = HashMap::new();
        let now = Instant::now();

        let result = validate_hmac(TEST_AUTH_KEY, &headers, &body, &mut nonces, timestamp, now);
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn hmac_replayed_nonce() {
        let body = Bytes::from(
            r#"{"token":"t","title":"T","body":"B","platform":"android","type":"message","data":{"channel_id":"c","post_id":"p","type":"message"}}"#,
        );
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let nonce = "nonce-replay";
        let signature = generate_signature(TEST_AUTH_KEY, timestamp, nonce, &body);
        let headers = make_headers(timestamp, nonce, &signature);
        let mut nonces = HashMap::new();
        let now = Instant::now();

        let result = validate_hmac(TEST_AUTH_KEY, &headers, &body, &mut nonces, timestamp, now);
        assert!(result.is_ok());

        let result2 = validate_hmac(TEST_AUTH_KEY, &headers, &body, &mut nonces, timestamp, now);
        assert!(result2.is_err());
        let (status, _) = result2.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn route_android() {
        let state = test_state_without_auth();
        let app = app(state);

        let body = r#"{"token":"android-token","title":"Test","body":"Body","platform":"android","type":"message","data":{"channel_id":"ch1","post_id":"p1","type":"message"}}"#;

        let request = Request::builder()
            .method("POST")
            .uri("/send")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["message"], "FCM not configured");
    }

    #[tokio::test]
    async fn route_ios() {
        let state = test_state_without_auth();
        let app = app(state);

        let body = r#"{"token":"ios-token","title":"Test","body":"Body","platform":"ios","type":"call","data":{"channel_id":"ch1","post_id":"p1","type":"call","call_uuid":"call-123"}}"#;

        let request = Request::builder()
            .method("POST")
            .uri("/send")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["message"], "APNS not configured");
    }

    #[tokio::test]
    async fn health_check() {
        let state = test_state_without_auth();
        let app = app(state);

        let request = Request::builder()
            .method("GET")
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "rustchat-push-proxy");
    }
}
