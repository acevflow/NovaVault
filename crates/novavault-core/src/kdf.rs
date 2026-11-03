//! Argon2id key derivation.
//!
//! Turns a low-entropy secret (the master password or the recovery code)
//! into a 256-bit [`SecretKey`] used to wrap the vault key. Argon2id
//! (RFC 9106, version 0x13) is memory-hard, which makes offline guessing
//! against a stolen vault file expensive on GPUs and ASICs.
//!
//! Parameters are stored per key slot in the vault file, so they are
//! untrusted input when reading. [`KdfParams::new`] enforces both a floor
//! (so a tampered file cannot downgrade protection) and a ceiling (so a
//! crafted file cannot make the app allocate unbounded memory or spin for
//! hours).

use argon2::{Algorithm, Argon2, AssociatedData, ParamsBuilder, Version};
use secrecy::{ExposeSecret, ExposeSecretMut, SecretBox, SecretSlice};

use crate::error::{CryptoError, Result};
use crate::keys::{KEY_LEN, Salt, SecretKey};

/// Argon2id cost parameters.
///
/// The defaults follow the second recommended option of RFC 9106 §4
/// (64 MiB memory, 3 passes, 4 lanes), which targets well under a second
/// on a typical desktop while being far more expensive than the OWASP
/// minimum for an attacker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KdfParams {
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
}

impl KdfParams {
    /// Default memory cost: 64 MiB (RFC 9106 second recommended option).
    pub const DEFAULT_MEMORY_KIB: u32 = 64 * 1024;
    /// Default time cost (number of passes over memory).
    pub const DEFAULT_ITERATIONS: u32 = 3;
    /// Default degree of parallelism (lanes).
    pub const DEFAULT_PARALLELISM: u32 = 4;

    /// Minimum accepted memory cost: 19 MiB, the OWASP minimum for Argon2id.
    pub const MIN_MEMORY_KIB: u32 = 19 * 1024;
    /// Maximum accepted memory cost: 4 GiB, to bound allocation when
    /// reading untrusted parameters.
    pub const MAX_MEMORY_KIB: u32 = 4 * 1024 * 1024;
    /// Minimum accepted time cost (OWASP minimum at 19 MiB).
    pub const MIN_ITERATIONS: u32 = 2;
    /// Maximum accepted time cost, to bound unlock time.
    pub const MAX_ITERATIONS: u32 = 64;
    /// Minimum accepted parallelism.
    pub const MIN_PARALLELISM: u32 = 1;
    /// Maximum accepted parallelism.
    pub const MAX_PARALLELISM: u32 = 16;

    /// Creates validated parameters.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidKdfParams`] if any value is outside the
    /// `MIN_*`..=`MAX_*` bounds above.
    pub const fn new(memory_kib: u32, iterations: u32, parallelism: u32) -> Result<Self> {
        let params = Self {
            memory_kib,
            iterations,
            parallelism,
        };
        match params.validate() {
            Ok(()) => Ok(params),
            Err(err) => Err(err),
        }
    }

    /// Checks that the parameters are within NovaVault's accepted bounds.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidKdfParams`] describing the first
    /// violated bound.
    pub const fn validate(&self) -> Result<()> {
        let reason = if self.memory_kib < Self::MIN_MEMORY_KIB {
            "memory cost below minimum"
        } else if self.memory_kib > Self::MAX_MEMORY_KIB {
            "memory cost above maximum"
        } else if self.iterations < Self::MIN_ITERATIONS {
            "time cost below minimum"
        } else if self.iterations > Self::MAX_ITERATIONS {
            "time cost above maximum"
        } else if self.parallelism < Self::MIN_PARALLELISM {
            "parallelism below minimum"
        } else if self.parallelism > Self::MAX_PARALLELISM {
            "parallelism above maximum"
        } else {
            return Ok(());
        };
        Err(CryptoError::InvalidKdfParams { reason })
    }

    /// Memory cost in KiB.
    pub const fn memory_kib(&self) -> u32 {
        self.memory_kib
    }

    /// Time cost (number of passes).
    pub const fn iterations(&self) -> u32 {
        self.iterations
    }

    /// Degree of parallelism (lanes).
    pub const fn parallelism(&self) -> u32 {
        self.parallelism
    }
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            memory_kib: Self::DEFAULT_MEMORY_KIB,
            iterations: Self::DEFAULT_ITERATIONS,
            parallelism: Self::DEFAULT_PARALLELISM,
        }
    }
}

/// Derives a 256-bit key from `password` and `salt` with Argon2id.
///
/// The output is written directly into zeroizing storage. The same inputs
/// always produce the same key; use a fresh [`Salt`] for every new key slot.
///
/// # Errors
///
/// - [`CryptoError::InvalidKdfParams`] if `params` fail [`KdfParams::validate`]
///   (re-checked here so unchecked values can never reach Argon2).
/// - [`CryptoError::KeyDerivation`] if Argon2 itself fails, for example
///   when the memory cannot be allocated.
pub fn derive_key(
    password: &SecretSlice<u8>,
    salt: &Salt,
    params: &KdfParams,
) -> Result<SecretKey> {
    params.validate()?;
    let mut key = SecretBox::new(Box::new([0u8; KEY_LEN]));
    argon2id(
        password.expose_secret(),
        salt.as_bytes(),
        params,
        &[],
        &[],
        key.expose_secret_mut(),
    )?;
    Ok(SecretKey::from_secret_box(key))
}

