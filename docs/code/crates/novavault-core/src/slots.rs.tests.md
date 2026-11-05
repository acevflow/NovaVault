# [`crates/novavault-core/src/slots.rs`](../../../../../crates/novavault-core/src/slots.rs#L421) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `slots.rs` checks the key slot design end to end: both slots open the same vault key, wrong secrets and wrong headers fail generically, slot types are enforced, wrapping is randomized, the 101-byte encoding round-trips and is strictly parsed, every tampered byte is caught, and the password change, recovery code regeneration and forgotten-password flows behave as the security architecture requires.

## Public API

| Name                                          | Kind     | Signature                                                       | Description                                                             |
| --------------------------------------------- | -------- | --------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `fast_params`                                 | helper   | `fn fast_params() -> KdfParams`                                 | Lowest accepted cost: 19 MiB, 2 passes, 1 lane.                         |
| `secret`                                      | helper   | `fn secret(text: &str) -> SecretSlice<u8>`                      | Wraps a test password's UTF-8 bytes.                                    |
| `HEADER`                                      | constant | `const HEADER: &[u8] = b"test vault header"`                    | Stand-in associated data for the vault header.                          |
| `new_vault`                                   | helper   | `fn new_vault(password: &str) -> (KeySlots, SecretKey, RecoveryCode)` | `KeySlots::create` with `fast_params` and `HEADER`.               |
| `layout_sizes`                                | test     | `fn layout_sizes()`                                             | Metadata 29, sealed key 72, slot 101 bytes.                             |
| `password_and_recovery_slots_open_the_same_key` | test   | `fn password_and_recovery_slots_open_the_same_key()`            | Both single slots unwrap to the original vault key.                     |
| `wrong_secrets_fail`                          | test     | `fn wrong_secrets_fail()`                                       | Wrong password and wrong code return `Decryption`.                      |
| `wrong_header_fails`                          | test     | `fn wrong_header_fails()`                                       | A different header returns `Decryption`.                                |
| `slot_type_is_enforced`                       | test     | `fn slot_type_is_enforced()`                                    | Unwrapping a recovery slot as a password returns `MalformedKeySlot`.    |
| `wrapping_is_randomized`                      | test     | `fn wrapping_is_randomized()`                                   | Wrapping the same key twice gives different bytes.                      |
| `encoding_round_trips`                        | test     | `fn encoding_round_trips()`                                     | Field offsets are as documented; parse then unwrap returns the key.     |
| `every_tampered_byte_is_detected`             | test     | `fn every_tampered_byte_is_detected()`                          | A flipped bit in any of the 101 bytes is never accepted.                |
| `strict_parsing`                              | test     | `fn strict_parsing()`                                           | Wrong lengths, unknown types and weak KDF parameters are rejected.      |
| `create_wraps_one_key_in_both_slots`          | test     | `fn create_wraps_one_key_in_both_slots()`                       | `create` returns typed slots that both unlock the returned key.         |
| `unlock_with_wrong_secrets_fails`             | test     | `fn unlock_with_wrong_secrets_fails()`                          | `PW` instead of `pw` and a foreign code return `Decryption`.            |
| `regenerated_recovery_code_replaces_the_old_one` | test  | `fn regenerated_recovery_code_replaces_the_old_one()`           | The old code stops working, the new one works, the password slot is unchanged. |
| `password_change_keeps_data_accessible`       | test     | `fn password_change_keeps_data_accessible()`                    | Re-wrap only: data still decrypts and the recovery slot is unchanged.   |
| `forgotten_password_flow`                     | test     | `fn forgotten_password_flow()`                                  | Unlock with the code, set a new password, unlock with it.               |
| `from_slots_checks_slot_types`                | test     | `fn from_slots_checks_slot_types()`                             | Correct order rebuilds the table; swapped order is rejected.            |

## Details

### Helpers

`fast_params` uses `KdfParams::new(MIN_MEMORY_KIB, MIN_ITERATIONS, 1)` so debug-build tests stay fast while still going through the bound checks. `HEADER` is an arbitrary byte string, not a real vault header: the slot module treats the header as opaque associated data. All passwords (`correct horse battery staple`, `pw`, `right`, `wrong`, `old password`, `new password`, `forgotten`, `fresh`) are fake.

### `layout_sizes`

Pins `SLOT_METADATA_LEN = 29` (1 + 16 + 3 × 4), `SEALED_KEY_LEN = 72` (32 + 24 + 16) and `SLOT_LEN = 101`, the sizes in the vault format specification.

### `password_and_recovery_slots_open_the_same_key`

