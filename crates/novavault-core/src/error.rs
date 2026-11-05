//! Typed errors for `novavault-core`.
//!
//! Variants deliberately carry no secret material and no detail that could
//! help an attacker distinguish failure causes: in particular, every
//! authentication failure during decryption maps to the single
//! [`CryptoError::Decryption`] variant, whether the key was wrong, the
//! ciphertext was tampered with or the associated data did not match.
//! File-level failures use [`VaultError`].

use thiserror::Error;

/// Errors returned by `novavault-core`: cryptographic primitives, recovery
/// codes and key slots.
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

    /// A recovery code does not have the expected number of characters
    /// (separators and whitespace excluded).
    #[error("recovery code has the wrong length")]
    RecoveryCodeLength,

    /// A recovery code contains a character outside its alphabet.
    ///
    /// The offending character is deliberately not reported, so the error
    /// can be logged without leaking any part of the code.
    #[error("recovery code contains an invalid character")]
    RecoveryCodeCharacter,

    /// A recovery code's checksum does not match: most likely a typo.
    #[error("recovery code checksum mismatch")]
    RecoveryCodeChecksum,

    /// A serialized key slot is structurally invalid.
    ///
    /// `reason` is a static description of the problem; it never contains
    /// data read from the slot.
    #[error("malformed key slot: {reason}")]
    MalformedKeySlot {
        /// What is wrong with the slot.
        reason: &'static str,
    },
}

/// Convenience alias for results of cryptographic operations.
pub type Result<T> = core::result::Result<T, CryptoError>;

/// Errors returned when creating, opening, editing or saving a vault file.
///
/// Like [`CryptoError`], no variant carries secret material or bytes read
/// from the file. Failures that could reveal whether the secret or the file
/// was at fault while unlocking are folded into
/// [`VaultError::InvalidCredentials`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum VaultError {
    /// Reading or writing the vault file failed.
    #[error("vault file I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The file does not start with the NovaVault magic bytes.
    #[error("not a NovaVault vault file")]
    NotAVault,

    /// The file uses a format version this build cannot read.
    #[error("unsupported vault format version {found}")]
    UnsupportedVersion {
        /// The version found in the file header.
        found: u16,
    },

    /// The file declares an authenticated encryption algorithm this build
    /// does not know.
    #[error("unsupported encryption algorithm identifier {id}")]
    UnsupportedAlgorithm {
        /// The algorithm identifier found in the file header.
        id: u8,
    },

    /// The file is structurally invalid or failed an integrity check.
    ///
    /// `reason` is a static description; it never contains file contents.
    #[error("corrupted vault file: {reason}")]
    Corrupted {
        /// What is wrong with the file.
        reason: &'static str,
    },

    /// The master password or recovery code is wrong, or the key slot is
    /// corrupted. These cases are intentionally indistinguishable.
    #[error("wrong password or recovery code, or corrupted key slot")]
    InvalidCredentials,

    /// No item with the requested identifier exists in the vault.
    #[error("item not found")]
    ItemNotFound,

    /// A vault file already exists at the target path.
    #[error("a file already exists at the vault path")]
    AlreadyExists,

    /// An item cannot be stored, for example because it is too large.
    #[error("invalid item: {reason}")]
    InvalidItem {
        /// Why the item was rejected.
        reason: &'static str,
    },

    /// The item has no attachment with the requested identifier.
    #[error("attachment not found")]
    AttachmentNotFound,

    /// A long operation (adding or exporting an attachment) was stopped by
    /// its caller before it finished. Nothing was changed.
    #[error("operation cancelled")]
    Cancelled,

    /// A cryptographic operation failed outside of unlocking.
    #[error(transparent)]
    Crypto(#[from] CryptoError),
}

/// Convenience alias for results of vault file operations.
pub type VaultResult<T> = core::result::Result<T, VaultError>;

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

    #[test]
    fn unlock_failure_message_is_generic() {
        assert_eq!(
            VaultError::InvalidCredentials.to_string(),
            "wrong password or recovery code, or corrupted key slot"
        );
    }

    #[test]
    fn crypto_errors_convert() {
        let error = VaultError::from(CryptoError::Random);
        assert!(matches!(error, VaultError::Crypto(CryptoError::Random)));
    }
}
