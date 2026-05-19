use axum::{
    body::Bytes,
    http::HeaderMap,
    middleware,
    routing::{get, post, put},
    Router,
};
use serde::de::DeserializeOwned;

use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::middleware::rate_limit;

// Existing submodules (kept as is)
pub mod user_activity;
pub mod user_preferences;
pub mod user_sidebar_categories;

// New submodules
mod admin;
mod auth;
mod channel_members;
mod channels_ext;
mod management;
mod media;
mod members;
mod notifications;
mod profile;
mod search;
mod sessions;
mod stats;
mod tokens;
mod unreads;
mod user_channels;
mod user_teams;
mod utils;

// Re-exports from preferences (for backward compatibility)
pub use user_preferences::{
    delete_preferences_for_user, get_my_preferences_by_category,
    get_preference_by_category_and_name, get_preferences, get_preferences_by_category,
    get_preferences_for_user, update_preferences, update_preferences_for_user,
};

// Re-exports from sidebar_categories (for backward compatibility)
pub use user_sidebar_categories::{
    create_category, get_categories, get_my_categories, update_categories, update_category_order,
};
pub(crate) use user_sidebar_categories::{
    create_category_internal, get_categories_internal, get_category_order_internal,
    resolve_user_id, update_categories_internal, update_category_order_internal,
    CreateCategoryRequest, UpdateCategoriesPayload,
};

// Re-exports from user_activity
pub use user_activity::routes as activity_routes;

// Re-exports from auth module
pub use auth::{
    enforce_password_login_allowed, login, login_cws, login_sso_code_exchange, login_switch,
    login_type, parse_login_request, LoginCwsRequest, LoginRequest, LoginSsoCodeExchangeRequest,
    LoginSwitchRequest, LoginTypeRequest,
};

// Re-exports from profile module
pub use profile::{get_user_by_id, get_user_by_username, me, patch_me, patch_user, PatchMeRequest};

// Re-exports from user_teams module
pub use user_teams::{default_team, fetch_user_teams, get_teams_for_user, my_teams};

// Re-exports from members module
pub use members::{get_team_members_for_user, my_team_members};

// Re-exports from user_channels module
pub use user_channels::{
    get_channels_for_user, get_team_channels_for_user, hydrate_dm_display_names_batch,
    my_channels, my_team_channels, my_team_channels_not_members, resolve_team_id, MyChannelsQuery,
    MyTeamChannelsQuery, NotMembersQuery,
};

// Re-exports from channel_members module
pub use channel_members::my_team_channel_members;

// Re-exports from unreads module
pub use unreads::{
    compute_team_unreads, get_user_team_unread, get_user_teams_unread, my_teams_unread,
    parse_optional_team_query, TeamUnreadQuery, TeamsUnreadQuery,
};

// Re-exports from sessions module
pub use sessions::{
    attach_device, detach_device, get_sessions, get_user_sessions, logout, parse_mobile_device_id,
    resolve_device_token, revoke_all_sessions, revoke_user_session, revoke_user_sessions,
    AttachDeviceRequest, DetachDeviceRequest,
};

// Re-exports from notifications module
pub use notifications::{get_notifications, update_notifications};

// Re-exports from utils module
pub use utils::{
    normalize_notify_props, parse_timezone_for_update, UsersByIdsRequest, UsersByUsernamesRequest,
};

// Re-exports from search module
pub use search::{
    autocomplete_users, get_known_users, get_user_by_email, get_users_by_ids,
    get_users_by_usernames, list_users, search_users, AutocompleteQuery, UserSearchRequest,
    UsersQuery,
};

// Re-exports from media module
pub use media::{get_user_image, get_user_image_default, upload_user_image};

// Re-exports from management module
pub use management::{
    check_user_mfa, convert_user_to_bot, demote_user, generate_mfa_secret, get_user_audits,
    promote_user, reset_password, send_email_verification, send_password_reset, update_user_active,
    update_user_mfa, update_user_password, update_user_roles, verify_email, verify_member_email,
    CheckMfaRequest, SendVerificationRequest, UpdateMfaRequest, UpdatePasswordRequest,
    UserActiveRequest, UserRolesRequest, VerifyEmailRequest,
};

// Re-exports from tokens module
pub use tokens::{
    disable_token, enable_token, get_token, get_tokens, get_user_tokens, revoke_token,
    search_tokens,
};

