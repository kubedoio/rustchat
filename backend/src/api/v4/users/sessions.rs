use axum::{
    body::Bytes,
    extract::{Json, State},
    http::HeaderMap,
    response::IntoResponse,
};
use serde::Deserialize;

use super::MmAuthUser;
use crate::api::AppState;
use crate::auth::clear_auth_cookie;
use crate::error::{ApiResult, AppError};

#[derive(Deserialize)]
pub struct AttachDeviceRequest {
    pub device_id: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    pub platform: Option<String>,
    // Fields sent by mobile app but not used
    #[serde(default)]
    pub device_notification_disabled: Option<String>,
    #[serde(default)]
    pub mobile_version: Option<String>,
}

#[derive(Deserialize)]
pub struct DetachDeviceRequest {
    pub device_id: String,
}

/// Extract FCM token and platform from mobile app's device_id format
/// Format: "android_rn-v2:FCM_TOKEN" or "apple_rn-v2:FCM_TOKEN"
pub fn parse_mobile_device_id(device_id: &str) -> (String, String, String) {
    // device_id format: "prefix:FCM_TOKEN"
    // prefix examples: android_rn-v2, apple_rn-v2, android_rn-v2beta

    let parts: Vec<&str> = device_id.splitn(2, ':').collect();
    if parts.len() == 2 {
        let prefix = parts[0];
        let token = parts[1];

        // Extract platform from prefix
        let platform = if prefix.starts_with("android") {
            "android"
        } else if prefix.starts_with("apple") || prefix.starts_with("ios") {
            "ios"
        } else {
            "unknown"
        };

        // Return full device_id as stored ID, the token, and platform
        (
            device_id.to_string(),
            token.to_string(),
            platform.to_string(),
        )
    } else {
        // No colon found, treat entire string as device_id with no token
        (device_id.to_string(), String::new(), "unknown".to_string())
    }
}

pub fn resolve_device_token(parsed_token: &str, request_token: Option<&str>) -> Option<String> {
    if !parsed_token.is_empty() {
        return Some(parsed_token.to_string());
    }

    request_token
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
}

pub async fn attach_device(
    State(state): State<AppState>,
    auth: MmAuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<impl IntoResponse> {
    use tracing::{info, warn};

    let body_str = String::from_utf8_lossy(&body);
    info!(user_id = %auth.user_id, body = %body_str, "attach_device received");

    // Try to parse body, but accept empty/malformed requests gracefully
    let input: AttachDeviceRequest = match super::parse_body::<AttachDeviceRequest>(
        &headers,
        &body,
        "Invalid device body",
    ) {
        Ok(v) => {
            info!(user_id = %auth.user_id, device_id = ?v.device_id, "attach_device parsed successfully");
            v
        }
        Err(e) => {
            warn!(user_id = %auth.user_id, error = %e, body = %body_str, "attach_device parse error");
            // Return OK for malformed requests - mobile sends various formats
            return Ok(Json(serde_json::json!({"status": "OK"})));
        }
    };

    // Only insert if we have device_id
    if let Some(device_id) = input.device_id {
        // Parse device_id to extract FCM token (it's embedded in the device_id!)
        let (device_id_stored, parsed_token, platform) = parse_mobile_device_id(&device_id);
        let resolved_token = resolve_device_token(&parsed_token, input.token.as_deref());

        info!(
            user_id = %auth.user_id,
            device_id = %device_id_stored,
            has_token = resolved_token.is_some(),
            token_preview = %resolved_token.as_deref().map(|token| &token[..20.min(token.len())]).unwrap_or(""),
            platform = %platform,
            device_notification_disabled = ?input.device_notification_disabled,
            mobile_version = ?input.mobile_version,
            "Extracted token from device_id"
        );

        match sqlx::query(
            r#"
            INSERT INTO user_devices (user_id, device_id, token, platform)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, device_id)
            DO UPDATE SET token = $3, platform = $4, last_seen_at = NOW()
            "#,
        )
        .bind(auth.user_id)
        .bind(&device_id_stored)
        .bind(resolved_token.as_deref())
        .bind(&platform)
        .execute(&state.db)
        .await
        {
            Ok(result) => {
                info!(
                    user_id = %auth.user_id,
                    device_id = %device_id_stored,
                    rows_affected = result.rows_affected(),
                    "attach_device stored device registration"
                );
            }
            Err(e) => {
                warn!(
                    user_id = %auth.user_id,
                    device_id = %device_id_stored,
                    platform = %platform,
                    has_token = resolved_token.is_some(),
                    error = %e,
                    "attach_device failed to store device registration"
                );
            }
        }
    }

    Ok(Json(serde_json::json!({"status": "OK"})))
}

pub async fn detach_device(
    State(state): State<AppState>,
    auth: MmAuthUser,
    Json(input): Json<DetachDeviceRequest>,
) -> ApiResult<impl IntoResponse> {
    sqlx::query("DELETE FROM user_devices WHERE user_id = $1 AND device_id = $2")
        .bind(auth.user_id)
        .bind(input.device_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "OK"})))
}

pub async fn get_sessions() -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

pub async fn logout(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    let mut headers = HeaderMap::new();
    let cookie_header =
        axum::http::HeaderValue::from_str(&clear_auth_cookie(state.config.is_production()))
            .map_err(|_| AppError::Internal("Invalid cookie characters".into()))?;
    headers.insert(axum::http::header::SET_COOKIE, cookie_header);

    Ok((headers, Json(serde_json::json!({"status": "OK"}))))
}

pub async fn get_user_sessions(
    auth: MmAuthUser,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let _ = super::user_sidebar_categories::resolve_user_id(&user_id, &auth)?;
    Ok(Json(vec![]))
}

pub async fn revoke_user_session(
    auth: MmAuthUser,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let _ = super::user_sidebar_categories::resolve_user_id(&user_id, &auth)?;
    let _value: serde_json::Value = super::parse_body(&headers, &body, "Invalid session body")?;
    Ok(Json(serde_json::json!({"status": "OK"})))
}

pub async fn revoke_user_sessions(
    auth: MmAuthUser,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let _ = super::user_sidebar_categories::resolve_user_id(&user_id, &auth)?;
    Ok(Json(serde_json::json!({"status": "OK"})))
}

pub async fn revoke_all_sessions() -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}
