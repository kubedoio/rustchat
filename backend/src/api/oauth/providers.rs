use crate::error::AppError;
use crate::models::SsoConfig;
use crate::services::oidc_discovery::{find_signing_key, OidcDiscoveryService};

use super::{UserInfo, GITHUB_API_URL, GITHUB_TOKEN_URL};
use super::utils::send_with_retry;

/// Token response from OAuth provider
#[derive(Debug, serde::Deserialize)]
pub(crate) struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: Option<i64>,
    refresh_token: Option<String>,
    id_token: Option<String>,
    scope: Option<String>,
}

/// User info from OIDC userinfo endpoint
#[derive(Debug, serde::Deserialize)]
pub(crate) struct UserInfoResponse {
    sub: String,
    email: Option<String>,
    email_verified: Option<bool>,
    name: Option<String>,
    preferred_username: Option<String>,
    given_name: Option<String>,
    family_name: Option<String>,
    picture: Option<String>,
    #[serde(flatten)]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Claims from ID token
#[derive(Debug, serde::Deserialize)]
pub(crate) struct IdTokenClaims {
    iss: String,
    sub: String,
    aud: String,
    exp: i64,
    iat: i64,
    nonce: Option<String>,
    email: Option<String>,
    email_verified: Option<bool>,
    name: Option<String>,
    preferred_username: Option<String>,
    given_name: Option<String>,
    family_name: Option<String>,
    picture: Option<String>,
    groups: Option<Vec<String>>,
    #[serde(flatten)]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

/// GitHub user info
#[derive(Debug, serde::Deserialize)]
pub(crate) struct GitHubUser {
    id: i64,
    login: String,
    name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
}

/// GitHub email info
#[derive(Debug, serde::Deserialize)]
pub(crate) struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

/// Exchange code for GitHub token and get user info
pub(crate) async fn exchange_github_token(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    config: &SsoConfig,
) -> Result<(String, UserInfo), AppError> {
    let client = reqwest::Client::new();

    // Exchange code for token
    let token_response = send_with_retry(
        client
            .post(GITHUB_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("client_id", client_id),
                ("client_secret", client_secret),
            ]),
        "GitHub token exchange failed",
    )
    .await?;

    if !token_response.status().is_success() {
        let status = token_response.status();
        let body = token_response.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "GitHub token exchange failed: {} - {}",
            status, body
        )));
    }

    let tokens: TokenResponse = token_response
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to parse GitHub token: {}", e)))?;

    // Get user info
    let user_response = send_with_retry(
        client
            .get(format!("{}/user", GITHUB_API_URL))
            .header("Authorization", format!("token {}", tokens.access_token))
            .header("User-Agent", "RustChat"),
        "GitHub user request failed",
    )
    .await?;

    if !user_response.status().is_success() {
        return Err(AppError::Internal(
            "Failed to fetch GitHub user info".to_string(),
        ));
    }

    let github_user: GitHubUser = user_response
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to parse GitHub user: {}", e)))?;

    // Get primary verified email (GitHub may return null for email in user endpoint)
    let email = if let Some(email) = github_user.email {
        email
    } else {
        // Fetch emails from /user/emails endpoint
        let emails_response = send_with_retry(
            client
                .get(format!("{}/user/emails", GITHUB_API_URL))
                .header("Authorization", format!("token {}", tokens.access_token))
                .header("User-Agent", "RustChat"),
            "GitHub emails request failed",
        )
        .await?;

        if !emails_response.status().is_success() {
            return Err(AppError::Internal(
                "Failed to fetch GitHub emails".to_string(),
            ));
        }

        let emails: Vec<GitHubEmail> = emails_response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse GitHub emails: {}", e)))?;

        // Find primary verified email
        let primary_email = emails
            .iter()
            .find(|e| e.primary && e.verified)
            .or_else(|| emails.iter().find(|e| e.verified))
            .ok_or_else(|| {
                AppError::BadRequest("No verified email found for GitHub account".to_string())
            })?;

        primary_email.email.clone()
    };

    // Check GitHub organization/team restrictions if configured
    if let Some(ref org) = config.github_org {
        let org_check = send_with_retry(
            client
                .get(format!(
                    "{}/orgs/{}/members/{}",
                    GITHUB_API_URL, org, github_user.login
                ))
                .header("Authorization", format!("token {}", tokens.access_token))
                .header("User-Agent", "RustChat"),
            "GitHub org check failed",
        )
        .await?;

        if org_check.status() != reqwest::StatusCode::NO_CONTENT {
            return Err(AppError::Forbidden(format!(
                "User is not a member of required GitHub organization: {}",
                org
            )));
        }

        // Check team if specified
        if let Some(ref team) = config.github_team {
            // First get the team ID by name
            let teams_response = send_with_retry(
                client
                    .get(format!("{}/orgs/{}/teams", GITHUB_API_URL, org))
                    .header("Authorization", format!("token {}", tokens.access_token))
                    .header("User-Agent", "RustChat"),
                "GitHub teams request failed",
            )
            .await?;

            if !teams_response.status().is_success() {
                return Err(AppError::Internal(
                    "Failed to fetch GitHub teams".to_string(),
                ));
            }

            let teams: Vec<serde_json::Value> = teams_response
                .json()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to parse GitHub teams: {}", e)))?;

            let team_id = teams
                .iter()
                .find(|t| t.get("slug").and_then(|s| s.as_str()) == Some(team))
                .and_then(|t| t.get("id").and_then(|id| id.as_i64()))
                .ok_or_else(|| {
                    AppError::Internal(format!("GitHub team '{}' not found in org '{}'", team, org))
                })?;

            let team_check = send_with_retry(
                client
                    .get(format!(
                        "{}/teams/{}/memberships/{}",
                        GITHUB_API_URL, team_id, github_user.login
                    ))
                    .header("Authorization", format!("token {}", tokens.access_token))
                    .header("User-Agent", "RustChat"),
                "GitHub team check failed",
            )
            .await?;

            if !team_check.status().is_success() {
                return Err(AppError::Forbidden(format!(
                    "User is not a member of required GitHub team: {}",
                    team
                )));
            }
        }
    }

    let user_info = UserInfo {
        email: email.clone(),
        name: github_user.name,
        preferred_username: Some(github_user.login),
        groups: vec![],
        external_id: Some(github_user.id.to_string()),
    };

    Ok((email, user_info))
}

