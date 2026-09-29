//! Security headers middleware
//!
//! Implements zero-trust security headers for all HTTP responses.
//! Protects against common web attacks including XSS, clickjacking,
//! MIME sniffing, and other browser-based vulnerabilities.

use axum::{
    body::Body,
    http::{header, HeaderValue, Request, Response},
};
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Security headers configuration
#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    /// Content Security Policy
    pub csp: String,
    /// Whether to include HSTS header
    pub hsts_enabled: bool,
    /// HSTS max age in seconds
    pub hsts_max_age: u64,
    /// Whether to include HSTS preload
    pub hsts_preload: bool,
    /// Whether to include HSTS includeSubDomains
    pub hsts_include_subdomains: bool,
    /// X-Frame-Options value
    pub frame_options: String,
    /// X-Content-Type-Options value
    pub content_type_options: String,
    /// Referrer-Policy value
    pub referrer_policy: String,
    /// Permissions-Policy value
    pub permissions_policy: String,
    /// X-XSS-Protection value
    pub xss_protection: String,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self::strict()
    }
}

impl SecurityHeadersConfig {
    /// Strict security headers (recommended for production)
    pub fn strict() -> Self {
        Self {
            // Strict CSP - adjust based on your frontend needs
            // script-src does not allow inline scripts: the frontend must
            // not ship inline <script> blocks (the theme boot script is an
            // external same-origin file; see frontend/public/theme-boot.js
            // and scripts/check-p0-gates.sh).
            csp: "default-src 'self'; \
                   script-src 'self'; \
                   style-src 'self' 'unsafe-inline'; \
                   img-src 'self' data: blob: https:; \
                   font-src 'self' data:; \
                   connect-src 'self' wss: https:; \
                   media-src 'self' blob:; \
                   frame-ancestors 'self'; \
                   base-uri 'self'; \
                   form-action 'self';"
                .replace("\n", " ")
                .replace("  ", " "),
            hsts_enabled: true,
            hsts_max_age: 63072000, // 2 years
            hsts_preload: true,
            hsts_include_subdomains: true,
            frame_options: "SAMEORIGIN".to_string(),
            content_type_options: "nosniff".to_string(),
            referrer_policy: "strict-origin-when-cross-origin".to_string(),
            permissions_policy: "camera=(), microphone=(), geolocation=(), \
                                payment=(), usb=(), magnetometer=(), \
                                gyroscope=(), speaker=()"
                .replace("\n", " ")
                .replace("  ", " "),
            xss_protection: "1; mode=block".to_string(),
        }
    }

    /// Permissive headers for development
    pub fn development() -> Self {
        Self {
            csp: "default-src 'self' 'unsafe-inline' 'unsafe-eval' \
                   http: https: ws: wss: data: blob:;"
                .replace("\n", " ")
                .replace("  ", " "),
            hsts_enabled: false, // Don't force HTTPS in dev
            hsts_max_age: 0,
            hsts_preload: false,
            hsts_include_subdomains: false,
            frame_options: "SAMEORIGIN".to_string(),
            content_type_options: "nosniff".to_string(),
            referrer_policy: "strict-origin-when-cross-origin".to_string(),
            permissions_policy: "camera=*, microphone=*, geolocation=*".to_string(),
            xss_protection: "1; mode=block".to_string(),
        }
    }

    /// API-only headers (no CSP needed for pure API)
    pub fn api_only() -> Self {
        Self {
            csp: "default-src 'none'; frame-ancestors 'none';".to_string(),
            hsts_enabled: true,
            hsts_max_age: 63072000,
            hsts_preload: true,
            hsts_include_subdomains: true,
            frame_options: "DENY".to_string(),
            content_type_options: "nosniff".to_string(),
            referrer_policy: "no-referrer".to_string(),
            permissions_policy: "()".to_string(),
            xss_protection: "1; mode=block".to_string(),
        }
    }
}

