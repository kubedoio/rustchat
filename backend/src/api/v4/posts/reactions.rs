use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use super::{
    encode_mm_id, mm, parse_body, parse_mm_or_uuid, ApiResult, AppError, AppState, EventType,
    MmAuthUser, WsBroadcast, WsEnvelope,
};

#[derive(Deserialize)]
struct ReactionRequest {
    user_id: String,
    post_id: String,
    emoji_name: String,
}

fn reaction_event_payload(mm_reaction: &mm::Reaction) -> serde_json::Value {
    let reaction_json = serde_json::to_string(mm_reaction).unwrap_or_default();
    serde_json::json!({ "reaction": reaction_json })
}

/// Verify the caller is a member of the channel containing the post.
async fn check_channel_membership(state: &AppState, post_id: Uuid, user_id: Uuid) -> ApiResult<()> {
    let is_member: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM posts p
            JOIN channel_members cm ON cm.channel_id = p.channel_id
            WHERE p.id = $1 AND cm.user_id = $2
        )
        "#,
    )
    .bind(post_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    if !is_member {
        return Err(AppError::Forbidden(
            "You are not a member of the channel containing this post".to_string(),
        ));
    }
    Ok(())
}

pub(super) async fn add_reaction(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<mm::Reaction>)> {
    let input: ReactionRequest = parse_body(&headers, &body, "Invalid reaction body")?;
    let input_user_id = parse_mm_or_uuid(&input.user_id)
        .ok_or_else(|| AppError::Validation("Invalid user_id".to_string()))?;
    if input_user_id != auth.user_id {
        return Err(AppError::Forbidden(
            "Cannot react for other user".to_string(),
        ));
    }

    let post_id = parse_mm_or_uuid(&input.post_id)
        .ok_or_else(|| AppError::Validation("Invalid post_id".to_string()))?;

    // Verify the caller is a member of the channel containing this post
    check_channel_membership(&state, post_id, auth.user_id).await?;

    let emoji_name =
        crate::mattermost_compat::emoji_data::get_short_name_for_emoji(&input.emoji_name);

    if !crate::mattermost_compat::emoji_data::is_valid_emoji_name(&emoji_name) {
        return Err(AppError::BadRequest("Invalid emoji name".to_string()));
    }

    if !crate::mattermost_compat::emoji_data::is_system_emoji(&emoji_name) {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM custom_emojis WHERE name = $1 AND delete_at IS NULL)",
        )
        .bind(&emoji_name)
        .fetch_one(&state.db)
        .await?;

        if !exists {
            return Err(AppError::NotFound("Emoji not found".to_string()));
        }
    }

    let reaction: crate::models::reaction::Reaction = sqlx::query_as(
        r#"
        INSERT INTO reactions (user_id, post_id, emoji_name)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, post_id, emoji_name) DO UPDATE SET emoji_name = $3
        RETURNING *
        "#,
    )
    .bind(auth.user_id)
    .bind(post_id)
    .bind(&emoji_name)
    .fetch_one(&state.db)
    .await?;

    let channel_id: Uuid = sqlx::query_scalar("SELECT channel_id FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(&state.db)
        .await?;

    // Create reaction activity for the post author
    let post_info: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT p.user_id, c.team_id FROM posts p JOIN channels c ON p.channel_id = c.id WHERE p.id = $1"
    )
    .bind(post_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    if let Some((post_user_id, team_id)) = post_info {
        if post_user_id != auth.user_id {
            let _ = crate::services::activity::create_reaction_activity(
                &state,
                post_user_id,
                auth.user_id,
                channel_id,
                team_id,
                post_id,
                &emoji_name,
            )
            .await;
        }
    }

    let mm_reaction = mm::Reaction {
        user_id: encode_mm_id(reaction.user_id),
        post_id: encode_mm_id(reaction.post_id),
        emoji_name: reaction.emoji_name,
        create_at: reaction.create_at,
        update_at: reaction.create_at,
        delete_at: 0,
        channel_id: encode_mm_id(channel_id),
        remote_id: "".to_string(),
    };

    let data = reaction_event_payload(&mm_reaction);

    let broadcast = WsEnvelope::event(EventType::ReactionAdded, data, Some(channel_id))
        .with_broadcast(WsBroadcast {
            channel_id: Some(channel_id),
            team_id: None,
            user_id: None,
            exclude_user_id: None,
        });
    state.ws_hub.broadcast(broadcast).await;

    Ok((StatusCode::CREATED, Json(mm_reaction)))
}

pub(crate) async fn reactions_for_posts(
    state: &AppState,
    post_ids: &[Uuid],
) -> ApiResult<std::collections::HashMap<Uuid, Vec<mm::Reaction>>> {
    use std::collections::HashMap;

    if post_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let reactions: Vec<(Uuid, Uuid, String, i64, Uuid)> = sqlx::query_as(
        r#"
        SELECT r.post_id, r.user_id, r.emoji_name, r.create_at, p.channel_id
        FROM reactions r
        JOIN posts p ON p.id = r.post_id
        WHERE r.post_id = ANY($1)
        "#,
    )
    .bind(post_ids)
    .fetch_all(&state.db)
    .await?;

    let mut map: HashMap<Uuid, Vec<mm::Reaction>> = HashMap::new();
    for (post_id, user_id, emoji_name, create_at, channel_id) in reactions {
        map.entry(post_id).or_default().push(mm::Reaction {
            user_id: encode_mm_id(user_id),
            post_id: encode_mm_id(post_id),
            emoji_name: crate::mattermost_compat::emoji_data::get_short_name_for_emoji(&emoji_name),
            create_at,
            update_at: create_at,
            delete_at: 0,
            channel_id: encode_mm_id(channel_id),
            remote_id: "".to_string(),
        });
    }

    Ok(map)
}

