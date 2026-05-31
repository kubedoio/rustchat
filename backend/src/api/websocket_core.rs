//! Shared websocket internals used by both `/api/v1/ws` and `/api/v4/websocket`.
//!
//! This module centralizes behavior that must stay consistent across websocket
//! entry points: auth token normalization, connection limits, subscription
//! bootstrap, presence lifecycle, and shared command handling.

use axum::http::HeaderMap;
use chrono::{DateTime, TimeZone, Utc};
use deadpool_redis::redis::AsyncCommands;
use uuid::Uuid;

use crate::api::AppState;
use crate::auth::{validate_token_with_policy, Claims};
use crate::realtime::{
    ClientEnvelope, EventType, TypingCommandData, TypingEvent, WsBroadcast, WsEnvelope,
};
use crate::repositories::ChannelRepository;

#[derive(Debug, Clone, Copy)]
pub struct ConnectionLimitExceeded {
    pub current: usize,
    pub max: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct EnvelopeCommandOptions {
    pub allow_send_message: bool,
    pub emit_unknown_error: bool,
    pub acknowledge_unsubscribe: bool,
}

#[derive(Debug, Clone)]
pub struct WebSocketAuth {
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

impl EnvelopeCommandOptions {
    pub const V1: Self = Self {
        allow_send_message: true,
        emit_unknown_error: true,
        acknowledge_unsubscribe: true,
    };

