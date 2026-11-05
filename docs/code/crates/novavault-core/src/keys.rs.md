# [`crates/novavault-core/src/keys.rs`](../../../../../crates/novavault-core/src/keys.rs)

## Purpose

Defines `SecretKey`, the zeroizing container for every 256-bit symmetric key in the crate (the vault key and the Argon2id-derived wrapping keys), and `Salt`, the non-secret random input to key derivation. Since `SecretKey` only exposes its raw bytes inside the crate, the shell and the UI can hold and pass a key but never read it.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `KEY_LEN` | constant | `pub const KEY_LEN: usize = 32;` | Key length in bytes (256 bits). |
| `SALT_LEN` | constant | `pub const SALT_LEN: usize = 16;` | Salt length in bytes (128 bits, RFC 9106 recommendation). |
| `SecretKey` | struct | `pub struct SecretKey(SecretBox<[u8; KEY_LEN]>);` | A 256-bit key, wiped on drop, not `Clone`, redacted in `Debug`. |
| `SecretKey::generate` | function | `pub fn generate() -> Result<Self>` | New random key from the OS CSPRNG. |
| `SecretKey::from_bytes` | function | `pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self` | Copies raw bytes into zeroizing storage. |
| `SecretKey::from_secret_box` | function (crate) | `pub(crate) fn from_secret_box(bytes: SecretBox<[u8; KEY_LEN]>) -> Self` | Wraps bytes already in a `SecretBox`. |
| `SecretKey::try_from_slice` | function (crate) | `pub(crate) fn try_from_slice(bytes: &[u8]) -> Option<Self>` | Copies a slice of exactly `KEY_LEN` bytes. |
| `SecretKey::expose` | function (crate) | `pub(crate) fn expose(&self) -> &[u8; KEY_LEN]` | Raw key bytes, inside the crate only. |
| `Salt` | struct | `pub struct Salt([u8; SALT_LEN]);` | A random Argon2id salt. `Clone, Copy, PartialEq, Eq`. |
| `Salt::generate` | function | `pub fn generate() -> Result<Self>` | New random salt from the OS CSPRNG. |
| `Salt::from_bytes` | function | `pub const fn from_bytes(bytes: [u8; SALT_LEN]) -> Self` | Builds a salt from stored bytes. |
| `Salt::as_bytes` | function | `pub const fn as_bytes(&self) -> &[u8; SALT_LEN]` | The salt bytes. |

`KEY_LEN`, `SALT_LEN`, `SecretKey` and `Salt` are re-exported from the crate root.

## Details

### `KEY_LEN`

32 bytes. This is the key size of XChaCha20-Poly1305 and the Argon2id output length used by [`kdf`](kdf.rs.md).

### `SALT_LEN`

16 bytes, the salt length RFC 9106 recommends. It is also the size of the `kdf_salt` field of a key slot (see [`slots`](slots.rs.md)).

### `SecretKey`

A newtype over `secrecy::SecretBox<[u8; 32]>`.

- **Security**: the boxed bytes are zeroized when the key is dropped (`SecretBox` behaviour). The type does not implement `Clone`, so every key has exactly one owner and one place where it is wiped. A hand-written `Debug` prints `SecretKey([REDACTED])`. No public method returns the bytes.
- **Edge cases and invariants**: always exactly `KEY_LEN` bytes. There is no `PartialEq`, so keys cannot be compared outside the crate. Inside the crate, tests compare them through `expose()`.

### `SecretKey::generate`

1. Allocates a zeroed `SecretBox<[u8; 32]>`.
2. Fills it in place with `getrandom::fill`.

- **Returns**: a new random key.
- **Errors**:
  - `CryptoError::Random`: the OS random number generator failed.
- **Security**: the key is written straight into the boxed buffer, so it never sits in an unzeroized stack temporary.

### `SecretKey::from_bytes`

