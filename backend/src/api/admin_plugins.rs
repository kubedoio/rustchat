//! Admin plugin configuration endpoints

use axum::{extract::State, routing::get, Json, Router};

use crate::api::admin::require_admin;
use crate::api::AppState;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/admin/plugins/calls",
        get(get_calls_plugin_config).put(update_calls_plugin_config),
    )
}

// ============ Plugins - RustChat Calls Plugin ============

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct CallsPluginConfig {
    pub enabled: bool,
    pub turn_server_enabled: bool,
    pub turn_server_url: String,
    pub turn_server_username: String,
    #[serde(skip_serializing)]
    pub turn_server_credential: String,
    pub udp_port: u16,
    pub tcp_port: u16,
    pub ice_host_override: Option<String>,
    pub stun_servers: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateCallsPluginConfig {
    pub enabled: bool,
    pub turn_server_enabled: bool,
    pub turn_server_url: String,
    pub turn_server_username: String,
    pub turn_server_credential: Option<String>,
    pub udp_port: u16,
    pub tcp_port: u16,
    pub ice_host_override: Option<String>,
    pub stun_servers: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct CallsPluginConfigResponse {
    pub plugin_id: String,
    pub plugin_name: String,
    pub settings: CallsPluginConfig,
}

pub async fn get_calls_plugin_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<CallsPluginConfigResponse>> {
    require_admin(&auth)?;

    // Get config from database (server_config table, plugins column)
    let config: Option<(serde_json::Value,)> =
        sqlx::query_as("SELECT plugins->'calls' FROM server_config WHERE id = 'default'")
            .fetch_optional(&state.db)
            .await?;

    let calls_config = config
        .and_then(|(json,)| json.as_object().cloned())
        .map(|obj| CallsPluginConfig {
            enabled: obj
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(state.config.calls.enabled),
            turn_server_enabled: obj
                .get("turn_server_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(state.config.calls.turn_server_enabled),
            turn_server_url: obj
                .get("turn_server_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| state.config.calls.turn_server_url.clone()),
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
            udp_port: obj
                .get("udp_port")
                .and_then(|v| v.as_u64())
                .map(|v| v as u16)
                .unwrap_or(state.config.calls.udp_port),
            tcp_port: obj
                .get("tcp_port")
                .and_then(|v| v.as_u64())
                .map(|v| v as u16)
                .unwrap_or(state.config.calls.tcp_port),
            ice_host_override: obj
                .get("ice_host_override")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            stun_servers: obj
                .get("stun_servers")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_else(|| state.config.calls.stun_servers.clone()),
        })
        .unwrap_or_else(|| CallsPluginConfig {
            enabled: state.config.calls.enabled,
            turn_server_enabled: state.config.calls.turn_server_enabled,
            turn_server_url: state.config.calls.turn_server_url.clone(),
            turn_server_username: state.config.calls.turn_server_username.clone(),
            turn_server_credential: state.config.calls.turn_server_credential.clone(),
            udp_port: state.config.calls.udp_port,
            tcp_port: state.config.calls.tcp_port,
            ice_host_override: state.config.calls.ice_host_override.clone(),
            stun_servers: state.config.calls.stun_servers.clone(),
        });

    Ok(Json(CallsPluginConfigResponse {
        plugin_id: "com.rustchat.calls".to_string(),
        plugin_name: "RustChat Calls Plugin".to_string(),
        settings: calls_config,
    }))
}

pub async fn update_calls_plugin_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<serde_json::Value>,
) -> ApiResult<Json<CallsPluginConfigResponse>> {
    require_admin(&auth)?;

    // Log the incoming payload for debugging
    tracing::info!("Received Calls Plugin config update: {}", payload);

    // Deserialize manually to get better error messages
    let payload: UpdateCallsPluginConfig = serde_json::from_value(payload).map_err(|e| {
        tracing::error!("Failed to deserialize Calls Plugin config: {}", e);
        AppError::BadRequest(format!("Invalid configuration data: {}", e))
    })?;

    // Get existing credential if not provided in update
    let credential = if let Some(ref cred) = payload.turn_server_credential {
        cred.clone()
    } else {
        // Fetch existing credential from database
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT plugins->'calls'->>'turn_server_credential' FROM server_config WHERE id = 'default'"
        )
        .fetch_optional(&state.db)
        .await?;
        existing
            .map(|(s,)| s)
            .unwrap_or_else(|| state.config.calls.turn_server_credential.clone())
    };

    // Build JSON object for calls config
    let calls_config_json = serde_json::json!({
        "enabled": payload.enabled,
        "turn_server_enabled": payload.turn_server_enabled,
        "turn_server_url": payload.turn_server_url,
        "turn_server_username": payload.turn_server_username,
        "turn_server_credential": credential,
        "udp_port": payload.udp_port,
        "tcp_port": payload.tcp_port,
        "ice_host_override": payload.ice_host_override,
        "stun_servers": payload.stun_servers,
    });

    // Update server_config table
    sqlx::query(
        r#"
        INSERT INTO server_config (id, plugins, updated_at, updated_by)
        VALUES ('default', jsonb_build_object('calls', $1::jsonb), NOW(), $2)
        ON CONFLICT (id) DO UPDATE SET
            plugins = jsonb_set(
                COALESCE(server_config.plugins, '{}'::jsonb),
                '{calls}',
                $1::jsonb,
                true
            ),
            updated_at = NOW(),
            updated_by = $2
        "#,
    )
    .bind(calls_config_json)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(CallsPluginConfigResponse {
        plugin_id: "com.rustchat.calls".to_string(),
        plugin_name: "RustChat Calls Plugin".to_string(),
        settings: CallsPluginConfig {
            enabled: payload.enabled,
            turn_server_enabled: payload.turn_server_enabled,
            turn_server_url: payload.turn_server_url,
            turn_server_username: payload.turn_server_username,
            turn_server_credential: credential,
            udp_port: payload.udp_port,
            tcp_port: payload.tcp_port,
            ice_host_override: payload.ice_host_override,
            stun_servers: payload.stun_servers,
        },
    }))
}
