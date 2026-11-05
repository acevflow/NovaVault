# [`crates/novavault-core/src/header.rs`](../../../../../crates/novavault-core/src/header.rs#L264) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `header.rs` checks the 32-byte plaintext vault header against the published encoding, round trips, version 0 handling for migration, the key slot associated data, and strict, ordered rejection of malformed headers (magic, newer versions, truncation, unknown algorithm, flags, slot count, reserved bytes).

## Public API

| Name                                         | Kind     | Signature                                             | Description                                                                 |
| -------------------------------------------- | -------- | ----------------------------------------------------- | --------------------------------------------------------------------------- |
| `sample`                                     | helper   | `fn sample() -> Header`                               | Version 1 header with `vault_id = 00 01 … 0f`.                              |
| `SAMPLE_HEX`                                 | constant | `const SAMPLE_HEX: &str`                              | Hex of `sample().to_bytes()`, as published in `docs/vault-format.md`.       |
| `hex`                                        | helper   | `fn hex(bytes: &[u8]) -> String`                      | Lowercase hex encoding.                                                     |
| `known_encoding`                             | test     | `fn known_encoding()`                                 | `sample()` encodes to `SAMPLE_HEX`.                                         |
| `round_trips_with_trailing_data`             | test     | `fn round_trips_with_trailing_data()`                 | `parse` reads only the first 32 bytes of a longer file.                     |
| `reads_version_zero_for_migration`           | test     | `fn reads_version_zero_for_migration()`               | A version 0 header parses and keeps version 0.                              |
| `slot_ad_ignores_only_the_version`           | test     | `fn slot_ad_ignores_only_the_version()`               | `slot_ad` zeroes the version and nothing else.                              |
| `generated_ids_are_distinct`                 | test     | `fn generated_ids_are_distinct()`                     | Two generated `VaultId`s differ.                                            |
| `rejects_wrong_magic`                        | test     | `fn rejects_wrong_magic()`                            | Empty, short, wrong and case-flipped magic return `NotAVault`.              |
| `rejects_newer_versions_before_anything_else`| test     | `fn rejects_newer_versions_before_anything_else()`    | Versions 2, 256 and 65 535 return `UnsupportedVersion` first.               |
| `rejects_truncation`                         | test     | `fn rejects_truncation()`                             | Every length from 8 to 31 bytes returns `Corrupted`.                        |
| `rejects_unknown_algorithm`                  | test     | `fn rejects_unknown_algorithm()`                      | `aead_id` 0, 2 and 255 return `UnsupportedAlgorithm`.                       |
| `rejects_flags_slot_count_and_reserved`      | test     | `fn rejects_flags_slot_count_and_reserved()`          | A flipped bit in bytes 11 to 15 returns `Corrupted`.                        |

## Details

### `sample`, `SAMPLE_HEX` and `known_encoding`

`sample()` is `Header::new(VaultId::from_bytes([0x00, 0x01, …, 0x0f]))`, at `FORMAT_VERSION` 1. Its expected encoding:

| Offset | Field            | Hex                                |
| ------ | ---------------- | ---------------------------------- |
| 0      | magic `NOVAVLT\0`| `4e4f5641564c5400`                 |
| 8      | format_version 1 | `0100`                             |
| 10     | aead_id          | `01`                               |
| 11     | flags            | `00`                               |
| 12     | slot_count       | `02`                               |
| 13     | reserved         | `000000`                           |
| 16     | vault_id         | `000102030405060708090a0b0c0d0e0f` |

`known_encoding` asserts `hex(&sample().to_bytes()) == SAMPLE_HEX`. The same vector is published in [`docs/vault-format.md` §13](../../../../vault-format.md#13-test-vectors).

### `round_trips_with_trailing_data`

Appends `rest of the file` to the 32 encoded bytes; `Header::parse` returns a header equal to `sample()`.

### `reads_version_zero_for_migration`

Encodes `sample().with_version(0)` and parses it: the result has `version() == 0` and equals the original. Version 0 is `MIN_READ_VERSION`, readable so the vault can migrate it.

### `slot_ad_ignores_only_the_version`

1. The version 1 and version 0 forms of `sample()` have the same `slot_ad()`.
2. For version 0, `slot_ad()` equals `to_bytes()` (the version field is already zero), so slots written by version 0 stay valid.
3. For version 1, `slot_ad()` differs from `to_bytes()`.
4. A header with `vault_id = [0xEE; 16]` has a different `slot_ad()`, so the vault identifier is still bound.

### `generated_ids_are_distinct`

Two `VaultId::generate()` results compare unequal (probabilistic).

### `rejects_wrong_magic`

Inputs `b""`, `b"NOVA"`, `b"NOVAVLT!"` and `b"PK\x03\x04 zip file"` each return `NotAVault`, as does `sample()`'s encoding with byte 0 XORed with `0x20` (`N` to `n`). Short inputs fail on the magic, not as truncation.

### `rejects_newer_versions_before_anything_else`

For versions `2`, `0x0100` and `u16::MAX`, writes the version little-endian into bytes 8–9 and also sets `aead_id` (byte 10) to `0xFF`. `parse` must return `UnsupportedVersion { found }` with the written version, not `UnsupportedAlgorithm`. The first 10 bytes alone (magic and version) also return `UnsupportedVersion`, not truncation. This pins the check order: magic, then version, then everything else.

### `rejects_truncation`

For every length from `MAGIC.len()` (8) to `HEADER_LEN - 1` (31), a prefix of `sample()`'s encoding returns `Corrupted`. Lengths 8 and 9 fail while reading the version; 10 to 31 fail on the full-header read.

### `rejects_unknown_algorithm`

Byte 10 set to `0`, `2` and `0xFF` returns `UnsupportedAlgorithm { id }` with that identifier.

### `rejects_flags_slot_count_and_reserved`

For each of bytes 11 (flags), 12 (slot count) and 13–15 (reserved), XORs the low bit and asserts `Corrupted`. For byte 12 that turns `2` into `3`.

## Dependencies

- **Internal**: [`header`](header.rs.md) (`super::*`): `Header`, `VaultId`, the constants including the private `VERSION_END`, and the crate-private `Header::with_version`; [`error`](error.rs.md): `VaultError`.
- **External**: none.

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`crates/novavault-core/src/header.rs`](../../../../../crates/novavault-core/src/header.rs), documented in [`header.rs.md`](header.rs.md).

How to run:

```sh
cargo test -p novavault-core header::tests
cargo test -p novavault-core header::tests::known_encoding
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- `SAMPLE_HEX` must stay identical to the header vector in `docs/vault-format.md` §13; the test does not read the spec, so the two are kept in sync by hand. They match at the time of writing.
- The `slot_header` statement in the spec ("the same bytes with `0100` replaced by `0000`") is checked here only indirectly, by `slot_ad_ignores_only_the_version`, not as a hex vector.
