//! Auth API endpoints

use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    middleware,
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;

use super::AppState;
use crate::auth::{build_auth_cookie, clear_auth_cookie};
use crate::auth::{create_token_with_policy, hash_password, verify_password, AuthUser};
use crate::error::{ApiResult, AppError};
use crate::middleware::rate_limit::{self, RateLimitConfig};
use crate::models::{
    validate_username_token, AuthResponse, CreateUser, LoginRequest, User, UserResponse,
};
use crate::repositories::{SystemRepository, UserRepository};
use crate::services::membership_policies::apply_auto_membership_for_new_user;
use crate::services::password_reset::{
    request_password_reset, reset_password, validate_token, PasswordResetError,
};
use crate::services::turnstile;

/// Build auth routes
pub fn router(state: AppState) -> Router<AppState> {
    let registration_routes =
        Router::new()
            .route("/register", post(register))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                crate::middleware::rate_limit::register_ip_rate_limit,
            ));
    let login_routes = Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::rate_limit::auth_ip_rate_limit,
        ));
    let verification_routes = Router::new()
        .route("/verify-email", post(verify_email))
        .route("/resend-verification", post(resend_verification))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::rate_limit::auth_ip_rate_limit,
        ));
    let password_reset_routes = Router::new()
        .route("/password/forgot", post(forgot_password))
        .route("/password/reset", post(reset_password_handler))
        .route("/password/validate", post(validate_token_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::rate_limit::password_reset_ip_rate_limit,
        ));

    Router::new()
        .merge(registration_routes)
        .merge(login_routes)
        .merge(verification_routes)
        .merge(password_reset_routes)
        .route("/me", get(me))
        .route("/policy", get(get_auth_policy))
        .route("/config", get(get_public_auth_config))
}

/// Get current authentication policy
async fn get_auth_policy(
    State(state): State<AppState>,
) -> ApiResult<Json<crate::models::AuthConfig>> {
    let config = crate::services::auth_config::get_password_rules(&state.db).await?;
    Ok(Json(config))
}