/// Exchange code for OIDC token and validate ID token
#[allow(clippy::too_many_arguments)]
pub(crate) async fn exchange_oidc_token(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    issuer: &str,
    code_verifier: Option<&str>,
    expected_nonce: Option<&str>,
    config: &SsoConfig,
) -> Result<(String, UserInfo), AppError> {
    let client = reqwest::Client::new();
    let discovery = OidcDiscoveryService::new();

    // Get OIDC configuration
    let discovery_result = discovery
        .discover(issuer)
        .await
        .map_err(|e| AppError::Internal(format!("OIDC discovery failed: {}", e)))?;

    // Build token request
    let mut form_params = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ];

    // Add PKCE code verifier if present
    if let Some(verifier) = code_verifier {
        form_params.push(("code_verifier", verifier));
    }

    // Exchange code for tokens
    let token_response = send_with_retry(
        client
            .post(&discovery_result.token_endpoint)
            .form(&form_params),
        "OIDC token exchange failed",
    )
    .await?;

    if !token_response.status().is_success() {
        let status = token_response.status();
        let body = token_response.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "OIDC token exchange failed: {} - {}",
            status, body
        )));
    }

    let tokens: TokenResponse = token_response
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to parse OIDC token: {}", e)))?;

    // Validate ID token if present
    let mut claims: Option<IdTokenClaims> = None;

    if let Some(ref id_token) = tokens.id_token {
        claims = Some(
            validate_id_token(
                id_token,
                &discovery_result.jwks_uri,
                client_id,
                issuer,
                expected_nonce,
            )
            .await?,
        );
    }

    // Get user info from ID token claims or userinfo endpoint
    let user_info = if let Some(ref c) = claims {
        // Use claims from ID token
        UserInfo {
            email: c.email.clone().unwrap_or_default(),
            name: c.name.clone(),
            preferred_username: c.preferred_username.clone(),
            groups: extract_groups(c, config.groups_claim.as_deref()),
            external_id: Some(c.sub.clone()),
        }
    } else if let Some(ref userinfo_url) = discovery_result.userinfo_endpoint {
        // Fall back to userinfo endpoint
        let userinfo_response = send_with_retry(
            client.get(userinfo_url).bearer_auth(&tokens.access_token),
            "UserInfo request failed",
        )
        .await?;

        if !userinfo_response.status().is_success() {
            return Err(AppError::Internal("Failed to fetch UserInfo".to_string()));
        }

        let userinfo: UserInfoResponse = userinfo_response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse UserInfo: {}", e)))?;

        let email = userinfo.email.clone().unwrap_or_default();
        let name = userinfo.name.clone();
        let preferred_username = userinfo.preferred_username.clone();
        let groups = extract_groups_from_userinfo(&userinfo, config.groups_claim.as_deref());

        UserInfo {
            email,
            name,
            preferred_username,
            groups,
            external_id: Some(userinfo.sub.clone()),
        }
    } else {
        return Err(AppError::Internal(
            "No ID token or UserInfo endpoint available".to_string(),
        ));
    };

    let email = user_info.email.clone();

    // Check email_verified claim if present
    if let Some(ref c) = claims {
        if c.email_verified == Some(false) {
            return Err(AppError::Forbidden(
                "Email not verified with OAuth provider".to_string(),
            ));
        }
    }

    // Check domain restrictions for Google
    if config.provider_type == "google" {
        if let Some(ref allowed_domains) = config.allow_domains {
            // Empty array means no restrictions (same as None)
            if !allowed_domains.is_empty() {
                let email_domain = email
                    .split('@')
                    .nth(1)
                    .ok_or_else(|| AppError::BadRequest("Invalid email format".to_string()))?;

                if !allowed_domains.contains(&email_domain.to_string()) {
                    return Err(AppError::Forbidden(format!(
                        "Email domain '{}' not allowed",
                        email_domain
                    )));
                }
            }
        }
    }

    Ok((email, user_info))
}

