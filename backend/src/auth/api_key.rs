//! API key generation and validation for non-human entities
//!
//! This module provides secure API key generation and argon2-based hashing for
//! authentication of agents, services, and CI/CD systems. API keys consist of
//! "rck_" prefix plus 64 hex characters (68 total) hashed with argon2id.
//!
//! # Security Features
//!
//! - Cryptographically secure random generation using `rand::thread_rng()`
//! - Argon2id hashing for defense against brute force
//! - Constant-time validation to resist timing attacks
//! - Async-safe operations using `tokio::task::spawn_blocking` for CPU-intensive work
//!
//! # Example
//!
//! ```no_run
//! use rustchat::auth::api_key::{generate_api_key, hash_api_key, validate_api_key};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Generate new API key for a service account
//!     let api_key = generate_api_key();
//!     println!("Generated API key: {}", api_key);
//!
//!     // Hash it for storage in database
//!     let hash = hash_api_key(&api_key).await?;
//!     println!("Hash for storage: {}", hash);
//!
//!     // Later, validate an incoming API key
//!     let is_valid = validate_api_key(&api_key, &hash).await?;
//!     assert!(is_valid);
//!
//!     Ok(())
//! }
//! ```

use crate::error::AppError;
use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::Rng;

/// Generate a new API key with "rck_" prefix plus 64 hex characters
///
/// Generates 32 random bytes, encodes as 64 hex characters, then prepends "rck_" prefix.
/// Format: rck_[64 hex chars] (total 68 characters)
/// Prefix: First 16 characters (rck_XXXXXXXXXXXX where X = first 12 hex chars)
///
/// # Returns
///
/// A 68-character API key with deterministic prefix
///
/// # Example
///
/// ```rust
/// use rustchat::auth::api_key::generate_api_key;
///
/// let key = generate_api_key();
/// assert_eq!(key.len(), 68);
/// assert!(key.starts_with("rck_"));
/// // Example output: "rck_7a9f3c8b2d1e4c6f89a12b34567890abcdef1234567890abcdef1234567890abcd"
/// //                      └── 64 hex chars ────────────────────────────────────────┘
/// ```
pub fn generate_api_key() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen(); // 32 bytes = 256 bits
    let hex_key = hex::encode(bytes); // 32 bytes → 64 hex chars
    format!("rck_{}", hex_key) // "rck_" + 64 hex = 68 total chars
}

/// Extract the first 16 characters as the API key prefix
///
/// **Note:** This function expects API keys in the format produced by the
/// updated `generate_api_key()` function (Task 3), which prepends "rck_" to
/// the 64-character hex string.
///
/// The prefix consists of:
/// - "rck_" (4 chars)
/// - First 12 hex characters (12 chars)
/// - Total: 16 characters
///
/// # Arguments
/// * `key` - Full API key with format: "rck_" + 64 hex chars = 68 chars total
///
/// # Returns
/// * `Ok(String)` - The 16-character prefix ("rck_" + first 12 hex chars)
/// * `Err(AppError)` - If key format is invalid (wrong length or missing "rck_" prefix)
///
/// # Example
/// ```no_run
/// use rustchat::auth::api_key::extract_prefix;
///
/// let key = "rck_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
/// let prefix = extract_prefix(key).unwrap();
/// assert_eq!(prefix, "rck_0123456789ab");
/// assert_eq!(prefix.len(), 16);
/// ```
pub fn extract_prefix(key: &str) -> Result<String, AppError> {
    if key.len() != 68 {
        return Err(AppError::Validation(format!(
            "Invalid key length: expected 68, got {}",
            key.len()
        )));
    }

    if !key.starts_with("rck_") {
        return Err(AppError::Validation(
            "Invalid key format: must start with rck_".to_string(),
        ));
    }

    let prefix = &key[..16];
    Ok(prefix.to_string())
}