/// Get public auth configuration (safe to expose to frontend)
async fn get_public_auth_config(
    State(state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let auth_config = crate::services::auth_config::get_password_rules(&state.db).await?;
    Ok(Json(serde_json::json!({
        "turnstile": {
            "enabled": state.config.turnstile.enabled,
            "site_key": state.config.turnstile.site_key,
        },
        "registration_enabled": auth_config.allow_registration,
        "password_reset_enabled": true,
    })))
}

/// Register a new user
///
/// If password is provided, user is registered with that password.
/// If password is not provided, a password setup email is sent and user must set password via email link.
async fn register(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(input): Json<CreateUser>,
) -> ApiResult<Json<serde_json::Value>> {
    // Check if public registration is enabled
    let auth_config = crate::services::auth_config::get_password_rules(&state.db).await?;
    if !auth_config.allow_registration {
        return Err(AppError::Forbidden(
            "Public registration is disabled".to_string(),
        ));
    }

    // Check honeypot - if filled, likely a bot
    if let Some(ref honeypot) = input.honeypot {
        if !honeypot.is_empty() {
            tracing::warn!(
                "Honeypot field filled, likely bot attempt from {}",
                addr.ip()
            );
            // Return generic error without revealing honeypot detection
            return Err(AppError::Validation("Invalid request".to_string()));
        }
    }

    // Verify Turnstile token if enabled
    if state.config.turnstile.enabled {
        let token = input
            .turnstile_token
            .as_deref()
            .ok_or_else(|| AppError::Validation("Verification required".to_string()))?;

        let remote_ip = Some(addr.ip().to_string());
        if let Err(e) = turnstile::verify_token(
            &state.config.turnstile.secret_key,
            token,
            remote_ip.as_deref(),
        )
        .await
        {
            tracing::warn!("Turnstile verification failed: {}", e);
            return Err(AppError::Validation(
                "Verification failed. Please try again.".to_string(),
            ));
        }
    }

    // Validate input
    validate_username_token(&input.username)
        .map_err(|message| AppError::Validation(message.to_string()))?;

    if !input.email.contains('@') {
        return Err(AppError::Validation("Invalid email format".to_string()));
    }

    let repo = UserRepository::new(&state.db);

    // Check if email already exists
    if repo.get_by_email(&input.email).await?.is_some() {
        return Err(AppError::Conflict("Email already registered".to_string()));
    }

    // Check if username already exists
    if repo.get_by_username(&input.username).await?.is_some() {
        return Err(AppError::Conflict("Username already taken".to_string()));
    }

    // Determine if this is passwordless registration
    let password_ref = input.password.as_ref().filter(|p| !p.is_empty());
    let has_password = password_ref.is_some();

    // Validate password if provided
    let password_hash = if let Some(password) = password_ref {
        let config = crate::services::auth_config::get_password_rules(&state.db).await?;
        crate::services::auth_config::validate_password(password, &config)?;
        Some(hash_password(password)?)
    } else {
        None
    };

    // Passwordless registrations stay inactive until password setup succeeds.
    let is_active = has_password;

    // Insert user (email_verified defaults to false, password_hash may be NULL for passwordless)
    let user: User = repo
        .create_user(
            &input.username,
            &input.email,
            &password_hash,
            &input.display_name,
            input.org_id,
            is_active,
        )
        .await?;

    // Seed default preferences for the new user
    repo.seed_default_preferences(user.id).await?;

    // Apply auto-membership policies for the new user (global policies that add to teams/channels)
    match apply_auto_membership_for_new_user(&state, user.id).await {
        Ok(audit_entries) => {
            let success_count = audit_entries
                .iter()
                .filter(|e| e.status == "success" && e.action == "add")
                .count();
            if success_count > 0 {
                tracing::info!(
                    "Applied auto-membership policies for new user {}: {} memberships added",
                    user.id,
                    success_count
                );
            }
        }
        Err(e) => {
            // Don't fail registration if policy application fails, just log the error
            tracing::error!(
                "Failed to apply auto-membership policies for new user {}: {}",
                user.id,
                e
            );
        }
    }

    // Fetch site_url from server_config
    let site_url = SystemRepository::new(&state.db)
        .get_server_config()
        .await
        .ok()
        .and_then(|cfg| {
            let url = cfg.site.0.site_url;
            if url.is_empty() {
                None
            } else {
                Some(url)
            }
        });

    if let Some(site_url) = site_url {
        if has_password {
            // Send verification email for users who provided password
            let verification_base_url = format!("{}/verify-email", site_url);
            match crate::services::email_verification::send_verification_email(
                &state.db,
                user.id,
                &user.username,
                &user.email,
                &verification_base_url,
            )
            .await
            {
                Ok(_) => {
                    tracing::info!("Verification email sent to {}", user.email);
                }
                Err(e) => {
                    tracing::warn!("Failed to send verification email: {}", e);
                }
            }

            // Generate token for immediate login
            let token = create_token_with_policy(
                user.id,
                &user.email,
                &user.role,
                user.org_id,
                &state.jwt_secret,
                state.jwt_issuer.as_deref(),
                state.jwt_audience.as_deref(),
                state.jwt_expiry_hours,
            )?;

            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Registration successful. Please check your email to verify your account.",
                "requires_password_setup": false,
                "token": token,
                "user": UserResponse::from(user)
            })))
        } else {
            // Passwordless registration: send password setup email
            match crate::services::password_reset::send_password_setup_email(
                &state.db,
                user.id,
                &user.username,
                &user.email,
                &site_url,
            )
            .await
            {
                Ok(_) => {
                    tracing::info!("Password setup email sent to {}", user.email);
                }
                Err(e) => {
                    tracing::error!("Failed to send password setup email: {}", e);
                    // Don't fail registration, but inform user
                }
            }

            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Registration successful. Please check your email to set your password.",
                "requires_password_setup": true,
                "email": user.email
            })))
        }
    } else {
        tracing::warn!("site_url not configured, skipping email sending");

        if has_password {
            // Generate token for immediate login
            let token = create_token_with_policy(
                user.id,
                &user.email,
                &user.role,
                user.org_id,
                &state.jwt_secret,
                state.jwt_issuer.as_deref(),
                state.jwt_audience.as_deref(),
                state.jwt_expiry_hours,
            )?;

            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Registration successful.",
                "requires_password_setup": false,
                "token": token,
                "user": UserResponse::from(user)
            })))
        } else {
            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Registration successful. Please contact administrator to set your password.",
                "requires_password_setup": true,
                "email": user.email
            })))
        }
    }
}

