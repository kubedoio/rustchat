use axum::{Router, routing::get, routing::post};
use crate::api::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        // Discovery / Config
        .route("/config/client", get(super::v4::config_client::get_client_config))
        
        // Auth
        .route("/users/login", post(super::v4::users_login::login))
        .route("/users/me", get(super::v4::users::me))
        .route("/users/me/teams", get(super::v4::users::my_teams))
        .route("/users/me/teams/members", get(super::v4::users::my_team_members))
        .route("/users/me/teams/unread", get(super::v4::users::my_teams_unread))
        .route("/users/me/teams/{team_id}/channels", get(super::v4::users::get_team_channels))
        .route("/users/me/teams/{team_id}/channels/members", get(super::v4::users::get_team_channel_members))
        .route("/users/me/channels/categories", get(super::v4::users::get_categories))
        .route("/users", get(super::v4::users::list_users))
        .route("/users/search", post(super::v4::users::search_users))
        .route("/users/me/preferences", get(super::v4::users::get_preferences).put(super::v4::users::update_preferences))
        .route("/users/ids", post(super::v4::users::get_users_by_ids))
        .route("/users/status/ids", post(super::v4::users::get_statuses_by_ids))
        
        // Channels
        .route("/channels/direct", post(super::v4::channels::create_direct_channel))
        .route("/channels/group", post(super::v4::channels::create_group_channel))
        .route("/channels/search", post(super::v4::channels::search_channels))
        
        // Teams
        .route("/teams/search", post(super::v4::teams::search_teams))
        
        // Licensing
        .route("/license/client", get(super::v4::config_client::get_client_license))
        
        // WebSocket
        .route("/websocket", get(super::websocket::handle_websocket))
        
        // Posts
        .route("/posts", post(super::v4::posts::create_post))
        .route("/posts/{post_id}", get(super::v4::posts::get_post))
        .route("/channels/{channel_id}/posts", get(super::v4::posts::get_posts_for_channel))
        
        // Threads
        .route("/users/{user_id}/teams/{team_id}/threads", get(super::v4::threads::get_threads).put(super::v4::threads::mark_all_read))
        .route("/users/{user_id}/teams/{team_id}/threads/{thread_id}", get(super::v4::threads::get_thread))
        .route("/users/{user_id}/teams/{team_id}/threads/{thread_id}/following", post(super::v4::threads::follow_thread).delete(super::v4::threads::unfollow_thread))
        
        // Files
        .route("/files/{file_id}/info", get(super::v4::files::get_file_info))
        .route("/files/{file_id}", get(super::v4::files::get_file))
}
