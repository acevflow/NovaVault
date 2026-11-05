# [`crates/novavault-core/src/slots.rs`](../../../../../crates/novavault-core/src/slots.rs)

## Purpose

This module implements the key-slot scheme. A random 256-bit vault key encrypts all vault data, and it is stored only in wrapped form: once under a key derived from the master password, and once under a key derived from the recovery code. A `KeySlot` is one such wrapping, and `KeySlots` is the two-slot table with the create, unlock, change-password and regenerate-recovery-code operations. Changing a secret rewrites one slot and never re-encrypts the data.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `SLOT_METADATA_LEN` | constant | `pub const SLOT_METADATA_LEN: usize = 1 + SALT_LEN + 3 * 4;` | Type, salt and KDF parameters (29 bytes). |
| `SEALED_KEY_LEN` | constant | `pub const SEALED_KEY_LEN: usize = KEY_LEN + OVERHEAD;` | Nonce, wrapped key and tag (72 bytes). |
| `SLOT_LEN` | constant | `pub const SLOT_LEN: usize = SLOT_METADATA_LEN + SEALED_KEY_LEN;` | Serialized slot (101 bytes). |
| `SlotType` | enum | `pub enum SlotType { Password, Recovery }` | Which secret a slot is wrapped under. `Debug, Clone, Copy, PartialEq, Eq`. |
| `SlotType::id` | function | `pub const fn id(self) -> u8` | On-disk identifier (`1` or `2`). |
| `SlotType::from_id` | function | `pub const fn from_id(id: u8) -> Result<Self>` | Parses an identifier. |
| `KeySlot` | struct | `pub struct KeySlot { slot_type: SlotType, salt: Salt, params: KdfParams, sealed_key: [u8; SEALED_KEY_LEN] }` | One wrapped copy of the vault key. `Debug, Clone, PartialEq, Eq`. |
| `KeySlot::wrap_password` | function | `pub fn wrap_password(password: &SecretSlice<u8>, vault_key: &SecretKey, params: KdfParams, header: &[u8]) -> Result<Self>` | Wraps the vault key under a password. |
| `KeySlot::wrap_recovery_code` | function | `pub fn wrap_recovery_code(code: &RecoveryCode, vault_key: &SecretKey, params: KdfParams, header: &[u8]) -> Result<Self>` | Wraps the vault key under a recovery code. |
| `KeySlot::unwrap_password` | function | `pub fn unwrap_password(&self, password: &SecretSlice<u8>, header: &[u8]) -> Result<SecretKey>` | Recovers the vault key from a password slot. |
| `KeySlot::unwrap_recovery_code` | function | `pub fn unwrap_recovery_code(&self, code: &RecoveryCode, header: &[u8]) -> Result<SecretKey>` | Recovers the vault key from a recovery slot. |
| `KeySlot::slot_type` | function | `pub const fn slot_type(&self) -> SlotType` | The slot's type. |
| `KeySlot::kdf_params` | function | `pub const fn kdf_params(&self) -> KdfParams` | The slot's Argon2id parameters. |
| `KeySlot::to_bytes` | function | `pub fn to_bytes(&self) -> [u8; SLOT_LEN]` | 101-byte encoding. |
| `KeySlot::from_bytes` | function | `pub fn from_bytes(bytes: &[u8]) -> Result<Self>` | Strict parse. |
| `KeySlots` | struct | `pub struct KeySlots { password: KeySlot, recovery: KeySlot }` | The slot table. `Debug, Clone, PartialEq, Eq`. |
| `KeySlots::create` | function | `pub fn create(password: &SecretSlice<u8>, params: KdfParams, header: &[u8]) -> Result<(Self, SecretKey, RecoveryCode)>` | New vault key, new recovery code, both slots. |
| `KeySlots::from_slots` | function | `pub fn from_slots(password: KeySlot, recovery: KeySlot) -> Result<Self>` | Table from parsed slots. |
| `KeySlots::password_slot` | function | `pub const fn password_slot(&self) -> &KeySlot` | The password slot. |
| `KeySlots::recovery_slot` | function | `pub const fn recovery_slot(&self) -> &KeySlot` | The recovery slot. |
| `KeySlots::unlock_with_password` | function | `pub fn unlock_with_password(&self, password: &SecretSlice<u8>, header: &[u8]) -> Result<SecretKey>` | Unlock with the master password. |
| `KeySlots::unlock_with_recovery_code` | function | `pub fn unlock_with_recovery_code(&self, code: &RecoveryCode, header: &[u8]) -> Result<SecretKey>` | Unlock with the recovery code. |
| `KeySlots::change_password` | function | `pub fn change_password(&mut self, vault_key: &SecretKey, new_password: &SecretSlice<u8>, params: KdfParams, header: &[u8]) -> Result<()>` | Rewrites the password slot. |
| `KeySlots::regenerate_recovery_code` | function | `pub fn regenerate_recovery_code(&mut self, vault_key: &SecretKey, params: KdfParams, header: &[u8]) -> Result<RecoveryCode>` | New code, rewrites the recovery slot. |

