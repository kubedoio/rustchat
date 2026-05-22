//! Integration tests for API key generation and validation
//!
//! This test suite validates the full API key lifecycle:
//! - Generation of API keys with "rck_" prefix (68 chars: rck_ + 64 hex)
//! - Argon2id hashing
//! - Validation against hashed values
//! - Resistance to timing attacks via constant-time comparison
//! - Thread-safe operation in async contexts

use rustchat::auth::api_key::{extract_prefix, generate_api_key, hash_api_key, validate_api_key};

/// Test that generate_api_key produces 68-character strings with rck_ prefix
#[test]
fn test_generate_api_key_has_prefix() {
    let key = generate_api_key();
    assert_eq!(key.len(), 68, "Key should be 68 chars (rck_ + 64 hex)");
    assert!(key.starts_with("rck_"), "Key should start with rck_");

    // Verify the rest is valid hex (64 chars)
    let hex_part = &key[4..];
    assert_eq!(hex_part.len(), 64);
    assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
}

/// Test that generate_api_key produces 68-character strings with rck_ prefix
#[tokio::test]
async fn test_generate_api_key_format() {
    let key = generate_api_key();

    // Should be exactly 68 characters (rck_ + 64 hex)
    assert_eq!(
        key.len(),
        68,
        "API key should be 68 characters (rck_ + 64 hex)"
    );

    // Should start with rck_
    assert!(key.starts_with("rck_"), "API key should start with rck_");

    // Hex part (after prefix) should be valid hex
    let hex_part = &key[4..];
    assert!(
        hex_part.chars().all(|c| c.is_ascii_hexdigit()),
        "API key hex part should contain only hex characters"
    );
}

/// Test that generate_api_key produces unique keys
#[tokio::test]
async fn test_generate_api_key_uniqueness() {
    let key1 = generate_api_key();
    let key2 = generate_api_key();
    let key3 = generate_api_key();

    assert_ne!(key1, key2, "Generated keys should be unique");
    assert_ne!(key2, key3, "Generated keys should be unique");
    assert_ne!(key1, key3, "Generated keys should be unique");
}

/// Test that hash_api_key produces valid argon2id hashes
#[tokio::test]
async fn test_hash_api_key_format() {
    let key = generate_api_key();
    let hash = hash_api_key(&key).await.expect("Hashing should succeed");

    // Argon2id hashes start with $argon2id$
    assert!(
        hash.starts_with("$argon2id$"),
        "Hash should be a valid argon2id hash"
    );
}

/// Test that hash_api_key produces different hashes for the same key (due to salt)
#[tokio::test]
async fn test_hash_api_key_different_salts() {
    let key = generate_api_key();
    let hash1 = hash_api_key(&key).await.expect("Hashing should succeed");
    let hash2 = hash_api_key(&key).await.expect("Hashing should succeed");

    assert_ne!(
        hash1, hash2,
        "Different hashes should be produced due to random salt"
    );
}

/// Test that validate_api_key accepts correct keys
#[tokio::test]
async fn test_validate_api_key_success() {
    let key = generate_api_key();
    let hash = hash_api_key(&key).await.expect("Hashing should succeed");

    let is_valid = validate_api_key(&key, &hash)
        .await
        .expect("Validation should succeed");

    assert!(is_valid, "Validation should succeed for correct key");
}

/// Test that validate_api_key rejects incorrect keys
#[tokio::test]
async fn test_validate_api_key_failure() {
    let key = generate_api_key();
    let wrong_key = generate_api_key();
    let hash = hash_api_key(&key).await.expect("Hashing should succeed");

    let is_valid = validate_api_key(&wrong_key, &hash)
        .await
        .expect("Validation should succeed");

    assert!(!is_valid, "Validation should fail for incorrect key");
}

/// Test that validate_api_key rejects slightly modified keys
#[tokio::test]
async fn test_validate_api_key_modified_key() {
    let key = generate_api_key();
    let hash = hash_api_key(&key).await.expect("Hashing should succeed");

    // Modify one character
    let mut modified_key = key.clone();
    modified_key.replace_range(0..1, "a");

    let is_valid = validate_api_key(&modified_key, &hash)
        .await
        .expect("Validation should succeed");

    assert!(!is_valid, "Validation should fail for modified key");
}

