//! Key and salt types.
//!
//! [`SecretKey`] holds 256-bit symmetric key material (the vault key or a
//! wrapping key derived with Argon2id). It is stored in a
//! [`secrecy::SecretBox`], so the bytes are zeroized when the key is dropped,
//! are never printed by `Debug`, and are only reachable inside this crate.
//! [`Salt`] is a non-secret, per-derivation random value.

use secrecy::{ExposeSecret, ExposeSecretMut, SecretBox};
use zeroize::Zeroizing;

use crate::error::{CryptoError, Result};

/// Length in bytes of a [`SecretKey`] (256 bits).
pub const KEY_LEN: usize = 32;

/// Length in bytes of a [`Salt`] (128 bits, as recommended by RFC 9106).
pub const SALT_LEN: usize = 16;

/// A 256-bit symmetric key, zeroized on drop.
///
/// The type intentionally does not implement `Clone`, so each key has a
/// single owner and a single place where it is wiped. Raw bytes are only
/// exposed inside the crate: callers outside the core (the Tauri shell, and
/// through it the UI) can hold and pass a key but never read it.
pub struct SecretKey(SecretBox<[u8; KEY_LEN]>);

impl SecretKey {
    /// Generates a new random key from the operating system's CSPRNG.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::Random`] if the OS random number generator fails.
    pub fn generate() -> Result<Self> {
        // Fill the boxed buffer in place so the key never lives in an
        // unzeroized stack temporary.
        let mut key = SecretBox::new(Box::new([0u8; KEY_LEN]));
        getrandom::fill(key.expose_secret_mut()).map_err(|_| CryptoError::Random)?;
        Ok(Self(key))
    }

    /// Builds a key from raw bytes, taking ownership of them.
    ///
    /// The caller's array is moved into zeroizing storage; callers should not
    /// keep other copies of the bytes.
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        // Wrap the moved-in array so this copy is wiped once it is boxed.
        let bytes = Zeroizing::new(bytes);
        Self(SecretBox::init_with_mut(|key: &mut [u8; KEY_LEN]| {
            key.copy_from_slice(bytes.as_slice());
        }))
    }

    /// Wraps key bytes already held in a [`SecretBox`], so producers such as
    /// the KDF can write output directly into zeroizing storage.
    pub(crate) fn from_secret_box(bytes: SecretBox<[u8; KEY_LEN]>) -> Self {
        Self(bytes)
    }

    /// Copies key bytes from a slice (for example a decrypted, zeroizing
    /// buffer) straight into zeroizing storage.
    ///
    /// Returns `None` if `bytes` is not exactly [`KEY_LEN`] long.
    pub(crate) fn try_from_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != KEY_LEN {
            return None;
        }
        Some(Self(SecretBox::init_with_mut(|key: &mut [u8; KEY_LEN]| {
            key.copy_from_slice(bytes);
        })))
    }

    /// Returns the raw key bytes. Crate-internal so key material never
    /// crosses the public API.
    pub(crate) fn expose(&self) -> &[u8; KEY_LEN] {
        self.0.expose_secret()
    }
}

impl core::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("SecretKey([REDACTED])")
    }
}

/// A random salt for Argon2id key derivation.
///
/// Salts are not secret: they are stored in clear in each key slot. Their
/// purpose is to make every derivation unique so precomputed attacks and
/// cross-slot comparisons are useless.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Salt([u8; SALT_LEN]);

impl Salt {
    /// Generates a new random salt from the operating system's CSPRNG.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::Random`] if the OS random number generator fails.
    pub fn generate() -> Result<Self> {
        let mut bytes = [0u8; SALT_LEN];
        getrandom::fill(&mut bytes).map_err(|_| CryptoError::Random)?;
        Ok(Self(bytes))
    }

    /// Builds a salt from bytes read from a vault file.
    pub const fn from_bytes(bytes: [u8; SALT_LEN]) -> Self {
        Self(bytes)
    }

    /// Returns the salt bytes, for example to store them in a key slot.
    pub const fn as_bytes(&self) -> &[u8; SALT_LEN] {
        &self.0
    }
}

impl core::fmt::Debug for Salt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Salt(")?;
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_are_distinct() {
        let a = SecretKey::generate().unwrap();
        let b = SecretKey::generate().unwrap();
        assert_ne!(a.expose(), b.expose());
        assert_ne!(a.expose(), &[0u8; KEY_LEN]);
    }

    #[test]
    fn from_bytes_preserves_key() {
        let key = SecretKey::from_bytes([7u8; KEY_LEN]);
        assert_eq!(key.expose(), &[7u8; KEY_LEN]);
    }

    #[test]
    fn try_from_slice_checks_length() {
        let key = SecretKey::try_from_slice(&[5u8; KEY_LEN]).unwrap();
        assert_eq!(key.expose(), &[5u8; KEY_LEN]);
        assert!(SecretKey::try_from_slice(&[5u8; KEY_LEN - 1]).is_none());
        assert!(SecretKey::try_from_slice(&[5u8; KEY_LEN + 1]).is_none());
    }

    #[test]
    fn debug_redacts_key_material() {
        let key = SecretKey::from_bytes([0xAB; KEY_LEN]);
        let printed = format!("{key:?}");
        assert_eq!(printed, "SecretKey([REDACTED])");
        assert!(!printed.to_lowercase().contains("ab"));
    }

    #[test]
    fn generated_salts_are_distinct() {
        assert_ne!(Salt::generate().unwrap(), Salt::generate().unwrap());
    }

    #[test]
    fn salt_round_trips_bytes() {
        let bytes = [1u8; SALT_LEN];
        assert_eq!(Salt::from_bytes(bytes).as_bytes(), &bytes);
        assert_eq!(
            format!("{:?}", Salt::from_bytes([0x0f; SALT_LEN])),
            format!("Salt({})", "0f".repeat(SALT_LEN))
        );
    }
}