/// Login with email and password
async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> ApiResult<(HeaderMap, Json<AuthResponse>)> {
    let email = input.email.clone();
    match login_inner(State(state.clone()), Json(input)).await {
        Ok(response) => {
            let user_id = response.1.user.id;
            let _ = crate::services::audit::audit(
                &state.db,
                Some(user_id),
                crate::services::audit::AuditAction::LoginSuccess,
                "user",
                Some(user_id),
                serde_json::json!({ "email": email }),
            )
            .await;
            Ok(response)
        }
        Err(err) => {
            let _ = crate::services::audit::audit(
                &state.db,
                None,
                crate::services::audit::AuditAction::LoginFailed,
                "user",
                None,
                serde_json::json!({ "email": email }),
            )
            .await;
            Err(err)
        }
    }
}

async fn login_inner(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> ApiResult<(HeaderMap, Json<AuthResponse>)> {
    // Find user by email
    let user = UserRepository::new(&state.db)
        .get_active_by_email(&input.email)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

    enforce_password_login_allowed(&state, &user.email).await?;

    // Keep per-account throttle in addition to centralized per-IP middleware.
    if state.config.security.rate_limit_enabled {
        let config =
            RateLimitConfig::auth_per_minute(state.config.security.rate_limit_auth_per_minute);
        let user_key = format!("user:{}", user.id);
        let user_result = rate_limit::check_rate_limit(&state.redis, &config, &user_key).await?;

        if !user_result.allowed {
            tracing::warn!(user_id = %user.id, "Rate limit exceeded for user login");
            return Err(AppError::TooManyRequests(
                "Too many login attempts. Please try again later.".to_string(),
                Some(config.window_secs),
            ));
        }
    }

    // Verify password (OAuth users or users pending password setup cannot login with password)
    let password_hash = user.password_hash.as_deref().ok_or_else(|| {
        if user.email_verified {
            AppError::Unauthorized(
                "Please set your password using the link sent to your email.".to_string(),
            )
        } else {
            AppError::Unauthorized(
                "Please verify your email and set your password first.".to_string(),
            )
        }
    })?;

    if !verify_password(&input.password, password_hash)? {
        return Err(AppError::Unauthorized(
            "Invalid email or password".to_string(),
        ));
    }

    // Update last login
    UserRepository::new(&state.db)
        .update_last_login(user.id)
        .await?;

    // Generate token
    let token = create_token_with_policy(
        user.id,
        &user.email,
        &user.role,
        user.org_id,
        &state.jwt_secret,
        state.jwt_issuer.as_deref(),
        state.jwt_audience.as_deref(),
        state.jwt_expiry_hours,
    )?;

    let max_age = state.jwt_expiry_hours.saturating_mul(3600);
    let mut headers = HeaderMap::new();
    let cookie_value = build_auth_cookie(&token, max_age, state.config.is_production());
    let cookie_header = axum::http::HeaderValue::from_str(&cookie_value)
        .map_err(|_| AppError::Internal("Invalid cookie characters".into()))?;
    headers.insert(axum::http::header::SET_COOKIE, cookie_header);

    Ok((
        headers,
        Json(AuthResponse {
            token,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt_expiry_hours * 3600,
            user: UserResponse::from(user),
        }),
    ))
}

async fn logout(State(state): State<AppState>) -> ApiResult<(HeaderMap, Json<serde_json::Value>)> {
    let mut headers = HeaderMap::new();
    let cookie_header =
        axum::http::HeaderValue::from_str(&clear_auth_cookie(state.config.is_production()))
            .map_err(|_| AppError::Internal("Invalid cookie characters".into()))?;
    headers.insert(axum::http::header::SET_COOKIE, cookie_header);

    Ok((
        headers,
        Json(serde_json::json!({
            "success": true
        })),
    ))
}

async fn enforce_password_login_allowed(state: &AppState, user_email: &str) -> ApiResult<()> {
    let auth_config = crate::services::auth_config::get_password_rules(&state.db).await?;
    if !auth_config.require_sso {
        return Ok(());
    }

    let email_lc = user_email.trim().to_ascii_lowercase();
    let allowed = auth_config
        .sso_break_glass_emails
        .iter()
        .any(|email| email.trim().eq_ignore_ascii_case(&email_lc));
    if allowed {
        return Ok(());
    }

    Err(AppError::BadRequest(
        "Password login is disabled because SSO is required".to_string(),
    ))
}

/// Verify email with token
#[derive(Debug, serde::Deserialize)]
struct VerifyEmailRequest {
    token: String,
}

async fn verify_email(
    State(state): State<AppState>,
    Json(input): Json<VerifyEmailRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let user_id =
        crate::services::email_verification::verify_token(&state.db, &input.token, "registration")
            .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Email verified successfully",
        "user_id": user_id.to_string()
    })))
}

