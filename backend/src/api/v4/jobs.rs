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
        .route("/jobs", get(get_jobs))
        .route("/jobs/{job_id}", get(get_job))
        .route("/jobs/{job_id}/download", get(download_job_data))
        .route("/jobs/{job_id}/cancel", post(cancel_job))
        .route("/jobs/type/{type}", get(get_jobs_by_type))
        .route("/jobs/{job_id}/status", get(get_job_status))
}

/// GET /api/v4/jobs
async fn get_jobs(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/jobs/{job_id}
async fn get_job(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_job_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// GET /api/v4/jobs/{job_id}/download
async fn download_job_data(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_job_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// POST /api/v4/jobs/{job_id}/cancel
async fn cancel_job(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_job_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/jobs/type/{type}
async fn get_jobs_by_type(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_type): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// GET /api/v4/jobs/{job_id}/status
async fn get_job_status(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_job_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}
