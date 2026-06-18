#[cfg(test)]
mod tests {
    use crate::services::webhooks::{callback_http_client, is_valid_callback_url};

    #[tokio::test]
    async fn test_localhost_rejected_by_async_validator() {
        // localhost resolves to 127.0.0.1, which is loopback and must be rejected.
        assert!(
            callback_http_client("http://localhost/callback")
                .await
                .is_none(),
            "localhost should be rejected after DNS resolution"
        );
    }

    #[tokio::test]
    async fn test_public_ip_accepted() {
        // Public IPv4 addresses should be accepted without DNS lookup.
        assert!(
            callback_http_client("http://1.1.1.1/callback")
                .await
                .is_some(),
            "public IPv4 should be accepted"
        );
    }

    #[tokio::test]
    async fn test_private_ipv4_rejected() {
        assert!(callback_http_client("http://10.0.0.1/callback")
            .await
            .is_none());
        assert!(callback_http_client("http://172.16.0.1/callback")
            .await
            .is_none());
        assert!(callback_http_client("http://192.168.1.1/callback")
            .await
            .is_none());
        assert!(callback_http_client("http://127.0.0.1/callback")
            .await
            .is_none());
        assert!(callback_http_client("http://169.254.169.254/callback")
            .await
            .is_none());
        assert!(callback_http_client("http://0.0.0.0/callback")
            .await
            .is_none());
        assert!(callback_http_client("http://100.64.0.1/callback")
            .await
            .is_none());
        assert!(callback_http_client("http://100.127.255.255/callback")
            .await
            .is_none());
        assert!(callback_http_client("http://198.18.0.1/callback")
            .await
            .is_none());
        assert!(callback_http_client("http://198.19.255.255/callback")
            .await
            .is_none());
    }

    #[tokio::test]
    async fn test_private_ipv6_rejected() {
        assert!(callback_http_client("http://[::1]/callback")
            .await
            .is_none());
        assert!(callback_http_client("http://[fe80::1]/callback")
            .await
            .is_none());
        assert!(callback_http_client("http://[fc00::1]/callback")
            .await
            .is_none());
        assert!(callback_http_client("http://[::]/callback").await.is_none());
        assert!(callback_http_client("http://[2001:db8::1]/callback")
            .await
            .is_none());
        assert!(
            callback_http_client("http://[2001:db8:ffff:ffff::1]/callback")
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_ipv4_mapped_ipv6_rejected() {
        // IPv4-mapped IPv6 addresses must be treated as the underlying IPv4 address
        // and rejected when they are loopback, private, or link-local.
        assert!(callback_http_client("http://[::ffff:127.0.0.1]/callback")
            .await
            .is_none());
        assert!(callback_http_client("http://[::ffff:10.0.0.1]/callback")
            .await
            .is_none());
        assert!(callback_http_client("http://[::ffff:192.168.1.1]/callback")
            .await
            .is_none());
        assert!(
            callback_http_client("http://[::ffff:169.254.169.254]/callback")
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_public_ipv4_mapped_ipv6_accepted() {
        // Public IPv4-mapped IPv6 addresses should still be accepted.
        assert!(
            callback_http_client("http://[::ffff:1.1.1.1]/callback")
                .await
                .is_some(),
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
        assert!(callback_http_client("ftp://1.1.1.1/callback")
            .await
            .is_none());
        assert!(callback_http_client("file:///etc/passwd").await.is_none());
    }
}
