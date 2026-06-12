use axum::{
    routing::{get, post, put},
    Router,
};
use uuid::Uuid;

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::encode_mm_id, models as mm};
use crate::models::TeamMember;

// Submodules
mod batch;
mod channels;
mod commands;
mod crud;
mod groups;
mod icon;
mod import;
mod invites;
mod members;
mod scheme;
mod search;
mod stats;
mod utils;

// Re-export handler functions
pub use batch::add_team_members_batch;
pub use channels::{
    autocomplete_team_channels, get_team_channel_by_name, get_team_channel_by_name_for_team_name,
    get_team_channel_ids, get_team_channels, get_team_deleted_channels, get_team_private_channels,
    search_autocomplete_team_channels,
};
pub use commands::autocomplete_team_commands;
pub use crud::{
    get_team, get_team_by_name, get_teams, patch_team, restore_team, team_name_exists,
    update_team_privacy,
};
pub use groups::{get_team_groups, get_team_groups_by_channels, team_group_json_from_parts};
pub use icon::{delete_team_icon, get_team_image, update_team_icon};
pub use import::import_team;
pub use invites::{
    add_team_member_by_invite, generate_invite_token, get_team_by_invite, invite_guests_to_team,
    invite_users_to_team, invite_users_to_team_by_email, regenerate_team_invite_id,
};
pub use members::{
    add_team_member, get_team_member, get_team_member_ids, get_team_member_me, get_team_members,
    remove_team_member, update_team_member_roles, update_team_member_scheme_roles,
};
pub use scheme::{get_team_scheme, update_team_scheme};
pub use search::{search_channels, search_teams};
pub use stats::{get_team_members_minus_group_members, get_team_stats};
pub use utils::is_valid_email;

// Internal helpers shared across submodules

pub async fn ensure_team_member(state: &AppState, team_id: Uuid, user_id: Uuid) -> ApiResult<()> {
    let is_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2)",
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;
    if !is_member {
        return Err(AppError::NotOnTeam);
    }
    Ok(())
}

