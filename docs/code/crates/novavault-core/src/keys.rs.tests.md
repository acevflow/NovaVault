# [`crates/novavault-core/src/keys.rs`](../../../../../crates/novavault-core/src/keys.rs#L134) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `keys.rs` checks the `SecretKey` and `Salt` types: random generation, construction from bytes and slices, and the `Debug` output (redacted for keys, full hex for salts). Because it lives inside the crate, it can read key bytes through the crate-private `SecretKey::expose`.

## Public API

| Name                          | Kind | Signature                            | Description                                                         |
| ----------------------------- | ---- | ------------------------------------ | ------------------------------------------------------------------- |
| `generated_keys_are_distinct` | test | `fn generated_keys_are_distinct()`   | Two generated keys differ, and neither is all zeros.                |
| `from_bytes_preserves_key`    | test | `fn from_bytes_preserves_key()`      | `from_bytes([7; 32])` exposes exactly those bytes.                  |
| `try_from_slice_checks_length`| test | `fn try_from_slice_checks_length()`  | A 32-byte slice is accepted; 31 and 33 bytes return `None`.         |
| `debug_redacts_key_material`  | test | `fn debug_redacts_key_material()`    | `Debug` prints `SecretKey([REDACTED])` and no key byte.             |
| `generated_salts_are_distinct`| test | `fn generated_salts_are_distinct()`  | Two generated salts differ.                                         |
| `salt_round_trips_bytes`      | test | `fn salt_round_trips_bytes()`        | `from_bytes`/`as_bytes` round-trip; `Debug` prints lowercase hex.   |

## Details

### `generated_keys_are_distinct`

Generates two keys with `SecretKey::generate` and asserts their bytes differ and the first is not `[0; KEY_LEN]`. The all-zeros check catches a generator that returns the initial buffer without filling it.

### `from_bytes_preserves_key`

`SecretKey::from_bytes([7u8; KEY_LEN])` exposes `[7u8; KEY_LEN]`: the copy into the `SecretBox` is exact.

### `try_from_slice_checks_length`

`try_from_slice(&[5u8; KEY_LEN])` returns a key whose bytes are `[5u8; KEY_LEN]`. Slices of `KEY_LEN - 1` (31) and `KEY_LEN + 1` (33) bytes return `None`.

### `debug_redacts_key_material`

Formats `SecretKey::from_bytes([0xAB; KEY_LEN])` with `{:?}` and asserts the output is exactly `SecretKey([REDACTED])`, and that its lowercase form does not contain `ab` (the hex of the key byte).

### `generated_salts_are_distinct`

Two `Salt::generate()` results compare unequal (`Salt` derives `PartialEq`).

### `salt_round_trips_bytes`

`Salt::from_bytes([1u8; SALT_LEN]).as_bytes()` returns the same array. `Debug` of `Salt::from_bytes([0x0f; SALT_LEN])` is `Salt(` followed by `0f` repeated 16 times and `)`, which pins the lowercase, zero-padded hex format.

## Dependencies

- **Internal**: [`keys`](keys.rs.md) (`super::*`): `SecretKey`, `Salt`, `KEY_LEN`, `SALT_LEN`.
- **External**: none.

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`crates/novavault-core/src/keys.rs`](../../../../../crates/novavault-core/src/keys.rs), documented in [`keys.rs.md`](keys.rs.md).

How to run:

```sh
cargo test -p novavault-core keys::tests
cargo test -p novavault-core keys::tests::debug_redacts_key_material
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- Zeroization on drop is not tested here; it is provided by `secrecy::SecretBox` and `zeroize::Zeroizing` and cannot be observed safely from a test.
- `SecretKey::from_secret_box` has no direct test; it is exercised through the KDF tests in [`kdf.rs.tests.md`](kdf.rs.tests.md).
- The distinctness tests are probabilistic: they catch a fixed or unfilled output, not a weak generator.
