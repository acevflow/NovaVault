# [`crates/novavault-core/src/format.rs`](../../../../../crates/novavault-core/src/format.rs#L296) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `format.rs` checks the byte framing of a vault file and its integrity seal: encode/parse round trips, every truncation, strict record ordering, framing bounds, the file size limit, the seal's sensitivity to every byte and to removed records, and the domain separation of the associated data. It builds real key slots and real sealed records, but never decrypts records: that is the vault's job.

## Public API

| Name                                         | Kind   | Signature                                                         | Description                                                                 |
| -------------------------------------------- | ------ | ----------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `Fixture`                                    | struct | `struct Fixture { header, slots, key, records }`                  | A header, key slots, the vault key and three sealed note records.           |
| `fixture`                                    | helper | `fn fixture() -> Fixture`                                         | Builds a `Fixture` with minimum-cost KDF parameters.                        |
| `sealed_file`                                | helper | `fn sealed_file(f: &Fixture, records: &[RecordEnvelope]) -> Vec<u8>` | Encodes the body for `records` and appends its seal.                     |
| `round_trips_and_verifies`                   | test   | `fn round_trips_and_verifies()`                                   | Parsing returns the same header, slots and records; the seal verifies.      |
| `empty_vault_round_trips`                    | test   | `fn empty_vault_round_trips()`                                    | A vault with no records has the minimum length and verifies.                |
| `every_truncation_is_rejected`               | test   | `fn every_truncation_is_rejected()`                               | Every prefix fails to parse; one extra byte returns `Corrupted`.            |
| `seal_detects_every_changed_byte`            | test   | `fn seal_detects_every_changed_byte()`                            | A flipped bit anywhere in the file fails `verify_seal`.                     |
| `seal_detects_removed_and_replayed_records`  | test   | `fn seal_detects_removed_and_replayed_records()`                  | A file with its last record removed parses but fails the seal.              |
| `rejects_unordered_and_duplicate_records`    | test   | `fn rejects_unordered_and_duplicate_records()`                    | Reversed and duplicated records return `Corrupted`.                         |
| `rejects_bad_framing`                        | test   | `fn rejects_bad_framing()`                                        | Six framing violations each return `Corrupted`.                             |
| `oversized_file_is_rejected_after_the_header_check` | test | `fn oversized_file_is_rejected_after_the_header_check()`   | A 256 MiB + 1 file returns `Corrupted`, or `NotAVault` with a bad magic.    |
| `record_ad_is_domain_separated`              | test   | `fn record_ad_is_domain_separated()`                              | Record and seal associated data carry context bytes `0x01` and `0x02`.      |

## Details

### `Fixture`, `fixture` and `sealed_file`

`fixture` builds:

1. a random `Header` (`Header::generate`);
2. key slots with `KeySlots::create` for the fake password `pw`, KDF parameters `(MIN_MEMORY_KIB, MIN_ITERATIONS, 1)` (19 MiB, 2 passes, 1 lane) and associated data `header.slot_ad()`; the recovery code returned is discarded;
3. three `ItemKind::Note` records with identifiers `[1; 16]`, `[2; 16]`, `[3; 16]`, each the AEAD encryption of `[n; 64]` under the vault key with `record_ad(&header, &id, kind)`, sorted by identifier.

`sealed_file` runs `encode_body` then `seal` and appends the 40-byte seal.

### `round_trips_and_verifies`

`parse` of the sealed file returns the original header, slots and records, and `body_len == file.len() - SEAL_LEN`. `verify_seal` with the vault key succeeds.

### `empty_vault_round_trips`

With no records the file length is `HEADER_LEN + 2 * SLOT_LEN + 4 + SEAL_LEN` (32 + 202 + 4 + 40 = 278 bytes). It parses to an empty record list and its seal verifies.

### `every_truncation_is_rejected`

Every prefix of the three-record file, from 0 bytes to one byte short, makes `parse` return an error (any variant: short prefixes give `NotAVault`, longer ones `Corrupted`). The file with one `0` byte appended returns `Corrupted` ("wrong file length").

