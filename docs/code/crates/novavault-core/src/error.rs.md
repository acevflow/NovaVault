# [`crates/novavault-core/src/error.rs`](../../../../../crates/novavault-core/src/error.rs)

## Purpose

Defines the two typed error enums of the core crate: `CryptoError` for primitives, recovery codes and key slots, and `VaultError` for vault file operations. The variants are designed so that no error carries secret material or file bytes. Authentication failures are collapsed into a single variant, so an error never tells an attacker which check failed.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `CryptoError` | enum | `#[non_exhaustive] pub enum CryptoError` | Errors of KDF, AEAD, randomness, recovery codes and key slots. `Debug, Clone, Copy, PartialEq, Eq, Error`. |
| `Result` | type alias | `pub type Result<T> = core::result::Result<T, CryptoError>;` | Result of cryptographic operations. |
| `VaultError` | enum | `#[non_exhaustive] pub enum VaultError` | Errors of creating, opening, editing and saving a vault. `Debug, Error`. |
| `VaultResult` | type alias | `pub type VaultResult<T> = core::result::Result<T, VaultError>;` | Result of vault file operations. |

All four are re-exported from the crate root (`novavault_core::CryptoError`, and so on).

## Details

### `CryptoError`

| Variant | Display message | Raised when |
| ------- | --------------- | ----------- |
| `InvalidKdfParams { reason: &'static str }` | `invalid key derivation parameters: {reason}` | Argon2id parameters are outside the bounds in [`kdf`](kdf.rs.md) (`KdfParams::new`, `KdfParams::validate`, `derive_key`). |
| `KeyDerivation` | `key derivation failed` | Argon2 itself fails, for example when it cannot allocate memory. |
| `Encryption` | `encryption failed` | AEAD encryption fails (message too long for the cipher), or a sealed key has an unexpected length while a slot is being wrapped. |
| `Decryption` | `decryption failed: wrong key or corrupted data` | AEAD authentication fails: wrong key, modified nonce, ciphertext or tag, or different associated data. |
| `MalformedCiphertext` | `malformed ciphertext` | A sealed message is shorter than `OVERHEAD` (40 bytes). This is checked before decrypting. |
| `Random` | `random number generator failure` | `getrandom` fails. |
| `RecoveryCodeLength` | `recovery code has the wrong length` | A parsed recovery code does not have exactly 32 symbols. |
| `RecoveryCodeCharacter` | `recovery code contains an invalid character` | A parsed recovery code contains a character outside the alphabet. |
| `RecoveryCodeChecksum` | `recovery code checksum mismatch` | A parsed recovery code fails its checksum. |
| `MalformedKeySlot { reason: &'static str }` | `malformed key slot: {reason}` | A serialized slot has a wrong length or unknown type, a slot is used with the wrong secret type, or a slot table is out of order. |

- **Security**: `reason` fields are `&'static str`, so they can only be string literals written in the source. They never contain user data or bytes read from a file. `RecoveryCodeCharacter` deliberately does not report which character was wrong, so the error can be logged without leaking any part of the code. `Decryption` covers every authentication failure, so callers cannot tell a wrong key from tampered data.
- **Edge cases and invariants**: `#[non_exhaustive]`, so callers outside the crate must match with a wildcard arm. The type is `Copy`, which works because no variant owns heap data.

### `Result`

Shorthand for `core::result::Result<T, CryptoError>`, used by the `aead`, `kdf`, `keys`, `recovery`, `slots` and `header` (`generate` only) modules.

### `VaultError`

