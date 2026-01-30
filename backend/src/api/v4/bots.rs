use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/bots", get(list_bots).post(create_bot))
}
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::{ApiResult};
use crate::mattermost_compat::{id::{encode_mm_id}, models as mm};
use crate::models::{Bot};
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
    let rows: Vec<(Uuid, String, String, Option<String>, Uuid, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        r#"
        SELECT b.user_id, u.username, b.display_name, b.description, b.owner_id, b.created_at, b.updated_at
        FROM bots b
        JOIN users u ON b.user_id = u.id
        ORDER BY b.created_at DESC
        LIMIT $1 OFFSET $2
        "#
    )
    .bind(query.per_page)
    .bind(query.page * query.per_page)
    .fetch_all(&state.db)
    .await?;

    let mm_bots = rows.into_iter().map(|r| mm::Bot {
        user_id: encode_mm_id(r.0),
        username: r.1,
        display_name: r.2,
        description: r.3.unwrap_or_default(),
        owner_id: encode_mm_id(r.4),
        create_at: r.5.timestamp_millis(),
        update_at: r.6.timestamp_millis(),
        delete_at: 0,
    }).collect();

    Ok(Json(mm_bots))
}