// Re-exports from channels_ext module
pub use channels_ext::{
    get_profiles_in_group_channels, get_user_channel_members, get_user_group_channels,
    get_user_uploads, set_user_typing, user_typing,
};

// Re-exports from stats module
pub use stats::{get_invalid_emails, get_user_stats, get_user_stats_filtered};

// Re-exports from admin module
pub use admin::{
    accept_terms_of_service, get_authorized_oauth_apps, get_user_groups, migrate_auth_ldap,
    migrate_auth_saml, reset_failed_attempts, update_user_auth, GroupAssociationQuery,
};

// Re-export status module from parent
pub use super::status;

pub fn router(state: AppState) -> Router<AppState> {
    let login_routes =
        Router::new()
            .route("/users/login", post(login))
            .layer(middleware::from_fn_with_state(
                state,
                rate_limit::auth_ip_rate_limit,
            ));

    Router::new()
        .merge(login_routes)
        .route("/users/login/type", post(login_type))
        .route("/users/login/cws", post(login_cws))
        .route(
            "/users/login/sso/code-exchange",
            post(login_sso_code_exchange),
        )
        .route("/users/login/switch", post(login_switch))
        .route("/users/me", get(me))
        .route("/users/me/teams", get(my_teams))
        .route("/users/me/teams/members", get(my_team_members))
        .route("/users/me/channels/categories", get(get_my_categories))
        .route("/users/me/teams/{team_id}/channels", get(my_team_channels))
        .route("/users/me/channels", get(my_channels))
        .route("/users/{user_id}/teams", get(get_teams_for_user))
        .route(
            "/users/{user_id}/teams/members",
            get(get_team_members_for_user),
        )
        .route(
            "/users/{user_id}/teams/{team_id}/channels",
            get(get_team_channels_for_user),
        )
        .route("/users/{user_id}/channels", get(get_channels_for_user))
        .route(
            "/users/me/teams/{team_id}/channels/not_members",
            get(my_team_channels_not_members),
        )
        .route("/users", get(list_users))
        .route("/users/{user_id}", get(get_user_by_id))
        .route("/users/username/{username}", get(get_user_by_username))
        .route(
            "/users/me/teams/{team_id}/channels/members",
            get(my_team_channel_members),
        )
        .route("/users/me/teams/unread", get(my_teams_unread))
        .route("/users/{user_id}/teams/unread", get(get_user_teams_unread))
        .route(
            "/users/{user_id}/teams/{team_id}/unread",
            get(get_user_team_unread),
        )
        .route(
            "/users/sessions/device",
            post(attach_device).put(attach_device).delete(detach_device),
        )
        .route(
            "/users/me/preferences",
            get(get_preferences).put(update_preferences),
        )
        .route(
            "/users/me/preferences/{category}",
            get(get_my_preferences_by_category),
        )
        .route(
            "/users/{user_id}/preferences",
            get(get_preferences_for_user).put(update_preferences_for_user),
        )
        .route(
            "/users/{user_id}/preferences/delete",
            post(delete_preferences_for_user),
        )
        .route(
            "/users/{user_id}/preferences/{category}",
            get(get_preferences_by_category),
        )
        .route(
            "/users/{user_id}/preferences/{category}/name/{preference_name}",
            get(get_preference_by_category_and_name),
        )
        .route("/users/ids", post(get_users_by_ids))
        .route(
            "/users/{user_id}/channels/{channel_id}/typing",
            post(user_typing),
        )
        .route("/users/me/patch", put(patch_me))
        .route(
            "/users/{user_id}/image",
            get(get_user_image).post(upload_user_image),
        )
        .route(
            "/users/notifications",
            get(get_notifications).put(update_notifications),
        )
        .route("/users/me/sessions", get(get_sessions))
        .route("/users/logout", get(logout).post(logout))
        .route("/users/autocomplete", get(autocomplete_users))
        .route("/users/search", post(search_users))
        .route("/users/known", get(get_known_users))
        .route("/users/stats", get(get_user_stats))
        .route("/users/stats/filtered", post(get_user_stats_filtered))
        .route(
            "/users/group_channels",
            get(get_user_group_channels).post(get_profiles_in_group_channels),
        )
        .route(
            "/users/{user_id}/oauth/apps/authorized",
            get(get_authorized_oauth_apps),
        )
        .route("/users/usernames", post(get_users_by_usernames))
        .route("/users/email/{email}", get(get_user_by_email))
        .route("/users/{user_id}/patch", put(patch_user))
        .route("/users/{user_id}/roles", put(update_user_roles))
        .route("/users/{user_id}/active", put(update_user_active))
        .route(
            "/users/{user_id}/image/default",
            get(get_user_image_default),
        )
        .route("/users/password/reset", post(reset_password))
        .route("/users/password/reset/send", post(send_password_reset))
        .route("/users/mfa", post(check_user_mfa))
        .route("/users/{user_id}/mfa", put(update_user_mfa))
        .route("/users/{user_id}/mfa/generate", post(generate_mfa_secret))
        .route("/users/{user_id}/demote", post(demote_user))
        .route("/users/{user_id}/promote", post(promote_user))
        .route("/users/{user_id}/convert_to_bot", post(convert_user_to_bot))
        .route("/users/{user_id}/password", put(update_user_password))
        .route("/users/{user_id}/sessions", get(get_user_sessions))
        .route(
            "/users/{user_id}/sessions/revoke",
            post(revoke_user_session),
        )
        .route(
            "/users/{user_id}/sessions/revoke/all",
            post(revoke_user_sessions),
        )
        .route("/users/sessions/revoke/all", post(revoke_all_sessions))
        .route("/users/{user_id}/audits", get(get_user_audits))
        .route(
            "/users/{user_id}/email/verify/member",
            post(verify_member_email),
        )
        .route("/users/email/verify", post(verify_email))
        .route("/users/email/verify/send", post(send_email_verification))
        .route("/users/{user_id}/tokens", get(get_user_tokens))
        .route("/users/tokens", get(get_tokens))
        .route("/users/tokens/revoke", post(revoke_token))
        .route("/users/tokens/{token_id}", get(get_token))
        .route("/users/tokens/disable", post(disable_token))
        .route("/users/tokens/enable", post(enable_token))
        .route("/users/tokens/search", post(search_tokens))
        .route("/users/{user_id}/auth", put(update_user_auth))
        .route(
            "/users/{user_id}/terms_of_service",
            post(accept_terms_of_service),
        )
        .route("/users/{user_id}/typing", post(set_user_typing))
        .route("/users/{user_id}/uploads", get(get_user_uploads))
        .route(
            "/users/{user_id}/channel_members",
            get(get_user_channel_members),
        )
        .route("/users/migrate_auth/ldap", post(migrate_auth_ldap))
        .route("/users/migrate_auth/saml", post(migrate_auth_saml))
        .route("/users/invalid_emails", get(get_invalid_emails))
        .route(
            "/users/{user_id}/reset_failed_attempts",
            post(reset_failed_attempts),
        )
        .route(
            "/users/{user_id}/sidebar/categories",
            get(get_categories)
                .post(create_category)
                .put(update_categories),
        )
        .route(
            "/users/{user_id}/sidebar/categories/order",
            put(update_category_order),
        )
        .route("/users/{user_id}/groups", get(get_user_groups))
        .merge(user_activity::routes())
}

/// Parse request body from either JSON or form-urlencoded
pub fn parse_request_body<T: DeserializeOwned>(headers: &HeaderMap, body: &Bytes) -> ApiResult<T> {
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.starts_with("application/json") {
        serde_json::from_slice(body)
            .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))
    } else if content_type.starts_with("application/x-www-form-urlencoded") {
        serde_urlencoded::from_bytes(body)
            .map_err(|_| AppError::BadRequest("Invalid form body".to_string()))
    } else {
        serde_json::from_slice(body)
            .or_else(|_| serde_urlencoded::from_bytes(body))
            .map_err(|_| AppError::BadRequest("Unsupported request body".to_string()))
    }
}

/// Parse body with custom error message
pub fn parse_body<T: DeserializeOwned>(
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
        serde_json::from_slice(body)
            .or_else(|_| serde_urlencoded::from_bytes(body))
            .map_err(|_| AppError::BadRequest(message.to_string()))
    }
}

/// Helper for returning OK status
pub fn status_ok() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "OK"}))
}

// Import MmAuthUser from parent module
pub use super::extractors::MmAuthUser;

use axum::Json;
