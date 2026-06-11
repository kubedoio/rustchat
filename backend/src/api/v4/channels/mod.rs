use axum::{
    routing::{get, post, put},
    Router,
};

use super::extractors::MmAuthUser;
use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{
    id::{encode_mm_id, parse_mm_or_uuid},
    models as mm,
};
use crate::models::Channel;

mod compat;
mod crud;
mod direct;
mod helpers;
mod members;
mod posts;
mod search;
mod stats;
mod unread;
mod utils;
mod view;

use compat::{
    get_channel_access_control_attributes, get_channel_common_teams, get_channel_groups,
    get_channel_member_counts_by_group, get_channel_members_minus_group_members,
    get_channel_moderations, patch_channel_moderations, search_group_channels,
    update_channel_scheme,
};
use view::{view_channel, view_channel_for_user};

pub use direct::{create_direct_channel_internal, create_group_channel_internal};

pub fn router(state: AppState) -> Router<AppState> {
    let search_routes = Router::new()
        .route("/channels/search", post(search::search_channels_compat))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::rate_limit::search_ip_rate_limit,
        ));

    Router::new()
        .merge(search_routes)
        .route("/channels/{channel_id}/posts", get(posts::get_posts))
        .route(
            "/channels/{channel_id}",
            get(crud::get_channel)
                .put(crud::update_channel)
                .delete(crud::delete_channel),
        )
        .route("/channels/{channel_id}/patch", put(crud::patch_channel))
        .route(
            "/channels/{channel_id}/privacy",
            put(crud::update_channel_privacy),
        )
        .route(
            "/channels/{channel_id}/restore",
            post(crud::restore_channel),
        )
        .route("/channels/{channel_id}/move", post(crud::move_channel))
        .route(
            "/channels/{channel_id}/members",
            get(members::get_channel_members).post(members::add_channel_member),
        )
        .route(
            "/channels/{channel_id}/members/me",
            get(members::get_channel_member_me).delete(members::remove_channel_member_me),
        )
        .route(
            "/channels/{channel_id}/members/ids",
            post(members::get_channel_members_by_ids),
        )
        .route(
            "/channels/{channel_id}/members/{user_id}",
            get(members::get_channel_member_by_id).delete(members::remove_channel_member),
        )
        .route(
            "/channels/{channel_id}/members/{user_id}/roles",
            put(members::update_channel_member_roles),
        )
        .route(
            "/channels/{channel_id}/members/{user_id}/schemeRoles",
            put(members::update_channel_member_scheme_roles),
        )
        .route(
            "/channels/{channel_id}/members/{user_id}/notify_props",
            put(members::update_channel_member_notify_props),
        )
        // Mark as Read / Mark as Unread endpoints
        .route(
            "/channels/{channel_id}/members/{user_id}/read",
            post(unread::mark_channel_as_read),
        )
        .route(
            "/channels/{channel_id}/members/{user_id}/set_unread",
            post(unread::mark_channel_as_unread),
        )
        .route(
            "/channels/{channel_id}/timezones",
            get(stats::get_channel_timezones),
        )
        .route(
            "/channels/{channel_id}/stats",
            get(stats::get_channel_stats),
        )
        .route(
            "/channels/{channel_id}/unread",
            get(unread::get_channel_unread),
        )
        .route(
            "/channels/{channel_id}/pinned",
            get(posts::get_pinned_posts),
        )
        .route(
            "/channels/{channel_id}/posts/{post_id}/pin",
            post(posts::pin_post),
        )
        .route(
            "/channels/{channel_id}/posts/{post_id}/unpin",
            post(posts::unpin_post),
        )
        .route("/channels/members/me/view", post(view_channel))
        .route(
            "/channels/members/{user_id}/view",
            post(view_channel_for_user),
        )
        .route("/channels/direct", post(direct::create_direct_channel))
        .route("/channels/group", post(direct::create_group_channel))
        .route(
            "/channels",
            get(crud::get_all_channels).post(crud::create_channel),
        )
        .route("/channels/group/search", post(search_group_channels))
        .route("/channels/{channel_id}/scheme", put(update_channel_scheme))
        .route(
            "/channels/{channel_id}/members_minus_group_members",
            get(get_channel_members_minus_group_members),
        )
        .route(
            "/channels/{channel_id}/member_counts_by_group",
            get(get_channel_member_counts_by_group),
        )
        .route(
            "/channels/{channel_id}/moderations",
            get(get_channel_moderations),
        )
        .route(
            "/channels/{channel_id}/moderations/patch",
            put(patch_channel_moderations),
        )
        .route(
            "/channels/{channel_id}/common_teams",
            get(get_channel_common_teams),
        )
        .route("/channels/{channel_id}/groups", get(get_channel_groups))
        .route(
            "/channels/{channel_id}/access_control/attributes",
            get(get_channel_access_control_attributes),
        )
}