/// Runs Argon2id v0x13 without NovaVault's parameter bounds.
///
/// `secret` (the optional Argon2 key, "K") and `ad` (associated data, "X")
/// are empty in production; they exist so the RFC 9106 test vector can be
/// checked through exactly the same parameter wiring that `derive_key` uses.
fn argon2id(
    password: &[u8],
    salt: &[u8],
    params: &KdfParams,
    secret: &[u8],
    ad: &[u8],
    out: &mut [u8],
) -> Result<()> {
    let mut builder = ParamsBuilder::new();
    builder
        .m_cost(params.memory_kib)
        .t_cost(params.iterations)
        .p_cost(params.parallelism)
        .output_len(out.len());
    if !ad.is_empty() {
        builder.data(AssociatedData::new(ad).map_err(|_| CryptoError::KeyDerivation)?);
    }
    let argon2_params = builder.build().map_err(|_| CryptoError::KeyDerivation)?;
    let argon2 = if secret.is_empty() {
        Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params)
    } else {
        Argon2::new_with_secret(secret, Algorithm::Argon2id, Version::V0x13, argon2_params)
            .map_err(|_| CryptoError::KeyDerivation)?
    };
    argon2
        .hash_password_into(password, salt, out)
        .map_err(|_| CryptoError::KeyDerivation)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Small parameters that bypass the production floor, for fast tests.
    const fn unchecked(memory_kib: u32, iterations: u32, parallelism: u32) -> KdfParams {
        KdfParams {
            memory_kib,
            iterations,
            parallelism,
        }
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn secret(bytes: &[u8]) -> SecretSlice<u8> {
        SecretSlice::from(bytes.to_vec())
    }

    #[test]
    fn rfc9106_argon2id_test_vector() {
        // RFC 9106 §5.3.
        let mut out = [0u8; 32];
        argon2id(
            &[0x01; 32],
            &[0x02; 16],
            &unchecked(32, 3, 4),
            &[0x03; 8],
            &[0x04; 12],
            &mut out,
        )
        .unwrap();
        assert_eq!(
            hex(&out),
            "0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659"
        );
    }

    #[test]
    fn reference_implementation_test_vector() {
        // phc-winner-argon2 `test.c`: Argon2id v0x13, t=2, m=2^16 KiB, p=1.
        let mut out = [0u8; 32];
        argon2id(
            b"password",
            b"somesalt",
            &unchecked(1 << 16, 2, 1),
            &[],
            &[],
            &mut out,
        )
        .unwrap();
        assert_eq!(
            hex(&out),
            "09316115d5cf24ed5a15a31a3ba326e5cf32edc24702987c02b6566f61913cf7"
        );
    }

    #[test]
    fn derive_key_is_deterministic_and_input_sensitive() {
        let params = KdfParams::new(KdfParams::MIN_MEMORY_KIB, 2, 1).unwrap();
        let salt = Salt::from_bytes([9; 16]);
        let other_salt = Salt::from_bytes([8; 16]);
        let password = secret(b"correct horse battery staple");

        let a = derive_key(&password, &salt, &params).unwrap();
        let b = derive_key(&password, &salt, &params).unwrap();
        let c = derive_key(&password, &other_salt, &params).unwrap();
        let d = derive_key(&secret(b"correct horse battery stapler"), &salt, &params).unwrap();

        assert_eq!(a.expose(), b.expose());
        assert_ne!(a.expose(), c.expose());
        assert_ne!(a.expose(), d.expose());
    }

    #[test]
    fn default_params_are_valid_rfc9106_option() {
        let params = KdfParams::default();
        assert_eq!(params.validate(), Ok(()));
        assert_eq!(
            (
                params.memory_kib(),
                params.iterations(),
                params.parallelism()
            ),
            (65_536, 3, 4)
        );
    }

    #[test]
    fn out_of_bounds_params_are_rejected() {
        let cases = [
            (KdfParams::MIN_MEMORY_KIB - 1, 3, 4),
            (KdfParams::MAX_MEMORY_KIB + 1, 3, 4),
            (KdfParams::DEFAULT_MEMORY_KIB, 1, 4),
            (KdfParams::DEFAULT_MEMORY_KIB, 65, 4),
            (KdfParams::DEFAULT_MEMORY_KIB, 3, 0),
            (KdfParams::DEFAULT_MEMORY_KIB, 3, 17),
        ];
        for (m, t, p) in cases {
            assert!(
                matches!(
                    KdfParams::new(m, t, p),
                    Err(CryptoError::InvalidKdfParams { .. })
                ),
                "accepted m={m} t={t} p={p}"
            );
        }
    }

    #[test]
    fn derive_key_rechecks_params() {
        let result = derive_key(
            &secret(b"pw"),
            &Salt::from_bytes([0; 16]),
            &unchecked(32, 3, 4),
        );
        assert!(matches!(result, Err(CryptoError::InvalidKdfParams { .. })));
    }
}
