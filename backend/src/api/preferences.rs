//! User preferences and status API endpoints

use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use uuid::Uuid;

use super::AppState;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{
    ChannelNotificationSetting, CreateStatusPreset, StatusPreset, UpdateChannelNotification,
    UpdatePreferences, UpdateStatus, UserPreferences, UserStatus,
};
use crate::realtime::{EventType, PresenceEvent, WsEnvelope};

/// Build preferences routes
pub fn router() -> Router<AppState> {
    Router::new()
        // User status
        .route("/users/me/status", get(get_my_status))
        .route("/users/me/status", put(update_my_status))
        .route("/users/me/status", axum::routing::delete(clear_my_status))
        .route("/users/{user_id}/status", get(get_user_status))
        // User preferences
        .route("/users/me/preferences", get(get_my_preferences))
        .route("/users/me/preferences", put(update_my_preferences))
        // Status presets
        .route("/users/me/status/presets", get(list_status_presets))
        .route("/users/me/status/presets", post(create_status_preset))
        .route(
            "/users/me/status/presets/{preset_id}",
            axum::routing::delete(delete_status_preset),
        )
        // Channel notifications
        .route(
            "/channels/{channel_id}/notifications",
            get(get_channel_notifications),
        )
        .route(
            "/channels/{channel_id}/notifications",
            put(update_channel_notifications),
        )
}

/// Get current user's status
async fn get_my_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<UserStatus>> {
    let user = sqlx::query_as::<
        _,
        (
            String,
            Option<String>,
            Option<String>,
            Option<chrono::DateTime<Utc>>,
        ),
    >(
        "SELECT presence, status_text, status_emoji, status_expires_at FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(UserStatus {
        presence: Some(user.0),
        text: user.1,
        emoji: user.2,
        expires_at: user.3,
    }))
}

/// Update current user's status
async fn update_my_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<UpdateStatus>,
) -> ApiResult<Json<UserStatus>> {
    let expires_at = payload
        .duration_minutes
        .map(|mins| Utc::now() + Duration::minutes(mins as i64));

    // We only update presence if it's provided.
    // If text/emoji are provided, we update them.
    // This allows updating just presence or just status message.

    // First get current values to merge if needed, or we can just run dynamic query
    let mut builder = sqlx::QueryBuilder::new("UPDATE users SET updated_at = NOW()");

    if let Some(ref p) = payload.presence {
        builder.push(", presence = ");
        builder.push_bind(p);
    }

    if payload.text.is_some() || payload.emoji.is_some() {
        // If updating custom status, always update these fields
        // Note: passing None for text/emoji clears them (which is what we want if user clears status)
        // But the DTO has Option, so if it's missing (None), it might mean "don't change".
        // We need to clarify behavior.
        // Ideally:
        // - presence: Update if Some
        // - text/emoji: Update if Some. To clear, client should send Some("") or explicit null?
        // For simplicity, let's assume client sends all fields they want to change.
        // But typical "set status" UI might only send text/emoji. "Set presence" UI only sends presence.

        if let Some(ref t) = payload.text {
            builder.push(", status_text = ");
            builder.push_bind(t);
        }
        if let Some(ref e) = payload.emoji {
            builder.push(", status_emoji = ");
            builder.push_bind(e);
        }
        if expires_at.is_some() {
            builder.push(", status_expires_at = ");
            builder.push_bind(expires_at);
        }
    }

    builder.push(" WHERE id = ");
    builder.push_bind(auth.user_id);
    builder.push(" RETURNING presence, status_text, status_emoji, status_expires_at");

    let query = builder.build_query_as::<(
        String,
        Option<String>,
        Option<String>,
        Option<chrono::DateTime<Utc>>,
    )>();
    let user = query.fetch_one(&state.db).await?;

    // Update Hub and broadcast presence change
    state
        .ws_hub
        .set_presence(auth.user_id, user.0.clone())
        .await;

    let user_status = UserStatus {
        presence: Some(user.0.clone()),
        text: user.1.clone(),
        emoji: user.2.clone(),
        expires_at: user.3,
    };

    // Broadcast presence change
    let event = WsEnvelope::event(
        EventType::UserPresence,
        PresenceEvent {
            user_id: auth.user_id,
            status: user.0.clone(),
        },
        None,
    );
    state.ws_hub.broadcast(event).await;

    // Broadcast full user update (for status message/emoji)
    let full_user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

    let update_event = WsEnvelope::event(
        EventType::UserUpdated,
        crate::models::UserResponse::from(full_user),
        None,
    );
    state.ws_hub.broadcast(update_event).await;

    Ok(Json(user_status))
}