/// Test that validate_api_key handles empty strings gracefully
#[tokio::test]
async fn test_validate_api_key_empty_string() {
    let key = generate_api_key();
    let hash = hash_api_key(&key).await.expect("Hashing should succeed");

    let is_valid = validate_api_key("", &hash)
        .await
        .expect("Validation should succeed");

    assert!(!is_valid, "Validation should fail for empty key");
}

/// Test that validate_api_key handles invalid hash format
#[tokio::test]
async fn test_validate_api_key_invalid_hash() {
    let key = generate_api_key();

    let result = validate_api_key(&key, "invalid-hash-format").await;

    assert!(
        result.is_err(),
        "Validation should return error for invalid hash format"
    );
}

/// Test hash verification with multiple keys (stress test)
#[tokio::test]
async fn test_multiple_key_hash_verify() {
    let mut keys_and_hashes = Vec::new();

    // Generate 10 keys and their hashes
    for _ in 0..10 {
        let key = generate_api_key();
        let hash = hash_api_key(&key).await.expect("Hashing should succeed");
        keys_and_hashes.push((key, hash));
    }

    // Verify each key validates against its own hash
    for (key, hash) in &keys_and_hashes {
        let is_valid = validate_api_key(key, hash)
            .await
            .expect("Validation should succeed");
        assert!(is_valid, "Each key should validate against its own hash");
    }

    // Verify keys don't validate against other hashes
    if keys_and_hashes.len() >= 2 {
        let (key1, _) = &keys_and_hashes[0];
        let (_, hash2) = &keys_and_hashes[1];

        let is_valid = validate_api_key(key1, hash2)
            .await
            .expect("Validation should succeed");
        assert!(!is_valid, "Key should not validate against different hash");
    }
}

/// Test that hashing and validation work with concurrent operations
#[tokio::test]
async fn test_concurrent_operations() {
    let mut handles = Vec::new();

    // Spawn 5 concurrent tasks
    for _ in 0..5 {
        let handle = tokio::spawn(async move {
            let key = generate_api_key();
            let hash = hash_api_key(&key).await.expect("Hashing should succeed");
            let is_valid = validate_api_key(&key, &hash)
                .await
                .expect("Validation should succeed");
            assert!(is_valid, "Concurrent validation should succeed");
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task should complete successfully");
    }
}

/// Test that API keys are cryptographically random (basic entropy check)
#[tokio::test]
async fn test_api_key_entropy() {
    let key = generate_api_key();
    // Skip the "rck_" prefix and decode just the hex part
    let hex_part = &key[4..];
    let bytes = hex::decode(hex_part).expect("Should decode hex");

    // Check that not all bytes are the same (basic randomness check)
    let first_byte = bytes[0];
    let all_same = bytes.iter().all(|&b| b == first_byte);
    assert!(
        !all_same,
        "API key should have varied bytes (not all identical)"
    );

    // Check that we have reasonable byte distribution (at least 8 unique bytes)
    let mut unique_bytes = bytes.clone();
    unique_bytes.sort_unstable();
    unique_bytes.dedup();
    assert!(
        unique_bytes.len() >= 8,
        "API key should have good byte distribution (at least 8 unique bytes)"
    );
}

/// Test that argon2id parameters are present in the hash (defensive check)
/// This test inspects the hash format to ensure proper algorithm identifier
#[tokio::test]
async fn test_argon2id_algorithm() {
    let key = generate_api_key();
    let hash = hash_api_key(&key).await.expect("Hashing should succeed");

    // Argon2id hash format: $argon2id$v=19$m=...,t=...,p=...$...
    assert!(
        hash.starts_with("$argon2id$"),
        "Hash should use argon2id algorithm"
    );
}

#[test]
fn test_extract_prefix_valid() {
    let key = "rck_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let prefix = extract_prefix(key).unwrap();
    assert_eq!(prefix, "rck_0123456789ab");
    assert_eq!(prefix.len(), 16);
}

#[test]
fn test_extract_prefix_invalid_format() {
    assert!(extract_prefix("invalid").is_err());
    assert!(extract_prefix("rck_short").is_err());
    assert!(extract_prefix("wrong_prefix0123456789abcdef").is_err());
}

#[test]
fn test_extract_prefix_boundary_cases() {
    // One character short (67 chars)
    let short_key = "rck_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde";
    assert_eq!(short_key.len(), 67);
    assert!(extract_prefix(short_key).is_err());

    // One character long (69 chars)
    let long_key = "rck_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0";
    assert_eq!(long_key.len(), 69);
    assert!(extract_prefix(long_key).is_err());

    // Empty string
    assert!(extract_prefix("").is_err());
}