`KeySlot`, `KeySlots`, `SLOT_LEN` and `SlotType` are re-exported from the crate root.

## Details

### Layout and wrapping

```text
slot_type (1) ‖ kdf_salt (16) ‖ memory_kib (4) ‖ iterations (4) ‖ parallelism (4)
  ‖ nonce (24) ‖ wrapped vault key (32) ‖ tag (16)

wrapping_key = Argon2id(secret, kdf_salt, params)            32 bytes
sealed_key   = aead::encrypt(wrapping_key, vault_key, header ‖ slot_metadata)
```

Integers are little-endian. `slot_metadata` is the first 29 bytes. The caller supplies `header`. [`vault`](vault.rs.md) always passes `Header::slot_ad()`, the 32-byte header with its version zeroed (see [`header`](header.rs.md)).

### `SLOT_METADATA_LEN`, `SEALED_KEY_LEN`, `SLOT_LEN`

29, 72 and 101 bytes. `SLOT_LEN` is the slot size [`format`](format.rs.md) reads twice from the file.

### `SlotType`, `SlotType::id`, `SlotType::from_id`

`Password` is `1` and `Recovery` is `2`.

- **Errors** (`from_id`):
  - `CryptoError::MalformedKeySlot { reason: "unknown slot type" }`: any other identifier.

### `KeySlot`

- **Security**: a slot holds no secret material, only a public salt, public parameters and ciphertext. It can therefore be copied, compared, printed with the derived `Debug` (which shows the salt and ciphertext bytes) and written to disk.
- **Edge cases and invariants**: `params` is always within the bounds of [`kdf`](kdf.rs.md), because it comes either from the caller's `KdfParams` or from `KdfParams::new` in `from_bytes`.

### `KeySlot::wrap_password` and `KeySlot::wrap_recovery_code`

Both call the private `wrap` with the right `SlotType` and secret. The password is used as given; the recovery code goes in through `RecoveryCode::kdf_input`, its canonical 32 bytes. `wrap`:

1. Generates a new `Salt`.
2. Derives the wrapping key with `derive_key(secret, &salt, &params)`.
3. Builds the slot metadata from the type, salt and parameters.
4. Seals `vault_key` under the wrapping key with `aead::encrypt`, using `header ‖ metadata` as associated data.
5. Stores the 72-byte result.

- **Parameters**:
  - `password` or `code`: the secret.
  - `vault_key`: the key to wrap, borrowed.
  - `params`: Argon2id cost for this slot.
  - `header`: the vault binding (`slot_ad`).
- **Returns**: the new slot.
- **Errors**:
  - Any error of `derive_key` (`InvalidKdfParams`, `KeyDerivation`) or of `aead::encrypt` (`Random`, `Encryption`).
  - `CryptoError::Random`: generating the salt failed.
  - `CryptoError::Encryption`: the sealed output does not have the expected length, which cannot happen with these fixed sizes.
- **Security**:
  - **Key material**: the wrapping key is a `SecretKey`, wiped when `wrap` returns. The vault key is only borrowed, and its bytes go straight from `SecretKey::expose` to the cipher.
  - **Nonces and salts**: every wrap draws a fresh salt and, inside `aead::encrypt`, a fresh nonce. Wrapping the same key twice never produces the same slot, and a rewritten slot shares nothing with the previous one, so an old secret cannot open the new slot.

### `KeySlot::unwrap_password` and `KeySlot::unwrap_recovery_code`

Both call the private `unwrap` with the expected `SlotType`. `unwrap`:

