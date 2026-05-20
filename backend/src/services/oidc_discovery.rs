//! OIDC Discovery Service
//!
//! Implements OIDC discovery (RFC 8414) with in-memory caching.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::error::AppError;
use crate::middleware::reliability::{
    with_resilience, CircuitBreaker, CircuitError, RetryCondition, RetryConfig,
};

/// Cached discovery result with TTL
struct CachedDiscovery {
    result: DiscoveryResult,
    fetched_at: Instant,
}

/// OIDC Discovery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    /// Authorization endpoint URL
    pub authorization_endpoint: String,
    /// Token endpoint URL
    pub token_endpoint: String,
    /// UserInfo endpoint URL
    pub userinfo_endpoint: Option<String>,
    /// JWKS URI for signature verification
    pub jwks_uri: String,
    /// Issuer identifier
    pub issuer: String,
    /// Supported scopes
    pub scopes_supported: Option<Vec<String>>,
    /// Supported response types
    pub response_types_supported: Option<Vec<String>>,
    /// Supported subject types
    pub subject_types_supported: Option<Vec<String>>,
    /// Supported ID token signing algorithms
    pub id_token_signing_alg_values_supported: Option<Vec<String>>,
}

/// OIDC Discovery service with caching
pub struct OidcDiscoveryService {
    /// In-memory cache: issuer_url -> cached result
    cache: Arc<DashMap<String, CachedDiscovery>>,
    /// Cache TTL (default: 1 hour)
    ttl: Duration,
    /// HTTP client
    client: reqwest::Client,
    /// Circuit breaker for outbound OIDC calls
    circuit_breaker: Arc<CircuitBreaker>,
    /// Retry policy for transient OIDC failures
    retry_config: RetryConfig,
}

impl OidcDiscoveryService {
    /// Create a new discovery service with default 1-hour TTL
    pub fn new() -> Self {
        Self::with_ttl(Duration::from_secs(3600))
    }

    /// Create a new discovery service with custom TTL
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            ttl,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
            circuit_breaker: CircuitBreaker::default_config("oidc-discovery"),
            retry_config: RetryConfig {
                retry_if: RetryCondition::Default,
                ..Default::default()
            },
        }
    }

    /// Discover OIDC configuration for an issuer
    pub async fn discover(&self, issuer_url: &str) -> Result<DiscoveryResult, AppError> {
        let issuer = issuer_url.trim_end_matches('/');

        // Check cache first
        if let Some(cached) = self.cache.get(issuer) {
            if cached.fetched_at.elapsed() < self.ttl {
                tracing::debug!(issuer = %issuer, "Using cached OIDC discovery result");
                return Ok(cached.result.clone());
            }
            // Cache expired, remove it
            drop(cached);
            self.cache.remove(issuer);
        }

        // Fetch fresh discovery document
        let discovery_url = format!("{}/.well-known/openid-configuration", issuer);
        tracing::debug!(url = %discovery_url, "Fetching OIDC discovery document");

        let response = with_resilience(&self.circuit_breaker, &self.retry_config, {
            let client = self.client.clone();
            let discovery_url = discovery_url.clone();
            move || {
                let client = client.clone();
                let discovery_url = discovery_url.clone();
                async move {
                    client.get(&discovery_url).send().await.map_err(|e| {
                        AppError::ExternalService(format!("Failed to fetch OIDC discovery: {}", e))
                    })
                }
            }
        })
        .await
        .map_err(map_circuit_error)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read response body".to_string());
            return Err(AppError::Internal(format!(
                "OIDC discovery failed with status {}: {}",
                status, body
            )));
        }

        let discovery: OpenIdConfiguration = response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse OIDC discovery: {}", e)))?;

        // Validate required fields
        if discovery.authorization_endpoint.is_empty() {
            return Err(AppError::Internal(
                "OIDC discovery missing authorization_endpoint".to_string(),
            ));
        }
        if discovery.token_endpoint.is_empty() {
            return Err(AppError::Internal(
                "OIDC discovery missing token_endpoint".to_string(),
            ));
        }
        if discovery.jwks_uri.is_empty() {
            return Err(AppError::Internal(
                "OIDC discovery missing jwks_uri".to_string(),
            ));
        }

        // Validate issuer matches (case-sensitive exact match per OIDC spec)
        if discovery.issuer != issuer {
            return Err(AppError::Internal(format!(
                "OIDC issuer mismatch: expected '{}', got '{}'",
                issuer, discovery.issuer
            )));
        }

        let result = DiscoveryResult {
            authorization_endpoint: discovery.authorization_endpoint,
            token_endpoint: discovery.token_endpoint,
            userinfo_endpoint: discovery.userinfo_endpoint,
            jwks_uri: discovery.jwks_uri,
            issuer: discovery.issuer,
            scopes_supported: discovery.scopes_supported,
            response_types_supported: discovery.response_types_supported,
            subject_types_supported: discovery.subject_types_supported,
            id_token_signing_alg_values_supported: discovery.id_token_signing_alg_values_supported,
        };

        // Cache the result
        self.cache.insert(
            issuer.to_string(),
            CachedDiscovery {
                result: result.clone(),
                fetched_at: Instant::now(),
            },
        );

        Ok(result)
    }

    /// Fetch JWKS from the given URI
    pub async fn fetch_jwks(&self, jwks_uri: &str) -> Result<Jwks, AppError> {
        tracing::debug!(url = %jwks_uri, "Fetching JWKS");

        let response = with_resilience(&self.circuit_breaker, &self.retry_config, {
            let client = self.client.clone();
            let jwks_uri = jwks_uri.to_string();
            move || {
                let client = client.clone();
                let jwks_uri = jwks_uri.clone();
                async move {
                    client.get(&jwks_uri).send().await.map_err(|e| {
                        AppError::ExternalService(format!("Failed to fetch JWKS: {}", e))
                    })
                }
            }
        })
        .await
        .map_err(map_circuit_error)?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AppError::Internal(format!(
                "JWKS fetch failed with status {}",
                status
            )));
        }

        let jwks: Jwks = response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse JWKS: {}", e)))?;

        Ok(jwks)
    }

    /// Clear the cache (useful for testing or admin operations)
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache statistics (for debugging/monitoring)
    pub fn cache_stats(&self) -> (usize, usize) {
        let total = self.cache.len();
        let expired = self
            .cache
            .iter()
            .filter(|entry| entry.fetched_at.elapsed() >= self.ttl)
            .count();
        (total, expired)
    }
}

