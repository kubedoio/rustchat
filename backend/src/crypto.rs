//! Cryptography utilities
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use crate::error::AppError;

/// Encrypts a plaintext string using AES-GCM
pub fn encrypt(plaintext: &str, key: &str) -> String {
    let mc = new_magic_crypt!(key, 256);
    mc.encrypt_str_to_base64(plaintext)
}

/// Decrypts a ciphertext string using AES-GCM
pub fn decrypt(ciphertext: &str, key: &str) -> Result<String, AppError> {
    let mc = new_magic_crypt!(key, 256);
    mc.decrypt_base64_to_string(ciphertext)
        .map_err(|e| AppError::Internal(format!("Decryption failed: {}", e)))
}