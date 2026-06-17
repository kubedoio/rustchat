//! Webhook delivery service
//!
//! Handles incoming webhook execution (POST to create message) and
//! outgoing webhook triggers (send HTTP request on new posts).

use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::middleware::reliability::{send_reqwest_with_retry, RetryCondition, RetryConfig};
use crate::models::{IncomingWebhook, OutgoingWebhook, OutgoingWebhookPayload, WebhookPayload};
use crate::services::posts;
use std::net::{IpAddr, SocketAddr};
use std::sync::OnceLock;
use std::time::Duration;
use uuid::Uuid;

/// Comprehensive check for private, reserved, or otherwise unsafe IP addresses.
fn is_private_or_reserved_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_multicast()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_unspecified()
                // 100.64.0.0/10 (shared/CGNAT address space)
                || (octets[0] == 100 && (octets[1] & 0b11000000) == 0b01000000)
                // 198.18.0.0/15 (benchmarking range)
                || (octets[0] == 198 && (octets[1] & 0xfe) == 18)
        }
        IpAddr::V6(v6) => {
            // Handle IPv4-mapped IPv6 addresses such as ::ffff:127.0.0.1 by
            // delegating to the IPv4 checks. This prevents SSRF bypasses where
            // an attacker uses IPv6 literal syntax to reach IPv4-internal hosts.
            if let Some(mapped_v4) = v6.to_ipv4_mapped() {
                return is_private_or_reserved_ip(IpAddr::V4(mapped_v4));
            }

            if v6.is_unspecified() {
                return true;
            }

            let segments = v6.segments();
            // ::1 (loopback)
            if segments == [0, 0, 0, 0, 0, 0, 0, 1] {
                return true;
            }
            // fe80::/10 (link-local)
            if segments[0] & 0xffc0 == 0xfe80 {
                return true;
            }
            // fc00::/7 (unique local addresses)
            if segments[0] & 0xfe00 == 0xfc00 {
                return true;
            }
            // ff00::/8 (multicast)
            if segments[0] & 0xff00 == 0xff00 {
                return true;
            }
            // 2001:db8::/32 (IPv6 documentation range)
            if segments[0] == 0x2001 && segments[1] == 0x0db8 {
                return true;
            }
            false
        }
    }
}

/// Validates that a URL is safe for webhook callbacks (no SSRF to internal networks)
pub fn is_valid_callback_url(url: &str) -> bool {
    let Ok(parsed) = url.parse::<reqwest::Url>() else {
        return false;
    };

    // Only allow http and https schemes
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return false,
    }

    // Check host is present
    let Some(host) = parsed.host() else {
        return false;
    };

    // Block localhost and loopback hostnames
    if host == url::Host::Domain("localhost".to_string()) {
        return false;
    }

    // Check if host is an IP address and reject unsafe ranges.
    match host {
        url::Host::Ipv4(ip) => {
            if is_private_or_reserved_ip(IpAddr::V4(ip)) {
                return false;
            }
        }
        url::Host::Ipv6(ip) => {
            if is_private_or_reserved_ip(IpAddr::V6(ip)) {
                return false;
            }
        }
        url::Host::Domain(host) => {
            // Block cloud metadata endpoints by hostname
            let host_lower = host.to_lowercase();
            if host_lower == "169.254.169.254"
                || host_lower.ends_with(".internal")
                || host_lower == "metadata.google.internal"
                || host_lower == "instance-data.ec2.internal"
                || host_lower == "metadata.azure.internal"
            {
                return false;
            }
        }
    }

    true
}