impl Default for OidcDiscoveryService {
    fn default() -> Self {
        Self::new()
    }
}

fn map_circuit_error(err: CircuitError<AppError>) -> AppError {
    match err {
        CircuitError::Open => AppError::ExternalService(
            "OIDC endpoint temporarily unavailable (circuit open)".to_string(),
        ),
        CircuitError::Inner(inner) => inner,
    }
}

/// OpenID Connect Discovery response
/// See: https://openid.net/specs/openid-connect-discovery-1_0.html
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenIdConfiguration {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: Option<String>,
    pub jwks_uri: String,
    #[serde(default)]
    pub scopes_supported: Option<Vec<String>>,
    #[serde(default)]
    pub response_types_supported: Option<Vec<String>>,
    #[serde(default)]
    pub subject_types_supported: Option<Vec<String>>,
    #[serde(default)]
    pub id_token_signing_alg_values_supported: Option<Vec<String>>,
    #[serde(default)]
    pub token_endpoint_auth_methods_supported: Option<Vec<String>>,
    #[serde(default)]
    pub claims_supported: Option<Vec<String>>,
}

/// JSON Web Key Set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

/// JSON Web Key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwk {
    pub kty: String,
    pub kid: Option<String>,
    #[serde(rename = "use")]
    pub use_: Option<String>,
    pub key_ops: Option<Vec<String>>,
    pub alg: Option<String>,
    pub x5c: Option<Vec<String>>,
    pub x5t: Option<String>,
    #[serde(rename = "x5t#S256")]
    pub x5t_s256: Option<String>,
    pub n: Option<String>,   // RSA modulus
    pub e: Option<String>,   // RSA exponent
    pub crv: Option<String>, // EC curve
    pub x: Option<String>,   // EC x coordinate
    pub y: Option<String>,   // EC y coordinate
    #[serde(rename = "d")]
    pub d: Option<String>, // Private key (should not be present in JWKS)
}

/// Find a signing key in JWKS by key ID
pub fn find_signing_key<'a>(jwks: &'a Jwks, kid: Option<&str>) -> Option<&'a Jwk> {
    jwks.keys.iter().find(|key| {
        // Key must be for signing
        let is_signing = key.use_.as_deref() == Some("sig") || key.use_.is_none();

        // If kid is specified, match it; otherwise return first valid signing key
        let kid_matches = kid.is_none_or(|k| key.kid.as_deref() == Some(k));

        is_signing && kid_matches
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oidc_provider_type_defaults() {
        use crate::models::SsoProviderType;

        assert_eq!(
            SsoProviderType::GitHub.default_scopes(),
            vec!["read:user", "user:email"]
        );
        assert_eq!(
            SsoProviderType::Google.default_scopes(),
            vec!["openid", "profile", "email"]
        );
        assert_eq!(
            SsoProviderType::Oidc.default_scopes(),
            vec!["openid", "profile", "email"]
        );

        assert!(SsoProviderType::Google.uses_oidc_discovery());
        assert!(SsoProviderType::Oidc.uses_oidc_discovery());
        assert!(!SsoProviderType::GitHub.uses_oidc_discovery());
    }

    #[test]
    fn test_cache_stats() {
        let service = OidcDiscoveryService::with_ttl(Duration::from_secs(3600));
        assert_eq!(service.cache_stats(), (0, 0));
    }
}
