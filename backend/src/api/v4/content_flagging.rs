use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/content_flagging/flag/config", get(get_content_flagging_flag_config))
        .route(
            "/content_flagging/team/{team_id}/status",
            get(get_content_flagging_team_status),
        )
        .route(
            "/content_flagging/post/{post_id}/flag",
            post(flag_content_post),
        )
        .route("/content_flagging/fields", get(get_content_flagging_fields))
        .route(
            "/content_flagging/post/{post_id}/field_values",
            get(get_content_flagging_post_field_values),
        )
        .route("/content_flagging/post/{post_id}", get(get_content_flagging_post))
        .route(
            "/content_flagging/post/{post_id}/remove",
            post(remove_content_flagging_post),
        )
        .route(
            "/content_flagging/post/{post_id}/keep",
            post(keep_content_flagging_post),
        )
        .route("/content_flagging/config", get(get_content_flagging_config))
        .route(
            "/content_flagging/team/{team_id}/reviewers/search",
            post(search_content_flagging_reviewers),
        )
        .route(
            "/content_flagging/post/{post_id}/assign/{content_reviewer_id}",
            post(assign_content_flagging_post),
        )
}

/// GET /api/v4/content_flagging/flag/config
async fn get_content_flagging_flag_config(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// GET /api/v4/content_flagging/team/{team_id}/status
async fn get_content_flagging_team_status(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_team_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// POST /api/v4/content_flagging/post/{post_id}/flag
async fn flag_content_post(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_post_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/content_flagging/fields
async fn get_content_flagging_fields(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/content_flagging/post/{post_id}/field_values
async fn get_content_flagging_post_field_values(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_post_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/content_flagging/post/{post_id}
async fn get_content_flagging_post(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_post_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// POST /api/v4/content_flagging/post/{post_id}/remove
async fn remove_content_flagging_post(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_post_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/content_flagging/post/{post_id}/keep
async fn keep_content_flagging_post(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_post_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/content_flagging/config
async fn get_content_flagging_config(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// POST /api/v4/content_flagging/team/{team_id}/reviewers/search
async fn search_content_flagging_reviewers(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_team_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// POST /api/v4/content_flagging/post/{post_id}/assign/{content_reviewer_id}
async fn assign_content_flagging_post(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path((_post_id, _reviewer_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}
