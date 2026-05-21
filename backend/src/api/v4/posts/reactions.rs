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
use crate::repositories::PostRepository;

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
    let is_member = PostRepository::new(state.db.clone())
        .check_channel_membership(post_id, user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

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
        .ok_or_else(|| AppError::ValidationInvalidUserId)?;
    if input_user_id != auth.user_id {
        return Err(AppError::Forbidden(
            "Cannot react for other user".to_string(),
        ));
    }

    let post_id = parse_mm_or_uuid(&input.post_id)
        .ok_or_else(|| AppError::ValidationInvalidPostId)?;

    // Verify the caller is a member of the channel containing this post
    check_channel_membership(&state, post_id, auth.user_id).await?;

    let emoji_name =
        crate::mattermost_compat::emoji_data::get_short_name_for_emoji(&input.emoji_name);

    if !crate::mattermost_compat::emoji_data::is_valid_emoji_name(&emoji_name) {
        return Err(AppError::BadRequest("Invalid emoji name".to_string()));
    }

    let repo = PostRepository::new(state.db.clone());

    if !crate::mattermost_compat::emoji_data::is_system_emoji(&emoji_name) {
        let exists = repo
            .custom_emoji_exists(&emoji_name)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if !exists {
            return Err(AppError::EmojiNotFound);
        }
    }

    let reaction = repo
        .add_reaction(post_id, auth.user_id, &emoji_name)
        .await?;

    let channel_id = repo.get_post_channel_id(post_id).await?;

    // Create reaction activity for the post author
    if let Some((post_user_id, team_id)) =
        repo.get_post_author_and_team(post_id).await.ok().flatten()
    {
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

    let reactions = PostRepository::new(state.db.clone())
        .get_reactions_with_channel_for_posts(post_ids)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

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
        .ok_or_else(|| AppError::InvalidPostId)?;

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
            .ok_or_else(|| AppError::InvalidUserId)?
    };

    if resolved_user_id != auth.user_id {
        return Err(AppError::Forbidden(
            "Cannot remove reactions for other user".to_string(),
        ));
    }

    let post_id = parse_mm_or_uuid(&post_id)
        .ok_or_else(|| AppError::InvalidPostId)?;

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

    let repo = PostRepository::new(state.db.clone());

    if let Some(r) = repo
        .get_reaction(user_id, post_id, &emoji_name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
    {
        repo.remove_reaction(post_id, user_id, &emoji_name).await?;

        let channel_id = repo.get_post_channel_id(post_id).await?;

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
        .ok_or_else(|| AppError::InvalidPostId)?;

    // Verify the caller is a member of the channel that owns this post.
    check_channel_membership(&state, post_id, auth.user_id).await?;

    let reactions = PostRepository::new(state.db.clone())
        .get_reactions_with_channel_for_post(post_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let mm_reactions = reactions
        .into_iter()
        .map(
            |(user_id, _post_id, emoji_name, create_at, channel_id)| mm::Reaction {
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
