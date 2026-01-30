use axum::{
    extract::State,
    http::{header::HeaderName, HeaderMap},
    response::IntoResponse,
    Json,
};
use crate::api::AppState;
use crate::auth::{create_token, verify_password};
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::{models as mm};
use crate::models::user::User;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub login_id: String,
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    // 1. Authenticate user by username or email
    let user: User = sqlx::query_as(
        "SELECT * FROM users WHERE (username = $1 OR email = $1) AND is_active = true",
    )
    .bind(&payload.login_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    // 2. Verify password
    if !verify_password(&payload.password, &user.password_hash)? {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // 3. Create session token (JWT)
    let token = create_token(
        user.id,
        &user.email,
        &user.role,
        user.org_id,
        &state.jwt_secret,
        state.jwt_expiry_hours,
    )?;

    // 4. Update last login
    sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await?;

    // 5. Build response
    let mm_user: mm::User = user.into();
    
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("token"),
        token.parse().unwrap_or_else(|_| "invalid".parse().unwrap()),
    );
    headers.insert(
        axum::http::header::AUTHORIZATION,
        format!("Token {}", token).parse().unwrap_or_else(|_| "invalid".parse().unwrap()),
    );

    Ok((headers, Json(mm_user)))
}
