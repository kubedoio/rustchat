//! Auth API endpoints

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};

use super::AppState;
use crate::auth::{create_token, hash_password, verify_password, AuthUser};
use crate::error::{ApiResult, AppError};
use crate::models::{AuthResponse, CreateUser, LoginRequest, User, UserResponse};

/// Build auth routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me))
        .route("/policy", get(get_auth_policy))
}

/// Get current authentication policy
async fn get_auth_policy(
    State(state): State<AppState>,
) -> ApiResult<Json<crate::models::AuthConfig>> {
    let config = crate::services::auth_config::get_password_rules(&state.db).await?;
    Ok(Json(config))
}

/// Register a new user
async fn register(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> ApiResult<Json<AuthResponse>> {
    // Validate input
    if input.username.len() < 3 {
        return Err(AppError::Validation(
            "Username must be at least 3 characters".to_string(),
        ));
    }

    // Enforce password complexity
    let config = crate::services::auth_config::get_password_rules(&state.db).await?;
    crate::services::auth_config::validate_password(&input.password, &config)?;

    if !input.email.contains('@') {
        return Err(AppError::Validation("Invalid email format".to_string()));
    }

    // Check if email already exists
    let existing: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = $1")
        .bind(&input.email)
        .fetch_optional(&state.db)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("Email already registered".to_string()));
    }

    // Check if username already exists
    let existing_username: Option<User> = sqlx::query_as("SELECT * FROM users WHERE username = $1")
        .bind(&input.username)
        .fetch_optional(&state.db)
        .await?;

    if existing_username.is_some() {
        return Err(AppError::Conflict("Username already taken".to_string()));
    }

    // Hash password
    let password_hash = hash_password(&input.password)?;

    // Insert user
    let user: User = sqlx::query_as(
        r#"
        INSERT INTO users (username, email, password_hash, display_name, org_id, role)
        VALUES ($1, $2, $3, $4, $5, 'member')
        RETURNING *
        "#,
    )
    .bind(&input.username)
    .bind(&input.email)
    .bind(&password_hash)
    .bind(&input.display_name)
    .bind(input.org_id)
    .fetch_one(&state.db)
    .await?;

    // Generate token
    let token = create_token(
        user.id,
        &user.email,
        &user.role,
        user.org_id,
        &state.jwt_secret,
        state.jwt_expiry_hours,
    )?;

    Ok(Json(AuthResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: state.jwt_expiry_hours * 3600,
        user: UserResponse::from(user),
    }))
}

/// Login with email and password
async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> ApiResult<Json<AuthResponse>> {
    // Find user by email
    let user: User = sqlx::query_as("SELECT * FROM users WHERE email = $1 AND is_active = true")
        .bind(&input.email)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

    // Verify password
    if !verify_password(&input.password, &user.password_hash)? {
        return Err(AppError::Unauthorized(
            "Invalid email or password".to_string(),
        ));
    }

    // Update last login
    sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await?;

    // Generate token
    let token = create_token(
        user.id,
        &user.email,
        &user.role,
        user.org_id,
        &state.jwt_secret,
        state.jwt_expiry_hours,
    )?;

    Ok(Json(AuthResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: state.jwt_expiry_hours * 3600,
        user: UserResponse::from(user),
    }))
}

/// Get current authenticated user
async fn me(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Json<UserResponse>> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(UserResponse::from(user)))
}
