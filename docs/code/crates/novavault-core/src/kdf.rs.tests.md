# [`crates/novavault-core/src/kdf.rs`](../../../../../crates/novavault-core/src/kdf.rs#L204) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `kdf.rs` checks the Argon2id wiring against two published known-answer vectors, then checks `derive_key`'s determinism and input sensitivity, the default parameters, and the parameter bounds. The vectors run through the private `argon2id` function, the same parameter wiring `derive_key` uses, so a mis-wired cost parameter would fail them.

## Public API

| Name                                              | Kind   | Signature                                                                   | Description                                                                    |
| ------------------------------------------------- | ------ | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `unchecked`                                       | helper | `const fn unchecked(memory_kib: u32, iterations: u32, parallelism: u32) -> KdfParams` | Builds `KdfParams` without bounds, for vectors below the production floor. |
| `hex`                                             | helper | `fn hex(bytes: &[u8]) -> String`                                            | Lowercase hex encoding for comparing outputs.                                  |
| `secret`                                          | helper | `fn secret(bytes: &[u8]) -> SecretSlice<u8>`                                | Wraps test bytes in a `SecretSlice` for `derive_key`.                          |
| `rfc9106_argon2id_test_vector`                    | test   | `fn rfc9106_argon2id_test_vector()`                                         | RFC 9106 §5.3 Argon2id vector, with secret and associated data.                |
| `reference_implementation_test_vector`            | test   | `fn reference_implementation_test_vector()`                                 | phc-winner-argon2 `password`/`somesalt` vector.                                |
| `derive_key_is_deterministic_and_input_sensitive` | test   | `fn derive_key_is_deterministic_and_input_sensitive()`                      | Same inputs give the same key; a different salt or password gives another.     |
| `default_params_are_valid_rfc9106_option`         | test   | `fn default_params_are_valid_rfc9106_option()`                              | `Default` validates and equals `(65_536, 3, 4)`.                               |
| `out_of_bounds_params_are_rejected`               | test   | `fn out_of_bounds_params_are_rejected()`                                    | One step outside each of the six bounds is rejected by `new`.                  |
| `derive_key_rechecks_params`                      | test   | `fn derive_key_rechecks_params()`                                           | `derive_key` rejects unchecked parameters itself.                              |

## Details

### Helpers

`unchecked` constructs `KdfParams` directly through its private fields, which only code inside `kdf.rs` can do. It is needed for the RFC 9106 vector, whose 32 KiB memory cost is far below `MIN_MEMORY_KIB`, and for `derive_key_rechecks_params`. The reference-implementation vector (64 MiB, t=2, p=1) is within bounds but also uses `unchecked`.

### `rfc9106_argon2id_test_vector`

RFC 9106 §5.3 (Argon2id, version 0x13):

| Input                   | Value                                                                  |
| ----------------------- | ---------------------------------------------------------------------- |
| Password (P)            | `[0x01; 32]`                                                           |
| Salt (S)                | `[0x02; 16]`                                                           |
| Secret (K)              | `[0x03; 8]`                                                            |
| Associated data (X)     | `[0x04; 12]`                                                           |
| Memory, passes, lanes   | 32 KiB, 3, 4                                                           |
| Output length           | 32 bytes                                                               |
| Expected tag            | `0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659`     |

Covers the `new_with_secret` and `AssociatedData` branches of `argon2id`, which production never takes (both inputs are empty there).

### `reference_implementation_test_vector`

From the phc-winner-argon2 reference `test.c` (Argon2id v0x13):

| Input                   | Value                                                                  |
| ----------------------- | ---------------------------------------------------------------------- |
| Password                | `password` (ASCII)                                                     |
| Salt                    | `somesalt` (ASCII)                                                     |
| Memory, passes, lanes   | 2^16 KiB (64 MiB), 2, 1                                                |
| Secret, associated data | empty                                                                  |
| Output length           | 32 bytes                                                               |
| Expected tag            | `09316115d5cf24ed5a15a31a3ba326e5cf32edc24702987c02b6566f61913cf7`     |

Covers the production path of `argon2id` (no secret, no associated data) with a production-sized memory cost.

### `derive_key_is_deterministic_and_input_sensitive`

Uses `KdfParams::new(KdfParams::MIN_MEMORY_KIB, 2, 1)` (19 MiB, the smallest valid parameters, to keep the test fast), salt `[9; 16]`, alternative salt `[8; 16]` and password `correct horse battery staple`.

1. Two derivations with the same inputs return equal key bytes.
2. Changing the salt changes the key.
3. Changing the password to `correct horse battery stapler` (one appended character) changes the key.

### `default_params_are_valid_rfc9106_option`

`KdfParams::default()` passes `validate()` and its accessors return `(65_536, 3, 4)`: 64 MiB, 3 passes, 4 lanes, RFC 9106's second recommended option.

### `out_of_bounds_params_are_rejected`

Each case is one step outside one bound, with the other two values valid:

| Case (m, t, p)                                | Violated bound            |
| --------------------------------------------- | ------------------------- |
| `(MIN_MEMORY_KIB - 1, 3, 4)` = (19 455, 3, 4) | memory below minimum      |
| `(MAX_MEMORY_KIB + 1, 3, 4)` = (4 194 305, 3, 4) | memory above maximum   |
| `(DEFAULT_MEMORY_KIB, 1, 4)`                  | time cost below minimum   |
| `(DEFAULT_MEMORY_KIB, 65, 4)`                 | time cost above maximum   |
| `(DEFAULT_MEMORY_KIB, 3, 0)`                  | parallelism below minimum |
| `(DEFAULT_MEMORY_KIB, 3, 17)`                 | parallelism above maximum |

Every case must return `Err(CryptoError::InvalidKdfParams { .. })`; the failure message prints the accepted triple. The `reason` string is not checked.

### `derive_key_rechecks_params`

Calls `derive_key` with `unchecked(32, 3, 4)` (below the memory floor), password `pw` and salt `[0; 16]`. It returns `InvalidKdfParams`, so values that bypassed `KdfParams::new` still never reach Argon2 through the public function.

## Dependencies

- **Internal**: [`kdf`](kdf.rs.md) (`super::*`): `argon2id`, `derive_key`, `KdfParams`; [`keys`](keys.rs.md): `Salt::from_bytes`, `SecretKey::expose`; [`error`](error.rs.md): `CryptoError`.
- **External**: `secrecy::SecretSlice` (via `super::*`): the password type `derive_key` takes.

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`crates/novavault-core/src/kdf.rs`](../../../../../crates/novavault-core/src/kdf.rs), documented in [`kdf.rs.md`](kdf.rs.md).

How to run:

```sh
cargo test -p novavault-core kdf::tests
cargo test -p novavault-core kdf::tests::rfc9106_argon2id_test_vector
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- `reference_implementation_test_vector` allocates 64 MiB and `derive_key_is_deterministic_and_input_sensitive` runs four 19 MiB derivations; they are the slowest tests in this module, noticeably so in debug builds.
- The `KeyDerivation` error path (for example an allocation failure) is not tested.
