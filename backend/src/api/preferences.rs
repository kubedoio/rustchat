//! User preferences and status API endpoints

use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use super::AppState;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{
    ChannelNotificationSetting, CreateStatusPreset, StatusPreset, UpdateChannelNotification,
    UpdatePreferences, UserPreferences, UserStatus,
};
use crate::realtime::{EventType, PresenceEvent, WsEnvelope};
use crate::repositories::UserRepository;

/// Build preferences routes
pub fn router() -> Router<AppState> {
    Router::new()
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

fn to_system_time(last_activity: Option<chrono::DateTime<Utc>>) -> SystemTime {
    last_activity
        .and_then(|value| {
            let millis = value.timestamp_millis();
            if millis >= 0 {
                Some(UNIX_EPOCH + StdDuration::from_millis(millis as u64))
            } else {
                None
            }
        })
        .unwrap_or_else(SystemTime::now)
}

/// Get current user's status
#[allow(dead_code)]
async fn get_my_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<UserStatus>> {
    let repo = UserRepository::new(&state.db);
    let user = repo
        .get_user_status_fields(auth.user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::UserNotFound)?;

    Ok(Json(UserStatus {
        presence: Some(user.0),
        manual: user.1,
        last_activity: to_system_time(user.2),
        text: user.3,
        emoji: user.4,
        expires_at: user.5,
    }))
}

/// Update current user's status
#[allow(dead_code)]
async fn update_my_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<crate::models::UpdateStatus>,
) -> ApiResult<Json<UserStatus>> {
    let expires_at = payload
        .duration_minutes
        .map(|mins| Utc::now() + Duration::minutes(mins as i64));

    let repo = UserRepository::new(&state.db);

    // Build dynamic update using QueryBuilder
    let mut builder = sqlx::QueryBuilder::new("UPDATE users SET updated_at = NOW()");

    if let Some(ref p) = payload.presence {
        builder.push(", presence = ");
        builder.push_bind(p);
        builder.push(", presence_manual = ");
        builder.push_bind(crate::api::websocket_core::status_is_manual(p));
        builder.push(", last_login_at = NOW()");
    }

    if payload.text.is_some() || payload.emoji.is_some() {
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
    builder.push(" RETURNING presence, COALESCE(presence_manual, false), last_login_at, status_text, status_emoji, status_expires_at");

    let query = builder.build_query_as::<(
        String,
        bool,
        Option<chrono::DateTime<Utc>>,
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
        manual: user.1,
        last_activity: to_system_time(user.2),
        text: user.3.clone(),
        emoji: user.4.clone(),
        expires_at: user.5,
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
    let full_user = repo
        .get_by_id(auth.user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::UserNotFound)?;

    let update_event = WsEnvelope::event(
        EventType::UserUpdated,
        crate::models::UserResponse::from(full_user),
        None,
    );
    state.ws_hub.broadcast(update_event).await;

    Ok(Json(user_status))
}

/// Clear current user's status
#[allow(dead_code)]
async fn clear_my_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<UserStatus>> {
    let repo = UserRepository::new(&state.db);
    let user = repo
        .clear_status_returning(auth.user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Update Hub and broadcast presence change
    state
        .ws_hub
        .set_presence(auth.user_id, user.0.clone())
        .await;

    let user_status = UserStatus {
        presence: Some(user.0.clone()),
        manual: user.1,
        last_activity: to_system_time(user.2),
        text: user.3.clone(),
        emoji: user.4.clone(),
        expires_at: user.5,
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
    let full_user = repo
        .get_by_id(auth.user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::UserNotFound)?;

    let update_event = WsEnvelope::event(
        EventType::UserUpdated,
        crate::models::UserResponse::from(full_user),
        None,
    );
    state.ws_hub.broadcast(update_event).await;

    Ok(Json(user_status))
}

/// Get another user's status
#[allow(dead_code)]
async fn get_user_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Json<UserStatus>> {
    // Only allow querying your own status
    if user_id != auth.user_id {
        return Err(AppError::Forbidden(
            "You can only view your own status".to_string(),
        ));
    }

    let repo = UserRepository::new(&state.db);
    let user = repo
        .get_user_status_fields(user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::UserNotFound)?;

    // Check if status has expired
    let mut text = user.3;
    let mut emoji = user.4;
    let expires = user.5;

    if let Some(exp) = expires {
        if exp < Utc::now() {
            text = None;
            emoji = None;
        }
    }

    Ok(Json(UserStatus {
        presence: Some(user.0),
        manual: user.1,
        last_activity: to_system_time(user.2),
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
    let repo = UserRepository::new(&state.db);
    let prefs = repo
        .get_or_create_preferences(auth.user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(prefs))
}

/// Update current user's preferences
async fn update_my_preferences(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<UpdatePreferences>,
) -> ApiResult<Json<UserPreferences>> {
    let repo = UserRepository::new(&state.db);
    let prefs = repo
        .upsert_preferences(auth.user_id, &payload)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(prefs))
}

/// List status presets (default + user custom)
async fn list_status_presets(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<StatusPreset>>> {
    let repo = UserRepository::new(&state.db);
    let presets = repo
        .list_status_presets(auth.user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(presets))
}

/// Create a custom status preset
async fn create_status_preset(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateStatusPreset>,
) -> ApiResult<Json<StatusPreset>> {
    let repo = UserRepository::new(&state.db);
    let preset = repo
        .create_status_preset(
            auth.user_id,
            &payload.emoji,
            &payload.text,
            payload.duration_minutes,
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(preset))
}

/// Delete a custom status preset
async fn delete_status_preset(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(preset_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = UserRepository::new(&state.db);
    let rows = repo
        .delete_status_preset(preset_id, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if rows == 0 {
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
    let repo = UserRepository::new(&state.db);
    let setting = repo
        .get_channel_notification(auth.user_id, channel_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(setting))
}

/// Update channel notification settings
async fn update_channel_notifications(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
    Json(payload): Json<UpdateChannelNotification>,
) -> ApiResult<Json<ChannelNotificationSetting>> {
    let repo = UserRepository::new(&state.db);
    let setting = repo
        .upsert_channel_notification(auth.user_id, channel_id, &payload)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(setting))
}