### `seal_detects_every_changed_byte`

For every byte of the file (header, slots, record framing, sealed payloads and the seal itself), XORs the low bit and asserts `verify_seal(&key, &tampered, body_len)` is an error. No key derivation is involved, so checking every byte stays fast.

### `seal_detects_removed_and_replayed_records`

Encodes a body with only the first two records and appends the seal of the original three-record file. The result parses (the framing is valid) but `verify_seal` returns `Corrupted`. This is the attack per-record authentication alone cannot catch.

### `rejects_unordered_and_duplicate_records`

Two files are encoded and sealed correctly (the writer does not check order): one with the records reversed, one with the third record appended a second time. `parse` returns `Corrupted` for both ("records not in ascending identifier order").

### `rejects_bad_framing`

Starting from a valid file, with `count_at = HEADER_LEN + 2 * SLOT_LEN` (234) and `first_record = count_at + 4`:

| Case            | Change                                                                                     |
| --------------- | ------------------------------------------------------------------------------------------ |
| record count    | `record_count` set to `MAX_RECORDS + 1` (100 001)                                          |
| record type     | first record's type byte set to `9` (unknown)                                              |
| short record    | first record's `sealed_len` set to `OVERHEAD - 1` (39)                                     |
| huge record     | first record's `sealed_len` set to `u32::MAX`                                              |
| slot order      | the two slots swapped (recovery slot first)                                                |
| weak KDF        | password slot's `memory_kib` (slot bytes 17–20) set to 1024 KiB, below the 19 MiB floor    |

Each returns `Corrupted`; the assertion message names the case.

### `oversized_file_is_rejected_after_the_header_check`

Allocates `MAX_FILE_LEN + 1` zero bytes (lazily mapped, so cheap) and writes a valid header at the start: `parse` returns `Corrupted` ("file is too large"). Changing byte 0 to `X` makes it return `NotAVault`, which shows the header check runs before the size check.

### `record_ad_is_domain_separated`

`record_ad` for identifier `[7; 16]` and `ItemKind::Secret` has length `HEADER_LEN + 1 + ITEM_ID_LEN + 1` (50) and byte 32 equal to `RECORD_CONTEXT` (`0x01`); it differs from the same identifier with `ItemKind::Note`. `seal_ad` of the header bytes has byte 32 equal to `SEAL_CONTEXT` (`0x02`).

## Dependencies

- **Internal**: [`format`](format.rs.md) (`super::*`): `encode_body`, `seal`, `verify_seal`, `parse`, `record_ad`, the private `seal_ad`, context bytes and length constants; [`header`](header.rs.md): `Header`; [`slots`](slots.rs.md): `KeySlots::create`, `SLOT_LEN`; [`kdf`](kdf.rs.md): `KdfParams`; [`aead`](aead.rs.md): `encrypt`, `OVERHEAD`; [`item`](item.rs.md): `ItemId`, `ItemKind`, `ITEM_ID_LEN`.
- **External**: `secrecy::SecretSlice`: holds the fixture password.

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`crates/novavault-core/src/format.rs`](../../../../../crates/novavault-core/src/format.rs), documented in [`format.rs.md`](format.rs.md).

How to run:

```sh
cargo test -p novavault-core format::tests
cargo test -p novavault-core format::tests::seal_detects_every_changed_byte
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- `seal_detects_removed_and_replayed_records` only tests a removed record. Replaying an old record is covered at the vault level by `vault::tests::replaying_an_old_record_is_detected` (see [`vault.rs.tests.md`](vault.rs.tests.md)); the test name here promises more than it checks.
- Every test calls `fixture`, which runs Argon2id twice at 19 MiB (password and recovery slots), so this module is slower than its size suggests.
- `seal_detects_every_changed_byte` and `every_truncation_is_rejected` assert only that an error occurs, not which variant.
- The `MAX_FILE_LEN` check in `encode_body` (writer side) has no test here.
