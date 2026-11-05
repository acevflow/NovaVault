# [`src-tauri/src/error.rs`](../../../../src-tauri/src/error.rs)

## Purpose

Defines `CommandError`, the only error type that crosses the Rust/UI boundary, and its stable string codes. Every core error is mapped to one of its variants and loses its details on the way, so a failed command never sends a path, a corruption reason or any command argument to the webview.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `CommandError` | enum | `pub enum CommandError` | A failed command as reported to the UI. `Debug, Clone, Copy, PartialEq, Eq, Error`. Not `#[non_exhaustive]`. |
| `CommandError::code` | method | `pub const fn code(self) -> &'static str` | Stable snake_case code the UI branches on. |
| `impl Serialize for CommandError` | trait impl | `fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>` | Hand-written: emits `{ "code", "message" }`. |
| `impl From<VaultError> for CommandError` | trait impl | `fn from(error: VaultError) -> Self` | Maps a core vault error, dropping its details. |
| `impl From<CryptoError> for CommandError` | trait impl | `fn from(error: CryptoError) -> Self` | Maps a core cryptography error, dropping its details. |
| `CommandResult` | type alias | `pub type CommandResult<T> = Result<T, CommandError>;` | Result of every command and of the session logic. |

## Details

### `CommandError`

| Variant | Code | Message | Raised when |
| ------- | ---- | ------- | ----------- |
| `Locked` | `locked` | `the vault is locked` | A command that needs the vault runs while it is locked ([`Session::vault`](session.rs.md#vault-and-vault_mut-private)). |
| `VaultNotFound` | `vault_not_found` | `no vault exists yet` | `unlock` finds no vault file. |
| `VaultExists` | `vault_exists` | `a vault already exists` | `create_vault` finds a file at the vault path (`VaultError::AlreadyExists`). |
| `InvalidCredentials` | `invalid_credentials` | `wrong password or recovery code` | Wrong password or recovery code, or a damaged key slot. These cases are deliberately the same. |
| `InvalidRecoveryCode` | `invalid_recovery_code` | `the recovery code is not valid; check it for typos` | A recovery code fails parsing (length, character or checksum), before any key derivation. |
| `InvalidInput` | `invalid_input` | `invalid input` | A malformed argument: empty password, malformed identifier, unknown field for the item kind, kind mismatch on update, out-of-range setting, or a picked path without a file name. |
| `ItemNotFound` | `item_not_found` | `item not found` | No item has the identifier. |
| `AttachmentNotFound` | `attachment_not_found` | `attachment not found` | The item has no such attachment. |
| `FileAccess` | `file_access` | `the file could not be read or written` | A file dialog fails, or a user-picked file cannot be read, or an export destination cannot be written (or is a vault file). |
| `Cancelled` | `cancelled` | `the operation was cancelled` | A lock stopped an attachment transfer. |
| `InvalidItem` | `invalid_item` | `the item cannot be stored` | The core rejects an item or attachment name, or a format limit is reached. |
| `UnsupportedVault` | `unsupported_vault` | `the vault file is not supported by this version of NovaVault` | Not a vault, or an unknown version or algorithm. |
| `Corrupted` | `corrupted` | `the vault file is corrupted` | A structural or integrity check fails, in the vault file or in an attachment blob. |
| `Storage` | `storage` | `the vault file could not be read or written` | I/O on the vault file, its directory or the settings file fails. |
| `ClipboardUnavailable` | `clipboard_unavailable` | `the clipboard is not available` | The clipboard cannot be opened or written. |
| `Internal` | `internal` | `internal error` | Anything unexpected: a worker thread panic, a stopped clipboard worker, a random number generator failure, or a future core variant. |

- **Security**: every message is a string literal from `#[error(...)]`. No variant has fields, so nothing a command received and no detail from the core can be stored in the error. That keeps it out of the UI and out of logs.
- **Edge cases and invariants**: the type is `Copy`, which works because no variant owns data.

### `CommandError::code`

Returns the code from the table above. The doc comment calls these strings a contract with `ui/src/api`. Renaming one breaks both sides.

### `impl Serialize for CommandError`

Serializes a struct named `CommandError` with two fields: `code` (from `code()`) and `message` (from `Display`). For example, `Locked` becomes `{ "code": "locked", "message": "the vault is locked" }`. A derived `Serialize` would emit only the variant name, which is why this is written by hand.

### `impl From<VaultError> for CommandError`

| Core error | Becomes |
| ---------- | ------- |
| `Io(_)` | `Storage` |
| `NotAVault`, `UnsupportedVersion { .. }`, `UnsupportedAlgorithm { .. }` | `UnsupportedVault` |
| `Corrupted { .. }` | `Corrupted` |
| `InvalidCredentials` | `InvalidCredentials` |
| `ItemNotFound` | `ItemNotFound` |
| `AlreadyExists` | `VaultExists` |
| `InvalidItem { .. }` | `InvalidItem` |
| `AttachmentNotFound` | `AttachmentNotFound` |
| `Cancelled` | `Cancelled` |
| `Crypto(error)` | the `From<CryptoError>` mapping below |
| any other (the enum is `#[non_exhaustive]`) | `Internal` |

- **Security**: the I/O error's text (which can hold a path), the `reason` strings and the version and algorithm numbers are all dropped.
- **Edge cases and invariants**: attachment transfers map `Io` to `FileAccess` instead, through [`file_error`](session.rs.md#file_error-and-password_bytes-private) in `session.rs`. Export also maps `InvalidItem` to `FileAccess`. Both happen before this conversion.

### `impl From<CryptoError> for CommandError`

| Core error | Becomes |
| ---------- | ------- |
| `RecoveryCodeLength`, `RecoveryCodeCharacter`, `RecoveryCodeChecksum` | `InvalidRecoveryCode` |
| `Decryption` | `InvalidCredentials` |
| `MalformedKeySlot { .. }`, `MalformedCiphertext` | `Corrupted` |
| any other (`InvalidKdfParams`, `KeyDerivation`, `Encryption`, `Random`, future variants) | `Internal` |

- **Security**: a failed decryption is always reported as wrong credentials, never as a separate crypto failure.

### `CommandResult`

Shorthand for `Result<T, CommandError>`. The return type of every handler in [`commands.rs`](commands.rs.md) and every fallible method of [`Session`](session.rs.md).

## Dependencies

- **Internal**: none.
- **External**:
  - `novavault_core`: `CryptoError` and `VaultError`, the source types of the mappings.
  - `serde`: the hand-written `Serialize`.
  - `thiserror`: derives `Display` and `Error` from the messages.

## Used by

- [`commands.rs`](commands.rs.md): the return type of every command.
- [`session.rs`](session.rs.md): the result of every session operation, plus the `?` conversion from core errors.
- [`dto.rs`](dto.rs.md): `InvalidInput` from identifier parsing, field lookup and update kind checks.
- [`settings.rs`](settings.rs.md): `InvalidInput` for out-of-range values.
- [`clipboard.rs`](clipboard.rs.md): `ClipboardUnavailable` and `Internal`.
- `ui/src/api/errors.ts`: the UI side of the code contract.

## Tests

- [`serializes_as_code_and_message`](../../../../src-tauri/src/error.rs#L196): `Locked` serializes to the expected `{ code, message }` object, and the 16 codes are unique.
- [`io_details_are_dropped`](../../../../src-tauri/src/error.rs#L209): an I/O error whose text holds a user name and path becomes `Storage`, and neither string appears in the JSON.
- [`core_errors_map_to_codes`](../../../../src-tauri/src/error.rs#L221): a sample of `VaultError` variants maps as in the table above, `Crypto(Random)` maps to `Internal`, and all three recovery code parse errors map to `InvalidRecoveryCode`.
- [`messages_are_fixed_strings`](../../../../src-tauri/src/error.rs#L261): for every variant, the JSON has exactly two fields, equal to `code()` and `to_string()`.

## Notes

- `VaultError::Io` is reported as `Storage`, whose message names "the vault file", even when the failing file is `settings.json`. The settings code produces `Storage` directly. The `storage` row of [`docs/api-reference.md`](../../../api-reference.md#errors) mentions the settings file, but the message does not.
- `Corrupted` covers attachment blob damage too, while its message says "the vault file".
- A key slot whose ciphertext was altered fails authentication and is reported as `InvalidCredentials`, while a slot whose structure is malformed (`MalformedKeySlot`, if it reaches the shell) maps to `Corrupted`. So "damaged key slot" in the `InvalidCredentials` doc comment covers only the first case.
- `core_errors_map_to_codes` does not cover `UnsupportedAlgorithm`, `InvalidItem`, `Io` (covered separately), `Decryption`, `MalformedKeySlot` or `MalformedCiphertext`.
