use axum::{
    extract::{Path, State},
    Json,
};

use super::{encode_mm_id, mm, parse_mm_or_uuid, ApiResult, AppState, MmAuthUser};

pub async fn get_channel_stats(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Path(channel_id): Path<String>,
) -> ApiResult<Json<mm::ChannelStats>> {
    let channel_id =
        parse_mm_or_uuid(&channel_id).ok_or_else(|| crate::error::AppError::InvalidChannelId)?;
    let is_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM channel_members WHERE channel_id = $1 AND user_id = $2)",
    )
    .bind(channel_id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if !is_member {
        return Err(crate::error::AppError::Forbidden(
            "Not a member of this channel".to_string(),
        ));
    }

    let member_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM channel_members WHERE channel_id = $1")
            .bind(channel_id)
            .fetch_one(&state.db)
            .await?;

    Ok(Json(mm::ChannelStats {
        channel_id: encode_mm_id(channel_id),
        member_count,
    }))
}

pub async fn get_channel_timezones(
    Path(_channel_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}