pub(super) async fn remove_reaction(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((post_id, emoji_name)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;

    // Verify the caller is a member of the channel containing this post
    check_channel_membership(&state, post_id, auth.user_id).await?;

    remove_reaction_internal(&state, auth.user_id, post_id, &emoji_name).await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

pub(super) async fn remove_reaction_for_user(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path((user_id, post_id, emoji_name)): Path<(String, String, String)>,
) -> ApiResult<impl IntoResponse> {
    let resolved_user_id = if user_id == "me" {
        auth.user_id
    } else {
        parse_mm_or_uuid(&user_id)
            .ok_or_else(|| AppError::BadRequest("Invalid user_id".to_string()))?
    };

    if resolved_user_id != auth.user_id {
        return Err(AppError::Forbidden(
            "Cannot remove reactions for other user".to_string(),
        ));
    }

    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;

    // Verify the caller is a member of the channel containing this post
    check_channel_membership(&state, post_id, auth.user_id).await?;

    remove_reaction_internal(&state, resolved_user_id, post_id, &emoji_name).await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn remove_reaction_internal(
    state: &AppState,
    user_id: Uuid,
    post_id: Uuid,
    emoji_name: &str,
) -> ApiResult<()> {
    let emoji_name = crate::mattermost_compat::emoji_data::get_short_name_for_emoji(emoji_name);

    let reaction: Option<crate::models::reaction::Reaction> = sqlx::query_as(
        "SELECT * FROM reactions WHERE user_id = $1 AND post_id = $2 AND emoji_name = $3",
    )
    .bind(user_id)
    .bind(post_id)
    .bind(&emoji_name)
    .fetch_optional(&state.db)
    .await?;

    if let Some(r) = reaction {
        sqlx::query(
            "DELETE FROM reactions WHERE user_id = $1 AND post_id = $2 AND emoji_name = $3",
        )
        .bind(user_id)
        .bind(post_id)
        .bind(&emoji_name)
        .execute(&state.db)
        .await?;

        let channel_id: Uuid = sqlx::query_scalar("SELECT channel_id FROM posts WHERE id = $1")
            .bind(post_id)
            .fetch_one(&state.db)
            .await?;

        let mm_reaction = mm::Reaction {
            user_id: encode_mm_id(r.user_id),
            post_id: encode_mm_id(r.post_id),
            emoji_name: r.emoji_name,
            create_at: r.create_at,
            update_at: r.create_at,
            delete_at: 0,
            channel_id: encode_mm_id(channel_id),
            remote_id: "".to_string(),
        };

        let data = reaction_event_payload(&mm_reaction);

        let broadcast = WsEnvelope::event(EventType::ReactionRemoved, data, Some(channel_id))
            .with_broadcast(WsBroadcast {
                channel_id: Some(channel_id),
                team_id: None,
                user_id: None,
                exclude_user_id: None,
            });
        state.ws_hub.broadcast(broadcast).await;
    }

    Ok(())
}

pub(super) async fn get_reactions(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(post_id): Path<String>,
) -> ApiResult<Json<Vec<mm::Reaction>>> {
    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::BadRequest("Invalid post_id".to_string()))?;

    // Verify the caller is a member of the channel that owns this post.
    check_channel_membership(&state, post_id, auth.user_id).await?;

    let reactions: Vec<(Uuid, Uuid, String, i64, Uuid)> = sqlx::query_as(
        r#"
        SELECT r.user_id, r.post_id, r.emoji_name, r.create_at, p.channel_id
        FROM reactions r
        JOIN posts p ON p.id = r.post_id
        WHERE r.post_id = $1
        "#,
    )
    .bind(post_id)
    .fetch_all(&state.db)
    .await?;

    let mm_reactions = reactions
        .into_iter()
        .map(
            |(user_id, post_id, emoji_name, create_at, channel_id)| mm::Reaction {
                user_id: encode_mm_id(user_id),
                post_id: encode_mm_id(post_id),
                emoji_name: crate::mattermost_compat::emoji_data::get_short_name_for_emoji(
                    &emoji_name,
                ),
                create_at,
                update_at: create_at,
                delete_at: 0,
                channel_id: encode_mm_id(channel_id),
                remote_id: "".to_string(),
            },
        )
        .collect();

    Ok(Json(mm_reactions))
}

#[cfg(test)]
mod tests {
    use super::reaction_event_payload;

    #[test]
    fn wraps_reaction_payload_as_string() {
        let reaction = crate::mattermost_compat::models::Reaction {
            user_id: "u".to_string(),
            post_id: "p".to_string(),
            emoji_name: "+1".to_string(),
            create_at: 1,
            update_at: 1,
            delete_at: 0,
            channel_id: "c".to_string(),
            remote_id: "".to_string(),
        };

        let payload = reaction_event_payload(&reaction);
        assert!(payload.get("reaction").and_then(|v| v.as_str()).is_some());
    }
}
