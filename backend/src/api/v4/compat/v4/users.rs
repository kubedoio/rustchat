use axum::extract::State;
use axum::Json;
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::ApiResult;
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};
use crate::models::user::User;
use crate::models::{Team, TeamMember};
use crate::models::channel_category::ChannelCategory;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListUsersQuery {
    #[serde(default)]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    pub in_team: Option<String>,
    pub not_in_team: Option<String>,
    pub in_channel: Option<String>,
    pub not_in_channel: Option<String>,
    pub group_constrained: Option<bool>,
    pub sort: Option<String>,
}

fn default_per_page() -> i64 {
    60
}

#[derive(Deserialize)]
pub struct UserSearchRequest {
    pub term: String,
    pub team_id: Option<String>,
    pub not_in_team_id: Option<String>,
    pub channel_id: Option<String>,
    pub not_in_channel_id: Option<String>,
    #[serde(default)]
    pub allow_inactive: bool,
    #[serde(default)]
    pub limit: i64,
}

pub async fn me(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<mm::User>> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(user.into()))
}

pub async fn my_teams(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Team>>> {
    let teams: Vec<Team> = sqlx::query_as(
        r#"
        SELECT t.* FROM teams t
        JOIN team_members tm ON t.id = tm.team_id
        WHERE tm.user_id = $1
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(teams.into_iter().map(|t| t.into()).collect()))
}

pub async fn my_team_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::TeamMember>>> {
    let members: Vec<TeamMember> = sqlx::query_as(
        "SELECT * FROM team_members WHERE user_id = $1",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(members.into_iter().map(|m| m.into()).collect()))
}

pub async fn my_teams_unread(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let unread = crate::services::unreads::get_unread_overview(&state, auth.user_id).await?;
    
    let mut resp = Vec::new();
    for team in unread.teams {
        resp.push(serde_json::json!({
            "team_id": encode_mm_id(team.team_id),
            "msg_count": team.unread_count,
            "mention_count": 0
        }));
    }
    
    Ok(Json(resp))
}

pub async fn get_team_channels(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Path(team_id_str): axum::extract::Path<String>,
) -> ApiResult<Json<Vec<mm::Channel>>> {
    let team_id = parse_mm_or_uuid(&team_id_str)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;
    
    let channels: Vec<crate::models::channel::Channel> = sqlx::query_as(
        r#"
        SELECT c.* FROM channels c
        JOIN channel_members cm ON c.id = cm.channel_id
        WHERE c.team_id = $1 AND cm.user_id = $2
        "#,
    )
    .bind(team_id)
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(channels.into_iter().map(|c| c.into()).collect()))
}

pub async fn get_team_channel_members(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Path(team_id_str): axum::extract::Path<String>,
) -> ApiResult<Json<Vec<mm::ChannelMember>>> {
    let team_id = parse_mm_or_uuid(&team_id_str)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;

    let members: Vec<crate::models::channel::ChannelMember> = sqlx::query_as(
        r#"
        SELECT cm.* FROM channel_members cm
        JOIN channels c ON cm.channel_id = c.id
        WHERE c.team_id = $1 AND cm.user_id = $2
        "#,
    )
    .bind(team_id)
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(members.into_iter().map(|m| m.into()).collect()))
}

pub async fn get_categories(
    State(state): State<AppState>,
    auth: MmAuthUser,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<mm::SidebarCategories>> {
    let team_id_str = query.get("team_id").ok_or_else(|| crate::error::AppError::BadRequest("Missing team_id".to_string()))?;
    let team_id = parse_mm_or_uuid(team_id_str)
        .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;
        
    let categories: Vec<ChannelCategory> = sqlx::query_as(
        "SELECT * FROM channel_categories WHERE user_id = $1 AND team_id = $2"
    )
    .bind(auth.user_id)
    .bind(team_id)
    .fetch_all(&state.db)
    .await?;

    let mut mm_categories = Vec::new();

    if categories.is_empty() {
        // Return default categories
        mm_categories.push(mm::SidebarCategory {
            id: format!("{}_fav", encode_mm_id(team_id)),
            user_id: encode_mm_id(auth.user_id),
            team_id: encode_mm_id(team_id),
            display_name: "Favorites".to_string(),
            category_type: "favorites".to_string(),
            channel_ids: vec![],
            sorting: "alpha".to_string(),
            muted: false,
            collapsed: false,
            sort_order: 0,
            create_at: 0,
            update_at: 0,
            delete_at: 0,
        });
        mm_categories.push(mm::SidebarCategory {
            id: format!("{}_chans", encode_mm_id(team_id)),
            user_id: encode_mm_id(auth.user_id),
            team_id: encode_mm_id(team_id),
            display_name: "Channels".to_string(),
            category_type: "channels".to_string(),
            channel_ids: vec![],
            sorting: "alpha".to_string(),
            muted: false,
            collapsed: false,
            sort_order: 10,
            create_at: 0,
            update_at: 0,
            delete_at: 0,
        });
    } else {
        for cat in categories {
            // Fetch channel IDs for this category
            let channel_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
                "SELECT channel_id FROM channel_category_channels WHERE category_id = $1 ORDER BY sort_order"
            )
            .bind(cat.id)
            .fetch_all(&state.db)
            .await?;

            mm_categories.push(mm::SidebarCategory {
                id: encode_mm_id(cat.id),
                team_id: encode_mm_id(cat.team_id),
                user_id: encode_mm_id(cat.user_id),
                category_type: cat.category_type,
                display_name: cat.display_name,
                sorting: cat.sorting,
                muted: cat.muted,
                collapsed: cat.collapsed,
                channel_ids: channel_ids.into_iter().map(encode_mm_id).collect(),
                sort_order: cat.sort_order,
                create_at: cat.create_at,
                update_at: cat.update_at,
                delete_at: cat.delete_at,
            });
        }
    }

    Ok(Json(mm::SidebarCategories {
        categories: mm_categories,
        order: vec![],
    }))
}

