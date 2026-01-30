use axum::{Router, routing::get, routing::post};
use crate::api::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        // Discovery / Config
        .route("/config/client", get(super::v4::config_client::get_client_config))
        .route("/license/client", get(super::v4::config_client::get_client_license))
        
        // Auth / User Extras
        .route("/users/me/channels/categories", get(super::v4::users::get_categories))
        .route("/users/{user_id}/posts/{post_id}/reminder", post(super::v4::posts::set_post_reminder))
        
        // Channels / Teams Extras
        .route("/channels/search", post(super::v4::channels::search_channels))
        .route("/teams/search", post(super::v4::teams::search_teams))
        
        // WebSocket
        .route("/websocket", get(super::websocket::handle_websocket))
        
        // Advanced Messaging
        .route("/posts/ephemeral", post(super::v4::posts::create_ephemeral_post))
        .route("/posts/schedule", post(super::v4::posts::create_scheduled_post))
        .route("/posts/scheduled/team/{team_id}", get(super::v4::posts::list_scheduled_posts))
        
        // Emojis / Reactions (New unique endpoints only, others are in posts.rs)
        .route("/emoji/search", post(super::v4::emoji::search_emoji))
        .route("/emoji/{emoji_id}", get(super::v4::emoji::get_emoji))
        
        // Integrations
        .route("/hooks/incoming", get(super::v4::hooks::list_incoming_hooks).post(super::v4::hooks::create_incoming_hook))
        .route("/hooks/outgoing", get(super::v4::hooks::list_outgoing_hooks).post(super::v4::hooks::create_outgoing_hook))
        .route("/bots", get(super::v4::bots::list_bots).post(super::v4::bots::create_bot))
        
        // Audits / Admin
        .route("/audits", get(super::v4::admin::get_audits))
        .route("/plugins/statuses", get(super::v4::plugins::get_plugin_statuses))
        
        // System Admin
        .route("/caches/invalidate", post(super::v4::system::invalidate_caches))
        .route("/logs", post(super::v4::system::post_logs))
        .route("/database/recycle", post(super::v4::system::recycle_database))
}