/// Clear current user's status
async fn clear_my_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<UserStatus>> {
    let user = sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<chrono::DateTime<Utc>>)>(
        "UPDATE users SET status_text = NULL, status_emoji = NULL, status_expires_at = NULL, updated_at = NOW() WHERE id = $1 RETURNING presence, status_text, status_emoji, status_expires_at"
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    // Update Hub and broadcast presence change
    state
        .ws_hub
        .set_presence(auth.user_id, user.0.clone())
        .await;

    let user_status = UserStatus {
        presence: Some(user.0.clone()),
        text: user.1.clone(),
        emoji: user.2.clone(),
        expires_at: user.3,
    };

    // Broadcast presence change
    let event = WsEnvelope::event(
        EventType::UserPresence,
        PresenceEvent {
            user_id: auth.user_id,
            status: user.0.clone(),
        },
        None,
    );
    state.ws_hub.broadcast(event).await;

    // Broadcast full user update (for cleared status)
    let full_user: crate::models::User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

    let update_event = WsEnvelope::event(
        EventType::UserUpdated,
        crate::models::UserResponse::from(full_user),
        None,
    );
    state.ws_hub.broadcast(update_event).await;

    Ok(Json(user_status))
}

/// Get another user's status
async fn get_user_status(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Json<UserStatus>> {
    let user = sqlx::query_as::<
        _,
        (
            String,
            Option<String>,
            Option<String>,
            Option<chrono::DateTime<Utc>>,
        ),
    >(
        "SELECT presence, status_text, status_emoji, status_expires_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // Check if status has expired
    let mut text = user.1;
    let mut emoji = user.2;
    let expires = user.3;

    if let Some(exp) = expires {
        if exp < Utc::now() {
            text = None;
            emoji = None;
        }
    }

    Ok(Json(UserStatus {
        presence: Some(user.0),
        text,
        emoji,
        expires_at: expires,
    }))
}

/// Get current user's preferences
async fn get_my_preferences(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<UserPreferences>> {
    // Try to get existing preferences
    let prefs =
        sqlx::query_as::<_, UserPreferences>("SELECT * FROM user_preferences WHERE user_id = $1")
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?;

    match prefs {
        Some(p) => Ok(Json(p)),
        None => {
            // Create default preferences
            let prefs = sqlx::query_as::<_, UserPreferences>(
                r#"
                INSERT INTO user_preferences (user_id) VALUES ($1)
                RETURNING *
                "#,
            )
            .bind(auth.user_id)
            .fetch_one(&state.db)
            .await?;
            Ok(Json(prefs))
        }
    }
}

/// Update current user's preferences
async fn update_my_preferences(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<UpdatePreferences>,
) -> ApiResult<Json<UserPreferences>> {
    // Upsert preferences
    let prefs = sqlx::query_as::<_, UserPreferences>(
        r#"
        INSERT INTO user_preferences (user_id, notify_desktop, notify_push, notify_email, notify_sounds,
            dnd_enabled, message_display, sidebar_behavior, time_format, mention_keywords)
        VALUES ($1, COALESCE($2, 'all'), COALESCE($3, 'all'), COALESCE($4, 'none'), COALESCE($5, true),
            COALESCE($6, false), COALESCE($7, 'standard'), COALESCE($8, 'unreads_first'), COALESCE($9, '12h'), $10)
        ON CONFLICT (user_id) DO UPDATE SET
            notify_desktop = COALESCE($2, user_preferences.notify_desktop),
            notify_push = COALESCE($3, user_preferences.notify_push),
            notify_email = COALESCE($4, user_preferences.notify_email),
            notify_sounds = COALESCE($5, user_preferences.notify_sounds),
            dnd_enabled = COALESCE($6, user_preferences.dnd_enabled),
            message_display = COALESCE($7, user_preferences.message_display),
            sidebar_behavior = COALESCE($8, user_preferences.sidebar_behavior),
            time_format = COALESCE($9, user_preferences.time_format),
            mention_keywords = COALESCE($10, user_preferences.mention_keywords),
            updated_at = NOW()
        RETURNING *
        "#
    )
    .bind(auth.user_id)
    .bind(&payload.notify_desktop)
    .bind(&payload.notify_push)
    .bind(&payload.notify_email)
    .bind(payload.notify_sounds)
    .bind(payload.dnd_enabled)
    .bind(&payload.message_display)
    .bind(&payload.sidebar_behavior)
    .bind(&payload.time_format)
    .bind(&payload.mention_keywords)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(prefs))
}

/// List status presets (default + user custom)
async fn list_status_presets(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<StatusPreset>>> {
    let presets = sqlx::query_as::<_, StatusPreset>(
        "SELECT * FROM status_presets WHERE user_id IS NULL OR user_id = $1 ORDER BY is_default DESC, sort_order"
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(presets))
}

/// Create a custom status preset
async fn create_status_preset(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateStatusPreset>,
) -> ApiResult<Json<StatusPreset>> {
    let preset = sqlx::query_as::<_, StatusPreset>(
        r#"
        INSERT INTO status_presets (user_id, emoji, text, duration_minutes, sort_order)
        VALUES ($1, $2, $3, $4, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM status_presets WHERE user_id = $1))
        RETURNING *
        "#
    )
    .bind(auth.user_id)
    .bind(&payload.emoji)
    .bind(&payload.text)
    .bind(payload.duration_minutes)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(preset))
}

/// Delete a custom status preset
async fn delete_status_preset(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(preset_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query(
        "DELETE FROM status_presets WHERE id = $1 AND user_id = $2 AND is_default = false",
    )
    .bind(preset_id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Preset not found or cannot be deleted".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

/// Get channel notification settings
async fn get_channel_notifications(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
) -> ApiResult<Json<Option<ChannelNotificationSetting>>> {
    let setting = sqlx::query_as::<_, ChannelNotificationSetting>(
        "SELECT * FROM channel_notification_settings WHERE user_id = $1 AND channel_id = $2",
    )
    .bind(auth.user_id)
    .bind(channel_id)
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(setting))
}

/// Update channel notification settings
async fn update_channel_notifications(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
    Json(payload): Json<UpdateChannelNotification>,
) -> ApiResult<Json<ChannelNotificationSetting>> {
    let setting = sqlx::query_as::<_, ChannelNotificationSetting>(
        r#"
        INSERT INTO channel_notification_settings (user_id, channel_id, notify_level, is_muted, mute_until)
        VALUES ($1, $2, COALESCE($3, 'default'), COALESCE($4, false), $5)
        ON CONFLICT (user_id, channel_id) DO UPDATE SET
            notify_level = COALESCE($3, channel_notification_settings.notify_level),
            is_muted = COALESCE($4, channel_notification_settings.is_muted),
            mute_until = COALESCE($5, channel_notification_settings.mute_until),
            updated_at = NOW()
        RETURNING *
        "#
    )
    .bind(auth.user_id)
    .bind(channel_id)
    .bind(&payload.notify_level)
    .bind(payload.is_muted)
    .bind(payload.mute_until)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(setting))
}
