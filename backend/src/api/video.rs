//! Video calling API endpoints

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppState;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{JoinBehavior, MiroTalkConfig};
use crate::services::mirotalk::MiroTalkClient;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/meetings", post(create_meeting))
        .route("/meetings/active", get(get_active_meetings))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingScope {
    Channel,
    Dm,
}

#[derive(Debug, Deserialize)]
pub struct CreateMeetingRequest {
    pub scope: MeetingScope,
    pub channel_id: Option<Uuid>,
    pub dm_user_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct CreateMeetingResponse {
    pub meeting_url: String,
    pub mode: JoinBehavior,
}

async fn create_meeting(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateMeetingRequest>,
) -> ApiResult<Json<CreateMeetingResponse>> {
    // 1. Get MiroTalk config
    let config: MiroTalkConfig =
        sqlx::query_as("SELECT * FROM mirotalk_config WHERE is_active = true")
            .fetch_optional(&state.db)
            .await?
            .unwrap_or_else(|| MiroTalkConfig {
                is_active: true,
                mode: crate::models::MiroTalkMode::Disabled,
                base_url: "".to_string(),
                api_key_secret: "".to_string(),
                default_room_prefix: None,
                join_behavior: crate::models::JoinBehavior::NewTab,
                updated_at: Utc::now(),
                updated_by: None,
            });

    if !config.is_enabled() {
        return Err(AppError::Config(
            "MiroTalk integration is not enabled".to_string(),
        ));
    }

    // 2. Validate scope and IDs
    let channel_id = match payload.scope {
        MeetingScope::Channel => payload.channel_id.ok_or(AppError::BadRequest(
            "channel_id required for channel scope".to_string(),
        ))?,
        MeetingScope::Dm => {
            let target_id = payload.dm_user_id.ok_or(AppError::BadRequest(
                "dm_user_id required for dm scope".to_string(),
            ))?;
            // Resolve DM channel ID between auth.user_id and target_id
            let dm_channel: Option<Uuid> = sqlx::query_scalar(
                r#"
                SELECT c.id FROM channels c
                JOIN channel_members cm1 ON c.id = cm1.channel_id
                JOIN channel_members cm2 ON c.id = cm2.channel_id
                WHERE c.channel_type = 'D' AND cm1.user_id = $1 AND cm2.user_id = $2
                "#,
            )
            .bind(auth.user_id)
            .bind(target_id)
            .fetch_optional(&state.db)
            .await?;

            dm_channel.ok_or(AppError::BadRequest("DM channel not found".to_string()))?
        }
    };

    // 3. Generate room name
    let prefix = config
        .default_room_prefix
        .clone()
        .unwrap_or_else(|| "rustchat".to_string());
    // Use channel ID as part of room name to keep it related to context
    let timestamp = Utc::now().timestamp();
    let room_name = format!("{}-{}-{}", prefix, channel_id, timestamp);

    // 4. Create meeting via client
    let client = MiroTalkClient::new(config.clone(), state.http_client.clone())?;
    let meeting_url = client.create_meeting(&room_name).await?;

    // 5. Post system message
    let message_text = "started a video call".to_string();
    let props = serde_json::json!({
        "type": "video_call",
        "meeting_url": meeting_url,
        "mode": config.join_behavior,
        "initiator_id": auth.user_id,
        "initiator_email": auth.email
    });

    // Use `create_post` service
    // Note: `create_post` expects `client_msg_id` option.
    let create_post_input = crate::models::CreatePost {
        message: message_text,
        file_ids: vec![],
        props: Some(props),
        root_post_id: None,
    };

    let _post_response = crate::services::posts::create_post(
        &state,
        auth.user_id,
        channel_id,
        create_post_input,
        None,
    )
    .await?;

    Ok(Json(CreateMeetingResponse {
        meeting_url,
        mode: config.join_behavior,
    }))
}

async fn get_active_meetings(
    State(state): State<AppState>,
    _auth: AuthUser, // Require auth
) -> ApiResult<Json<Vec<String>>> {
    // Check for admin or permission if needed. For now just let auth users see.

    let config: MiroTalkConfig =
        sqlx::query_as("SELECT * FROM mirotalk_config WHERE is_active = true")
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::Config("MiroTalk config not found".to_string()))?;

    if !config.is_enabled() {
        return Ok(Json(vec![]));
    }

    let client = MiroTalkClient::new(config, state.http_client.clone())?;
    let meetings = client.get_active_meetings().await?;

    Ok(Json(meetings))
}
