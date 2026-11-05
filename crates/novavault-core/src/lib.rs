//! Core library of NovaVault, a local-first encrypted vault.
//!
//! This crate owns everything that touches keys or plaintext secrets: key
//! derivation (Argon2id), authenticated encryption, the key-slot scheme
//! (vault key wrapped under the master password and the recovery code), and
//! reading, writing and validating the versioned vault format specified in
//! `docs/vault-format.md`.
//!
//! It deliberately has no dependency on Tauri or any UI so it can be reviewed
//! and tested on its own. Security invariants the implementation must keep:
//!
//! - Only audited cryptographic crates are used; no custom primitives.
//! - Secret material is held in `secrecy`/`zeroize` types and never logged.
//! - Raw key material never crosses the public API towards the UI.
//!
//! Modules:
//!
//! - [`vault`]: the unlocked [`Vault`]: create, open with the master
//!   password or the recovery code (migrating older format versions), list,
//!   read, insert, update, remove and save items atomically, add, export
//!   and remove file attachments, change the master password and
//!   regenerate the recovery code.
//! - [`item`]: the [`Item`] model: [`PasswordItem`], [`NoteItem`] and
//!   [`SecretItem`], identified by an [`ItemId`], with [`Attachment`]
//!   metadata.
//! - [`attachment`]: streaming, chunked encryption of attachment files.
//! - [`header`]: the versioned plaintext file [`Header`].
//! - [`slots`]: [`KeySlot`]s wrapping the vault key under the master
//!   password or the recovery code, and the [`KeySlots`] table with the
//!   create, unlock, change-password and regenerate-recovery-code
//!   operations.
//! - [`recovery`]: human-readable emergency [`RecoveryCode`]s with a
//!   typo-detecting checksum.
//! - [`kdf`]: Argon2id key derivation with validated [`KdfParams`].
//! - [`aead`]: XChaCha20-Poly1305 [`encrypt`] / [`decrypt`] with a fresh
//!   random nonce per message.
//! - [`keys`]: the zeroizing [`SecretKey`] type and [`Salt`].
//! - [`error`]: the typed [`CryptoError`] and [`VaultError`].
//!
//! Two private modules complete the picture for reviewers: `format` frames
//! the vault file's bytes and computes its integrity seal, and `atomic`
//! replaces files through a flushed temporary file so a crash never leaves
//! a half-written vault or export.

// No unsafe code in the trusted core, ever.
#![forbid(unsafe_code)]

pub mod aead;
mod atomic;
pub mod attachment;
pub mod error;
mod format;
pub mod header;
pub mod item;
pub mod kdf;
pub mod keys;
pub mod recovery;
pub mod slots;
#[cfg(test)]
mod test_util;
pub mod vault;

pub use aead::{NONCE_LEN, OVERHEAD, TAG_LEN, decrypt, encrypt};
pub use error::{CryptoError, Result, VaultError, VaultResult};
pub use header::{AeadId, FORMAT_VERSION, HEADER_LEN, Header, MAGIC, MIN_READ_VERSION, VaultId};
pub use item::{
    Attachment, AttachmentId, Item, ItemContent, ItemId, ItemKind, ItemSummary, NoteItem,
    PasswordItem, SecretItem,
};
pub use kdf::{KdfParams, derive_key};
pub use keys::{KEY_LEN, SALT_LEN, Salt, SecretKey};
pub use recovery::RecoveryCode;
pub use slots::{KeySlot, KeySlots, SLOT_LEN, SlotType};
pub use vault::{ATTACHMENTS_DIR_SUFFIX, Vault};
