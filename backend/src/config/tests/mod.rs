use std::sync::Mutex;

use super::*;

mod cors_defaults;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_default_values() {
    assert_eq!(default_host(), "0.0.0.0");
    assert_eq!(default_port(), 3000);
    assert_eq!(default_log_level(), "info");
}

#[test]
fn test_retention_job_defaults() {
    let config = RetentionJobConfig::default();
    assert!(!config.orphan_scan_enabled);
    assert_eq!(config.orphan_scan_interval_hours, 24);
    assert_eq!(config.orphan_scan_page_size, 1000);
    assert_eq!(config.orphan_scan_page_delay_ms, 100);
    assert_eq!(config.orphan_scan_min_age_seconds, 300);
}

#[test]
fn test_retention_env_overrides() {
    let _guard = ENV_LOCK.lock().unwrap();

    std::env::set_var("RUSTCHAT_RETENTION_ORPHAN_SCAN_ENABLED", "true");
    std::env::set_var("RUSTCHAT_RETENTION_ORPHAN_SCAN_INTERVAL_HOURS", "12");
    std::env::set_var("RUSTCHAT_RETENTION_ORPHAN_SCAN_PAGE_SIZE", "500");
    std::env::set_var("RUSTCHAT_RETENTION_ORPHAN_SCAN_PAGE_DELAY_MS", "250");
    std::env::set_var("RUSTCHAT_RETENTION_ORPHAN_SCAN_MIN_AGE_SECONDS", "600");

    let mut config = RetentionJobConfig::default();
    apply_retention_env_overrides_to(&mut config).unwrap();

    assert!(config.orphan_scan_enabled);
    assert_eq!(config.orphan_scan_interval_hours, 12);
    assert_eq!(config.orphan_scan_page_size, 500);
    assert_eq!(config.orphan_scan_page_delay_ms, 250);
    assert_eq!(config.orphan_scan_min_age_seconds, 600);

    std::env::remove_var("RUSTCHAT_RETENTION_ORPHAN_SCAN_ENABLED");
    std::env::remove_var("RUSTCHAT_RETENTION_ORPHAN_SCAN_INTERVAL_HOURS");
    std::env::remove_var("RUSTCHAT_RETENTION_ORPHAN_SCAN_PAGE_SIZE");
    std::env::remove_var("RUSTCHAT_RETENTION_ORPHAN_SCAN_PAGE_DELAY_MS");
    std::env::remove_var("RUSTCHAT_RETENTION_ORPHAN_SCAN_MIN_AGE_SECONDS");
}

#[test]
fn test_buzz_integration_defaults() {
    let config = BuzzIntegrationConfig::default();
    assert!(!config.enabled, "buzz bridge must be off by default");
    assert_eq!(config.poll_interval_secs, 5);
    assert_eq!(config.max_attempts, 10);
    assert_eq!(config.backoff_base_secs, 5);
    assert_eq!(config.backoff_max_secs, 3600);
    assert_eq!(config.batch_size, 20);
    assert_eq!(config.in_flight_lease_secs, 300);
    assert!(
        config.run_dispatcher,
        "single-instance default drains the outbox"
    );
}

#[test]
fn test_buzz_integration_env_overrides() {
    let _guard = ENV_LOCK.lock().unwrap();

    for var in [
        "RUSTCHAT_INTEGRATIONS_BUZZ_ENABLED",
        "RUSTCHAT_INTEGRATIONS_BUZZ_POLL_INTERVAL_SECS",
        "RUSTCHAT_INTEGRATIONS_BUZZ_MAX_ATTEMPTS",
        "RUSTCHAT_INTEGRATIONS_BUZZ_BACKOFF_BASE_SECS",
        "RUSTCHAT_INTEGRATIONS_BUZZ_BACKOFF_MAX_SECS",
        "RUSTCHAT_INTEGRATIONS_BUZZ_IN_FLIGHT_LEASE_SECS",
        "RUSTCHAT_INTEGRATIONS_BUZZ_BATCH_SIZE",
        "RUSTCHAT_INTEGRATIONS_BUZZ_RUN_DISPATCHER",
    ] {
        std::env::remove_var(var);
    }

    std::env::set_var("RUSTCHAT_INTEGRATIONS_BUZZ_ENABLED", "true");
    std::env::set_var("RUSTCHAT_INTEGRATIONS_BUZZ_POLL_INTERVAL_SECS", "2");
    std::env::set_var("RUSTCHAT_INTEGRATIONS_BUZZ_MAX_ATTEMPTS", "5");
    std::env::set_var("RUSTCHAT_INTEGRATIONS_BUZZ_BACKOFF_BASE_SECS", "3");
    std::env::set_var("RUSTCHAT_INTEGRATIONS_BUZZ_BACKOFF_MAX_SECS", "120");
    std::env::set_var("RUSTCHAT_INTEGRATIONS_BUZZ_IN_FLIGHT_LEASE_SECS", "30");
    std::env::set_var("RUSTCHAT_INTEGRATIONS_BUZZ_BATCH_SIZE", "7");
    std::env::set_var("RUSTCHAT_INTEGRATIONS_BUZZ_RUN_DISPATCHER", "false");

    let mut cfg: crate::config::Config = serde_json::from_str(
        r#"{"database_url":"postgres://x:x@localhost/x","jwt_secret":"s","encryption_key":"k"}"#,
    )
    .expect("minimal config");
    cfg.apply_integrations_env_overrides().unwrap();
    let buzz = &cfg.integrations.buzz;

    assert!(buzz.enabled);
    assert_eq!(buzz.poll_interval_secs, 2);
    assert_eq!(buzz.max_attempts, 5);
    assert_eq!(buzz.backoff_base_secs, 3);
    assert_eq!(buzz.backoff_max_secs, 120);
    assert_eq!(buzz.in_flight_lease_secs, 30);
    assert_eq!(buzz.batch_size, 7);
    assert!(!buzz.run_dispatcher);

    for var in [
        "RUSTCHAT_INTEGRATIONS_BUZZ_ENABLED",
        "RUSTCHAT_INTEGRATIONS_BUZZ_POLL_INTERVAL_SECS",
        "RUSTCHAT_INTEGRATIONS_BUZZ_MAX_ATTEMPTS",
        "RUSTCHAT_INTEGRATIONS_BUZZ_BACKOFF_BASE_SECS",
        "RUSTCHAT_INTEGRATIONS_BUZZ_BACKOFF_MAX_SECS",
        "RUSTCHAT_INTEGRATIONS_BUZZ_IN_FLIGHT_LEASE_SECS",
        "RUSTCHAT_INTEGRATIONS_BUZZ_BATCH_SIZE",
        "RUSTCHAT_INTEGRATIONS_BUZZ_RUN_DISPATCHER",
    ] {
        std::env::remove_var(var);
    }
}