/// Extract groups from ID token claims
fn extract_groups(claims: &IdTokenClaims, groups_claim: Option<&str>) -> Vec<String> {
    let claim_name = groups_claim.unwrap_or("groups");

    // Try standard groups field first
    if let Some(ref groups) = claims.groups {
        return groups.clone();
    }

    // Try to find in extra claims
    if let Some(value) = claims.extra.get(claim_name) {
        if let Some(arr) = value.as_array() {
            return arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
    }

    vec![]
}

/// Extract groups from userinfo response
fn extract_groups_from_userinfo(
    userinfo: &UserInfoResponse,
    groups_claim: Option<&str>,
) -> Vec<String> {
    let claim_name = groups_claim.unwrap_or("groups");

    if let Some(value) = userinfo.extra.get(claim_name) {
        if let Some(arr) = value.as_array() {
            return arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
    }

    vec![]
}

/// Validate ID token signature and claims
async fn validate_id_token(
    id_token: &str,
    jwks_uri: &str,
    client_id: &str,
    expected_issuer: &str,
    expected_nonce: Option<&str>,
) -> Result<IdTokenClaims, AppError> {
    use jsonwebtoken::{decode, decode_header, Algorithm, Validation};

    // Decode header to get key ID
    let header = decode_header(id_token)
        .map_err(|e| AppError::Internal(format!("Failed to decode ID token header: {}", e)))?;

    // Fetch JWKS
    let discovery = OidcDiscoveryService::new();
    let jwks = discovery
        .fetch_jwks(jwks_uri)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch JWKS: {}", e)))?;

    // Find signing key
    let jwk = find_signing_key(&jwks, header.kid.as_deref())
        .ok_or_else(|| AppError::Internal("No suitable signing key found in JWKS".to_string()))?;

    // Build decoding key from JWK
    let decoding_key = jwk_to_decoding_key(jwk)?;

    // Determine algorithm from header
    let algorithm = match header.alg {
        jsonwebtoken::Algorithm::RS256 => Algorithm::RS256,
        jsonwebtoken::Algorithm::RS384 => Algorithm::RS384,
        jsonwebtoken::Algorithm::RS512 => Algorithm::RS512,
        jsonwebtoken::Algorithm::ES256 => Algorithm::ES256,
        jsonwebtoken::Algorithm::ES384 => Algorithm::ES384,
        _ => Algorithm::RS256,
    };

    // Validate token
    let mut validation = Validation::new(algorithm);
    validation.set_audience(&[client_id]);
    validation.set_issuer(&[expected_issuer]);

    let token_data = decode::<IdTokenClaims>(id_token, &decoding_key, &validation)
        .map_err(|e| AppError::Internal(format!("ID token validation failed: {}", e)))?;

    let claims = token_data.claims;

    // Validate nonce if provided
    if let Some(expected) = expected_nonce {
        let actual = claims.nonce.as_deref().unwrap_or("");
        if actual != expected {
            return Err(AppError::Internal("ID token nonce mismatch".to_string()));
        }
    }

    // Check token expiration
    let now = chrono::Utc::now().timestamp();
    if claims.exp < now {
        return Err(AppError::Internal("ID token expired".to_string()));
    }

    Ok(claims)
}

/// Convert JWK to DecodingKey
fn jwk_to_decoding_key(
    jwk: &crate::services::oidc_discovery::Jwk,
) -> Result<jsonwebtoken::DecodingKey, AppError> {
    use jsonwebtoken::DecodingKey;

    match jwk.kty.as_str() {
        "RSA" => {
            let n = jwk
                .n
                .as_ref()
                .ok_or_else(|| AppError::Internal("RSA key missing modulus".to_string()))?;
            let e = jwk
                .e
                .as_ref()
                .ok_or_else(|| AppError::Internal("RSA key missing exponent".to_string()))?;
            DecodingKey::from_rsa_components(n, e)
                .map_err(|e| AppError::Internal(format!("Failed to build RSA decoding key: {}", e)))
        }
        "EC" => {
            // For EC keys, we need to use the x5c certificate chain or build from components
            if let Some(ref x5c) = jwk.x5c {
                if let Some(cert) = x5c.first() {
                    return DecodingKey::from_ec_pem(
                        format!(
                            "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
                            cert
                        )
                        .as_bytes(),
                    )
                    .map_err(|e| {
                        AppError::Internal(format!(
                            "Failed to build EC decoding key from cert: {}",
                            e
                        ))
                    });
                }
            }
            Err(AppError::Internal(
                "EC key format not supported".to_string(),
            ))
        }
        _ => Err(AppError::Internal(format!(
            "Unsupported key type: {}",
            jwk.kty
        ))),
    }
}