/// Resend verification email
#[derive(Debug, serde::Deserialize)]
struct ResendVerificationRequest {
    email: String,
}

async fn resend_verification(
    State(state): State<AppState>,
    Json(input): Json<ResendVerificationRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Find user by email
    let user = UserRepository::new(&state.db)
        .get_active_by_email(&input.email)
        .await?;

    let user = match user {
        Some(u) => u,
        None => {
            // Return success even if user not found to prevent email enumeration
            return Ok(Json(serde_json::json!({
                "success": true,
                "message": "If the email exists, a verification email has been sent"
            })));
        }
    };

    if user.email_verified {
        return Ok(Json(serde_json::json!({
            "success": true,
            "message": "Email is already verified"
        })));
    }

    // Send verification email
    // Fetch site_url from server_config
    let site_url = SystemRepository::new(&state.db)
        .get_server_config()
        .await
        .ok()
        .and_then(|cfg| {
            let url = cfg.site.0.site_url;
            if url.is_empty() {
                None
            } else {
                Some(url)
            }
        });

    let verification_result = if let Some(site_url) = site_url {
        let verification_base_url = format!("{}/verify-email", site_url);
        crate::services::email_verification::send_verification_email(
            &state.db,
            user.id,
            &user.username,
            &user.email,
            &verification_base_url,
        )
        .await
    } else {
        tracing::warn!("site_url not configured, cannot send verification email");
        Ok(())
    };

    match verification_result {
        Ok(_) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Verification email sent"
        }))),
        Err(_) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "If the email exists, a verification email has been sent"
        }))),
    }
}

