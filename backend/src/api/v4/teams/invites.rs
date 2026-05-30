use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::utils::is_valid_email;
use super::{ensure_team_admin_or_system_manage, ensure_team_member, map_team_member};
use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{
    id::{encode_mm_id, parse_mm_or_uuid},
    models as mm,
};
use crate::models::TeamMember;
use crate::repositories::TeamRepository;

#[derive(Deserialize)]
pub struct AddTeamMemberByInviteQuery {
    token: Option<String>,
    invite_id: Option<String>,
}

pub async fn add_team_member_by_invite(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Query(query): Query<AddTeamMemberByInviteQuery>,
) -> ApiResult<(axum::http::StatusCode, Json<mm::TeamMember>)> {
    let token = query
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let invite_id = query
        .invite_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let repo = TeamRepository::new(&state.db);

    let team_id = if let Some(token_value) = token {
        let mut tx = state.db.begin().await?;
        let token_row = repo
            .get_team_invite_token_for_update(token_value, &mut tx)
            .await?
            .ok_or_else(|| AppError::BadRequest("Invalid invitation token".to_string()))?;

        if token_row.used_at.is_some() || token_row.expires_at <= chrono::Utc::now() {
            return Err(AppError::BadRequest(
                "Invalid or expired invitation token".to_string(),
            ));
        }

        let team = repo
            .get_team_by_id(token_row.team_id)
            .await?
            .ok_or_else(|| AppError::TeamNotFound)?;

        repo.upsert_team_member_in_tx(team.id, auth.user_id, "member", &mut tx)
            .await?;

        repo.mark_invite_token_used(token_value, &mut tx).await?;

        tx.commit().await?;
        team.id
    } else if let Some(invite_value) = invite_id {
        if auth.has_role("guest") {
            return Err(AppError::Forbidden(
                "Guests cannot join teams through invite_id".to_string(),
            ));
        }

        let team = repo
            .get_team_by_invite_id(invite_value)
            .await?
            .ok_or_else(|| AppError::TeamNotFound)?;

        if !team.allow_open_invite {
            return Err(AppError::Forbidden(
                "This team does not allow open joining".to_string(),
            ));
        }

        repo.upsert_team_member(team.id, auth.user_id, "member")
            .await?;

        team.id
    } else {
        return Err(AppError::BadRequest(
            "Missing invitation token or invite_id".to_string(),
        ));
    };

    if let Err(err) =
        crate::services::team_membership::apply_default_channel_membership_for_team_join(
            &state,
            team_id,
            auth.user_id,
        )
        .await
    {
        tracing::warn!(
            team_id = %team_id,
            user_id = %auth.user_id,
            error = %err,
            "Default channel auto-join failed after v4 add_team_member_by_invite"
        );
    }

    Ok((
        axum::http::StatusCode::CREATED,
        Json(map_team_member(TeamMember {
            team_id,
            user_id: auth.user_id,
            role: "member".to_string(),
            created_at: chrono::Utc::now(),
            presence: "offline".to_string(),
        })),
    ))
}

#[derive(Deserialize)]
pub struct InviteUsersRequest {
    user_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct TeamInviteResponse {
    user_id: String,
    team_id: String,
    token: String,
    expires_at: i64,
}

pub fn generate_invite_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect()
}

pub async fn invite_users_to_team(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    Json(input): Json<InviteUsersRequest>,
) -> ApiResult<Json<Vec<TeamInviteResponse>>> {
    let team_id = parse_mm_or_uuid(&team_id).ok_or_else(|| AppError::InvalidTeamId)?;

    ensure_team_member(&state, team_id, auth.user_id).await?;

    let repo = TeamRepository::new(&state.db);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
    let mut responses = Vec::new();

    for user_id_str in &input.user_ids {
        let user_id = match Uuid::parse_str(user_id_str) {
            Ok(id) => id,
            Err(_) => continue,
        };

        if !repo.user_exists(user_id).await? {
            continue;
        }

        if repo.is_team_member(team_id, user_id).await? {
            continue;
        }

        let existing = repo
            .get_active_team_invitation_by_user(team_id, user_id)
            .await?;

        let (token, inv_expires_at) = if let Some((t, ea)) = existing {
            (t, ea)
        } else {
            let token = generate_invite_token();
            repo.upsert_team_invitation_for_user(
                team_id,
                user_id,
                auth.user_id,
                &token,
                expires_at,
            )
            .await?;
            (token, expires_at)
        };

        responses.push(TeamInviteResponse {
            user_id: encode_mm_id(user_id),
            team_id: encode_mm_id(team_id),
            token,
            expires_at: inv_expires_at.timestamp(),
        });
    }

    Ok(Json(responses))
}

