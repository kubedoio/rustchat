//! Public site configuration and metadata.

use super::AppState;
use crate::error::ApiResult;
use crate::models::server_config::{AuthConfig, SiteConfig};
use crate::repositories::SystemRepository;
use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
pub struct PublicConfig {
    pub site_name: String,
    pub logo_url: Option<String>,
    pub enable_sso: bool,
    pub require_sso: bool,
    pub post_edit_time_limit_seconds: i32,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/site/info", get(get_site_info))
}

/// Map the persisted site/authentication configuration into the public site
/// payload. Kept as a pure function so the mapping is unit-testable without a
/// database (RI-C04 pilot coverage).
fn build_public_config(site: &SiteConfig, auth: &AuthConfig) -> PublicConfig {
    PublicConfig {
        site_name: site.site_name.clone(),
        logo_url: site.logo_url.clone(),
        enable_sso: auth.enable_sso,
        require_sso: auth.require_sso,
        post_edit_time_limit_seconds: site.post_edit_time_limit_seconds,
    }
}

// Pilot extraction for RI-C04: persistence now lives behind the cohesive
// `SystemRepository` boundary instead of inline SQL in the handler. The
// `ServerConfig` row carries both the `site` and `authentication` JSONB
// columns, so a single repository read preserves the previous behavior and
// error mapping.
async fn get_site_info(State(state): State<AppState>) -> ApiResult<Json<PublicConfig>> {
    let config = SystemRepository::new(&state.db).get_server_config().await?;
    Ok(Json(build_public_config(
        &config.site.0,
        &config.authentication.0,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn auth(enable_sso: bool, require_sso: bool) -> AuthConfig {
        serde_json::from_value(json!({
            "enable_sso": enable_sso,
            "require_sso": require_sso,
        }))
        .expect("AuthConfig deserializes from partial JSON using serde defaults")
    }

    #[test]
    fn maps_site_and_auth_fields() {
        let site = SiteConfig {
            site_name: "RustChat".to_string(),
            logo_url: Some("https://example.test/logo.png".to_string()),
            post_edit_time_limit_seconds: 300,
            ..Default::default()
        };
        let auth = auth(true, true);
        let cfg = build_public_config(&site, &auth);
        assert_eq!(cfg.site_name, "RustChat");
        assert_eq!(
            cfg.logo_url,
            Some("https://example.test/logo.png".to_string())
        );
        assert!(cfg.enable_sso);
        assert!(cfg.require_sso);
        assert_eq!(cfg.post_edit_time_limit_seconds, 300);
    }

    #[test]
    fn defaults_to_sso_disabled_when_absent() {
        let site = SiteConfig {
            site_name: "Solo".to_string(),
            logo_url: None,
            post_edit_time_limit_seconds: 0,
            ..Default::default()
        };
        let auth = auth(false, false);
        let cfg = build_public_config(&site, &auth);
        assert_eq!(cfg.site_name, "Solo");
        assert_eq!(cfg.logo_url, None);
        assert!(!cfg.enable_sso);
        assert!(!cfg.require_sso);
        assert_eq!(cfg.post_edit_time_limit_seconds, 0);
    }
}
