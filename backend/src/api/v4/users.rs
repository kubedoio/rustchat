use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::auth::{create_token, verify_password};
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::models as mm;
use crate::models::{channel::Channel, channel::ChannelMember, Team, TeamMember, User};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users/login", post(login))
        .route("/users/me", get(me))
        .route("/users/me/teams", get(my_teams))
        .route("/users/me/teams/members", get(my_team_members))
        .route("/users/me/teams/{team_id}/channels", get(my_team_channels))
        .route("/users/me/channels", get(my_channels))
        .route(
            "/users/me/teams/{team_id}/channels/members",
            get(my_team_channel_members),
        )
        .route("/users/me/teams/unread", get(my_teams_unread))
        .route(
            "/users/sessions/device",
            post(attach_device).delete(detach_device),
        )
        .route(
            "/users/me/preferences",
            get(get_preferences).put(update_preferences),
        )
        .route("/users/status/ids", post(get_statuses_by_ids))
        .route("/users/{user_id}/status", get(get_status))
        .route("/users/me/status", put(update_status))
        .route("/users/{user_id}/channels/{channel_id}/typing", post(user_typing))
}

#[derive(Deserialize)]
struct LoginRequest {
    login_id: String,
    password: String,
    #[allow(dead_code)]
    device_id: Option<String>,
}

async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    let user: Option<User> = sqlx::query_as(
        "SELECT * FROM users WHERE (email = $1 OR username = $1) AND is_active = true",
    )
    .bind(&input.login_id)
    .fetch_optional(&state.db)
    .await?;

    let user =
        user.ok_or_else(|| AppError::Unauthorized("Invalid login credentials".to_string()))?;

    if !verify_password(&input.password, &user.password_hash)? {
        return Err(AppError::Unauthorized(
            "Invalid login credentials".to_string(),
        ));
    }

    // Update last login
    sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await?;

    // Generate token
    let token = create_token(
        user.id,
        &user.email,
        &user.role,
        user.org_id,
        &state.jwt_secret,
        state.jwt_expiry_hours,
    )?;

    let mm_user: mm::User = user.into();

    let mut headers = HeaderMap::new();
    headers.insert("Token", HeaderValue::from_str(&token).unwrap());

    Ok((headers, Json(mm_user)))
}

async fn me(State(state): State<AppState>, auth: MmAuthUser) -> ApiResult<Json<mm::User>> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(user.into()))
}

async fn my_teams(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Team>>> {
    let teams: Vec<Team> = sqlx::query_as(
        r#"
        SELECT t.* FROM teams t
        JOIN team_members tm ON t.id = tm.team_id
        WHERE tm.user_id = $1
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mm_teams: Vec<mm::Team> = teams.into_iter().map(|t| t.into()).collect();
    Ok(Json(mm_teams))
}

async fn my_team_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::TeamMember>>> {
    let members: Vec<TeamMember> = sqlx::query_as("SELECT * FROM team_members WHERE user_id = $1")
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await?;

    let mm_members = members
        .into_iter()
        .map(|m| mm::TeamMember {
            team_id: m.team_id.to_string(),
            user_id: m.user_id.to_string(),
            roles: "team_user".to_string(),
            delete_at: 0,
            scheme_guest: false,
            scheme_user: true,
            scheme_admin: false,
        })
        .collect();

    Ok(Json(mm_members))
}

async fn my_team_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<Uuid>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let channels: Vec<Channel> = sqlx::query_as(
        r#"
        SELECT c.* FROM channels c
        JOIN channel_members cm ON c.id = cm.channel_id
        WHERE c.team_id = $1 AND cm.user_id = $2
        "#,
    )
    .bind(team_id)
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mm_channels: Vec<mm::Channel> = channels.into_iter().map(|c| c.into()).collect();
    Ok(Json(mm_channels))
}

async fn my_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let channels: Vec<Channel> = sqlx::query_as(
        r#"
        SELECT c.* FROM channels c
        JOIN channel_members cm ON c.id = cm.channel_id
        WHERE cm.user_id = $1
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mm_channels: Vec<mm::Channel> = channels.into_iter().map(|c| c.into()).collect();
    Ok(Json(mm_channels))
}

async fn my_team_channel_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<Uuid>,
) -> ApiResult<Json<Vec<mm::ChannelMember>>> {
    let members: Vec<ChannelMember> = sqlx::query_as(
        r#"
        SELECT cm.*, c.name as username, c.display_name, NULL as avatar_url, NULL as presence
        FROM channel_members cm
        JOIN channels c ON cm.channel_id = c.id
        WHERE c.team_id = $1 AND cm.user_id = $2
        "#,
    )
    .bind(team_id)
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mm_members = members
        .into_iter()
        .map(|m| mm::ChannelMember {
            channel_id: m.channel_id.to_string(),
            user_id: m.user_id.to_string(),
            roles: "channel_user".to_string(),
            last_viewed_at: m.last_viewed_at.map(|t| t.timestamp_millis()).unwrap_or(0),
            msg_count: 0,
            mention_count: 0,
            notify_props: m.notify_props,
            last_update_at: 0,
            scheme_guest: false,
            scheme_user: true,
            scheme_admin: false,
        })
        .collect();

    Ok(Json(mm_members))
}