| Variant | Display message | Raised when |
| ------- | --------------- | ----------- |
| `Io(#[from] std::io::Error)` | `vault file I/O error: {0}` | Reading or writing the vault file, an attachment blob, an attachment source or an export destination fails. |
| `NotAVault` | `not a NovaVault vault file` | The file does not start with `MAGIC`. |
| `UnsupportedVersion { found: u16 }` | `unsupported vault format version {found}` | The vault header version is outside `MIN_READ_VERSION..=FORMAT_VERSION`, or an attachment blob's version is not `BLOB_VERSION`. |
| `UnsupportedAlgorithm { id: u8 }` | `unsupported encryption algorithm identifier {id}` | The header or blob header declares an unknown `aead_id`. |
| `Corrupted { reason: &'static str }` | `corrupted vault file: {reason}` | Any structural or integrity failure: bad framing, failed seal, failed record authentication, malformed payload, damaged or missing attachment blob. |
| `InvalidCredentials` | `wrong password or recovery code, or corrupted key slot` | Unwrapping the key slot fails with `CryptoError::Decryption` while opening or verifying a credential. |
| `ItemNotFound` | `item not found` | No item has the requested `ItemId`. |
| `AlreadyExists` | `a file already exists at the vault path` | `Vault::create` finds anything at the target path, a dangling symbolic link included. |
| `InvalidItem { reason: &'static str }` | `invalid item: {reason}` | An item, attachment name or export destination is rejected, or the vault exceeds a format limit. |
| `AttachmentNotFound` | `attachment not found` | The item has no attachment with the requested `AttachmentId`. |
| `Cancelled` | `operation cancelled` | A progress callback returned `ControlFlow::Break` during an attachment transfer. |
| `Crypto(#[from] CryptoError)` | transparent (the inner message) | Any other `CryptoError`, for example `Random` or `KeyDerivation`. |

- **Security**: no variant holds secret material or file contents. `InvalidCredentials` folds a wrong secret and a damaged slot into one case. The mapping from `CryptoError::Decryption` happens in [`vault`](vault.rs.md), not here. `Io` wraps `std::io::Error`, whose message can include an OS path but never vault contents.
- **Edge cases and invariants**: `#[non_exhaustive]`. `VaultError` is not `Clone`, `Copy` or `PartialEq`, because `std::io::Error` is none of those. `From<std::io::Error>` and `From<CryptoError>` let the `?` operator convert both.

### `VaultResult`

Shorthand for `core::result::Result<T, VaultError>`, used by [`header`](header.rs.md) (parsing), [`item`](item.rs.md), [`format`](format.rs.md), [`attachment`](attachment.rs.md) and [`vault`](vault.rs.md).

## Dependencies

- **Internal**: none.
- **External**: `thiserror`: derives `std::error::Error` and `Display` from the `#[error(...)]` attributes, with no runtime code.

## Used by

- Every other module of the crate: [`aead`](aead.rs.md), [`kdf`](kdf.rs.md), [`keys`](keys.rs.md), [`recovery`](recovery.rs.md), [`header`](header.rs.md), [`slots`](slots.rs.md), [`item`](item.rs.md), [`format`](format.rs.md), [`attachment`](attachment.rs.md) and [`vault`](vault.rs.md).
- [`lib.rs`](lib.rs.md): re-exports `CryptoError`, `Result`, `VaultError` and `VaultResult`.
- [`src-tauri/src/error.rs`](../../../../../src-tauri/src/error.rs): maps both enums to the shell's command errors.

## Tests

- [`decryption_message_is_generic`](../../../../../crates/novavault-core/src/error.rs#L168): the `Decryption` message does not say whether the key or the data was at fault.
- [`unlock_failure_message_is_generic`](../../../../../crates/novavault-core/src/error.rs#L177): the `InvalidCredentials` message covers a wrong secret and a corrupted slot alike.
- [`crypto_errors_convert`](../../../../../crates/novavault-core/src/error.rs#L185): `VaultError::from(CryptoError)` produces `VaultError::Crypto`.

## Notes

- The doc comment on `CryptoError` says it covers "cryptographic primitives, recovery codes and key slots". In practice `Encryption` is also returned when a freshly sealed vault key has an unexpected length in `KeySlot::wrap`, a case that cannot happen with the fixed sizes in use.
- `VaultError::UnsupportedVersion` is shared by the vault header and the attachment blob header, even though the two version numbers are independent (`FORMAT_VERSION` and `BLOB_VERSION`). From the error alone, a caller cannot tell which of the two it refers to.
- `VaultError::Corrupted` has the message prefix `corrupted vault file:`, including for attachment blob failures and for a record that fails authentication in memory. The vault file itself may be intact in those cases.
