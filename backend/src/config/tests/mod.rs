use super::*;

mod cors_defaults;

#[test]
fn test_default_values() {
    assert_eq!(default_host(), "0.0.0.0");
    assert_eq!(default_port(), 3000);
    assert_eq!(default_log_level(), "info");
}
