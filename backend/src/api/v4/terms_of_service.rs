use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::error::{ApiResult, AppError};
use crate::models::terms::*;
use crate::repositories::TermsRepository;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use uuid::Uuid;

use super::extractors::MmAuthUser;

pub fn router() -> Router<AppState> {
    Router::new()
        // Public endpoints
        .route("/terms_of_service/current", get(get_current_terms))
        .route("/terms_of_service/status", get(get_terms_status))
        .route("/terms_of_service/accept", post(accept_terms))
        // Admin endpoints
        .route("/terms_of_service", get(list_terms).post(create_terms))
        .route(
            "/terms_of_service/{id}",
            get(get_terms).put(update_terms).delete(delete_terms),
        )
        .route("/terms_of_service/{id}/activate", post(activate_terms))
        .route("/terms_of_service/{id}/stats", get(get_terms_stats))
        .route("/terms_of_service/stats/summary", get(get_all_terms_stats))
}

// Public endpoints

async fn get_current_terms(
    State(state): State<AppState>,
) -> ApiResult<Json<Option<TermsOfService>>> {
    let repo = TermsRepository::new(&state.db);
    let terms = repo.get_current_terms().await?;
    Ok(Json(terms))
}

async fn get_terms_status(
    State(state): State<AppState>,
    auth_user: MmAuthUser,
) -> ApiResult<Json<TermsStatusResponse>> {
    let repo = TermsRepository::new(&state.db);

    // Get current active terms
    let current_terms = repo.get_current_terms().await?;

    let Some(ref terms) = current_terms else {
        return Ok(Json(TermsStatusResponse {
            has_accepted: true,
            current_terms: None,
            accepted_version: None,
            acceptance_required: false,
        }));
    };

    // Check if user has accepted
    let accepted = repo.has_user_accepted(auth_user.user_id, terms.id).await?;

    // Get accepted version if any
    let accepted_version = if accepted {
        Some(terms.version.clone())
    } else {
        None
    };

    Ok(Json(TermsStatusResponse {
        has_accepted: accepted,
        current_terms: current_terms.clone(),
        accepted_version,
        acceptance_required: !accepted,
    }))
}

async fn accept_terms(
    State(state): State<AppState>,
    auth_user: MmAuthUser,
    Json(req): Json<TermsAcceptanceRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = TermsRepository::new(&state.db);

    // Verify terms exist
    let terms = repo.get_terms_by_id(req.terms_id).await?;
    if terms.is_none() {
        return Err(AppError::TermsNotFound);
    }

    // Insert acceptance
    repo.accept_terms(auth_user.user_id, req.terms_id, Utc::now())
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Terms accepted successfully"
    })))
}

// Admin endpoints

async fn list_terms(
    State(state): State<AppState>,
    auth_user: MmAuthUser,
) -> ApiResult<Json<Vec<TermsOfService>>> {
    if !auth_user.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::AdminRequired);
    }
    let repo = TermsRepository::new(&state.db);
    let terms = repo.list_terms().await?;
    Ok(Json(terms))
}

async fn get_terms(
    State(state): State<AppState>,
    auth_user: MmAuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<TermsOfService>> {
    if !auth_user.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::AdminRequired);
    }
    let repo = TermsRepository::new(&state.db);
    let terms = repo
        .get_terms_by_id(id)
        .await?
        .ok_or_else(|| AppError::TermsNotFound)?;

    Ok(Json(terms))
}

async fn create_terms(
    State(state): State<AppState>,
    auth_user: MmAuthUser,
    Json(req): Json<CreateTermsRequest>,
) -> ApiResult<Json<TermsOfService>> {
    if !auth_user.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::AdminRequired);
    }
    let repo = TermsRepository::new(&state.db);

    // Validate version uniqueness
    let existing = repo.version_exists(&req.version).await?;
    if existing {
        return Err(AppError::Validation("Version already exists".to_string()));
    }

    let terms = repo
        .create_terms(
            &req.version,
            &req.title,
            &req.content,
            &req.summary,
            req.effective_date,
            auth_user.user_id,
        )
        .await?;

    Ok(Json(terms))
}

async fn update_terms(
    State(state): State<AppState>,
    auth_user: MmAuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTermsRequest>,
) -> ApiResult<Json<TermsOfService>> {
    if !auth_user.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::AdminRequired);
    }
    let repo = TermsRepository::new(&state.db);

    let terms = repo.get_terms_by_id(id).await?;
    if terms.is_none() {
        return Err(AppError::TermsNotFound);
    }

    let updated = repo
        .update_terms(
            id,
            &req.title,
            &req.content,
            &req.summary,
            &req.effective_date,
        )
        .await?;

    Ok(Json(updated))
}

async fn delete_terms(
    State(state): State<AppState>,
    auth_user: MmAuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    if !auth_user.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::AdminRequired);
    }
    let repo = TermsRepository::new(&state.db);

    // Check if terms is active
    let is_active = repo.is_terms_active(id).await?;
    if is_active == Some(true) {
        return Err(AppError::Validation(
            "Cannot delete active terms. Deactivate first.".to_string(),
        ));
    }

    repo.delete_terms(id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn activate_terms(
    State(state): State<AppState>,
    auth_user: MmAuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<TermsOfService>> {
    if !auth_user.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::AdminRequired);
    }
    let repo = TermsRepository::new(&state.db);
    let terms = repo
        .activate_terms(id)
        .await?
        .ok_or_else(|| AppError::TermsNotFound)?;

    Ok(Json(terms))
}

async fn get_terms_stats(
    State(state): State<AppState>,
    auth_user: MmAuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<TermsStats>> {
    if !auth_user.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::AdminRequired);
    }
    let repo = TermsRepository::new(&state.db);
    let stats = repo.get_terms_stats(id).await?;
    Ok(Json(stats))
}

async fn get_all_terms_stats(
    State(state): State<AppState>,
    auth_user: MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    if !auth_user.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(AppError::AdminRequired);
    }
    let repo = TermsRepository::new(&state.db);

    // Get current active terms
    let current_terms = repo.get_current_terms().await?;

    let Some(terms) = current_terms else {
        return Ok(Json(serde_json::json!({
            "has_active_terms": false,
            "total_users": 0,
            "accepted_count": 0,
            "pending_count": 0,
            "acceptance_rate": 0.0,
            "pending_users": []
        })));
    };

    // Get stats
    let total_users = repo.count_active_users().await?;
    let accepted_count = repo.count_accepted_users(terms.id).await?;
    let pending_users = repo.get_pending_users(terms.id, 50).await?;

    let pending_count = total_users - accepted_count;
    let acceptance_rate = if total_users > 0 {
        (accepted_count as f64 / total_users as f64) * 100.0
    } else {
        0.0
    };

    Ok(Json(serde_json::json!({
        "has_active_terms": true,
        "current_terms": terms,
        "total_users": total_users,
        "accepted_count": accepted_count,
        "pending_count": pending_count,
        "acceptance_rate": acceptance_rate,
        "pending_users": pending_users
    })))
}
