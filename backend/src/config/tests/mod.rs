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