pub async fn list_users(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    axum::extract::Query(query): axum::extract::Query<ListUsersQuery>,
) -> ApiResult<Json<Vec<mm::User>>> {
    let per_page = if query.per_page > 0 { query.per_page } else { 60 }.min(200);
    let offset = query.page * per_page;

    let users: Vec<User> = if let Some(team_id_str) = query.in_team {
        let team_id = parse_mm_or_uuid(&team_id_str)
            .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;
        sqlx::query_as(
            r#"
            SELECT u.* FROM users u
            JOIN team_members tm ON u.id = tm.user_id
            WHERE tm.team_id = $1 AND u.is_active = true
            ORDER BY u.username ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(team_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT * FROM users WHERE is_active = true ORDER BY username ASC LIMIT $1 OFFSET $2",
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    };

    Ok(Json(users.into_iter().map(|u| u.into()).collect()))
}

pub async fn search_users(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Json(input): Json<UserSearchRequest>,
) -> ApiResult<Json<Vec<mm::User>>> {
    let term = format!("%{}%", input.term.to_lowercase());
    let limit = if input.limit > 0 { input.limit } else { 100 }.min(200);

    let users: Vec<User> = if let Some(team_id_str) = input.team_id {
        let team_id = parse_mm_or_uuid(&team_id_str)
            .ok_or_else(|| crate::error::AppError::BadRequest("Invalid team_id".to_string()))?;
        sqlx::query_as(
            r#"
            SELECT u.* FROM users u
            JOIN team_members tm ON u.id = tm.user_id
            WHERE tm.team_id = $1 
              AND (LOWER(u.username) LIKE $2 OR LOWER(u.display_name) LIKE $2 OR LOWER(u.email) LIKE $2)
              AND u.is_active = true
            ORDER BY u.username ASC
            LIMIT $3
            "#,
        )
        .bind(team_id)
        .bind(&term)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            r#"
            SELECT * FROM users 
            WHERE (LOWER(username) LIKE $1 OR LOWER(display_name) LIKE $1 OR LOWER(email) LIKE $1)
              AND is_active = true
            ORDER BY username ASC
            LIMIT $2
            "#,
        )
        .bind(&term)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    };

    Ok(Json(users.into_iter().map(|u| u.into()).collect()))
}

pub async fn get_preferences(
    State(state): State<AppState>,
    auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Preference>>> {
    let rows: Vec<(uuid::Uuid, String, String, String)> = sqlx::query_as(
        "SELECT user_id, category, name, value FROM user_preferences WHERE user_id = $1",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let prefs = rows.into_iter().map(|(id, cat, name, val)| mm::Preference {
        user_id: encode_mm_id(id),
        category: cat,
        name,
        value: val,
    }).collect();

    Ok(Json(prefs))
}

pub async fn update_preferences(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(prefs): Json<Vec<mm::Preference>>,
) -> ApiResult<Json<bool>> {
    for pref in prefs {
        sqlx::query(
            r#"
            INSERT INTO user_preferences (user_id, category, name, value)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, category, name) DO UPDATE SET value = $4
            "#,
        )
        .bind(auth.user_id)
        .bind(&pref.category)
        .bind(&pref.name)
        .bind(&pref.value)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(true))
}

pub async fn get_users_by_ids(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Json(ids): Json<Vec<String>>,
) -> ApiResult<Json<Vec<mm::User>>> {
    let uuids: Vec<uuid::Uuid> = ids.iter().filter_map(|id| parse_mm_or_uuid(id)).collect();
    
    let users: Vec<User> = sqlx::query_as(
        "SELECT * FROM users WHERE id = ANY($1)"
    )
    .bind(&uuids)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(users.into_iter().map(|u| u.into()).collect()))
}

pub async fn get_statuses_by_ids(
    State(state): State<AppState>,
    _auth: MmAuthUser,
    Json(ids): Json<Vec<String>>,
) -> ApiResult<Json<Vec<mm::Status>>> {
    let uuids: Vec<uuid::Uuid> = ids.iter().filter_map(|id| parse_mm_or_uuid(id)).collect();

    let users: Vec<User> = sqlx::query_as(
        "SELECT id, presence, last_login_at FROM users WHERE id = ANY($1)"
    )
    .bind(&uuids)
    .fetch_all(&state.db)
    .await?;

    let statuses = users.into_iter().map(|u| mm::Status {
        user_id: encode_mm_id(u.id),
        status: u.presence,
        manual: false,
        last_activity_at: u.last_login_at.map(|t| t.timestamp_millis()).unwrap_or(0),
    }).collect();

    Ok(Json(statuses))
}
