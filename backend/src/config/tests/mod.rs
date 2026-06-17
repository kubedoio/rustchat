use super::*;

mod cors_defaults;

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
}
