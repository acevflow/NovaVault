//! Typed errors for the cryptographic primitives.
//!
//! Variants deliberately carry no secret material and no detail that could
//! help an attacker distinguish failure causes: in particular, every
//! authentication failure during decryption maps to the single
//! [`CryptoError::Decryption`] variant, whether the key was wrong, the
//! ciphertext was tampered with or the associated data did not match.

use thiserror::Error;

/// Errors returned by the cryptographic primitives of `novavault-core`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum CryptoError {
    /// Argon2id parameters are outside the bounds accepted by NovaVault.
    ///
    /// `reason` is a static, human-readable description of the violated
    /// bound; it never contains user data.
    #[error("invalid key derivation parameters: {reason}")]
    InvalidKdfParams {
        /// Which bound was violated.
        reason: &'static str,
    },

    /// Argon2id failed to derive a key (for example, out of memory).
    #[error("key derivation failed")]
    KeyDerivation,

    /// Authenticated encryption failed.
    #[error("encryption failed")]
    Encryption,

    /// Authenticated decryption failed: wrong key, tampered data or
    /// mismatched associated data. These causes are intentionally not
    /// distinguished.
    #[error("decryption failed: wrong key or corrupted data")]
    Decryption,

    /// The input is too short to be a valid sealed message.
    #[error("malformed ciphertext")]
    MalformedCiphertext,

    /// The operating system's random number generator failed.
    #[error("random number generator failure")]
    Random,
}

/// Convenience alias for results of cryptographic operations.
pub type Result<T> = core::result::Result<T, CryptoError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decryption_message_is_generic() {
        // The message must not reveal whether the key or the data was at fault.
        assert_eq!(
            CryptoError::Decryption.to_string(),
            "decryption failed: wrong key or corrupted data"
        );
    }
}
