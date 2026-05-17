//! User Status API
//!
//! Handles user presence status (online, away, dnd, offline) and custom status

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use crate::mattermost_compat::models as mm;
use crate::realtime::WsEnvelope;
use crate::repositories::UserRepository;

/// Build status routes
pub fn router() -> Router<AppState> {
    Router::new()
        // GET /api/v4/users/{user_id}/status
        .route(
            "/users/{user_id}/status",
            get(get_user_status).put(update_user_status),
        )
        // GET /api/v4/users/me/status
        .route("/users/me/status", get(get_my_status).put(update_my_status))
        // POST /api/v4/users/status/ids
        .route("/users/status/ids", post(get_statuses_by_ids))
        // Custom status endpoints
        .route(
            "/users/{user_id}/status/custom",
            put(update_user_custom_status).delete(clear_user_custom_status),
        )
        .route(
            "/users/{user_id}/status/custom/recent",
            get(get_recent_custom_statuses),
        )
        .route(
            "/users/{user_id}/status/custom/recent/delete",
            post(delete_recent_custom_status),
        )
}

/// User status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStatusResponse {
    pub user_id: String,
    pub status: String, // online, away, dnd, offline
    pub manual: bool,
    pub last_activity_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStatusSnapshot {
    pub user_id: String,
    pub status: String,
    pub manual: bool,
    pub last_activity_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct UserStatusSnapshotRow {
    presence: String,
    manual: bool,
    last_login_at: Option<DateTime<Utc>>,
    text: Option<String>,
    emoji: Option<String>,
    expires_at: Option<DateTime<Utc>>,
}

/// Custom status duration options (Mattermost-compatible)
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CustomStatusDuration {
    #[serde(alias = "")]
    #[default]
    DontClear,
    ThirtyMinutes,
    OneHour,
    FourHours,
    Today,
    ThisWeek,
    DateAndTime,
    CustomDateTime,
}

impl CustomStatusDuration {
    /// Convert duration to expires_at timestamp
    fn to_expires_at(&self, custom_time: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        match self {
            CustomStatusDuration::DontClear => None,
            CustomStatusDuration::ThirtyMinutes => Some(now + Duration::minutes(30)),
            CustomStatusDuration::OneHour => Some(now + Duration::hours(1)),
            CustomStatusDuration::FourHours => Some(now + Duration::hours(4)),
            CustomStatusDuration::Today => {
                // End of today (midnight tonight)
                let tomorrow = now.date_naive().succ_opt()?;
                Some(DateTime::from_naive_utc_and_offset(
                    tomorrow.and_hms_opt(0, 0, 0)?,
                    chrono::Utc,
                ))
            }
            CustomStatusDuration::ThisWeek => {
                // End of this week (Sunday midnight)
                let days_until_sunday = (7 - now.weekday().num_days_from_sunday()) % 7;
                let end_of_week = now + Duration::days(days_until_sunday as i64);
                let next_week = end_of_week.date_naive().succ_opt()?;
                Some(DateTime::from_naive_utc_and_offset(
                    next_week.and_hms_opt(0, 0, 0)?,
                    chrono::Utc,
                ))
            }
            CustomStatusDuration::DateAndTime | CustomStatusDuration::CustomDateTime => custom_time,
        }
    }
}

/// Custom status request/response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomStatus {
    pub emoji: String,
    pub text: String,
    #[serde(default)]
    pub duration: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl CustomStatus {
    /// Pre-save processing - truncate text and set duration if needed
    fn pre_save(&mut self) {
        const MAX_RUNES: usize = 128;
        let runes: Vec<char> = self.text.chars().collect();
        if runes.len() > MAX_RUNES {
            self.text = runes[..MAX_RUNES].iter().collect();
        }

        // If duration is empty but expires_at is set, set duration to date_and_time
        if self.duration.is_empty() && self.expires_at.is_some() {
            self.duration = "date_and_time".to_string();
        }
    }
}

pub fn should_update_custom_status(body: &UpdateMyStatusRequest) -> bool {
    body.text.is_some() || body.emoji.is_some() || body.clear == Some(true)
}

pub fn build_custom_status_from_update_request(
    body: &UpdateMyStatusRequest,
) -> Option<CustomStatus> {
    if body.clear == Some(true)
        || (body.text.as_ref().map(|t| t.is_empty()).unwrap_or(false)
            && body.emoji.as_ref().map(|e| e.is_empty()).unwrap_or(false))
    {
        return None;
    }

    let mut cs = CustomStatus {
        emoji: body.emoji.clone().unwrap_or_default(),
        text: body.text.clone().unwrap_or_default(),
        duration: body.duration.clone().unwrap_or_default(),
        expires_at: None,
    };

    let duration_enum = match cs.duration.as_str() {
        "thirty_minutes" => Some(CustomStatusDuration::ThirtyMinutes),
        "one_hour" => Some(CustomStatusDuration::OneHour),
        "four_hours" => Some(CustomStatusDuration::FourHours),
        "today" => Some(CustomStatusDuration::Today),
        "this_week" => Some(CustomStatusDuration::ThisWeek),
        "date_and_time" | "custom_date_time" => Some(CustomStatusDuration::DateAndTime),
        "" | "dont_clear" => Some(CustomStatusDuration::DontClear),
        _ => {
            if let Some(minutes) = body.duration_minutes {
                match minutes {
                    30 => Some(CustomStatusDuration::ThirtyMinutes),
                    60 => Some(CustomStatusDuration::OneHour),
                    240 => Some(CustomStatusDuration::FourHours),
                    0 => Some(CustomStatusDuration::Today),
                    _ => None,
                }
            } else {
                None
            }
        }
    };

    if let Some(dur) = duration_enum {
        cs.expires_at = dur.to_expires_at(None);
        if cs.expires_at.is_some() && cs.duration.is_empty() {
            cs.duration = "date_and_time".to_string();
        }
    }

    cs.pre_save();
    Some(cs)
}

/// Update status request (for presence status)
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String, // online, away, dnd, offline
    #[serde(default)]
    pub dnd_end_time: Option<i64>, // Unix timestamp in milliseconds for DND duration
}

