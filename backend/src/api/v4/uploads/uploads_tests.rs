//! Unit tests for resumable upload validation.

use crate::api::file_validation::{
    validate_file_extension, validate_file_upload, ALLOWED_EXTENSIONS,
};

#[test]
fn create_upload_rejects_exe_extension() {
    assert!(
        validate_file_extension("malware.exe").is_err(),
        ".exe should be rejected"
    );
    assert!(
        validate_file_extension("malware.EXE").is_err(),
        ".EXE should be rejected case-insensitively"
    );
}

#[test]
fn create_upload_rejects_other_disallowed_extensions() {
    for ext in ["bat", "com", "dll", "doc", "svg", "js", "php", ""] {
        assert!(
            validate_file_extension(&format!("file.{}", ext)).is_err(),
            "expected .{} to be rejected",
            ext
        );
    }
    assert!(
        validate_file_extension("no_extension").is_err(),
        "missing extension should be rejected"
    );
}

#[test]
fn create_upload_accepts_allowed_extensions() {
    for ext in ALLOWED_EXTENSIONS {
        assert!(
            validate_file_extension(&format!("file.{}", ext)).is_ok(),
            "expected .{} to be accepted",
            ext
        );
    }
}

#[test]
fn upload_finalization_rejects_content_mismatch() {
    let result = validate_file_upload("innocent.png", b"this is not image data");
    assert!(
        result.is_err(),
        "expected mismatched content to be rejected"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("content") || err.contains("Expected") || err.contains("match"),
        "unexpected error: {}",
        err
    );
}

#[test]
fn upload_finalization_accepts_matching_content() {
    // Minimal PNG header bytes.
    let png = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let result = validate_file_upload("avatar.png", png);
    assert!(
        result.is_ok(),
        "expected PNG bytes to be accepted, got {:?}",
        result
    );
}