Wraps the parameter in `Zeroizing` (so this function's own copy is wiped on return), then copies it into a new `SecretBox` with `SecretBox::init_with_mut`.

- **Parameters**: `bytes`, the 32 raw key bytes.
- **Returns**: the key.
- **Errors**: none.
- **Security**: `[u8; 32]` is `Copy`, so passing it by value copies it. The caller's original array is **not** wiped. Callers holding key bytes outside a zeroizing type must wipe them themselves. Within the crate, this function appears only in tests; the production paths use `generate`, `from_secret_box` and `try_from_slice`.

### `SecretKey::from_secret_box`

Crate-internal. Takes ownership of a `SecretBox` already filled by the caller. [`kdf::derive_key`](kdf.rs.md) uses it so that Argon2 writes its output directly into zeroizing storage.

### `SecretKey::try_from_slice`

Crate-internal. Returns `None` unless `bytes.len() == KEY_LEN`. Otherwise it copies the slice into a new `SecretBox` with `init_with_mut`.

- **Security**: there is no intermediate copy. The slice itself stays the caller's to wipe; [`slots`](slots.rs.md) passes the `SecretSlice` returned by `aead::decrypt`, which wipes itself.

### `SecretKey::expose`

Crate-internal accessor for the raw bytes, used by [`aead`](aead.rs.md) to build the cipher and by [`slots`](slots.rs.md) to seal the vault key. Keeping it `pub(crate)` is what stops key material from crossing the public API.

### `Salt`

A plain 16-byte array. A salt is not secret: it is stored in clear in each key slot.

- **Security**: `Debug` prints the full hexadecimal value (`Salt(0f0f…)`). The code comment justifies this: the salt is already stored in clear in the vault file.
- **Edge cases and invariants**: `Copy` and `PartialEq` are derived on purpose. The salt is public, and slots compare by value in tests.

### `Salt::generate`

Fills a stack array with `getrandom::fill`.

- **Errors**:
  - `CryptoError::Random`: the OS random number generator failed.
- **Security**: the stack array is not zeroized. That is acceptable because a salt is public.

### `Salt::from_bytes`

`const fn`. Builds a salt from bytes read from a key slot. It performs no validation: every 16-byte value is a valid salt.

### `Salt::as_bytes`

`const fn`. Returns the 16 bytes, for example to serialize them into a slot or to pass them to Argon2.

## Dependencies

- **Internal**: [`error`](error.rs.md): `CryptoError` and `Result`.
- **External**:
  - `secrecy`: `SecretBox`, plus the `ExposeSecret` and `ExposeSecretMut` traits, for zeroize-on-drop boxed storage with explicit access.
  - `zeroize`: `Zeroizing`, to wipe the by-value parameter of `from_bytes`.
  - `getrandom`: OS CSPRNG.

## Used by

- [`aead`](aead.rs.md): `SecretKey` as the cipher key (`expose`).
- [`kdf`](kdf.rs.md): `KEY_LEN`, `Salt`, `SecretKey::from_secret_box`.
- [`slots`](slots.rs.md): `KEY_LEN`, `SALT_LEN`, `Salt`, `SecretKey` (`generate`, `expose`, `try_from_slice`).
- [`format`](format.rs.md), [`attachment`](attachment.rs.md) and [`vault`](vault.rs.md): `SecretKey` as the vault key.
- [`lib.rs`](lib.rs.md): re-exports `KEY_LEN`, `SALT_LEN`, `Salt`, `SecretKey`.

## Tests

- [`generated_keys_are_distinct`](../../../../../crates/novavault-core/src/keys.rs#L139): two generated keys differ, and neither is all zeros.
- [`from_bytes_preserves_key`](../../../../../crates/novavault-core/src/keys.rs#L147): `from_bytes` stores the given bytes.
- [`try_from_slice_checks_length`](../../../../../crates/novavault-core/src/keys.rs#L153): exactly 32 bytes are accepted; 31 and 33 are rejected.
- [`debug_redacts_key_material`](../../../../../crates/novavault-core/src/keys.rs#L161): `Debug` prints `SecretKey([REDACTED])` and none of the key bytes.
- [`generated_salts_are_distinct`](../../../../../crates/novavault-core/src/keys.rs#L169): two generated salts differ.
- [`salt_round_trips_bytes`](../../../../../crates/novavault-core/src/keys.rs#L174): `from_bytes` / `as_bytes` round-trip, and `Debug` prints the hexadecimal value.

## Notes

- `SecretKey::from_bytes` is public, although nothing outside tests uses it, inside the crate or in `src-tauri`. Through it a caller outside the core can build a key from bytes it already knows, but no caller can read a key's bytes back.
- The module documentation says the bytes "are only reachable inside this crate". That holds for the public API. Memory hygiene still depends on the process: the bytes can reach swap or crash dumps (see [security-design.md](../../../../security-design.md#memory-handling)).
