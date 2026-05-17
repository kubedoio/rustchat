use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};

use super::turn::{TurnCredentialGenerator, TurnServerConfig};

pub(crate) struct VersionResponse {
    version: String,
    rtcd: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ConfigResponse {
    #[serde(rename = "ICEServersConfigs")]
    ice_servers_configs: Vec<IceServer>,
    #[serde(rename = "NeedsTURNCredentials")]
    needs_turn_credentials: bool,
    #[serde(rename = "DefaultEnabled")]
    default_enabled: bool,
    #[serde(rename = "AllowEnableCalls")]
    allow_enable_calls: bool,
    #[serde(rename = "GroupCallsAllowed")]
    group_calls_allowed: bool,
    #[serde(rename = "EnableRinging")]
    enable_ringing: bool,
    #[serde(rename = "HostControlsAllowed")]
    host_controls_allowed: bool,
    #[serde(rename = "EnableRecordings")]
    enable_recordings: bool,
    #[serde(rename = "MaxCallParticipants")]
    max_call_participants: i32,
    #[serde(rename = "AllowScreenSharing")]
    allow_screen_sharing: bool,
    #[serde(rename = "EnableSimulcast")]
    enable_simulcast: bool,
    #[serde(rename = "EnableAV1")]
    enable_av1: bool,
    #[serde(rename = "MaxRecordingDuration")]
    max_recording_duration: i32,
    #[serde(rename = "TranscribeAPI")]
    transcribe_api: String,
    #[serde(rename = "sku_short_name")]
    sku_short_name: String,
    #[serde(rename = "EnableDCSignaling")]
    enable_dc_signaling: bool,
    #[serde(rename = "EnableTranscriptions")]
    enable_transcriptions: bool,
    #[serde(rename = "EnableLiveCaptions")]
    enable_live_captions: bool,
}
#[derive(Debug, Serialize)]
pub(crate) struct IceServer {
    urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    credential: Option<String>,
    #[serde(rename = "credentialType", skip_serializing_if = "Option::is_none")]
    credential_type: Option<String>,
}
struct EffectiveCallsConfig {
    turn_server_enabled: bool,
    turn_server_url: String,
    turn_server_username: String,
    turn_server_credential: String,
    turn_static_auth_secret: String,
    stun_servers: Vec<String>,
}
fn ensure_protocol(url: &str, protocol: &str) -> String {
    let url = url.trim();
    if url.is_empty() {
        return url.to_string();
    }
    let lower = url.to_lowercase();
    // For TURN, we also accept turns:
    if lower.starts_with(protocol) || (protocol == "turn:" && lower.starts_with("turns:")) {
        url.to_string()
    } else {
        format!("{}{}", protocol, url)
    }
}
async fn load_effective_calls_config(state: &AppState) -> EffectiveCallsConfig {
    // Try to read the database-saved config (same query the admin GET uses)
    let db_config: Option<(serde_json::Value,)> =
        sqlx::query_as("SELECT plugins->'calls' FROM server_config WHERE id = 'default'")
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

    if let Some((json,)) = db_config {
        if let Some(obj) = json.as_object() {
            return EffectiveCallsConfig {
                turn_server_enabled: obj
                    .get("turn_server_enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(state.config.calls.turn_server_enabled),
                turn_server_url: obj
                    .get("turn_server_url")
                    .and_then(|v| v.as_str())
                    .map(|s| ensure_protocol(s, "turn:"))
                    .unwrap_or_else(|| {
                        ensure_protocol(&state.config.calls.turn_server_url, "turn:")
                    }),
                turn_server_username: obj
                    .get("turn_server_username")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| state.config.calls.turn_server_username.clone()),
                turn_server_credential: obj
                    .get("turn_server_credential")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| state.config.calls.turn_server_credential.clone()),
                turn_static_auth_secret: obj
                    .get("turn_static_auth_secret")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| state.config.calls.turn_static_auth_secret.clone()),
                stun_servers: obj
                    .get("stun_servers")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| ensure_protocol(s, "stun:"))
                            .collect()
                    })
                    .unwrap_or_else(|| {
                        state
                            .config
                            .calls
                            .stun_servers
                            .iter()
                            .map(|s| ensure_protocol(s, "stun:"))
                            .collect()
                    }),
            };
        }
    }

    // No database overrides — use env var defaults
    EffectiveCallsConfig {
        turn_server_enabled: state.config.calls.turn_server_enabled,
        turn_server_url: state.config.calls.turn_server_url.clone(),
        turn_server_username: state.config.calls.turn_server_username.clone(),
        turn_server_credential: state.config.calls.turn_server_credential.clone(),
        turn_static_auth_secret: state.config.calls.turn_static_auth_secret.clone(),
        stun_servers: state.config.calls.stun_servers.clone(),
    }
}
pub(crate) async fn get_version(State(_state): State<AppState>) -> ApiResult<Json<VersionResponse>> {
    Ok(Json(VersionResponse {
        version: "0.28.0".to_string(),
        rtcd: false, // We're using integrated mode
    }))
}
pub(crate) async fn get_config(
    State(state): State<AppState>,
    _auth: MmAuthUser,
) -> ApiResult<Json<ConfigResponse>> {
    let effective = load_effective_calls_config(&state).await;

    // Build ice servers list — STUN only.
    // TURN is intentionally omitted from this response because including a credential-less
    // TURN entry causes browsers to attempt (and fail) auth. The client already handles
    // `NeedsTURNCredentials: true` by fetching proper creds via /turn-credentials.
    let mut ice_servers = vec![];

    for stun_url in &effective.stun_servers {
        ice_servers.push(IceServer {
            urls: vec![stun_url.clone()],
            username: None,
            credential: None,
            credential_type: None,
        });
    }

    Ok(Json(ConfigResponse {
        ice_servers_configs: ice_servers,
        needs_turn_credentials: effective.turn_server_enabled,
        default_enabled: true,
        allow_enable_calls: true,
        group_calls_allowed: true,
        enable_ringing: true,
        host_controls_allowed: true,
        enable_recordings: false,
        max_call_participants: 0,
        allow_screen_sharing: true,
        enable_simulcast: false,
        enable_av1: false,
        max_recording_duration: 60,
        transcribe_api: "whisper.cpp".to_string(),
        sku_short_name: "starter".to_string(),
        enable_dc_signaling: false,
        enable_transcriptions: false,
        enable_live_captions: false,
    }))
}
pub(crate) async fn get_turn_credentials(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<IceServer>>> {
    let effective = load_effective_calls_config(&state).await;

    if !effective.turn_server_enabled {
        return Err(AppError::BadRequest("TURN server is disabled".to_string()));
    }

    let turn_config = TurnServerConfig {
        enabled: true,
        url: effective.turn_server_url.clone(),
        username: effective.turn_server_username.clone(),
        credential: effective.turn_server_credential.clone(),
    };

    // If static credentials are provided (via admin console), use them directly.
    // Otherwise, generate ephemeral HMAC-SHA1 credentials using the best available secret.
    let generator = if turn_config.username.is_empty() || turn_config.credential.is_empty() {
        // Prefer explicit TURN static auth secret; fallback to general encryption key
        let secret = if !effective.turn_static_auth_secret.is_empty() {
            effective.turn_static_auth_secret.clone()
        } else {
            state.config.encryption_key.clone()
        };

        TurnCredentialGenerator::with_rest_api(secret, state.config.calls.turn_ttl_minutes)
    } else {
        TurnCredentialGenerator::with_static_credentials(turn_config)
    };

    let credentials = generator.generate_credentials(&auth.user_id.to_string());

    Ok(Json(vec![IceServer {
        urls: vec![effective.turn_server_url],
        username: Some(credentials.username),
        credential: Some(credentials.credential),
        credential_type: Some("password".to_string()),
    }]))
}
