#[cfg(test)]
mod tests {
    use crate::services::webhooks::{is_valid_callback_url, validate_callback_url_at_request_time};

    #[tokio::test]
    async fn test_localhost_rejected_by_async_validator() {
        // localhost resolves to 127.0.0.1, which is loopback and must be rejected.
        assert!(
            !validate_callback_url_at_request_time("http://localhost/callback").await,
            "localhost should be rejected after DNS resolution"
        );
    }

    #[tokio::test]
    async fn test_public_ip_accepted() {
        // Public IPv4 addresses should be accepted without DNS lookup.
        assert!(
            validate_callback_url_at_request_time("http://1.1.1.1/callback").await,
            "public IPv4 should be accepted"
        );
    }

    #[tokio::test]
    async fn test_private_ipv4_rejected() {
        assert!(!validate_callback_url_at_request_time("http://10.0.0.1/callback").await);
        assert!(!validate_callback_url_at_request_time("http://172.16.0.1/callback").await);
        assert!(!validate_callback_url_at_request_time("http://192.168.1.1/callback").await);
        assert!(!validate_callback_url_at_request_time("http://127.0.0.1/callback").await);
        assert!(!validate_callback_url_at_request_time("http://169.254.169.254/callback").await);
    }

    #[tokio::test]
    async fn test_private_ipv6_rejected() {
        assert!(!validate_callback_url_at_request_time("http://[::1]/callback").await);
        assert!(!validate_callback_url_at_request_time("http://[fe80::1]/callback").await);
        assert!(!validate_callback_url_at_request_time("http://[fc00::1]/callback").await);
    }

    #[tokio::test]
    async fn test_ipv4_mapped_ipv6_rejected() {
        // IPv4-mapped IPv6 addresses must be treated as the underlying IPv4 address
        // and rejected when they are loopback, private, or link-local.
        assert!(!validate_callback_url_at_request_time("http://[::ffff:127.0.0.1]/callback").await);
        assert!(!validate_callback_url_at_request_time("http://[::ffff:10.0.0.1]/callback").await);
        assert!(
            !validate_callback_url_at_request_time("http://[::ffff:192.168.1.1]/callback").await
        );
        assert!(
            !validate_callback_url_at_request_time("http://[::ffff:169.254.169.254]/callback")
                .await
        );
    }

    #[tokio::test]
    async fn test_public_ipv4_mapped_ipv6_accepted() {
        // Public IPv4-mapped IPv6 addresses should still be accepted.
        assert!(
            validate_callback_url_at_request_time("http://[::ffff:1.1.1.1]/callback").await,
            "public IPv4-mapped IPv6 should be accepted"
        );
    }

    #[test]
    fn test_static_validator_rejects_ipv4_mapped_ipv6() {
        // The static validator should reject IPv4-mapped loopback/private literals
        // directly, before any DNS resolution occurs.
        assert!(!is_valid_callback_url("http://[::ffff:127.0.0.1]/callback"));
        assert!(!is_valid_callback_url("http://[::ffff:10.0.0.1]/callback"));
        assert!(!is_valid_callback_url(
            "http://[::ffff:169.254.169.254]/callback"
        ));
    }

    #[tokio::test]
    async fn test_non_http_scheme_rejected() {
        assert!(!validate_callback_url_at_request_time("ftp://1.1.1.1/callback").await);
        assert!(!validate_callback_url_at_request_time("file:///etc/passwd").await);
    }
}
