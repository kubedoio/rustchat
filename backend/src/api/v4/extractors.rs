use axum::{
    extract::FromRequestParts,
    http::{
        header::{HeaderName, AUTHORIZATION},
        request::Parts,
    },
};
use uuid::Uuid;

use crate::api::AppState;
use crate::auth::middleware::{ensure_user_access_active, FromRef};
use crate::auth::policy::{AuthzResult, Permission, PolicyEngine};
use crate::auth::{validate_token_with_policy, Claims};
use crate::error::AppError;

pub struct MmAuthUser {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
    #[allow(dead_code)]
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

impl MmAuthUser {
    pub fn has_role(&self, role: &str) -> bool {
        self.role.split_whitespace().any(|r| r == role)
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        matches!(
            PolicyEngine::check_permission(&self.role, permission),
            AuthzResult::Allow
        )
    }

    pub fn can_access_owned(&self, owner_id: Uuid, permission: &Permission) -> bool {
        matches!(
            PolicyEngine::check_ownership(&self.role, permission, self.user_id, owner_id),
            AuthzResult::Allow
        )
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
            let auth_str = auth_header
                .to_str()
                .map_err(|_| AppError::Unauthorized("Invalid authorization header".to_string()))?;
            if let Some(stripped) = auth_str.strip_prefix("Bearer ") {
                stripped.trim()
            } else if let Some(stripped) = auth_str.strip_prefix("Token ") {
                stripped.trim()
            } else {
                auth_str.trim()
            }
        } else if let Some(token_header) = parts.headers.get(HeaderName::from_static("token")) {
            token_header
                .to_str()
                .map_err(|_| AppError::Unauthorized("Invalid token header".to_string()))?
                .trim()
        } else if let Some(cookie_header) = parts.headers.get(HeaderName::from_static("cookie")) {
            // Parse cookies to find MMAUTHTOKEN - used by img/video tags that can't send headers
            let cookie_str = cookie_header
                .to_str()
                .map_err(|_| AppError::Unauthorized("Invalid cookie header".to_string()))?;

            cookie_str
                .split(';')
                .map(|s| s.trim())
                .find_map(|cookie| cookie.strip_prefix("MMAUTHTOKEN="))
                .ok_or_else(|| AppError::Unauthorized("Missing MMAUTHTOKEN cookie".to_string()))?
        } else {
            return Err(AppError::Unauthorized(
                "Missing authorization header".to_string(),
            ));
        };

        let token_data = validate_token_with_policy(
            token,
            &app_state.jwt_secret,
            app_state.jwt_issuer.as_deref(),
            app_state.jwt_audience.as_deref(),
        )?;
        ensure_user_access_active(&app_state, token_data.claims.sub).await?;

        Ok(MmAuthUser::from(token_data.claims))
    }
}

#[cfg(test)]
mod tests {
    use super::MmAuthUser;
    use uuid::Uuid;

    #[test]
    fn mm_auth_user_role_helpers_support_multi_role_strings() {
        let user = MmAuthUser {
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            role: "member admin".to_string(),
            org_id: None,
        };

        assert!(user.has_role("admin"));
        assert!(user.has_role("member"));
        assert!(!user.has_role("org_admin"));
    }

    #[test]
    fn mm_auth_user_permission_helpers_work() {
        use crate::auth::policy::permissions;

        let user = MmAuthUser {
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            role: "org_admin member".to_string(),
            org_id: None,
        };

        assert!(user.has_permission(&permissions::USER_MANAGE));
        assert!(!user.has_permission(&permissions::ADMIN_FULL));
        assert!(user.can_access_owned(user.user_id, &permissions::ADMIN_FULL));
    }
}