/// Security header values pre-parsed once at layer construction.
///
/// All values currently come from compile-time presets, but parsing them
/// once here — with a descriptive failure — means a malformed value fails
/// loudly at startup instead of panicking on every request inside the
/// service (the previous behavior parsed and unwrapped per response).
#[derive(Debug, Clone)]
struct ParsedSecurityHeaders {
    csp: HeaderValue,
    frame_options: HeaderValue,
    content_type_options: HeaderValue,
    referrer_policy: HeaderValue,
    permissions_policy: HeaderValue,
    xss_protection: HeaderValue,
    /// Present only when HSTS is enabled.
    hsts: Option<HeaderValue>,
}

impl ParsedSecurityHeaders {
    fn from_config(config: &SecurityHeadersConfig) -> Self {
        fn parse(field: &'static str, value: &str) -> HeaderValue {
            value.parse().unwrap_or_else(|e| {
                panic!(
                    "invalid security header value for {field}: {value:?} ({e}); \
                     fix the SecurityHeadersConfig preset"
                )
            })
        }

        let hsts = if config.hsts_enabled {
            let hsts_value = format!(
                "max-age={}{}{}",
                config.hsts_max_age,
                if config.hsts_include_subdomains {
                    "; includeSubDomains"
                } else {
                    ""
                },
                if config.hsts_preload { "; preload" } else { "" }
            );
            Some(parse("hsts", &hsts_value))
        } else {
            None
        };

        Self {
            csp: parse("csp", &config.csp),
            frame_options: parse("frame_options", &config.frame_options),
            content_type_options: parse("content_type_options", &config.content_type_options),
            referrer_policy: parse("referrer_policy", &config.referrer_policy),
            permissions_policy: parse("permissions_policy", &config.permissions_policy),
            xss_protection: parse("xss_protection", &config.xss_protection),
            hsts,
        }
    }
}

/// Security headers middleware layer
#[derive(Debug, Clone)]
pub struct SecurityHeadersLayer {
    config: SecurityHeadersConfig,
}

impl SecurityHeadersLayer {
    pub fn new(config: SecurityHeadersConfig) -> Self {
        Self { config }
    }

    pub fn strict() -> Self {
        Self::new(SecurityHeadersConfig::strict())
    }

    pub fn development() -> Self {
        Self::new(SecurityHeadersConfig::development())
    }

    pub fn api_only() -> Self {
        Self::new(SecurityHeadersConfig::api_only())
    }
}

impl<S> Layer<S> for SecurityHeadersLayer {
    type Service = SecurityHeadersService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityHeadersService {
            inner,
            headers: ParsedSecurityHeaders::from_config(&self.config),
        }
    }
}

/// Security headers middleware service
#[derive(Debug, Clone)]
pub struct SecurityHeadersService<S> {
    inner: S,
    headers: ParsedSecurityHeaders,
}

impl<S, B> Service<Request<B>> for SecurityHeadersService<S>
where
    S: Service<Request<B>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let headers = self.headers.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let mut response = inner.call(req).await?;
            let headers_map = response.headers_mut();

            // All values were parsed and validated once at layer construction.
            // Content Security Policy
            headers_map.insert(header::CONTENT_SECURITY_POLICY, headers.csp);

            // X-Frame-Options
            headers_map.insert(header::X_FRAME_OPTIONS, headers.frame_options);

            // X-Content-Type-Options
            headers_map.insert(header::X_CONTENT_TYPE_OPTIONS, headers.content_type_options);

            // Referrer-Policy
            headers_map.insert(
                header::HeaderName::from_static("referrer-policy"),
                headers.referrer_policy,
            );

            // Permissions-Policy (formerly Feature-Policy)
            headers_map.insert(
                header::HeaderName::from_static("permissions-policy"),
                headers.permissions_policy,
            );

            // X-XSS-Protection (legacy but still useful)
            headers_map.insert(header::X_XSS_PROTECTION, headers.xss_protection);

            // Strict-Transport-Security (HSTS) — only when enabled
            if let Some(hsts) = headers.hsts {
                headers_map.insert(header::STRICT_TRANSPORT_SECURITY, hsts);
            }

            // Remove server information (if present)
            headers_map.remove(header::SERVER);

            Ok(response)
        })
    }
}