1. Checks that the slot's type is the expected one. This runs before the slow key derivation, because a password tried against the recovery slot is a caller bug, not a wrong secret.
2. Re-derives the wrapping key from the secret and the slot's salt and parameters.
3. Opens the sealed key with `aead::decrypt` against `header ‖ metadata`.
4. Copies the 32 plaintext bytes into a new `SecretKey` with `try_from_slice`.

- **Returns**: the vault key.
- **Errors**:
  - `CryptoError::MalformedKeySlot { reason: "unexpected slot type" }`: the slot is of the other type.
  - Any error of `derive_key`.
  - `CryptoError::Decryption`: wrong secret, a modified slot, or a different `header`. These cases are deliberately indistinguishable.
  - `CryptoError::MalformedKeySlot { reason: "wrapped key has the wrong length" }`: the decrypted key is not 32 bytes. This cannot happen, because `sealed_key` has a fixed size.
- **Security**: the decrypted bytes live in the `SecretSlice` returned by `aead::decrypt`, which is wiped on drop, and are copied directly into zeroizing storage. The wrapping key is wiped when `unwrap` returns.

### `KeySlot::slot_type` and `KeySlot::kdf_params`

Getters for public metadata.

### `KeySlot::to_bytes`

Concatenates `metadata()` and `sealed_key` into 101 bytes.

### `KeySlot::from_bytes`

Parsing is strict. The length must be exactly `SLOT_LEN`, the type must be known, and the KDF parameters must be within bounds. Authenticity is not established here; that happens only when the slot is unwrapped.

- **Errors**:
  - `CryptoError::MalformedKeySlot { reason: "wrong length" }`: `bytes.len() != SLOT_LEN`.
  - `CryptoError::MalformedKeySlot { reason: "unknown slot type" }`: unknown type byte.
  - `CryptoError::InvalidKdfParams`: parameters out of bounds. This check runs before any Argon2id work, so a crafted file can neither downgrade nor inflate the derivation.

### `KeySlots`

Exactly one password slot and one recovery slot, both wrapping the same vault key.

- **Edge cases and invariants**: every operation that rewrites a slot builds the new slot completely before assigning it, so an error leaves the table unchanged. Persisting the updated table, atomically, is the caller's job ([`vault`](vault.rs.md)).

### `KeySlots::create`

Generates a random vault key and a new recovery code, then wraps the key under the password and under the code, using the same `params` and `header` for both.

- **Returns**: `(table, vault_key, recovery_code)`. The vault key encrypts the vault's data; the code must be shown to the user once.
- **Errors**: any error of `SecretKey::generate`, `RecoveryCode::generate`, `wrap_password` or `wrap_recovery_code`.
- **Security**: the table does not store the recovery code. Once the returned `RecoveryCode` is dropped, it exists nowhere in memory.

### `KeySlots::from_slots`

Rebuilds a table from two parsed slots.

- **Errors**:
  - `CryptoError::MalformedKeySlot { reason: "unexpected slot type" }`: the first slot is not a password slot or the second is not a recovery slot. [`format`](format.rs.md) reports this as `Corrupted { reason: "key slots are out of order" }`.

### `KeySlots::password_slot` and `KeySlots::recovery_slot`

Borrowing getters, used by [`format`](format.rs.md) to serialize the table.

### `KeySlots::unlock_with_password` and `KeySlots::unlock_with_recovery_code`

Delegate to the matching slot's unwrap function.

- **Errors**: `CryptoError::Decryption` for a wrong secret or corrupted slot, plus the other errors of the unwrap functions.

### `KeySlots::change_password`

Replaces the password slot with `KeySlot::wrap_password(new_password, vault_key, params, header)`. The vault key, and therefore all encrypted data, stays the same. `vault_key` may come from an unlock with the recovery code; that is the forgotten-password flow.

- **Errors**: any error of `wrap_password`; the table is then unchanged.

### `KeySlots::regenerate_recovery_code`

Generates a new code and replaces the recovery slot, which gets a fresh salt.

