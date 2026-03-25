use deepwrite::services::update_checker::{is_check_due, is_newer_version};

#[test]
fn test_newer_version() {
    assert!(is_newer_version("0.1.0", "0.2.0"));
    assert!(is_newer_version("0.1.0", "0.1.1"));
    assert!(is_newer_version("0.1.0", "1.0.0"));
}

#[test]
fn test_same_version() {
    assert!(!is_newer_version("0.1.0", "0.1.0"));
}

#[test]
fn test_older_version() {
    assert!(!is_newer_version("0.2.0", "0.1.0"));
}

#[test]
fn test_version_with_v_prefix() {
    assert!(is_newer_version("0.1.0", "v0.2.0"));
}

#[test]
fn test_check_is_due_after_interval() {
    assert!(is_check_due(0, 86_400));
}

#[test]
fn test_check_is_not_due_before_interval() {
    assert!(!is_check_due(1_000, 1_060));
}
