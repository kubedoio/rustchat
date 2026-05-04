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
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
    });

    // Build routes
    let app = Router::new()
        .route("/send", post(send_notification))
        .route("/health", get(health_check))
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http());

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

async fn send_notification(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<PushRequest>,
) -> Result<StatusCode, (StatusCode, Json<PushResponse>)> {
    // Validate shared-secret auth key if configured
    if let Some(expected) = state.auth_key.as_ref() {
        let provided = headers
            .get("x-push-proxy-key")
            .and_then(|v| v.to_str().ok());
        if provided != Some(expected) {
            warn!("Rejecting unauthenticated push request");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(PushResponse {
                    success: false,
                    message: "Unauthorized".to_string(),
                }),
            ));
        }
    }
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
