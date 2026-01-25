use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
    http::{HeaderMap, HeaderValue},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::AppState;
use crate::auth::{create_token, verify_password};
use crate::error::{ApiResult, AppError};
use crate::models::{User, Team, TeamMember, channel::Channel, channel::ChannelMember};
use crate::mattermost_compat::models as mm;
use super::extractors::MmAuthUser;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users/login", post(login))
        .route("/users/me", get(me))
        .route("/users/me/teams", get(my_teams))
        .route("/users/me/teams/members", get(my_team_members))
        .route("/users/me/teams/:team_id/channels", get(my_team_channels))
        .route("/users/me/teams/:team_id/channels/members", get(my_team_channel_members))
        .route("/users/me/teams/unread", get(my_teams_unread))
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
        "SELECT * FROM users WHERE (email = $1 OR username = $1) AND is_active = true"
    )
    .bind(&input.login_id)
    .fetch_optional(&state.db)
    .await?;

    let user = user.ok_or_else(|| AppError::Unauthorized("Invalid login credentials".to_string()))?;

    if !verify_password(&input.password, &user.password_hash)? {
        return Err(AppError::Unauthorized("Invalid login credentials".to_string()));
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

async fn me(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<mm::User>> {
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
        "#
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
    let members: Vec<TeamMember> = sqlx::query_as(
        "SELECT * FROM team_members WHERE user_id = $1"
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mm_members = members.into_iter().map(|m| {
        mm::TeamMember {
            team_id: m.team_id.to_string(),
            user_id: m.user_id.to_string(),
            roles: "team_user".to_string(),
            delete_at: 0,
            scheme_guest: false,
            scheme_user: true,
            scheme_admin: false,
        }
    }).collect();

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
        "#
    )
    .bind(team_id)
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
        "#
    )
    .bind(team_id)
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let mm_members = members.into_iter().map(|m| {
        mm::ChannelMember {
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
        }
    }).collect();

    Ok(Json(mm_members))
}

async fn my_teams_unread(
    State(_state): State<AppState>,
    _auth: MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}