/// Build a reqwest client that is bound to the validated IP addresses for `url`.
///
/// For IP-literal URLs this returns the shared no-redirect client after validating
/// the address. For domain URLs it resolves the hostname once, verifies that every
/// returned address is public, and then configures the client with `resolve` so
/// reqwest connects to the vetted addresses instead of performing a second DNS lookup.
/// This closes the DNS-rebinding SSRF window.
pub(crate) async fn callback_http_client(url: &str) -> Option<(reqwest::Client, reqwest::Url)> {
    let parsed: reqwest::Url = url.parse().ok()?;
    if !is_valid_callback_url(url) {
        return None;
    }

    let parsed_return = parsed.clone();
    let host = parsed.host()?;
    let port = parsed.port_or_known_default().unwrap_or(80);

    match host {
        url::Host::Ipv4(ip) => {
            if is_private_or_reserved_ip(IpAddr::V4(ip)) {
                return None;
            }
            Some((shared_webhook_client(), parsed_return))
        }
        url::Host::Ipv6(ip) => {
            if is_private_or_reserved_ip(IpAddr::V6(ip)) {
                return None;
            }
            Some((shared_webhook_client(), parsed_return))
        }
        url::Host::Domain(hostname) => {
            let lookup = tokio::time::timeout(
                Duration::from_secs(5),
                tokio::net::lookup_host((hostname, port)),
            )
            .await;
            let Ok(Ok(addrs)) = lookup else {
                return None;
            };
            let addrs: Vec<SocketAddr> = addrs.collect();
            if !callback_addresses_are_safe(&addrs) {
                return None;
            }

            let client = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(30))
                // Bypass system proxies so the pinned addresses are the only route used.
                // Without this, a proxy could re-resolve the hostname and defeat SSRF checks.
                .no_proxy()
                .resolve_to_addrs(hostname, &addrs)
                .build()
                .ok()?;
            Some((client, parsed_return))
        }
    }
}

/// Returns true only when every resolved address is safe and at least one address exists.
/// An empty resolution list is rejected because `Iterator::all` vacuously returns true.
fn callback_addresses_are_safe(addrs: &[SocketAddr]) -> bool {
    if addrs.is_empty() {
        return false;
    }
    addrs
        .iter()
        .all(|socket_addr| !is_private_or_reserved_ip(socket_addr.ip()))
}

/// Shared no-redirect reqwest client for outgoing webhook delivery.
///
/// Building one client and cloning it per task avoids the overhead of
/// reconstructing TLS state for every spawned webhook request.
fn shared_webhook_client() -> reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                // Do not follow redirects for SSRF safety. A redirect target could resolve
                // to an internal endpoint after the initial request passed validation.
                .redirect(reqwest::redirect::Policy::none())
                // Apply a safe default timeout for all requests; callers can still override
                // per-request via `.timeout(...)` on individual RequestBuilders.
                .timeout(Duration::from_secs(30))
                // Bypass system proxies so outbound delivery uses only the validated IP.
                // Without this, a proxy could re-resolve the hostname and defeat SSRF checks.
                .no_proxy()
                .build()
                .expect("Failed to build shared no-redirect webhook client")
        })
        .clone()
}

/// Execute an incoming webhook - creates a post in the target channel
pub async fn execute_incoming_webhook(
    state: &AppState,
    token: &str,
    payload: WebhookPayload,
) -> ApiResult<()> {
    // 1. Find the webhook by token
    let hook: IncomingWebhook =
        sqlx::query_as("SELECT * FROM incoming_webhooks WHERE token = $1 AND is_active = true")
            .bind(token)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::NotFound("Webhook not found or inactive".to_string()))?;

    // 2. Get the bot user or creator as the poster
    let poster_id =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE is_bot = true LIMIT 1")
            .fetch_optional(&state.db)
            .await?
            .unwrap_or(hook.creator_id);

    // 3. Build props with override info
    let mut props = payload.props.as_object().cloned().unwrap_or_default();
    if let Some(username) = &payload.username {
        props.insert("override_username".to_string(), serde_json::json!(username));
    }
    if let Some(icon) = &payload.icon_url {
        props.insert("override_icon_url".to_string(), serde_json::json!(icon));
    }
    props.insert("from_webhook".to_string(), serde_json::json!(true));
    props.insert(
        "webhook_display_name".to_string(),
        serde_json::json!(hook.display_name),
    );

    // 4. Create the post
    let input = crate::models::CreatePost {
        message: payload.text,
        root_post_id: None,
        file_ids: vec![],
        props: Some(serde_json::Value::Object(props)),
        client_msg_id: None,
    };

    posts::create_post(state, poster_id, hook.channel_id, input, None).await?;

    Ok(())
}