/// Get current authenticated user
async fn me(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Json<UserResponse>> {
    let user = UserRepository::new(&state.db)
        .get_by_id(auth.user_id)
        .await?
        .ok_or_else(|| AppError::UserNotFound)?;

    Ok(Json(UserResponse::from(user)))
}

// ============================================
// Password Reset Handlers
// ============================================

#[derive(Debug, serde::Deserialize)]
struct ForgotPasswordRequest {
    email: String,
    /// Cloudflare Turnstile token (bot protection)
    #[serde(rename = "cf-turnstile-response")]
    turnstile_token: Option<String>,
    /// Honeypot field - should be empty (bots usually fill this)
    #[serde(rename = "website")]
    honeypot: Option<String>,
}

/// Request password reset email
/// Returns same response regardless of email existence (anti-enumeration)
async fn forgot_password(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(input): Json<ForgotPasswordRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Check honeypot - if filled, likely a bot
    if let Some(ref honeypot) = input.honeypot {
        if !honeypot.is_empty() {
            tracing::warn!(
                "Honeypot field filled, likely bot attempt from {}",
                addr.ip()
            );
            // Return success response to not reveal honeypot detection
            return Ok(Json(serde_json::json!({
                "success": true,
                "message": "If an account with that email exists, you will receive a password reset link"
            })));
        }
    }

    // Verify Turnstile token if enabled
    if state.config.turnstile.enabled {
        let token = match input.turnstile_token.as_deref() {
            Some(t) if !t.is_empty() => t,
            _ => {
                return Ok(Json(serde_json::json!({
                    "success": true,
                    "message": "If an account with that email exists, you will receive a password reset link"
                })));
            }
        };

        let remote_ip = Some(addr.ip().to_string());
        if let Err(e) = turnstile::verify_token(
            &state.config.turnstile.secret_key,
            token,
            remote_ip.as_deref(),
        )
        .await
        {
            tracing::warn!("Turnstile verification failed: {}", e);
            // Return success response to not reveal verification failure
            return Ok(Json(serde_json::json!({
                "success": true,
                "message": "If an account with that email exists, you will receive a password reset link"
            })));
        }
    }

    // Get IP address from connection
    let ip_address = Some(addr.ip());

    // Request password reset (always returns Ok for anti-enumeration)
    let result = request_password_reset(
        &state.db,
        &input.email,
        ip_address,
        None, // user_agent could be extracted from headers if needed
    )
    .await;

    // Log but don't expose errors
    if let Err(ref e) = result {
        tracing::debug!("Password reset request result: {:?}", e);
    }

    // Always return same response
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "If an account with that email exists, you will receive a password reset link"
    })))
}

#[derive(Debug, serde::Deserialize)]
struct ResetPasswordRequest {
    token: String,
    new_password: String,
}

/// Reset password with token
async fn reset_password_handler(
    State(state): State<AppState>,
    Json(input): Json<ResetPasswordRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    match reset_password(&state.db, &input.token, &input.new_password).await {
        Ok(user_id) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Password reset successful",
            "user_id": user_id.to_string()
        }))),
        Err(
            PasswordResetError::TokenNotFound
            | PasswordResetError::TokenExpired
            | PasswordResetError::TokenAlreadyUsed,
        ) => Err(AppError::BadRequest("Invalid or expired token".to_string())),
        Err(PasswordResetError::InvalidPassword(msg)) => Err(AppError::Validation(msg)),
        Err(PasswordResetError::RateLimitExceeded) => Err(AppError::TooManyRequests(
            "Too many attempts. Please try again later.".to_string(),
            None,
        )),
        Err(e) => {
            tracing::error!("Password reset error: {}", e);
            Err(AppError::Internal("Failed to reset password".to_string()))
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct ValidateTokenRequest {
    token: String,
}

/// Validate token without consuming it (for UI)
async fn validate_token_handler(
    State(state): State<AppState>,
    Json(input): Json<ValidateTokenRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    match validate_token(&state.db, &input.token).await {
        Ok((user_id, email)) => Ok(Json(serde_json::json!({
            "valid": true,
            "user_id": user_id.to_string(),
            "email": email
        }))),
        Err(
            PasswordResetError::TokenNotFound
            | PasswordResetError::TokenExpired
            | PasswordResetError::TokenAlreadyUsed,
        ) => Ok(Json(serde_json::json!({
            "valid": false
        }))),
        Err(e) => {
            tracing::error!("Token validation error: {}", e);
            Err(AppError::Internal("Failed to validate token".to_string()))
        }
    }
}