async fn my_teams_unread(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

#[derive(Deserialize)]
struct AttachDeviceRequest {
    device_id: String,
    #[allow(dead_code)]
    token: String,
    #[allow(dead_code)]
    platform: Option<String>,
}

async fn attach_device(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<AttachDeviceRequest>,
) -> ApiResult<impl IntoResponse> {
    sqlx::query(
        r#"
        INSERT INTO user_devices (user_id, device_id, token, platform)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id, device_id)
        DO UPDATE SET token = $3, platform = $4, last_seen_at = NOW()
        "#,
    )
    .bind(auth.user_id)
    .bind(input.device_id)
    .bind(input.token)
    .bind(input.platform.unwrap_or_else(|| "unknown".to_string()))
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

#[derive(Deserialize)]
struct DetachDeviceRequest {
    device_id: String,
}

async fn detach_device(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<DetachDeviceRequest>,
) -> ApiResult<impl IntoResponse> {
    sqlx::query("DELETE FROM user_devices WHERE user_id = $1 AND device_id = $2")
        .bind(auth.user_id)
        .bind(input.device_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn get_preferences(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Preference>>> {
    let rows = sqlx::query("SELECT user_id, category, name, value FROM mattermost_preferences WHERE user_id = $1")
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let mut prefs = Vec::new();
    for row in rows {
        use sqlx::Row;
        let uid: Uuid = row.try_get("user_id").unwrap_or_default();
        prefs.push(mm::Preference {
            user_id: uid.to_string(),
            category: row.try_get("category").unwrap_or_default(),
            name: row.try_get("name").unwrap_or_default(),
            value: row.try_get("value").unwrap_or_default(),
        });
    }

    Ok(Json(prefs))
}

async fn update_preferences(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(prefs): Json<Vec<mm::Preference>>,
) -> ApiResult<impl IntoResponse> {
    let mut tx = state.db.begin().await?;

    for p in prefs {
        sqlx::query(
            r#"
            INSERT INTO mattermost_preferences (user_id, category, name, value)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, category, name)
            DO UPDATE SET value = $4
            "#,
        )
        .bind(auth.user_id)
        .bind(p.category)
        .bind(p.name)
        .bind(p.value)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn get_statuses_by_ids(
    State(state): State<AppState>,
    Json(ids): Json<Vec<String>>,
) -> ApiResult<Json<Vec<mm::Status>>> {
    let uuids: Vec<Uuid> = ids.iter().filter_map(|id| Uuid::parse_str(id).ok()).collect();

    if uuids.is_empty() {
        return Ok(Json(vec![]));
    }

    let users: Vec<(Uuid, String, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, presence, last_login_at FROM users WHERE id = ANY($1)",
    )
    .bind(&uuids)
    .fetch_all(&state.db)
    .await?;

    let statuses = users.into_iter().map(|(id, presence, last_login)| {
        mm::Status {
            user_id: id.to_string(),
            status: if presence.is_empty() { "offline".to_string() } else { presence },
            manual: false,
            last_activity_at: last_login.map(|t| t.timestamp_millis()).unwrap_or(0),
        }
    }).collect();

    Ok(Json(statuses))
}

async fn get_status(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Json<mm::Status>> {
    let (presence, last_login): (String, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT presence, last_login_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(mm::Status {
        user_id: user_id.to_string(),
        status: if presence.is_empty() { "offline".to_string() } else { presence },
        manual: false,
        last_activity_at: last_login.map(|t| t.timestamp_millis()).unwrap_or(0),
    }))
}

#[derive(Deserialize)]
struct UpdateStatusRequest {
    user_id: String,
    status: String,
}

async fn update_status(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<UpdateStatusRequest>,
) -> ApiResult<Json<mm::Status>> {
    if input.user_id != auth.user_id.to_string() {
         return Err(AppError::Forbidden("Cannot update other user's status".to_string()));
    }

    sqlx::query("UPDATE users SET presence = $1 WHERE id = $2")
        .bind(&input.status)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    let status = mm::Status {
        user_id: input.user_id.clone(),
        status: input.status.clone(),
        manual: true,
        last_activity_at: Utc::now().timestamp_millis(),
    };

    // Broadcast status change
    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::UserUpdated, // Mapping to status_change in WS handler
        serde_json::json!({
             "user_id": auth.user_id,
             "status": input.status,
             "manual": true,
             "last_activity_at": status.last_activity_at
        }),
        None,
    );
    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(status))
}

async fn user_typing(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((user_id, channel_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    if user_id != auth.user_id {
         return Err(AppError::Forbidden("Mismatch user_id".to_string()));
    }

    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::UserTyping,
        crate::realtime::TypingEvent {
            user_id: auth.user_id,
            display_name: "".to_string(), // Fetched by client usually
            thread_root_id: None,
        },
        Some(channel_id),
    ).with_broadcast(crate::realtime::WsBroadcast {
        channel_id: Some(channel_id),
        team_id: None,
        user_id: None,
        exclude_user_id: Some(auth.user_id),
    });

    state.ws_hub.broadcast(broadcast).await;

    Ok(Json(serde_json::json!({"status": "OK"})))
}