/// Helper to create a CORS-appropriate security headers configuration
/// that works with the CORS layer
pub fn cors_compatible_config() -> SecurityHeadersConfig {
    SecurityHeadersConfig {
        csp: "default-src 'self'; \
               script-src 'self'; \
               style-src 'self' 'unsafe-inline'; \
               img-src 'self' data: blob: https:; \
               font-src 'self' data:; \
               connect-src *; \
               media-src 'self' blob:; \
               frame-ancestors 'self'; \
               base-uri 'self'; \
               form-action 'self';"
            .replace("\n", " ")
            .replace("  ", " "),
        hsts_enabled: true,
        hsts_max_age: 63072000,
        hsts_preload: true,
        hsts_include_subdomains: true,
        frame_options: "SAMEORIGIN".to_string(),
        content_type_options: "nosniff".to_string(),
        referrer_policy: "strict-origin-when-cross-origin".to_string(),
        permissions_policy: "camera=(self), microphone=(self), geolocation=()".to_string(),
        xss_protection: "1; mode=block".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_config() {
        let config = SecurityHeadersConfig::strict();
        assert!(config.hsts_enabled);
        assert_eq!(config.hsts_max_age, 63072000);
        assert!(config.hsts_preload);
    }

    #[test]
    fn test_development_config() {
        let config = SecurityHeadersConfig::development();
        assert!(!config.hsts_enabled);
        assert!(config.csp.contains("unsafe-inline"));
    }

    #[test]
    fn test_production_presets_forbid_inline_scripts() {
        // M4 (CSP hardening): production presets must not allow inline
        // scripts. style-src keeps 'unsafe-inline' (Vue inline styles), so
        // the script-src directive is parsed and checked explicitly rather
        // than substring-matching the whole policy.
        fn script_src_directives(csp: &str) -> Vec<&str> {
            csp.split(';')
                .map(str::trim)
                .filter(|d| d.starts_with("script-src"))
                .collect()
        }
        for config in [SecurityHeadersConfig::strict(), cors_compatible_config()] {
            let directives = script_src_directives(&config.csp);
            assert!(
                !directives.is_empty(),
                "preset must define a script-src directive"
            );
            for directive in directives {
                assert!(
                    !directive.contains("unsafe-inline"),
                    "script-src must not allow unsafe-inline: {directive}"
                );
                assert!(
                    !directive.contains("unsafe-eval"),
                    "script-src must not allow unsafe-eval: {directive}"
                );
            }
        }
    }

    #[test]
    fn test_api_only_config() {
        let config = SecurityHeadersConfig::api_only();
        assert!(config.csp.contains("default-src 'none'"));
        assert_eq!(config.frame_options, "DENY");
    }

    #[test]
    fn test_all_presets_parse_into_valid_header_values() {
        // Regression: the service used to .parse().unwrap() per request;
        // presets must therefore all be valid HeaderValues so that
        // construction-time parsing cannot fail at startup.
        for config in [
            SecurityHeadersConfig::strict(),
            SecurityHeadersConfig::development(),
            SecurityHeadersConfig::api_only(),
            cors_compatible_config(),
        ] {
            let parsed = ParsedSecurityHeaders::from_config(&config);
            assert!(!parsed.csp.is_empty());
            assert!(!parsed.frame_options.is_empty());
            assert!(!parsed.content_type_options.is_empty());
            assert_eq!(
                parsed.hsts.is_some(),
                config.hsts_enabled,
                "HSTS header presence must match hsts_enabled"
            );
            if let Some(hsts) = parsed.hsts {
                let hsts = hsts.to_str().expect("hsts is ASCII");
                assert!(hsts.starts_with("max-age="));
            }
        }
    }

    #[test]
    #[should_panic(expected = "invalid security header value for csp")]
    fn test_invalid_config_value_fails_at_construction_with_field_name() {
        // A malformed header value must fail loudly at construction
        // (startup / router assembly), naming the offending field — never
        // per request.
        let mut config = SecurityHeadersConfig::strict();
        config.csp = "default-src 'self';\nscript-src 'self'".to_string(); // \n is invalid in a HeaderValue
        let _ = ParsedSecurityHeaders::from_config(&config);
    }
}
