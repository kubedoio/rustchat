use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

use super::utils::parse_body;
use super::{encode_mm_id, parse_mm_or_uuid, ApiResult, AppState, MmAuthUser};
use crate::error::AppError;

#[derive(Deserialize)]
struct ViewChannelRequest {
    #[serde(default)]
    channel_id: Option<String>,
    #[serde(rename = "prev_channel_id", default)]
    prev_channel_id: Option<String>,
    #[serde(default)]
    _collapsed_threads_supported: bool,
}

fn parse_view_channel_request(headers: &HeaderMap, body: &Bytes) -> ApiResult<ViewChannelRequest> {
    parse_body(headers, body, "Invalid view body")
}

fn parse_optional_channel_id(raw: Option<String>, field_name: &str) -> ApiResult<Option<Uuid>> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    if raw.is_empty() {
        return Ok(None);
    }

    parse_mm_or_uuid(&raw)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid {}", field_name)))
        .map(Some)
}

async fn mark_channel_viewed(
    state: &AppState,
    user_id: Uuid,
    channel_id: Uuid,
) -> ApiResult<Option<i64>> {
    let affected = sqlx::query(
        r#"
        UPDATE channel_members
        SET last_viewed_at = NOW(),
            manually_unread = false,
            last_update_at = NOW()
        WHERE channel_id = $1 AND user_id = $2
        "#,
    )
    .bind(channel_id)
    .bind(user_id)
    .execute(&state.db)
    .await?;

    if affected.rows_affected() == 0 {
        return Err(AppError::Forbidden(
            "Not a member of this channel".to_string(),
        ));
    }

    sqlx::query(
        r#"
        INSERT INTO channel_reads (user_id, channel_id, last_read_message_id, last_read_at)
        VALUES ($1, $2, (SELECT COALESCE(MAX(seq), 0) FROM posts WHERE channel_id = $2 AND deleted_at IS NULL), NOW())
        ON CONFLICT (user_id, channel_id)
        DO UPDATE SET last_read_message_id = EXCLUDED.last_read_message_id, last_read_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(channel_id)
    .execute(&state.db)
    .await?;

    let last_post_at: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT (EXTRACT(EPOCH FROM MAX(created_at)) * 1000)::BIGINT
        FROM posts
        WHERE channel_id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(channel_id)
    .fetch_one(&state.db)
    .await?;

    Ok(last_post_at)
}

async fn view_channel_internal(
    state: &AppState,
    user_id: Uuid,
    channel_id: Option<Uuid>,
    prev_channel_id: Option<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut last_viewed_at_times: HashMap<String, i64> = HashMap::new();

    if let Some(cid) = channel_id {
        let last_post_at = mark_channel_viewed(state, user_id, cid).await?.unwrap_or(0);
        last_viewed_at_times.insert(encode_mm_id(cid), last_post_at);

        let broadcast = crate::realtime::WsEnvelope::event(
            crate::realtime::EventType::ChannelViewed,
            serde_json::json!({
                "channel_id": cid,
            }),
            Some(cid),
        )
        .with_broadcast(crate::realtime::WsBroadcast {
            channel_id: None,
            team_id: None,
            user_id: Some(user_id),
            exclude_user_id: None,
        });
        state.ws_hub.broadcast(broadcast).await;
    }

    if let Some(cid) = prev_channel_id {
        let last_post_at = mark_channel_viewed(state, user_id, cid).await?.unwrap_or(0);
        last_viewed_at_times.insert(encode_mm_id(cid), last_post_at);
    }

    Ok(Json(serde_json::json!({
        "status": "OK",
        "last_viewed_at_times": last_viewed_at_times,
    })))
}

pub(super) async fn view_channel(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    if body.is_empty() {
        return Err(AppError::BadRequest("Invalid view body".to_string()));
    }

    let input = parse_view_channel_request(&headers, &body)?;
    let channel_id = parse_optional_channel_id(input.channel_id, "channel_view.channel_id")?;
    let prev_channel_id =
        parse_optional_channel_id(input.prev_channel_id, "channel_view.prev_channel_id")?;

    view_channel_internal(&state, auth.user_id, channel_id, prev_channel_id).await
}

pub(super) async fn view_channel_for_user(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(user_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let resolved_user_id = if user_id == "me" {
        auth.user_id
    } else {
        parse_mm_or_uuid(&user_id).ok_or_else(|| crate::error::AppError::InvalidUserId)?
    };

    if resolved_user_id != auth.user_id {
        return Err(crate::error::AppError::Forbidden(
            "Cannot view channel for other users".to_string(),
        ));
    }

    if body.is_empty() {
        return Err(AppError::BadRequest("Invalid view body".to_string()));
    }

    let input = parse_view_channel_request(&headers, &body)?;
    let channel_id = parse_optional_channel_id(input.channel_id, "channel_view.channel_id")?;
    let prev_channel_id =
        parse_optional_channel_id(input.prev_channel_id, "channel_view.prev_channel_id")?;

    view_channel_internal(&state, auth.user_id, channel_id, prev_channel_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json"),
        );
        headers
    }

    #[test]
    fn parses_view_request_json() {
        let headers = json_headers();
        let body = Bytes::from_static(br#"{"channel_id":"test","prev_channel_id":"prev"}"#);

        let parsed = parse_view_channel_request(&headers, &body).expect("valid request");
        assert_eq!(parsed.channel_id.as_deref(), Some("test"));
        assert_eq!(parsed.prev_channel_id.as_deref(), Some("prev"));
    }

    #[test]
    fn rejects_non_json_view_request() {
        let headers = json_headers();
        let body = Bytes::from_static(b"not-json");

        let parsed = parse_view_channel_request(&headers, &body);
        assert!(parsed.is_err());
    }
}