/// Hash an API key using argon2id
///
/// This function performs CPU-intensive argon2 hashing in a blocking thread
/// pool to avoid blocking the async runtime. Uses the default argon2id
/// parameters, providing strong defense against brute force attacks.
///
/// # Arguments
///
/// * `api_key` - The plaintext API key to hash (68 chars: rck_ + 64 hex)
///
/// # Returns
///
/// An argon2id hash string (format: `$argon2id$...`) or an error
///
/// # Errors
///
/// Returns an error if argon2 hashing fails
///
/// # Example
///
/// ```no_run
/// use rustchat::auth::api_key::{generate_api_key, hash_api_key};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let api_key = generate_api_key();
///     let hash = hash_api_key(&api_key).await?;
///     println!("Hash: {}", hash);
///     Ok(())
/// }
/// ```
pub async fn hash_api_key(api_key: &str) -> Result<String> {
    let api_key = api_key.to_string();

    // Perform CPU-intensive argon2 hashing in blocking thread pool
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(api_key.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| anyhow::anyhow!("Failed to hash API key: {}", e))
    })
    .await
    .context("Argon2 hashing task panicked")?
}

/// Validate an API key against its argon2id hash
///
/// This function performs constant-time comparison using argon2's built-in
/// verification to resist timing attacks. The CPU-intensive verification is
/// performed in a blocking thread pool to avoid blocking the async runtime.
///
/// # Arguments
///
/// * `api_key` - The plaintext API key to validate
/// * `hash` - The argon2id hash to verify against
///
/// # Returns
///
/// `Ok(true)` if the API key matches the hash, `Ok(false)` if it doesn't match,
/// or an error if the hash format is invalid or verification fails
///
/// # Errors
///
/// Returns an error if:
/// - The hash format is invalid
/// - The verification process fails
/// - The blocking task panics
///
/// # Security
///
/// Uses constant-time comparison via argon2 to prevent timing attacks that
/// could leak information about the hash.
///
/// # Example
///
/// ```no_run
/// use rustchat::auth::api_key::{generate_api_key, hash_api_key, validate_api_key};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let api_key = generate_api_key();
///     let hash = hash_api_key(&api_key).await?;
///
///     let is_valid = validate_api_key(&api_key, &hash).await?;
///     assert!(is_valid);
///
///     let wrong_key = generate_api_key();
///     let is_invalid = validate_api_key(&wrong_key, &hash).await?;
///     assert!(!is_invalid);
///
///     Ok(())
/// }
/// ```
pub async fn validate_api_key(api_key: &str, hash: &str) -> Result<bool> {
    let api_key = api_key.to_string();
    let hash = hash.to_string();

    // Perform CPU-intensive argon2 verification in blocking thread pool
    tokio::task::spawn_blocking(move || {
        let parsed_hash = PasswordHash::new(&hash)
            .map_err(|e| anyhow::anyhow!("Invalid API key hash format: {}", e))?;
        Ok(Argon2::default()
            .verify_password(api_key.as_bytes(), &parsed_hash)
            .is_ok())
    })
    .await
    .context("Argon2 verification task panicked")?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key_sync() {
        let key = generate_api_key();
        assert_eq!(key.len(), 68);
        assert!(key.starts_with("rck_"));
        // Check hex part (after prefix)
        let hex_part = &key[4..];
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_api_key_uniqueness_sync() {
        let key1 = generate_api_key();
        let key2 = generate_api_key();
        assert_ne!(key1, key2);
    }

    #[tokio::test]
    async fn test_hash_and_validate() {
        let key = generate_api_key();
        let hash = hash_api_key(&key).await.unwrap();

        assert!(hash.starts_with("$argon2id$"));

        let is_valid = validate_api_key(&key, &hash).await.unwrap();
        assert!(is_valid);

        let wrong_key = generate_api_key();
        let is_invalid = validate_api_key(&wrong_key, &hash).await.unwrap();
        assert!(!is_invalid);
    }
}
