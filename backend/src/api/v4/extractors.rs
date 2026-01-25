use axum::{
    extract::FromRequestParts,
    http::{header::{AUTHORIZATION, HeaderName}, request::Parts},
};
use uuid::Uuid;

use crate::api::AppState;
use crate::auth::{validate_token, Claims};
use crate::error::AppError;
use crate::auth::middleware::FromRef;

pub struct MmAuthUser {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
    pub org_id: Option<Uuid>,
}

impl From<Claims> for MmAuthUser {
    fn from(claims: Claims) -> Self {
        Self {
            user_id: claims.sub,
            email: claims.email,
            role: claims.role,
            org_id: claims.org_id,
        }
    }
}

impl<S> FromRequestParts<S> for MmAuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let token = if let Some(auth_header) = parts.headers.get(AUTHORIZATION) {
            let auth_str = auth_header.to_str().map_err(|_| AppError::Unauthorized("Invalid authorization header".to_string()))?;
            if auth_str.starts_with("Bearer ") {
                auth_str[7..].trim()
            } else if auth_str.starts_with("Token ") {
                auth_str[6..].trim()
            } else {
                 auth_str.trim()
            }
        } else if let Some(token_header) = parts.headers.get(HeaderName::from_static("token")) {
             token_header.to_str().map_err(|_| AppError::Unauthorized("Invalid token header".to_string()))?.trim()
        } else {
            return Err(AppError::Unauthorized("Missing authorization header".to_string()));
        };

        let token_data = validate_token(token, &app_state.jwt_secret)?;

        Ok(MmAuthUser::from(token_data.claims))
    }
}