- **Returns**: the new code.
- **Errors**: any error of `RecoveryCode::generate` or `wrap_recovery_code`; the table is then unchanged.
- **Security**: once the updated table is saved, the previous code no longer opens the file. An older copy of the file still opens with the old code (see [threat-model.md](../../../../threat-model.md#cryptography-and-key-management)).

### `associated_data` (private)

`header ‖ metadata`. The metadata has a fixed length, so the concatenation is unambiguous for any header length.

## Dependencies

- **Internal**:
  - [`aead`](aead.rs.md): `encrypt`, `decrypt`, `OVERHEAD`.
  - [`error`](error.rs.md): `CryptoError`, `Result`.
  - [`kdf`](kdf.rs.md): `KdfParams`, `derive_key`.
  - [`keys`](keys.rs.md): `KEY_LEN`, `SALT_LEN`, `Salt`, `SecretKey`.
  - [`recovery`](recovery.rs.md): `RecoveryCode` (`generate`, `kdf_input`).
- **External**: `secrecy`: `SecretSlice` for the password and `ExposeSecret` to read the decrypted key.

## Used by

- [`format`](format.rs.md): `KeySlot::from_bytes`, `to_bytes`, `KeySlots::from_slots`, `password_slot`, `recovery_slot`, `SLOT_LEN`.
- [`vault`](vault.rs.md): `KeySlots::create`, unlock, `change_password` and `regenerate_recovery_code`.
- [`lib.rs`](lib.rs.md): re-exports.

## Tests

- [`layout_sizes`](../../../../../crates/novavault-core/src/slots.rs#L437): 29, 72 and 101 bytes.
- [`password_and_recovery_slots_open_the_same_key`](../../../../../crates/novavault-core/src/slots.rs#L444): both slot types return the wrapped key.
- [`wrong_secrets_fail`](../../../../../crates/novavault-core/src/slots.rs#L461): a wrong password or another code gives `Decryption`.
- [`wrong_header_fails`](../../../../../crates/novavault-core/src/slots.rs#L485): another `header` gives `Decryption`.
- [`slot_type_is_enforced`](../../../../../crates/novavault-core/src/slots.rs#L497): unwrapping a recovery slot as a password slot gives `MalformedKeySlot`.
- [`wrapping_is_randomized`](../../../../../crates/novavault-core/src/slots.rs#L508): two wraps of the same key and password differ.
- [`encoding_round_trips`](../../../../../crates/novavault-core/src/slots.rs#L517): field offsets, round trip through bytes, and the parsed slot still unwraps.
- [`every_tampered_byte_is_detected`](../../../../../crates/novavault-core/src/slots.rs#L534): a flipped bit in any of the 101 bytes is caught by parsing, the type check or authentication.
- [`strict_parsing`](../../../../../crates/novavault-core/src/slots.rs#L552): wrong lengths, unknown types and a memory cost below the floor are rejected.
- [`create_wraps_one_key_in_both_slots`](../../../../../crates/novavault-core/src/slots.rs#L589): `create` returns the key both slots unwrap to.
- [`unlock_with_wrong_secrets_fails`](../../../../../crates/novavault-core/src/slots.rs#L600): a different-case password and another code give `Decryption`.
- [`regenerated_recovery_code_replaces_the_old_one`](../../../../../crates/novavault-core/src/slots.rs#L618): the old code fails, the new one works, and the password slot is unchanged.
- [`password_change_keeps_data_accessible`](../../../../../crates/novavault-core/src/slots.rs#L639): the old password fails, the new one works, data encrypted under the vault key still decrypts, and the recovery slot is unchanged.
- [`forgotten_password_flow`](../../../../../crates/novavault-core/src/slots.rs#L670): unlock with the code, set a new password, unlock with it.
- [`from_slots_checks_slot_types`](../../../../../crates/novavault-core/src/slots.rs#L683): the correct order is accepted; swapped slots are rejected.

## Notes

- The doc comment of `wrap_password` says the password is "its UTF-8 bytes, used as given". The parameter is a `SecretSlice<u8>`, and nothing checks that the bytes are UTF-8. The shell does pass UTF-8, which is what [vault-format.md §6.3](../../../../vault-format.md#63-wrapping-and-unwrapping) requires.
- `create` uses the same `params` for both slots. The format allows each slot its own parameters, and `change_password` and `regenerate_recovery_code` can set new parameters for a single slot.
- `header` is an arbitrary byte slice. Nothing in this module forces it to be `Header::slot_ad()`; that binding is a convention kept by [`vault`](vault.rs.md).