    pub const V4: Self = Self {
        allow_send_message: false,
        emit_unknown_error: false,
        acknowledge_unsubscribe: false,
    };
}

pub async fn get_max_simultaneous_connections(state: &AppState) -> usize {
    let value: Option<String> = sqlx::query_scalar(
        "SELECT site->>'max_simultaneous_connections' FROM server_config WHERE id = 'default'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    match value.and_then(|val| val.parse::<i64>().ok()) {
        Some(max) if max > 0 => max as usize,
        _ => 20,
    }
}

async fn is_manual_presence(state: &AppState, user_id: Uuid) -> bool {
    sqlx::query_scalar::<_, Option<bool>>("SELECT presence_manual FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .flatten()
        .unwrap_or(false)
}

fn presence_connection_key(user_id: Uuid) -> String {
    format!("rustchat:presence:user:{user_id}:connections")
}

fn presence_heartbeat_key(user_id: Uuid, connection_id: &str) -> String {
    format!("rustchat:presence:user:{user_id}:connection:{connection_id}:heartbeat")
}

const PRESENCE_HEARTBEAT_TTL_SECONDS: u64 = 90;

pub async fn register_presence_connection(state: &AppState, user_id: Uuid, connection_id: &str) {
    let connection_id = connection_id.trim();
    if connection_id.is_empty() {
        return;
    }

    let mut conn = match state.redis.get().await {
        Ok(conn) => conn,
        Err(err) => {
            tracing::warn!(
                user_id = %user_id,
                connection_id = connection_id,
                error = %err,
                "Presence registry unavailable while registering websocket connection",
            );
            return;
        }
    };

    let key = presence_connection_key(user_id);
    let heartbeat_key = presence_heartbeat_key(user_id, connection_id);
    let now = Utc::now().timestamp();

    if let Err(err) = conn.sadd::<_, _, usize>(&key, connection_id).await {
        tracing::warn!(
            user_id = %user_id,
            connection_id = connection_id,
            error = %err,
            "Failed to register websocket connection in presence registry",
        );
        return;
    }

    if let Err(err) = conn
        .set_ex::<_, _, ()>(&heartbeat_key, now, PRESENCE_HEARTBEAT_TTL_SECONDS)
        .await
    {
        tracing::warn!(
            user_id = %user_id,
            connection_id = connection_id,
            error = %err,
            "Failed to register websocket heartbeat in presence registry",
        );
    }

    if let Err(err) = conn
        .expire::<_, ()>(&key, (PRESENCE_HEARTBEAT_TTL_SECONDS * 2) as i64)
        .await
    {
        tracing::warn!(
            user_id = %user_id,
            connection_id = connection_id,
            error = %err,
            "Failed to refresh presence registry TTL",
        );
    }
}

pub async fn unregister_presence_connection(state: &AppState, user_id: Uuid, connection_id: &str) {
    let connection_id = connection_id.trim();
    if connection_id.is_empty() {
        return;
    }

    let mut conn = match state.redis.get().await {
        Ok(conn) => conn,
        Err(err) => {
            tracing::warn!(
                user_id = %user_id,
                connection_id = connection_id,
                error = %err,
                "Presence registry unavailable while unregistering websocket connection",
            );
            return;
        }
    };

    let key = presence_connection_key(user_id);
    let heartbeat_key = presence_heartbeat_key(user_id, connection_id);
    if let Err(err) = conn.srem::<_, _, usize>(&key, connection_id).await {
        tracing::warn!(
            user_id = %user_id,
            connection_id = connection_id,
            error = %err,
            "Failed to unregister websocket connection from presence registry",
        );
    }

    if let Err(err) = conn.del::<_, usize>(&heartbeat_key).await {
        tracing::warn!(
            user_id = %user_id,
            connection_id = connection_id,
            error = %err,
            "Failed to remove websocket heartbeat from presence registry",
        );
    }

    match conn.scard::<_, usize>(&key).await {
        Ok(0) => {
            let _ = conn.del::<_, usize>(&key).await;
        }
        Ok(_) => {
            let _ = conn
                .expire::<_, ()>(&key, (PRESENCE_HEARTBEAT_TTL_SECONDS * 2) as i64)
                .await;
        }
        Err(err) => {
            tracing::warn!(
                user_id = %user_id,
                connection_id = connection_id,
                error = %err,
                "Failed to read connection count during unregister",
            );
        }
    }
}

pub async fn heartbeat_presence_connection(state: &AppState, user_id: Uuid, connection_id: &str) {
    let connection_id = connection_id.trim();
    if connection_id.is_empty() {
        return;
    }

    let mut conn = match state.redis.get().await {
        Ok(conn) => conn,
        Err(err) => {
            tracing::warn!(
                user_id = %user_id,
                connection_id = connection_id,
                error = %err,
                "Presence registry unavailable while heartbeating websocket connection",
            );
            return;
        }
    };

    let key = presence_connection_key(user_id);
    let heartbeat_key = presence_heartbeat_key(user_id, connection_id);
    let now = Utc::now().timestamp();

    if let Err(err) = conn.sadd::<_, _, usize>(&key, connection_id).await {
        tracing::warn!(
            user_id = %user_id,
            connection_id = connection_id,
            error = %err,
            "Failed to refresh websocket connection in presence registry",
        );
        return;
    }

    if let Err(err) = conn
        .set_ex::<_, _, ()>(&heartbeat_key, now, PRESENCE_HEARTBEAT_TTL_SECONDS)
        .await
    {
        tracing::warn!(
            user_id = %user_id,
            connection_id = connection_id,
            error = %err,
            "Failed to refresh websocket heartbeat in presence registry",
        );
        return;
    }

    if let Err(err) = conn
        .expire::<_, ()>(&key, (PRESENCE_HEARTBEAT_TTL_SECONDS * 2) as i64)
        .await
    {
        tracing::warn!(
            user_id = %user_id,
            connection_id = connection_id,
            error = %err,
            "Failed to refresh presence registry TTL during heartbeat",
        );
    }
}

async fn global_presence_connection_count(state: &AppState, user_id: Uuid) -> Option<usize> {
    match crate::realtime::get_global_connection_count(state, user_id).await {
        Ok(count) => Some(count),
        Err(err) => {
            tracing::warn!(
                user_id = %user_id,
                error = %err,
                "Presence registry unavailable while reading global connection count",
            );
            None
        }
    }
}

pub async fn enforce_connection_limit(
    state: &AppState,
    user_id: Uuid,
) -> Result<(), ConnectionLimitExceeded> {
    let max = get_max_simultaneous_connections(state).await;

    let Some(current) = global_presence_connection_count(state, user_id).await else {
        tracing::error!(
            user_id = %user_id,
            "Failed to get cluster connection count - cannot enforce limit"
        );
        // When Redis is unavailable, we cannot safely allow connections
        // as we don't know the true count. Fail closed.
        return Err(ConnectionLimitExceeded { current: max, max });
    };

    if current >= max {
        tracing::warn!(
            user_id = %user_id,
            current = current,
            max = max,
            "Connection limit exceeded (cluster-wide)"
        );
        Err(ConnectionLimitExceeded { current, max })
    } else {
        Ok(())
    }
}

pub async fn fetch_username(state: &AppState, user_id: Uuid) -> Result<String, sqlx::Error> {
    sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
}

pub fn requested_protocol(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Sec-WebSocket-Protocol")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
}

pub fn normalize_auth_token(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("undefined") {
        return None;
    }

    let stripped = trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("Token "))
        .unwrap_or(trimmed)
        .trim();

    if stripped.is_empty() || stripped.eq_ignore_ascii_case("undefined") {
        return None;
    }

    Some(stripped.to_string())
}

/// Resolve auth token using ONLY secure methods (no query params)
/// This is the recommended method for production use
pub fn resolve_auth_token_secure(
    headers: &HeaderMap,
    protocol_token: Option<&str>,
) -> Option<String> {
    // Only Authorization header - never query params
    if let Some(auth_header) = headers.get("Authorization").and_then(|v| v.to_str().ok()) {
        if let Some(token) = normalize_auth_token(auth_header) {
            return Some(token);
        }
    }

    // Sec-WebSocket-Protocol header as fallback
    if let Some(protocol) = protocol_token.and_then(normalize_auth_token) {
        // Only accept if it looks like a JWT (contains dot and reasonable length)
        if protocol.len() > 20 && protocol.contains('.') {
            return Some(protocol);
        }
    }

    None
}

/// Resolve auth token using secure transport only.
pub fn resolve_auth_token(headers: &HeaderMap, protocol_token: Option<&str>) -> Option<String> {
    resolve_auth_token_secure(headers, protocol_token)
}

fn claims_to_websocket_auth(claims: Claims) -> Option<WebSocketAuth> {
    let expires_at = Utc.timestamp_opt(claims.exp, 0).single()?;
    Some(WebSocketAuth {
        user_id: claims.sub,
        expires_at,
    })
}

pub fn validate_auth_token(token: &str, state: &AppState) -> Option<WebSocketAuth> {
    let token_data = validate_token_with_policy(
        token,
        &state.jwt_secret,
        state.jwt_issuer.as_deref(),
        state.jwt_audience.as_deref(),
    )
    .ok()?;

    claims_to_websocket_auth(token_data.claims)
}

pub fn validate_auth_context(token: Option<&str>, state: &AppState) -> Option<WebSocketAuth> {
    token.and_then(|t| validate_auth_token(t, state))
}

pub async fn initialize_connection_state(
    state: &AppState,
    user_id: Uuid,
    subscribe_channels: bool,
) {
    subscribe_default_scopes(state, user_id, subscribe_channels).await;
    if is_manual_presence(state, user_id).await {
        return;
    }
    persist_presence_and_broadcast(state, user_id, "online", false).await;
}

pub async fn handle_disconnect(state: &AppState, user_id: Uuid, connection_id: &str) {
    unregister_presence_connection(state, user_id, connection_id).await;

    // Local short-circuit: still connected on this node.
    if state.ws_hub.user_connection_count(user_id).await > 0 {
        return;
    }

    // Manual presence (busy/dnd/away/offline set by user) must not be
    // overwritten by disconnect-driven offline updates.
    if is_manual_presence(state, user_id).await {
        return;
    }

    let Some(global_count) = global_presence_connection_count(state, user_id).await else {
        // Conservative behavior: if we cannot determine global connection count,
        // do not force an offline transition.
        return;
    };

    if global_count == 0 {
        persist_presence_and_broadcast(state, user_id, "offline", false).await;
    }
}

pub async fn persist_presence_and_broadcast(
    state: &AppState,
    user_id: Uuid,
    status: &str,
    manual: bool,
) {
    let now = Utc::now();

    // Update user presence and last activity in database
    let _ = sqlx::query(
        "UPDATE users SET presence = $1, presence_manual = $2, last_login_at = $3 WHERE id = $4",
    )
    .bind(status)
    .bind(manual)
    .bind(now)
    .bind(user_id)
    .execute(&state.db)
    .await;

    state.ws_hub.set_presence(user_id, status.to_string()).await;

    // Create status change event with full broadcast info
    let evt = WsEnvelope::event(
        EventType::UserPresence,
        serde_json::json!({
            "user_id": user_id,
            "status": status,
            "manual": manual,
            "last_activity_at": now.timestamp_millis()
        }),
        None,
    );
    // No .with_broadcast(...) filter means it broadcasts to ALL connected users
    // which is necessary for everyone else to see this user's status change.

    tracing::debug!(
        user_id = %user_id,
        status = %status,
        "Broadcasting status change event"
    );

    state.ws_hub.broadcast(evt).await;
}

pub async fn handle_client_envelope_message(
    state: &AppState,
    user_id: Uuid,
    username: &str,
    text: &str,
    options: EnvelopeCommandOptions,
) -> bool {
    let envelope = match serde_json::from_str::<ClientEnvelope>(text) {
        Ok(envelope) => envelope,
        Err(_) => return false,
    };

    handle_client_envelope(state, user_id, username, envelope, options).await;
    true
}

pub async fn handle_client_envelope(
    state: &AppState,
    user_id: Uuid,
    username: &str,
    envelope: ClientEnvelope,
    options: EnvelopeCommandOptions,
) {
    match envelope.event.as_str() {
        "send_message" if options.allow_send_message => {
            if let Some(channel_id) = envelope.channel_id {
                if let Ok(input) =
                    serde_json::from_value::<crate::models::CreatePost>(envelope.data)
                {
                    if let Err(e) = crate::services::posts::create_post(
                        state,
                        user_id,
                        channel_id,
                        input,
                        envelope.client_msg_id,
                    )
                    .await
                    {
                        send_direct(
                            state,
                            user_id,
                            WsEnvelope::error(&format!("Failed to send message: {}", e)),
                        )
                        .await;
                    }
                }
            }
        }
        "subscribe_channel" => {
            if let Some(channel_id) = envelope.channel_id {
                let is_member = ChannelRepository::new(&state.db)
                    .is_channel_member(channel_id, user_id)
                    .await
                    .unwrap_or(false);
                if !is_member {
                    let evt = WsEnvelope::event(
                        EventType::Error,
                        serde_json::json!({ "message": "Not a member of this channel" }),
                        None,
                    );
                    send_direct(state, user_id, evt).await;
                    return;
                }
                state.ws_hub.subscribe_channel(user_id, channel_id).await;
                let evt = WsEnvelope::event(
                    EventType::ChannelSubscribed,
                    serde_json::json!({ "channel_id": channel_id }),
                    None,
                );
                send_direct(state, user_id, evt).await;
            }
        }
        "unsubscribe_channel" => {
            if let Some(channel_id) = envelope.channel_id {
                state.ws_hub.unsubscribe_channel(user_id, channel_id).await;
                if options.acknowledge_unsubscribe {
                    let evt = WsEnvelope::event(
                        EventType::ChannelUnsubscribed,
                        serde_json::json!({ "channel_id": channel_id }),
                        None,
                    );
                    send_direct(state, user_id, evt).await;
                }
            }
        }
        "typing" | "typing_start" => {
            if let Some(channel_id) = envelope.channel_id {
                let is_member = ChannelRepository::new(&state.db)
                    .is_channel_member(channel_id, user_id)
                    .await
                    .unwrap_or(false);
                if !is_member {
                    return;
                }
                let thread_root_id = serde_json::from_value::<TypingCommandData>(envelope.data)
                    .ok()
                    .and_then(|v| v.thread_root_id);

                let event = WsEnvelope::event(
                    EventType::UserTyping,
                    TypingEvent {
                        user_id,
                        display_name: username.to_string(),
                        thread_root_id,
                    },
                    Some(channel_id),
                )
                .with_broadcast(WsBroadcast {
                    channel_id: Some(channel_id),
                    user_id: None,
                    team_id: None,
                    exclude_user_id: Some(user_id),
                });
                state.ws_hub.broadcast(event).await;
            }
        }
        "typing_stop" => {
            if let Some(channel_id) = envelope.channel_id {
                let is_member = ChannelRepository::new(&state.db)
                    .is_channel_member(channel_id, user_id)
                    .await
                    .unwrap_or(false);
                if !is_member {
                    return;
                }
                let thread_root_id = serde_json::from_value::<TypingCommandData>(envelope.data)
                    .ok()
                    .and_then(|v| v.thread_root_id);

                let event = WsEnvelope::event(
                    EventType::UserTypingStop,
                    TypingEvent {
                        user_id,
                        display_name: username.to_string(),
                        thread_root_id,
                    },
                    Some(channel_id),
                )
                .with_broadcast(WsBroadcast {
                    channel_id: Some(channel_id),
                    user_id: None,
                    team_id: None,
                    exclude_user_id: Some(user_id),
                });
                state.ws_hub.broadcast(event).await;
            }
        }
        "presence" => {
            if let Some(status) = envelope.data.get("status").and_then(|v| v.as_str()) {
                persist_presence_and_broadcast(state, user_id, status, status_is_manual(status))
                    .await;
            }
        }
        "ping" => {
            // Extract seq from the envelope for the response
            let seq = envelope.seq;
            send_direct(state, user_id, WsEnvelope::pong(seq)).await;
        }
        _ => {
            if options.emit_unknown_error {
                send_direct(state, user_id, WsEnvelope::error("Unknown command")).await;
            }
        }
    }
}

pub fn status_is_manual(status: &str) -> bool {
    !status.eq_ignore_ascii_case("online")
}

async fn send_direct(state: &AppState, user_id: Uuid, envelope: WsEnvelope) {
    state
        .ws_hub
        .broadcast(envelope.with_broadcast(WsBroadcast {
            user_id: Some(user_id),
            channel_id: None,
            team_id: None,
            exclude_user_id: None,
        }))
        .await;
}

async fn subscribe_default_scopes(state: &AppState, user_id: Uuid, subscribe_channels: bool) {
    let teams =
        sqlx::query_scalar::<_, Uuid>("SELECT team_id FROM team_members WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    for team_id in teams {
        state.ws_hub.subscribe_team(user_id, team_id).await;
    }

    if !subscribe_channels {
        return;
    }

    let channels =
        sqlx::query_scalar::<_, Uuid>("SELECT channel_id FROM channel_members WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    for channel_id in channels {
        state.ws_hub.subscribe_channel(user_id, channel_id).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn normalize_token_handles_prefixes() {
        assert_eq!(
            normalize_auth_token("Bearer abc.def.ghi"),
            Some("abc.def.ghi".to_string())
        );
        assert_eq!(
            normalize_auth_token("Token abc.def.ghi"),
            Some("abc.def.ghi".to_string())
        );
        assert_eq!(normalize_auth_token("   "), None);
        assert_eq!(normalize_auth_token("undefined"), None);
    }

    #[test]
    fn resolve_token_uses_authorization_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str("Bearer auth-token").unwrap(),
        );

        let token = resolve_auth_token_secure(&headers, None);
        assert_eq!(token.as_deref(), Some("auth-token"));
    }

