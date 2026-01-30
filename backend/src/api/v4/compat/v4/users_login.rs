use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};
use crate::error::ApiResult;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub login_id: String,
    pub password: String,
}

pub async fn login(
    Json(payload): Json<LoginRequest>
) -> ApiResult<impl IntoResponse> {
    // 1. Authenticate user
    // 2. Create session
    // 3. Return user object and set Token header (Mattermost style)
    
    println!("Login request for: {}", payload.login_id);
    
    Ok(Json(serde_json::json!({
        "id": "user_id_here",
        "username": payload.login_id,
        "email": "user@example.com"
    })))
}
