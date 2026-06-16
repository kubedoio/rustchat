#[cfg(test)]
mod tests {
    use crate::services::webhooks::validate_callback_url_at_request_time;

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
    async fn test_non_http_scheme_rejected() {
        assert!(!validate_callback_url_at_request_time("ftp://1.1.1.1/callback").await);
        assert!(!validate_callback_url_at_request_time("file:///etc/passwd").await);
    }
}
