use axum::extract::State;
use axum::Json;
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::ApiResult;
use crate::mattermost_compat::{id::{encode_mm_id, parse_mm_or_uuid}, models as mm};
use crate::models::user::User;
use crate::models::{Team, TeamMember};
use crate::models::channel_category::ChannelCategory;

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
