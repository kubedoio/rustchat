use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::models as mm;
use crate::mattermost_compat::{id::encode_mm_id, MM_VERSION};
use crate::models::server_config::SiteConfig;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/config/client", get(client_config))
        .route("/license/client", get(client_license))
}

async fn client_config(
    State(state): State<AppState>,
    Query(query): Query<LicenseQuery>,
) -> ApiResult<impl IntoResponse> {
    let site = sqlx::query_as::<_, (sqlx::types::Json<SiteConfig>,)>(
        "SELECT site FROM server_config WHERE id = 'default'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .map(|row| row.0 .0)
    .unwrap_or_default();

    let body = if matches!(query.format.as_deref(), Some("old")) {
        let diagnostic_id = diagnostic_id(&site);
        legacy_config(&site, &diagnostic_id)
    } else {
        serde_json::to_value(mm::Config {
            site_url: site.site_url.clone(),
            version: MM_VERSION.to_string(),
            enable_push_notifications: "false".to_string(),
            // Hardcoded diagnostic ID to satisfy client requirements
            diagnostic_id: "00000000-0000-0000-0000-000000000000".to_string(),
        })
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
    };

    Ok(Json(body))
}

#[derive(Deserialize)]
pub struct LicenseQuery {
    #[allow(dead_code)]
    pub format: Option<String>,
}

async fn client_license(
    State(_state): State<AppState>,
    Query(query): Query<LicenseQuery>,
) -> ApiResult<impl IntoResponse> {
    let body = if matches!(query.format.as_deref(), Some("old")) {
        serde_json::json!({
            "IsLicensed": "false"
        })
    } else {
        serde_json::to_value(mm::License {
            is_licensed: false,
            issued_at: 0,
            starts_at: 0,
            expires_at: 0,
        })
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
    };

    Ok(Json(body))
}

fn legacy_config(site: &SiteConfig, diagnostic_id: &str) -> serde_json::Value {
    use serde_json::{Map, Value};

    let mut map = Map::new();
    let insert = |map: &mut Map<String, Value>, key: &str, value: &str| {
        map.insert(key.to_string(), Value::String(value.to_string()));
    };

    insert(
        &mut map,
        "AboutLink",
        "https://docs.mattermost.com/about/product.html/",
    );
    insert(&mut map, "AllowDownloadLogs", "true");
    insert(
        &mut map,
        "AndroidAppDownloadLink",
        "https://mattermost.com/mattermost-android-app/",
    );
    insert(&mut map, "AndroidLatestVersion", "");
    insert(&mut map, "AndroidMinVersion", "");
    insert(
        &mut map,
        "AppDownloadLink",
        "https://mattermost.com/download/#mattermostApps",
    );
    insert(&mut map, "AppsPluginEnabled", "true");
    insert(&mut map, "AsymmetricSigningPublicKey", "");
    insert(&mut map, "BuildDate", "");
    insert(&mut map, "BuildEnterpriseReady", "false");
    insert(&mut map, "BuildHash", "");
    insert(&mut map, "BuildHashEnterprise", "none");
    insert(&mut map, "BuildNumber", "dev");
    insert(&mut map, "CWSURL", "");
    insert(&mut map, "CustomBrandText", "");
    insert(&mut map, "CustomDescriptionText", "");
    insert(&mut map, "DefaultClientLocale", "en");
    insert(&mut map, "DiagnosticId", diagnostic_id);
    insert(&mut map, "DiagnosticsEnabled", "true");
    insert(&mut map, "EmailLoginButtonBorderColor", "#2389D7");
    insert(&mut map, "EmailLoginButtonColor", "#0000");
    insert(&mut map, "EmailLoginButtonTextColor", "#2389D7");
    insert(&mut map, "EnableAskCommunityLink", "true");
    insert(&mut map, "EnableBotAccountCreation", "false");
    insert(&mut map, "EnableClientMetrics", "true");
    insert(&mut map, "EnableComplianceExport", "false");
    insert(&mut map, "EnableCustomBrand", "false");
    insert(&mut map, "EnableCustomEmoji", "true");
    insert(&mut map, "EnableDesktopLandingPage", "true");
    insert(&mut map, "EnableDiagnostics", "true");
    insert(&mut map, "EnableFile", "true");
    insert(&mut map, "EnableGuestAccounts", "false");
    insert(&mut map, "EnableJoinLeaveMessageByDefault", "true");
    insert(&mut map, "EnableLdap", "false");
    insert(&mut map, "EnableMultifactorAuthentication", "true");
    insert(&mut map, "EnableOpenServer", "false");
    insert(&mut map, "EnableSaml", "false");
    insert(&mut map, "EnableSignInWithEmail", "true");
    insert(&mut map, "EnableSignInWithUsername", "true");
    insert(&mut map, "EnableSignUpWithEmail", "true");
    insert(&mut map, "EnableSignUpWithGitLab", "true");
    insert(&mut map, "EnableSignUpWithGoogle", "false");
    insert(&mut map, "EnableSignUpWithOffice365", "false");
    insert(&mut map, "EnableSignUpWithOpenId", "false");
    insert(&mut map, "EnableUserCreation", "true");
    insert(&mut map, "EnableUserStatuses", "true");
    insert(&mut map, "EnforceMultifactorAuthentication", "false");
    insert(&mut map, "FeatureFlagAppsEnabled", "false");
    insert(&mut map, "FeatureFlagAttributeBasedAccessControl", "true");
    insert(&mut map, "FeatureFlagChannelBookmarks", "true");
    insert(&mut map, "FeatureFlagCloudAnnualRenewals", "false");
    insert(&mut map, "FeatureFlagCloudDedicatedExportUI", "false");
    insert(&mut map, "FeatureFlagCloudIPFiltering", "false");
    insert(&mut map, "FeatureFlagConsumePostHook", "false");
    insert(&mut map, "FeatureFlagCustomProfileAttributes", "true");
    insert(&mut map, "FeatureFlagDeprecateCloudFree", "false");
    insert(&mut map, "FeatureFlagEnableExportDirectDownload", "false");
    insert(&mut map, "FeatureFlagEnableRemoteClusterService", "false");
    insert(&mut map, "FeatureFlagEnableSharedChannelsDMs", "false");
    insert(
        &mut map,
        "FeatureFlagEnableSharedChannelsMemberSync",
        "false",
    );
    insert(&mut map, "FeatureFlagEnableSharedChannelsPlugins", "true");
    insert(
        &mut map,
        "FeatureFlagEnableSyncAllUsersForRemoteCluster",
        "false",
    );
    insert(
        &mut map,
        "FeatureFlagExperimentalAuditSettingsSystemConsoleUI",
        "false",
    );
    insert(&mut map, "FeatureFlagMoveThreadsEnabled", "false");
    insert(&mut map, "FeatureFlagNormalizeLdapDNs", "false");
    insert(&mut map, "FeatureFlagNotificationMonitoring", "true");
    insert(&mut map, "FeatureFlagOnboardingTourTips", "true");
    insert(&mut map, "FeatureFlagPermalinkPreviews", "false");
    insert(&mut map, "FeatureFlagStreamlinedMarketplace", "true");
    insert(&mut map, "FeatureFlagTestBoolFeature", "false");
    insert(&mut map, "FeatureFlagTestFeature", "off");
    insert(&mut map, "FeatureFlagWebSocketEventScope", "true");
    insert(&mut map, "FeatureFlagWysiwygEditor", "false");
    insert(&mut map, "FileLevel", "INFO");
    insert(&mut map, "ForgotPasswordLink", "");
    insert(&mut map, "GitLabButtonColor", "");
    insert(&mut map, "GitLabButtonText", "");
    insert(
        &mut map,
        "GuestAccountsEnforceMultifactorAuthentication",
        "false",
    );
    insert(&mut map, "HasImageProxy", "false");
    insert(&mut map, "HelpLink", "https://mattermost.com/default-help/");
    insert(&mut map, "HideGuestTags", "false");
    insert(
        &mut map,
        "IosAppDownloadLink",
        "https://mattermost.com/mattermost-ios-app/",
    );
    insert(&mut map, "IosLatestVersion", "");
    insert(&mut map, "IosMinVersion", "");
    insert(&mut map, "LdapLoginButtonBorderColor", "");
    insert(&mut map, "LdapLoginButtonColor", "");
    insert(&mut map, "LdapLoginButtonTextColor", "");
    insert(&mut map, "LdapLoginFieldName", "");
    insert(&mut map, "MobileExternalBrowser", "false");
    insert(&mut map, "NoAccounts", "false");
    insert(&mut map, "OpenIdButtonColor", "");
    insert(&mut map, "OpenIdButtonText", "");
    insert(&mut map, "PasswordEnableForgotLink", "true");
    insert(&mut map, "PasswordMinimumLength", "10");
    insert(&mut map, "PasswordRequireLowercase", "true");
    insert(&mut map, "PasswordRequireNumber", "true");
    insert(&mut map, "PasswordRequireSymbol", "true");
    insert(&mut map, "PasswordRequireUppercase", "true");
    insert(&mut map, "PluginsEnabled", "true");
    insert(&mut map, "PrivacyPolicyLink", "");
    insert(&mut map, "ReportAProblemLink", "");
    insert(&mut map, "ReportAProblemMail", "");
    insert(&mut map, "ReportAProblemType", "default");
    insert(&mut map, "SamlLoginButtonBorderColor", "");
    insert(&mut map, "SamlLoginButtonColor", "");
    insert(&mut map, "SamlLoginButtonText", "");
    insert(&mut map, "SamlLoginButtonTextColor", "");
    insert(&mut map, "ServiceEnvironment", "production");
    insert(&mut map, "SiteName", &site.site_name);
    insert(&mut map, "SiteURL", &site.site_url);
    insert(&mut map, "SupportEmail", "");
    insert(&mut map, "TelemetryId", diagnostic_id);
    insert(&mut map, "TermsOfServiceLink", "");
    insert(&mut map, "Version", MM_VERSION);
    insert(&mut map, "WebsocketPort", "80");
    insert(&mut map, "WebsocketSecurePort", "443");
    insert(&mut map, "WebsocketURL", "");

    Value::Object(map)
}
fn diagnostic_id(site: &SiteConfig) -> String {
    let seed = if !site.site_url.is_empty() {
        site.site_url.as_bytes()
    } else if !site.site_name.is_empty() {
        site.site_name.as_bytes()
    } else {
        b"rustchat"
    };

    let mut hasher = Sha256::new();
    hasher.update(seed);
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);

    Uuid::from_slice(&bytes)
        .map(encode_mm_id)
        .unwrap_or_else(|_| encode_mm_id(Uuid::new_v4()))
}
