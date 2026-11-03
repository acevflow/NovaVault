//! Core library of NovaVault, a local-first encrypted vault.
//!
//! This crate will own everything that touches keys or plaintext secrets:
//! key derivation (Argon2id), authenticated encryption, the key-slot scheme
//! (vault key wrapped under the master password and the recovery code), and
//! reading, writing and validating the versioned vault format described in
//! `docs/vault-format.md`.
//!
//! It deliberately has no dependency on Tauri or any UI so it can be reviewed
//! and tested on its own. Security invariants the implementation must keep:
//!
//! - Only audited cryptographic crates are used; no custom primitives.
//! - Secret material is held in `secrecy`/`zeroize` types and never logged.
//! - Raw key material never crosses the public API towards the UI.
//!
//! Currently implemented: the cryptographic primitives the rest of the core
//! builds on.
//!
//! - [`kdf`]: Argon2id key derivation with validated [`KdfParams`].
//! - [`aead`]: XChaCha20-Poly1305 [`encrypt`] / [`decrypt`] with a fresh
//!   random nonce per message.
//! - [`keys`]: the zeroizing [`SecretKey`] type and [`Salt`].
//! - [`error`]: the typed [`CryptoError`].
//!
//! Key slots and the vault format land in later releases.

// No unsafe code in the trusted core, ever.
#![forbid(unsafe_code)]

pub mod aead;
pub mod error;
pub mod kdf;
pub mod keys;

pub use aead::{NONCE_LEN, OVERHEAD, TAG_LEN, decrypt, encrypt};
pub use error::{CryptoError, Result};
pub use kdf::{KdfParams, derive_key};
pub use keys::{KEY_LEN, SALT_LEN, Salt, SecretKey};