/// Full status update request (includes custom status)
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateMyStatusRequest {
    // Legacy fields for backwards compatibility
    #[allow(dead_code)]
    #[serde(default)]
    pub user_id: Option<String>,

    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub presence: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub duration: Option<String>, // MM-compatible duration string
    #[serde(default)]
    pub duration_minutes: Option<i64>, // Legacy duration in minutes
    #[serde(default)]
    pub dnd_end_time: Option<i64>,
    #[serde(default)]
    pub clear: Option<bool>,
}

/// Bulk status request
#[derive(Debug, Clone, Deserialize)]
pub struct BulkStatusRequest {
    pub user_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum BulkStatusRequestCompat {
    Raw(Vec<String>),
    Wrapped(BulkStatusRequest),
}

impl BulkStatusRequestCompat {
    fn into_user_ids(self) -> Vec<String> {
        match self {
            Self::Raw(ids) => ids,
            Self::Wrapped(req) => req.user_ids,
        }
    }
}

/// Get current user's status
/// GET /api/v4/users/me/status
async fn get_my_status(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<UserStatusResponse>> {
    get_user_status_by_id(&state, auth.user_id).await
}

/// Get user status by ID
/// GET /api/v4/users/{user_id}/status
async fn get_user_status(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> ApiResult<Json<UserStatusResponse>> {
    let user_uuid = parse_mm_or_uuid(&user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user ID".to_string()))?;
    get_user_status_by_id(&state, user_uuid).await
}

/// Internal: Get user status by UUID
async fn get_user_status_by_id(
    state: &AppState,
    user_id: Uuid,
) -> ApiResult<Json<UserStatusResponse>> {
    let snapshot = fetch_user_status_snapshot(state, user_id).await?;

    Ok(Json(UserStatusResponse {
        user_id: snapshot.user_id,
        status: snapshot.status,
        manual: snapshot.manual,
        last_activity_at: snapshot.last_activity_at,
    }))
}

/// Update my status (full update including custom status)
/// PUT /api/v4/users/me/status
async fn update_my_status(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let body: UpdateMyStatusRequest = parse_body(&headers, &body, "Invalid status body")?;
    let mut should_broadcast = false;

    if let Some(presence) = body.status.clone().or(body.presence.clone()) {
        let dnd_end_time = body.dnd_end_time;
        let _ = update_user_status_internal(&state, auth.user_id, presence, dnd_end_time, false)
            .await?;
        should_broadcast = true;
    }

    if should_update_custom_status(&body) {
        let custom_status = build_custom_status_from_update_request(&body);
        update_custom_status_internal(&state, auth.user_id, custom_status, false).await?;
        should_broadcast = true;
    }

    if should_broadcast {
        broadcast_status_change(&state, auth.user_id).await?;
    }

    let snapshot = fetch_user_status_snapshot(&state, auth.user_id).await?;

    Ok(Json(serde_json::json!({
        "user_id": snapshot.user_id,
        "status": snapshot.status,
        "text": snapshot.text,
        "emoji": snapshot.emoji,
        "expires_at": snapshot.expires_at,
    })))
}

/// Update user status
/// PUT /api/v4/users/{user_id}/status
async fn update_user_status(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(user_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let target_user_id = parse_mm_or_uuid(&user_id)
        .ok_or_else(|| AppError::BadRequest("Invalid user ID".to_string()))?;

    // Users can only update their own status unless admin
    if target_user_id != auth.user_id && !auth.has_role("admin") {
        return Err(AppError::Forbidden(
            "Can only update your own status".to_string(),
        ));
    }

    let req: UpdateStatusRequest = parse_body(&headers, &body, "Invalid status body")?;
    update_user_status_internal(&state, target_user_id, req.status, req.dnd_end_time, true).await
}

/// Internal: Update user presence status
async fn update_user_status_internal(
    state: &AppState,
    user_id: Uuid,
    status: String,
    _dnd_end_time: Option<i64>,
    broadcast: bool,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate status
    let valid_statuses = ["online", "away", "dnd", "offline"];
    if !valid_statuses.contains(&status.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid status: {}. Must be one of: online, away, dnd, offline",
            status
        )));
    }

    // Determine if this is a manual status
    let manual = status != "online";

    // Update user presence
    let repo = UserRepository::new(&state.db);
    repo.update_presence(user_id, &status, manual)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if broadcast {
        broadcast_status_change(state, user_id).await?;
    }

    Ok(Json(serde_json::json!({"status": status})))
}

/// Internal: Update custom status
pub async fn update_custom_status_internal(
    state: &AppState,
    user_id: Uuid,
    custom_status: Option<CustomStatus>,
    broadcast: bool,
) -> ApiResult<()> {
    let repo = UserRepository::new(&state.db);

    let (text, emoji, expires_at, json_status) = if let Some(ref cs) = custom_status {
        let json = serde_json::json!({
            "emoji": cs.emoji,
            "text": cs.text,
            "duration": cs.duration,
            "expires_at": cs.expires_at.map(|t| t.to_rfc3339()),
        });
        (
            Some(cs.text.as_str()),
            Some(cs.emoji.as_str()),
            cs.expires_at,
            json,
        )
    } else {
        (
            None::<&str>,
            None::<&str>,
            None::<DateTime<Utc>>,
            Value::Null,
        )
    };

    repo.update_status_fields(user_id, text, emoji, expires_at, json_status)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Add to recent custom statuses (stored in user preferences)
    if let Some(cs) = custom_status {
        add_to_recent_custom_statuses(state, user_id, &cs).await?;
    }

    if broadcast {
        broadcast_status_change(state, user_id).await?;
    }

    Ok(())
}

/// Add a custom status to the recent list (stored in preferences)
async fn add_to_recent_custom_statuses(
    state: &AppState,
    user_id: Uuid,
    custom_status: &CustomStatus,
) -> ApiResult<()> {
    let repo = UserRepository::new(&state.db);

    // Get existing recent statuses from preferences
    let existing = repo
        .get_recent_custom_status_value(user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let mut recent: Vec<CustomStatus> = existing
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default();

    // Remove duplicate if exists
    recent.retain(|s| !(s.emoji == custom_status.emoji && s.text == custom_status.text));

    // Add new status at the beginning
    recent.insert(0, custom_status.clone());

    // Keep only last 5
    recent.truncate(5);

    // Save back to preferences
    let json_str = serde_json::to_string(&recent).unwrap_or_default();
    repo.save_recent_custom_status_value(user_id, &json_str)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(())
}

/// Update user's custom status
/// PUT /api/v4/users/{user_id}/status/custom
async fn update_user_custom_status(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(user_id): Path<String>,
    Json(body): Json<CustomStatus>,
) -> ApiResult<Json<serde_json::Value>> {
    let target_user_id = resolve_status_target_user_id(&user_id, &auth)?;

    // Users can only update their own custom status unless admin
    if target_user_id != auth.user_id && !auth.has_role("admin") {
        return Err(AppError::Forbidden(
            "Can only update your own custom status".to_string(),
        ));
    }

    let mut custom_status = body;

    // Parse duration and calculate expires_at
    let duration_enum = match custom_status.duration.as_str() {
        "thirty_minutes" => Some(CustomStatusDuration::ThirtyMinutes),
        "one_hour" => Some(CustomStatusDuration::OneHour),
        "four_hours" => Some(CustomStatusDuration::FourHours),
        "today" => Some(CustomStatusDuration::Today),
        "this_week" => Some(CustomStatusDuration::ThisWeek),
        "date_and_time" | "custom_date_time" => Some(CustomStatusDuration::DateAndTime),
        "" | "dont_clear" => Some(CustomStatusDuration::DontClear),
        _ => None,
    };

    if let Some(dur) = duration_enum {
        custom_status.expires_at = dur.to_expires_at(custom_status.expires_at);
        if custom_status.expires_at.is_some() && custom_status.duration.is_empty() {
            custom_status.duration = "date_and_time".to_string();
        }
    }

    custom_status.pre_save();
    update_custom_status_internal(&state, target_user_id, Some(custom_status.clone()), true)
        .await?;

    Ok(Json(serde_json::json!(custom_status)))
}

/// Clear user's custom status
/// DELETE /api/v4/users/{user_id}/status/custom
async fn clear_user_custom_status(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let target_user_id = resolve_status_target_user_id(&user_id, &auth)?;

    // Users can only clear their own custom status unless admin
    if target_user_id != auth.user_id && !auth.has_role("admin") {
        return Err(AppError::Forbidden(
            "Can only clear your own custom status".to_string(),
        ));
    }

    update_custom_status_internal(&state, target_user_id, None, true).await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// Get recent custom statuses for a user
/// GET /api/v4/users/{user_id}/status/custom/recent
async fn get_recent_custom_statuses(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(user_id): Path<String>,
) -> ApiResult<Json<Vec<CustomStatus>>> {
    let target_user_id = resolve_status_target_user_id(&user_id, &auth)?;

    // Users can only get their own recent statuses unless admin
    if target_user_id != auth.user_id && !auth.has_role("admin") {
        return Err(AppError::Forbidden(
            "Can only get your own recent custom statuses".to_string(),
        ));
    }

    let repo = UserRepository::new(&state.db);
    let recent = repo
        .get_recent_custom_status_value(target_user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let statuses: Vec<CustomStatus> = recent
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default();

    Ok(Json(statuses))
}

/// Delete a recent custom status
/// POST /api/v4/users/{user_id}/status/custom/recent/delete
async fn delete_recent_custom_status(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(user_id): Path<String>,
    Json(body): Json<CustomStatus>,
) -> ApiResult<Json<serde_json::Value>> {
    let target_user_id = resolve_status_target_user_id(&user_id, &auth)?;

    // Users can only delete their own recent statuses unless admin
    if target_user_id != auth.user_id && !auth.has_role("admin") {
        return Err(AppError::Forbidden(
            "Can only delete your own recent custom statuses".to_string(),
        ));
    }

    let repo = UserRepository::new(&state.db);

    // Get existing recent statuses
    let existing = repo
        .get_recent_custom_status_value(target_user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let mut recent: Vec<CustomStatus> = existing
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default();

    // Remove matching status
    recent.retain(|s| !(s.emoji == body.emoji && s.text == body.text));

    // Save back
    let json_str = serde_json::to_string(&recent).unwrap_or_default();
    repo.save_recent_custom_status_value(target_user_id, &json_str)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// Get statuses for multiple users
/// POST /api/v4/users/status/ids
async fn get_statuses_by_ids(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Vec<mm::Status>>> {
    let input: BulkStatusRequestCompat = parse_body(&headers, &body, "Invalid user_ids")?;
    let user_ids_raw = input.into_user_ids();

    if user_ids_raw.is_empty() {
        return Err(AppError::BadRequest("Invalid user_ids".to_string()));
    }

    let mut user_ids = Vec::with_capacity(user_ids_raw.len());
    for raw in user_ids_raw {
        let parsed = parse_mm_or_uuid(&raw)
            .ok_or_else(|| AppError::BadRequest("Invalid user_ids".to_string()))?;
        user_ids.push(parsed);
    }

    let repo = UserRepository::new(&state.db);
    let statuses = repo
        .get_statuses_by_ids(&user_ids)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let result = statuses
        .into_iter()
        .map(|(user_id, status, manual, last_login)| mm::Status {
            user_id: encode_mm_id(user_id),
            status: if status.is_empty() {
                "offline".to_string()
            } else {
                status
            },
            manual,
            last_activity_at: last_login.map(|t| t.timestamp_millis()).unwrap_or(0),
        })
        .collect();

    Ok(Json(result))
}

/// Parse request body helper - handles both JSON and form-urlencoded, and is
/// lenient about Content-Type header (for Mattermost mobile compatibility)
fn parse_body<T: serde::de::DeserializeOwned>(
    headers: &HeaderMap,
    body: &Bytes,
    message: &str,
) -> ApiResult<T> {
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.starts_with("application/json") {
        serde_json::from_slice(body).map_err(|_| AppError::BadRequest(message.to_string()))
    } else if content_type.starts_with("application/x-www-form-urlencoded") {
        serde_urlencoded::from_bytes(body).map_err(|_| AppError::BadRequest(message.to_string()))
    } else {
        // Try JSON first, then fall back to url-encoded (handles missing content-type)
        serde_json::from_slice(body)
            .or_else(|_| serde_urlencoded::from_bytes(body))
            .map_err(|_| AppError::BadRequest(message.to_string()))
    }
}

pub async fn clear_expired_custom_status_if_needed(
    state: &AppState,
    user_id: Uuid,
) -> ApiResult<bool> {
    UserRepository::new(&state.db)
        .clear_expired_custom_status_if_needed(user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

pub async fn clear_expired_custom_statuses_for_users(
    state: &AppState,
    user_ids: &[Uuid],
) -> ApiResult<i64> {
    if user_ids.is_empty() {
        return Ok(0);
    }

    UserRepository::new(&state.db)
        .clear_expired_custom_statuses_for_users(user_ids)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

pub async fn clear_expired_custom_statuses(state: &AppState) -> ApiResult<Vec<Uuid>> {
    UserRepository::new(&state.db)
        .clear_expired_custom_statuses()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

pub async fn fetch_user_status_snapshot(
    state: &AppState,
    user_id: Uuid,
) -> ApiResult<UserStatusSnapshot> {
    let _ = clear_expired_custom_status_if_needed(state, user_id).await?;

    let repo = UserRepository::new(&state.db);
    let result = repo
        .get_user_status_fields(user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let (presence, manual, last_login_at, text, emoji, expires_at) = result;

    Ok(UserStatusSnapshot {
        user_id: encode_mm_id(user_id),
        status: if presence.is_empty() {
            "offline".to_string()
        } else {
            presence
        },
        manual,
        last_activity_at: last_login_at.map(|t| t.timestamp_millis()).unwrap_or(0),
        text,
        emoji,
        expires_at: expires_at.map(|t| t.timestamp_millis()),
    })
}

/// Broadcast status change via WebSocket
pub async fn broadcast_status_change(state: &AppState, user_id: Uuid) -> ApiResult<()> {
    let snapshot = fetch_user_status_snapshot(state, user_id).await?;
    let event = WsEnvelope {
        msg_type: "event".to_string(),
        event: "status_change".to_string(),
        seq: None,
        channel_id: None,
        data: serde_json::json!({
            "user_id": snapshot.user_id,
            "status": snapshot.status,
            "manual": snapshot.manual,
            "last_activity_at": snapshot.last_activity_at,
            "text": snapshot.text,
            "emoji": snapshot.emoji,
            "expires_at": snapshot.expires_at,
        }),
        broadcast: None,
    };

    state.ws_hub.broadcast(event).await;
    Ok(())
}

fn resolve_status_target_user_id(user_id: &str, auth: &MmAuthUser) -> ApiResult<Uuid> {
    if user_id == "me" {
        return Ok(auth.user_id);
    }

    parse_mm_or_uuid(user_id).ok_or_else(|| AppError::BadRequest("Invalid user ID".to_string()))
}
