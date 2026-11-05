# [`crates/novavault-core/src/lib.rs`](../../../../../crates/novavault-core/src/lib.rs)

## Purpose

The crate root of `novavault-core`. This crate owns everything that touches keys or plaintext secrets: key derivation, authenticated encryption, the key-slot scheme, and the versioned vault format. The root declares the modules, forbids `unsafe` code, and re-exports the public API. The crate has no dependency on Tauri or any UI, so it can be reviewed and tested on its own.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `aead` | module | `pub mod aead;` | XChaCha20-Poly1305 with random nonces. |
| `attachment` | module | `pub mod attachment;` | Streaming chunked encryption of attachments. |
| `error` | module | `pub mod error;` | `CryptoError`, `VaultError`. |
| `header` | module | `pub mod header;` | The plaintext versioned header. |
| `item` | module | `pub mod item;` | The item model and payload encoding. |
| `kdf` | module | `pub mod kdf;` | Argon2id with validated parameters. |
| `keys` | module | `pub mod keys;` | `SecretKey`, `Salt`. |
| `recovery` | module | `pub mod recovery;` | Recovery codes. |
| `slots` | module | `pub mod slots;` | Key slots and the slot table. |
| `vault` | module | `pub mod vault;` | The unlocked `Vault`. |
| re-exports | `pub use` | `pub use aead::{NONCE_LEN, OVERHEAD, TAG_LEN, decrypt, encrypt};` | See [`aead`](aead.rs.md). |
| re-exports | `pub use` | `pub use error::{CryptoError, Result, VaultError, VaultResult};` | See [`error`](error.rs.md). |
| re-exports | `pub use` | `pub use header::{AeadId, FORMAT_VERSION, HEADER_LEN, Header, MAGIC, MIN_READ_VERSION, VaultId};` | See [`header`](header.rs.md). |
| re-exports | `pub use` | `pub use item::{Attachment, AttachmentId, Item, ItemContent, ItemId, ItemKind, ItemSummary, NoteItem, PasswordItem, SecretItem};` | See [`item`](item.rs.md). |
| re-exports | `pub use` | `pub use kdf::{KdfParams, derive_key};` | See [`kdf`](kdf.rs.md). |
| re-exports | `pub use` | `pub use keys::{KEY_LEN, SALT_LEN, Salt, SecretKey};` | See [`keys`](keys.rs.md). |
| re-exports | `pub use` | `pub use recovery::RecoveryCode;` | See [`recovery`](recovery.rs.md). |
| re-exports | `pub use` | `pub use slots::{KeySlot, KeySlots, SLOT_LEN, SlotType};` | See [`slots`](slots.rs.md). |
| re-exports | `pub use` | `pub use vault::{ATTACHMENTS_DIR_SUFFIX, Vault};` | See [`vault`](vault.rs.md). |

Private modules: `atomic` ([page](atomic.rs.md)), `format` ([page](format.rs.md)), and `test_util` ([page](test_util.rs.md), compiled only with `#[cfg(test)]`).

## Details

### `#![forbid(unsafe_code)]`

The crate rejects any `unsafe` block, and `forbid` means no inner `allow` can override it. The comment reads "No unsafe code in the trusted core, ever." The only `unsafe` code in the project lives in the shell's `os_lock` modules.

### Module declarations

Ten public modules and three private ones. `test_util` exists only in test builds.

- **Security**: the module documentation states the invariants the implementation must keep. The pages linked below show where each one is enforced.
  - Only audited cryptographic crates are used, and no custom primitives: see [`aead`](aead.rs.md) and [`kdf`](kdf.rs.md).
  - Secret material is held in `secrecy` or `zeroize` types and never logged: see [`keys`](keys.rs.md), [`recovery`](recovery.rs.md) and [`item`](item.rs.md).
  - Raw key material never crosses the public API toward the UI: `SecretKey::expose` is `pub(crate)`.

### Re-exports

The root re-exports what the shell needs, so it can write `use novavault_core::{Vault, KdfParams, …}`. Some public items are reachable only through their module path:

- `header::{SLOT_COUNT, VAULT_ID_LEN}`;
- `slots::{SLOT_METADATA_LEN, SEALED_KEY_LEN}`;
- `item::{validate_attachment_name, ITEM_ID_LEN, ATTACHMENT_ID_LEN, MAX_ATTACHMENTS_PER_ITEM, MAX_ATTACHMENT_NAME_LEN, PAD_BLOCK, MAX_PAYLOAD_LEN}`;
- `recovery::{ALPHABET, DATA_LEN, CHECKSUM_LEN, CODE_LEN, GROUP_LEN}`;
- the constants and `blob_len` of `attachment`.

## Dependencies

- **Internal**: all modules of the crate.
- **External**: none directly. The crate's dependencies are `argon2`, `chacha20poly1305`, `getrandom`, `secrecy`, `thiserror` and `zeroize`, each justified in [dependencies.md](../../../../dependencies.md#core-cratesnovavault-core).

## Used by

- [`src-tauri/src/session.rs`](../../../../../src-tauri/src/session.rs): `ItemContent`, `KdfParams`, `RecoveryCode`, `Vault`, `VaultError`.
- [`src-tauri/src/dto.rs`](../../../../../src-tauri/src/dto.rs): the item types, `RecoveryCode`.
- [`src-tauri/src/error.rs`](../../../../../src-tauri/src/error.rs): `CryptoError`, `VaultError`.
- [`src-tauri/src/main.rs`](../../../../../src-tauri/src/main.rs): `KdfParams`.

## Tests

_Not applicable: the crate root has no tests of its own. Each module's tests are listed on its page._

## Notes

- **Module list says "Two private modules".** The module documentation describes `format` and `atomic` as "two private modules". There is also a third private module, `test_util`, which is compiled only for tests.
- **Layering claim does not match the imports.** [architecture.md](../../../../architecture.md#core-library) says the core is "organized in layers, each using only the ones below it". Its table lists `item` above `attachment` and `format`, yet both `attachment` (for `AttachmentId`) and `format` (for `ItemId`, `ItemKind` and `MAX_PAYLOAD_LEN`) import from `item`. The dependency graph has no cycles, but it does not follow the table's order.