Generates a vault key and a recovery code, wraps the key with `wrap_password` (`correct horse battery staple`) and with `wrap_recovery_code`, then unwraps each with its own secret: both return bytes equal to the original key.

### `wrong_secrets_fail`

A password slot wrapped under `right` and unwrapped with `wrong`, and a recovery slot unwrapped with another generated code, both return exactly `CryptoError::Decryption`.

### `wrong_header_fails`

A slot wrapped with `HEADER` and unwrapped with the right password but header `another vault` returns `Decryption`: the slot cannot be moved to another vault.

### `slot_type_is_enforced`

Unwraps a recovery slot with `unwrap_password`, passing the recovery code's display text as the password. It returns `MalformedKeySlot` (checked before key derivation), not `Decryption`.

### `wrapping_is_randomized`

Two `wrap_password` calls with the same password, key and header produce different 101-byte encodings (fresh salt and nonce each time).

### `encoding_round_trips`

For a password slot: byte 0 is `SlotType::Password.id()` (1), bytes 1–16 are the salt, bytes 17–20 are `MIN_MEMORY_KIB` little-endian. `from_bytes` returns an equal slot, which unwraps with the password to the original key.

### `every_tampered_byte_is_detected`

For each of the 101 bytes, XORs the low bit, then runs `from_bytes` followed by `unwrap_password`. The combined result must be an error: depending on the byte, strict parsing (type or KDF bounds), the slot type check or AEAD authentication catches it. Each iteration that parses runs one 19 MiB Argon2id derivation.

### `strict_parsing`

- Lengths 0, 100 and 102 (the encoding truncated or zero-extended) return `MalformedKeySlot`.
- Slot type bytes `0`, `3` and `0xFF` return `MalformedKeySlot`.
- `memory_kib` (bytes 17–20) set to 1024 returns `InvalidKdfParams`: a file cannot downgrade the KDF cost.

### `create_wraps_one_key_in_both_slots`

`new_vault("pw")` returns a table whose password slot has type `Password` and recovery slot type `Recovery`; unlocking with `pw` and with the returned code both yield the returned vault key.

### `unlock_with_wrong_secrets_fails`

`unlock_with_password` with `PW` (case differs) and `unlock_with_recovery_code` with a fresh code both return `Decryption`. Passwords are compared byte for byte, with no case folding.

### `regenerated_recovery_code_replaces_the_old_one`

1. Creates a vault, unlocks it with the password and saves a copy of the password slot.
2. Calls `regenerate_recovery_code`.
3. The old code returns `Decryption`; the new code unlocks to the same vault key.
4. The password slot is byte-for-byte unchanged.

### `password_change_keeps_data_accessible`

1. Creates a vault with `old password`, encrypts `my secret note` under the vault key with associated data `record-1`, and saves a copy of the recovery slot.
2. Changes the password to `new password`.
3. The old password returns `Decryption`; the new one unlocks to the same vault key, which still decrypts the record.
4. The recovery slot is unchanged and the original code still unlocks.

This is the "changing the master password only re-wraps the vault key" rule from the security architecture.

### `forgotten_password_flow`

Creates a vault with `forgotten`, unlocks it with the recovery code only, changes the password to `fresh` using that key, and unlocks with `fresh` to the same key.

### `from_slots_checks_slot_types`

`from_slots(password, recovery)` rebuilds a table equal to the original; `from_slots(recovery, password)` returns `MalformedKeySlot`.

## Dependencies

- **Internal**: [`slots`](slots.rs.md) (`super::*`): `KeySlot`, `KeySlots`, `SlotType`, the length constants and the private `salt` field; [`kdf`](kdf.rs.md): `KdfParams`; [`keys`](keys.rs.md): `SecretKey`; [`recovery`](recovery.rs.md): `RecoveryCode`; [`aead`](aead.rs.md): `encrypt`, `decrypt`; [`error`](error.rs.md): `CryptoError`.
- **External**: `secrecy` (`SecretSlice`, `ExposeSecret`, via `super::*`): test passwords and reading decrypted bytes.

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`crates/novavault-core/src/slots.rs`](../../../../../crates/novavault-core/src/slots.rs), documented in [`slots.rs.md`](slots.rs.md).

How to run:

```sh
cargo test -p novavault-core slots::tests
cargo test -p novavault-core slots::tests::password_change_keeps_data_accessible
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- Almost every test runs several Argon2id derivations; `every_tampered_byte_is_detected` runs up to 101. This is one of the slowest test modules in the crate, especially in debug builds.
- The tests use a fixed byte string as the header, so the interaction with `Header::slot_ad` (version zeroed) is tested in [`header.rs.tests.md`](header.rs.tests.md) and at the vault level, not here.
