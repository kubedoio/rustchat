//! Calls API endpoints

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use super::AppState;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{Call, CallSession, CreateCall, CallParticipant};

/// Build calls routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/calls", post(create_call))
        .route("/calls/{id}", get(get_call))
        .route("/calls/{id}/join", post(join_call))
        .route("/calls/{id}/leave", post(leave_call))
        .route("/calls/{id}/end", post(end_call))
}

// ============ Calls ============

async fn create_call(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateCall>,
) -> ApiResult<Json<Call>> {
    // 1. Check if an active call exists for this channel
    let existing: Option<Call> = sqlx::query_as(
        "SELECT * FROM calls WHERE channel_id = $1 AND ended_at IS NULL"
    )
    .bind(payload.channel_id)
    .fetch_optional(&state.db)
    .await?;

    if let Some(call) = existing {
        return Ok(Json(call)); // Return existing active call
    }

    // 2. Create new call
    let call = sqlx::query_as::<_, Call>(
        r#"
        INSERT INTO calls (channel_id, type, owner_id)
        VALUES ($1, $2, $3)
        RETURNING *
        "#
    )
    .bind(payload.channel_id)
    .bind(payload.r#type.unwrap_or_else(|| "audio".to_string()))
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(call))
}

async fn get_call(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CallSession>> {
    let call = sqlx::query_as::<_, Call>(
        "SELECT * FROM calls WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Call not found".to_string()))?;

    let participants = sqlx::query_as::<_, CallParticipant>(
        "SELECT * FROM call_participants WHERE call_id = $1 AND left_at IS NULL"
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(CallSession { call, participants }))
}

async fn join_call(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CallParticipant>> {
    // Check if call exists and is active
    let _call = sqlx::query_as::<_, Call>(
        "SELECT * FROM calls WHERE id = $1 AND ended_at IS NULL"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Call not found or ended".to_string()))?;

    // Add participant
    let participant = sqlx::query_as::<_, CallParticipant>(
        r#"
        INSERT INTO call_participants (call_id, user_id, role)
        VALUES ($1, $2, 'attendee')
        ON CONFLICT (call_id, user_id) 
        DO UPDATE SET joined_at = NOW(), left_at = NULL
        RETURNING *
        "#
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(participant))
}

async fn leave_call(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    sqlx::query(
        "UPDATE call_participants SET left_at = NOW() WHERE call_id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({"status": "left"})))
}

async fn end_call(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Call>> {
    // Check ownership
    let call = sqlx::query_as::<_, Call>("SELECT * FROM calls WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Call not found".to_string()))?;

    if call.owner_id != Some(auth.user_id) {
        return Err(AppError::Forbidden("Only the host can end the call".to_string()));
    }

    let ended = sqlx::query_as::<_, Call>(
        "UPDATE calls SET ended_at = NOW() WHERE id = $1 RETURNING *"
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    // Also mark all participants as left? Optional but good for cleanup.
    // We'll leave that for now or rely on client disconnects.

    Ok(Json(ended))
}
