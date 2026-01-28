use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChannelCategory {
    pub id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    #[sqlx(rename = "type")]
    pub category_type: String,
    pub display_name: String,
    pub sorting: String,
    pub muted: bool,
    pub collapsed: bool,
    pub sort_order: i32,
    pub create_at: i64,
    pub update_at: i64,
    pub delete_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChannelCategoryChannel {
    pub category_id: Uuid,
    pub channel_id: Uuid,
    pub sort_order: i32,
}