/// Check for outgoing webhook triggers and execute them
pub async fn check_outgoing_triggers(
    state: &AppState,
    channel_id: Uuid,
    team_id: Uuid,
    user_id: Uuid,
    username: &str,
    channel_name: &str,
    message: &str,
) -> ApiResult<()> {
    // 1. Get words from message
    let first_word = message.split_whitespace().next().unwrap_or("");
    let message_lower = message.to_lowercase();

    // 2. Find matching outgoing webhooks
    let hooks: Vec<OutgoingWebhook> = sqlx::query_as(
        r#"
        SELECT * FROM outgoing_webhooks 
        WHERE is_active = true 
          AND team_id = $1
          AND (channel_id IS NULL OR channel_id = $2)
        "#,
    )
    .bind(team_id)
    .bind(channel_id)
    .fetch_all(&state.db)
    .await?;

    for hook in hooks {
        let matched_word = hook.trigger_words.iter().find(|tw| {
            let tw_lower = tw.to_lowercase();
            match hook.trigger_when.as_str() {
                "first_word" => first_word.to_lowercase() == tw_lower,
                _ => message_lower.contains(&tw_lower), // "any" match
            }
        });

        if let Some(trigger_word) = matched_word {
            // Build payload
            let payload = OutgoingWebhookPayload {
                token: hook.token.clone(),
                team_id,
                channel_id,
                channel_name: channel_name.to_string(),
                user_id,
                user_name: username.to_string(),
                text: message.to_string(),
                trigger_word: trigger_word.clone(),
            };

            // Spawn async task to call each callback URL (filter out SSRF-risky URLs)
            for url in &hook.callback_urls {
                if !is_valid_callback_url(url) {
                    tracing::warn!("Skipping outgoing webhook to invalid URL: {}", url);
                    continue;
                }
                let url = url.clone();
                let payload = payload.clone();
                let content_type = hook
                    .content_type
                    .clone()
                    .unwrap_or_else(|| "application/json".to_string());

                tokio::spawn(async move {
                    let Some((client, parsed_url)) = callback_http_client(&url).await else {
                        tracing::warn!("Skipping outgoing webhook to unsafe URL: {}", url);
                        return;
                    };

                    let request = client
                        .post(parsed_url.as_str())
                        .header("Content-Type", &content_type)
                        .json(&payload)
                        .timeout(Duration::from_secs(30));

                    let retry_config = RetryConfig {
                        max_attempts: 3,
                        initial_delay: Duration::from_millis(150),
                        max_delay: Duration::from_secs(2),
                        backoff_multiplier: 2.0,
                        retry_if: RetryCondition::Default,
                    };

                    let result = send_reqwest_with_retry(
                        request,
                        &retry_config,
                        |e| e.to_string(),
                        || "failed to clone outgoing webhook request".to_string(),
                    )
                    .await;

                    if let Err(e) = result {
                        tracing::warn!("Outgoing webhook to {} failed: {}", url, e);
                    }
                });
            }

            // Only trigger once per message
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_resolution_is_rejected() {
        let addrs: Vec<SocketAddr> = vec![];
        assert!(!callback_addresses_are_safe(&addrs));
    }

    #[test]
    fn only_public_addresses_are_accepted() {
        let addrs = vec![
            "1.1.1.1:443".parse::<SocketAddr>().unwrap(),
            "9.9.9.9:443".parse::<SocketAddr>().unwrap(),
        ];
        assert!(callback_addresses_are_safe(&addrs));
    }

    #[test]
    fn any_private_address_is_rejected() {
        let addrs = vec![
            "1.1.1.1:443".parse::<SocketAddr>().unwrap(),
            "192.168.1.1:443".parse::<SocketAddr>().unwrap(),
        ];
        assert!(!callback_addresses_are_safe(&addrs));
    }
}
