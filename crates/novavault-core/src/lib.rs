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
//! - [`recovery`]: human-readable emergency [`RecoveryCode`]s with a
//!   typo-detecting checksum.
//! - [`slots`]: [`KeySlot`]s wrapping the vault key under the master
//!   password or the recovery code, and the [`KeySlots`] table with the
//!   create, unlock, change-password and regenerate-recovery-code
//!   operations.
//! - [`error`]: the typed [`CryptoError`].
//!
//! The rest of the vault format lands in later releases.

// No unsafe code in the trusted core, ever.
#![forbid(unsafe_code)]

pub mod aead;
pub mod error;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the framing is used by the vault module")
)]
mod format;
pub mod header;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the payload codec is used by the vault module")
)]
pub mod item;
pub mod kdf;
pub mod keys;
pub mod recovery;
pub mod slots;

pub use aead::{NONCE_LEN, OVERHEAD, TAG_LEN, decrypt, encrypt};
pub use error::{CryptoError, Result, VaultError, VaultResult};
pub use header::{AeadId, FORMAT_VERSION, HEADER_LEN, Header, MAGIC, VaultId};
pub use item::{
    Item, ItemContent, ItemId, ItemKind, ItemSummary, NoteItem, PasswordItem, SecretItem,
};
pub use kdf::{KdfParams, derive_key};
pub use keys::{KEY_LEN, SALT_LEN, Salt, SecretKey};
pub use recovery::RecoveryCode;
pub use slots::{KeySlot, KeySlots, SLOT_LEN, SlotType};
