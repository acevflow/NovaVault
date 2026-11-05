# [`src-tauri/src/error.rs`](../../../../src-tauri/src/error.rs#L172) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `error.rs` checks that `CommandError` crosses the trust boundary as a fixed `{ code, message }` pair: codes are unique, messages are constant strings, I/O details (such as file paths) are dropped, and core errors map to the intended codes.

## Public API

| Name                         | Kind  | Signature                                  | Description                                                           |
| ---------------------------- | ----- | ------------------------------------------ | --------------------------------------------------------------------- |
| `ALL`                        | const | `const ALL: [CommandError; 16]`            | Every `CommandError` variant, in declaration order.                   |
| `serializes_as_code_and_message` | test | `fn serializes_as_code_and_message()` | `Locked` serializes to its code and message; all 16 codes are unique. |
| `io_details_are_dropped`     | test  | `fn io_details_are_dropped()`              | An I/O error with a path becomes `Storage` and its text is not sent.  |
| `core_errors_map_to_codes`   | test  | `fn core_errors_map_to_codes()`            | Nine `VaultError` cases and three recovery-code typos map correctly.  |
| `messages_are_fixed_strings` | test  | `fn messages_are_fixed_strings()`          | Every variant serializes to exactly `code` and its `Display` text.    |

## Details

### `ALL`

Lists the 16 variants. `CommandError` is a fieldless enum, so the array covers it fully; the compiler does not check that, and a new variant must be added here by hand.

### [`serializes_as_code_and_message`](../../../../src-tauri/src/error.rs#L196)

`CommandError::Locked` serializes to exactly `{ "code": "locked", "message": "the vault is locked" }`. Then the `code()` of every entry of `ALL` is collected, sorted and deduplicated, and the count must still be 16, so the UI can branch on `code` unambiguously.

### [`io_details_are_dropped`](../../../../src-tauri/src/error.rs#L209)

Builds `VaultError::Io` from an `io::Error` whose message is the fake path `C:\Users\alice\secret-path\vault.nvault`. It maps to `CommandError::Storage`, and the serialized JSON contains neither `alice` nor `secret-path`.

### [`core_errors_map_to_codes`](../../../../src-tauri/src/error.rs#L221)

| Core error                                  | Command error         |
| ------------------------------------------- | --------------------- |
| `VaultError::InvalidCredentials`            | `InvalidCredentials`  |
| `VaultError::NotAVault`                     | `UnsupportedVault`    |
| `VaultError::UnsupportedVersion { found: 9 }` | `UnsupportedVault`  |
| `VaultError::Corrupted { reason: "x" }`     | `Corrupted`           |
| `VaultError::AlreadyExists`                 | `VaultExists`         |
| `VaultError::ItemNotFound`                  | `ItemNotFound`        |
| `VaultError::AttachmentNotFound`            | `AttachmentNotFound`  |
| `VaultError::Cancelled`                     | `Cancelled`           |
| `VaultError::Crypto(CryptoError::Random)`   | `Internal`            |
| `CryptoError::RecoveryCodeLength`           | `InvalidRecoveryCode` |
| `CryptoError::RecoveryCodeCharacter`        | `InvalidRecoveryCode` |
| `CryptoError::RecoveryCodeChecksum`         | `InvalidRecoveryCode` |

The last three are typing mistakes in a recovery code; they get their own code so the UI can say "check the code" rather than "wrong secret".

### [`messages_are_fixed_strings`](../../../../src-tauri/src/error.rs#L261)

For every entry of `ALL`, the serialized object has exactly two keys, `code` equals `code()` and `message` equals `to_string()`. Together with `CommandError` having no fields, this shows that no message can embed caller-supplied data.

## Dependencies

- **Internal**: [`error`](error.rs.md) (`super::*`): `CommandError` and its `From<VaultError>` and `From<CryptoError>` impls.
- **External**: `novavault-core` (`VaultError`, `CryptoError`); `serde_json`.

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`src-tauri/src/error.rs`](../../../../src-tauri/src/error.rs), documented in [`error.rs.md`](error.rs.md).

How to run:

```sh
cargo test -p novavault error::tests
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- Mappings not covered: `VaultError::InvalidItem`, `VaultError::UnsupportedAlgorithm`, `VaultError::Crypto` with `Decryption`, `MalformedKeySlot` or `MalformedCiphertext`, and the `_ => Internal` fallbacks for future core variants.
- `messages_are_fixed_strings` checks the shape of each message, not that it is free of sensitive text; the guarantee comes from the enum being fieldless.