/// Ensure the user is a team admin or has system manage permission.
pub async fn ensure_team_admin_or_system_manage(
    state: &AppState,
    team_id: Uuid,
    auth: &MmAuthUser,
) -> ApiResult<()> {
    // System admins can manage any team
    if auth.has_permission(&permissions::SYSTEM_MANAGE) {
        return Ok(());
    }
    // Check if user is a team admin
    let role: Option<String> =
        sqlx::query_scalar("SELECT role FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?;
    match role.as_deref() {
        Some("admin") | Some("team_admin") => Ok(()),
        _ => Err(AppError::Forbidden(
            "Team admin privileges required".to_string(),
        )),
    }
}

pub fn map_team_member(member: TeamMember) -> mm::TeamMember {
    mm::TeamMember {
        team_id: encode_mm_id(member.team_id),
        user_id: encode_mm_id(member.user_id),
        roles: crate::mattermost_compat::mappers::map_team_role(&member.role),
        delete_at: 0,
        scheme_guest: member.role == "guest",
        scheme_user: member.role != "guest"
            && member.role != "admin"
            && member.role != "team_admin",
        scheme_admin: member.role == "admin" || member.role == "team_admin",
        presence: Some(member.presence).filter(|p| !p.is_empty()),
    }
}

// Re-export search_team_channels for use in channels module
use crate::mattermost_compat::models as mm_models;
use crate::models::Channel;

pub async fn search_team_channels(
    state: &AppState,
    user_id: Uuid,
    team_id: &str,
    term: &str,
    limit: i64,
) -> ApiResult<Vec<mm_models::Channel>> {
    use crate::mattermost_compat::id::parse_mm_or_uuid;

    let team_id = parse_mm_or_uuid(team_id).ok_or_else(|| AppError::InvalidTeamId)?;
    ensure_team_member(state, team_id, user_id).await?;
    let search_term = format!("%{}%", term.to_lowercase());
    let channels: Vec<Channel> = sqlx::query_as(
        r#"
        SELECT DISTINCT c.* FROM channels c
        LEFT JOIN channel_members cm ON c.id = cm.channel_id AND cm.user_id = $2
        WHERE c.team_id = $1
          AND (LOWER(c.name) LIKE $3 OR LOWER(c.display_name) LIKE $3)
          AND (c.type = 'public' OR cm.user_id IS NOT NULL)
        ORDER BY COALESCE(c.display_name, c.name) ASC
        LIMIT $4
        "#,
    )
    .bind(team_id)
    .bind(user_id)
    .bind(&search_term)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;
    Ok(channels.into_iter().map(|c| c.into()).collect())
}

pub fn router(state: AppState) -> Router<AppState> {
    let search_routes = Router::new()
        .route(
            "/teams/{team_id}/channels/search",
            post(search::search_channels),
        )
        .route("/teams/search", post(search::search_teams))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::rate_limit::search_ip_rate_limit,
        ));

    Router::new()
        .merge(search_routes)
        .route("/teams", get(crud::get_teams))
        .route("/teams/{team_id}", get(crud::get_team))
        .route("/teams/{team_id}/patch", put(crud::patch_team))
        .route("/teams/{team_id}/privacy", put(crud::update_team_privacy))
        .route("/teams/{team_id}/restore", post(crud::restore_team))
        .route("/teams/name/{name}", get(crud::get_team_by_name))
        .route("/teams/name/{name}/exists", get(crud::team_name_exists))
        .route(
            "/teams/{team_id}/members",
            get(members::get_team_members).post(members::add_team_member),
        )
        .route(
            "/teams/members/invite",
            post(invites::add_team_member_by_invite),
        )
        .route(
            "/teams/{team_id}/members/batch",
            post(batch::add_team_members_batch),
        )
        .route(
            "/teams/{team_id}/members/{user_id}",
            get(members::get_team_member).delete(members::remove_team_member),
        )
        .route(
            "/teams/{team_id}/members/ids",
            get(members::get_team_member_ids),
        )
        .route("/teams/{team_id}/stats", get(stats::get_team_stats))
        .route(
            "/teams/{team_id}/regenerate_invite_id",
            post(invites::regenerate_team_invite_id),
        )
        .route(
            "/teams/{team_id}/members/{user_id}/roles",
            put(members::update_team_member_roles),
        )
        .route(
            "/teams/{team_id}/members/{user_id}/schemeRoles",
            put(members::update_team_member_scheme_roles),
        )
        .route(
            "/teams/{team_id}/image",
            get(icon::get_team_image)
                .post(icon::update_team_icon)
                .delete(icon::delete_team_icon),
        )
        .route(
            "/teams/{team_id}/members/me",
            get(members::get_team_member_me),
        )
        .route(
            "/teams/{team_id}/invite",
            post(invites::invite_users_to_team),
        )
        .route(
            "/teams/{team_id}/invite-guests/email",
            post(invites::invite_guests_to_team),
        )
        .route(
            "/teams/invites/email",
            post(invites::invite_users_to_team_by_email),
        )
        .route("/teams/{team_id}/import", post(import::import_team))
        .route(
            "/teams/invite/{invite_id}",
            get(invites::get_team_by_invite),
        )
        .route(
            "/teams/{team_id}/scheme",
            get(scheme::get_team_scheme).put(scheme::update_team_scheme),
        )
        .route(
            "/teams/{team_id}/members_minus_group_members",
            get(stats::get_team_members_minus_group_members),
        )
        .route(
            "/teams/{team_id}/channels",
            get(channels::get_team_channels),
        )
        .route(
            "/teams/{team_id}/channels/ids",
            get(channels::get_team_channel_ids),
        )
        .route(
            "/teams/{team_id}/channels/private",
            get(channels::get_team_private_channels),
        )
        .route(
            "/teams/{team_id}/channels/deleted",
            get(channels::get_team_deleted_channels),
        )
        .route(
            "/teams/{team_id}/channels/autocomplete",
            get(channels::autocomplete_team_channels),
        )
        .route(
            "/teams/{team_id}/channels/search_autocomplete",
            get(channels::search_autocomplete_team_channels),
        )
        .route(
            "/teams/{team_id}/channels/name/{channel_name}",
            get(channels::get_team_channel_by_name),
        )
        .route(
            "/teams/name/{team_name}/channels/name/{channel_name}",
            get(channels::get_team_channel_by_name_for_team_name),
        )
        .route(
            "/teams/{team_id}/commands/autocomplete",
            get(commands::autocomplete_team_commands),
        )
        .route("/teams/{team_id}/groups", get(groups::get_team_groups))
        .route(
            "/teams/{team_id}/groups_by_channels",
            get(groups::get_team_groups_by_channels),
        )
}
