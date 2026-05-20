use axum::body::Bytes;
use serde::Deserialize;

use crate::constants::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};
use crate::error::{ApiResult, AppError};

/// Validates email format: must contain @ with non-empty local and domain parts
pub fn is_valid_email(email: &str) -> bool {
    let email = email.trim();
    if email.len() < 3 || email.len() > 254 {
        return false;
    }
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let local = parts[0];
    let domain = parts[1];
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

pub fn status_ok() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({"status": "OK"}))
}

pub fn parse_body<T: serde::de::DeserializeOwned>(
    headers: &axum::http::HeaderMap,
    body: &Bytes,
    message: &str,
) -> ApiResult<T> {
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.starts_with("application/json") {
        serde_json::from_slice(body).map_err(|_| AppError::BadRequest(message.to_string()))
    } else {
        serde_json::from_slice(body)
            .or_else(|_| serde_urlencoded::from_bytes(body))
            .map_err(|_| AppError::BadRequest(message.to_string()))
    }
}

#[derive(Deserialize)]
pub struct GroupAssociationQuery {
    pub q: Option<String>,
    pub include_member_count: Option<bool>,
    pub filter_allow_reference: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub paginate: Option<bool>,
}

pub fn pagination_from_group_query(query: &GroupAssociationQuery) -> (bool, usize, usize) {
    let _ = query.include_member_count.unwrap_or(false);
    let paginate = query.paginate.unwrap_or(true);
    let page = query.page.unwrap_or(0).max(0) as usize;
    let per_page = query
        .per_page
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE) as usize;
    let offset = page.saturating_mul(per_page);
    (paginate, offset, per_page)
}
