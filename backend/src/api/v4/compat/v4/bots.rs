use axum::{
    extract::{Path, State, Query},
    response::IntoResponse,
    Json,
};
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};
use crate::models::{Bot, User};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct CreateBotRequest {
    pub username: String,
    pub display_name: String,
    pub description: String,
}

#[derive(serde::Deserialize)]
pub struct BotQuery {
    #[serde(default)]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_per_page() -> i64 {
    50
}

pub async fn create_bot(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<CreateBotRequest>,
) -> ApiResult<Json<mm::Bot>> {
    // 1. Create a user for the bot
    let user_id = Uuid::new_v4();
    let _: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO users (id, username, email, password_hash, display_name, is_bot, role)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#
    )
    .bind(user_id)
    .bind(&input.username)
    .bind(format!("{}@bot.local", input.username))
    .bind("bot_no_password")
    .bind(&input.display_name)
    .bind(true)
    .bind("bot")
    .fetch_one(&state.db)
    .await?;

    // 2. Create the bot entry
    let bot: Bot = sqlx::query_as(
        r#"
        INSERT INTO bots (user_id, owner_id, display_name, description, is_active)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#
    )
    .bind(user_id)
    .bind(auth.user_id)
    .bind(&input.display_name)
    .bind(&input.description)
    .bind(true)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(map_bot(bot, input.username)))
}

pub async fn list_bots(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Query(query): Query<BotQuery>,
) -> ApiResult<Json<Vec<mm::Bot>>> {
    let bots: Vec<(Bot, String)> = sqlx::query_as(
        r#"
        SELECT b.*, u.username
        FROM bots b
        JOIN users u ON b.user_id = u.id
        LIMIT $1 OFFSET $2
        "#
    )
    .bind(query.per_page)
    .bind(query.page * query.per_page)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(bots.into_iter().map(|(b, username)| map_bot(b, username)).collect()))
}

fn map_bot(b: Bot, username: String) -> mm::Bot {
    mm::Bot {
        user_id: encode_mm_id(b.user_id),
        create_at: b.created_at.timestamp_millis(),
        update_at: b.updated_at.timestamp_millis(),
        delete_at: 0,
        username,
        display_name: b.display_name,
        description: b.description.unwrap_or_default(),
        owner_id: encode_mm_id(b.owner_id),
    }
}