#[derive(Deserialize)]
pub struct InviteGuestsRequest {
    emails: Vec<String>,
    #[serde(default)]
    channels: Vec<String>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Serialize)]
pub struct EmailInviteResponse {
    email: String,
    token: String,
    expires_at: i64,
}

pub async fn invite_guests_to_team(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
    Json(input): Json<InviteGuestsRequest>,
) -> ApiResult<Json<Vec<EmailInviteResponse>>> {
    let team_id = parse_mm_or_uuid(&team_id).ok_or_else(|| AppError::InvalidTeamId)?;

    ensure_team_admin_or_system_manage(&state, team_id, &auth).await?;

    // Note: channel restrictions and custom email message are not yet implemented.
    // The channels list and custom message from input are currently ignored.
    let _ = &input.channels;
    let _ = &input.message;

    let repo = TeamRepository::new(&state.db);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
    let mut responses = Vec::new();

    for email in &input.emails {
        if !email.contains('@') {
            continue;
        }

        let existing = repo
            .get_active_team_invitation_by_email(team_id, email)
            .await?;

        let (token, inv_expires_at) = if let Some((t, ea)) = existing {
            (t, ea)
        } else {
            let token = generate_invite_token();
            repo.upsert_team_invitation_for_email(
                team_id,
                auth.user_id,
                email,
                &token,
                "guest",
                expires_at,
            )
            .await?;
            (token, expires_at)
        };

        responses.push(EmailInviteResponse {
            email: email.clone(),
            token,
            expires_at: inv_expires_at.timestamp(),
        });
    }

    Ok(Json(responses))
}

#[derive(Deserialize)]
pub struct InviteByEmailRequest {
    team_id: String,
    emails: Vec<String>,
}

pub async fn invite_users_to_team_by_email(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<InviteByEmailRequest>,
) -> ApiResult<Json<Vec<EmailInviteResponse>>> {
    let team_id = parse_mm_or_uuid(&input.team_id).ok_or_else(|| AppError::InvalidTeamId)?;

    ensure_team_member(&state, team_id, auth.user_id).await?;

    let repo = TeamRepository::new(&state.db);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
    let mut responses = Vec::new();

    for email in &input.emails {
        // Validate email format
        if !is_valid_email(email) {
            continue;
        }

        let existing = repo
            .get_active_team_invitation_by_email(team_id, email)
            .await?;

        let (token, inv_expires_at) = if let Some((t, ea)) = existing {
            (t, ea)
        } else {
            let token = generate_invite_token();
            repo.upsert_team_invitation_for_email(
                team_id,
                auth.user_id,
                email,
                &token,
                "member",
                expires_at,
            )
            .await?;
            (token, expires_at)
        };

        responses.push(EmailInviteResponse {
            email: email.clone(),
            token,
            expires_at: inv_expires_at.timestamp(),
        });
    }

    Ok(Json(responses))
}

pub async fn get_team_by_invite(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Path(invite_id): Path<String>,
) -> ApiResult<Json<mm::Team>> {
    let repo = TeamRepository::new(&state.db);
    let team = repo
        .get_team_by_invite_id(&invite_id)
        .await?
        .ok_or_else(|| AppError::TeamNotFound)?;
    Ok(Json(team.into()))
}

pub async fn regenerate_team_invite_id(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(team_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let team_id = parse_mm_or_uuid(&team_id).ok_or_else(|| AppError::InvalidTeamId)?;
    ensure_team_admin_or_system_manage(&state, team_id, &auth).await?;

    let repo = TeamRepository::new(&state.db);
    let invite_id = repo
        .regenerate_team_invite_id(team_id)
        .await?
        .ok_or_else(|| AppError::TeamNotFound)?;

    Ok(Json(serde_json::json!({"invite_id": invite_id})))
}