    #[test]
    fn resolve_token_uses_protocol_fallback() {
        let headers = HeaderMap::new();
        let protocol_token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjMifQ.signature";
        let token = resolve_auth_token_secure(&headers, Some(protocol_token));
        assert_eq!(token.as_deref(), Some(protocol_token));
    }

    #[test]
    fn resolve_token_without_secure_transport_is_none() {
        let headers = HeaderMap::new();
        let token = resolve_auth_token(&headers, None);
        assert_eq!(token, None);
    }

    #[test]
    fn claims_to_websocket_auth_maps_valid_expiry() {
        let user_id = Uuid::new_v4();
        let exp = Utc::now().timestamp() + 120;
        let claims = Claims {
            sub: user_id,
            email: "ws@example.com".to_string(),
            role: "member".to_string(),
            org_id: None,
            iss: None,
            aud: None,
            iat: exp - 60,
            exp,
        };

        let auth = claims_to_websocket_auth(claims).expect("expected valid websocket auth");
        assert_eq!(auth.user_id, user_id);
        assert_eq!(auth.expires_at.timestamp(), exp);
    }

    #[test]
    fn claims_to_websocket_auth_rejects_invalid_expiry_timestamp() {
        let claims = Claims {
            sub: Uuid::new_v4(),
            email: "ws@example.com".to_string(),
            role: "member".to_string(),
            org_id: None,
            iss: None,
            aud: None,
            iat: 0,
            exp: i64::MAX,
        };

        assert!(claims_to_websocket_auth(claims).is_none());
    }
}
