use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub create_at: i64,
    pub update_at: i64,
    pub delete_at: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub nickname: String,
    pub email: String,
    pub email_verified: bool,
    pub auth_service: String,
    pub roles: String,
    pub locale: String,
    pub notify_props: Value,
    pub props: Value,
    pub last_password_update: i64,
    pub last_picture_update: i64,
    pub failed_attempts: i32,
    pub mfa_active: bool,
    pub timezone: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub create_at: i64,
    pub update_at: i64,
    pub delete_at: i64,
    pub display_name: String,
    pub name: String,
    pub description: String,
    pub email: String,
    #[serde(rename = "type")]
    pub team_type: String,
    pub company_name: String,
    pub allowed_domains: String,
    pub invite_id: String,
    pub allow_open_invite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub create_at: i64,
    pub update_at: i64,
    pub delete_at: i64,
    pub team_id: String,
    #[serde(rename = "type")]
    pub channel_type: String,
    pub display_name: String,
    pub name: String,
    pub header: String,
    pub purpose: String,
    pub last_post_at: i64,
    pub total_msg_count: i64,
    pub extra_update_at: i64,
    pub creator_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: String,
    pub create_at: i64,
    pub update_at: i64,
    pub delete_at: i64,
    pub edit_at: i64,
    pub user_id: String,
    pub channel_id: String,
    pub root_id: String,
    pub original_id: String,
    pub message: String,
    #[serde(rename = "type")]
    pub post_type: String,
    pub props: Value,
    pub hashtags: String,
    pub file_ids: Vec<String>,
    pub pending_post_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostList {
    pub order: Vec<String>,
    pub posts: std::collections::HashMap<String, Post>,
    pub next_post_id: String,
    pub prev_post_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub team_id: String,
    pub user_id: String,
    pub roles: String,
    pub delete_at: i64,
    pub scheme_guest: bool,
    pub scheme_user: bool,
    pub scheme_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMember {
    pub channel_id: String,
    pub user_id: String,
    pub roles: String,
    pub last_viewed_at: i64,
    pub msg_count: i64,
    pub mention_count: i64,
    pub notify_props: Value,
    pub last_update_at: i64,
    pub scheme_guest: bool,
    pub scheme_user: bool,
    pub scheme_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "SiteURL")]
    pub site_url: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "EnablePushNotifications")]
    pub enable_push_notifications: String,
    #[serde(rename = "DiagnosticId")]
    pub diagnostic_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    #[serde(rename = "IsLicensed")]
    pub is_licensed: bool,
    #[serde(rename = "IssuedAt")]
    pub issued_at: i64,
    #[serde(rename = "StartsAt")]
    pub starts_at: i64,
    #[serde(rename = "ExpiresAt")]
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<i64>,
    pub event: String,
    pub data: Value,
    pub broadcast: Broadcast,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preference {
    pub user_id: String,
    pub category: String,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub user_id: String,
    pub status: String,
    pub manual: bool,
    pub last_activity_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub user_id: String,
    pub post_id: String,
    pub emoji_name: String,
    pub create_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub id: String,
    pub user_id: String,
    pub create_at: i64,
    pub update_at: i64,
    pub delete_at: i64,
    pub name: String,
    pub extension: String,
    pub size: i64,
    pub mime_type: String,
    pub width: i32,
    pub height: i32,
    pub has_preview_image: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStats {
    pub channel_id: String,
    pub member_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Broadcast {
    pub omit_users: Option<Value>,
    pub user_id: String,
    pub channel_id: String,
    pub team_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarCategory {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    #[serde(rename = "type")]
    pub category_type: String,
    pub display_name: String,
    pub sorting: String,
    pub muted: bool,
    pub collapsed: bool,
    pub channel_ids: Vec<String>,
    pub create_at: i64,
    pub update_at: i64,
    pub delete_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarCategories {
    pub categories: Vec<SidebarCategory>,
    pub order: Vec<String>,
}
