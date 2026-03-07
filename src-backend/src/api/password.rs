use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

use super::errors::ApiError;

/// Hash a plain-text password with Argon2id and a random salt.
///
/// # Errors
///
/// Returns [`ApiError::Internal`] if the hashing operation fails.
pub fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ApiError::Internal(format!("Failed to hash password: {e}")))
}

/// Verify a plain-text password against a stored Argon2id hash.
///
/// # Errors
///
/// Returns [`ApiError::Internal`] if the stored hash is malformed.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, ApiError> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| ApiError::Internal(format!("Invalid password hash in DB: {e}")))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}
