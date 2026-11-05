# [`crates/novavault-core/src/format.rs`](../../../../../crates/novavault-core/src/format.rs)

## Purpose

Crate-internal (`mod format`, not `pub`). This module frames the bytes of a vault file and computes and verifies the whole-file integrity seal. It parses the structure (header, key slots, records, seal length) with bounds checks and no decryption, encodes the body for saving, and builds the domain-separated associated data for records and the seal. All of these are pure functions: unlocking, decryption and I/O are done by [`vault`](vault.rs.md).

## Public API

Crate-visible (`pub` or `pub(crate)` inside a private module) items:

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `SEAL_LEN` | constant | `pub const SEAL_LEN: usize = OVERHEAD;` | Seal length (40 bytes). |
| `MAX_RECORDS` | constant | `pub const MAX_RECORDS: u32 = 100_000;` | Maximum record count. |
| `MAX_SEALED_LEN` | constant | `pub const MAX_SEALED_LEN: usize = MAX_PAYLOAD_LEN + OVERHEAD;` | Largest sealed record (1 MiB + 40). |
| `MAX_FILE_LEN` | constant | `pub const MAX_FILE_LEN: usize = 256 * 1024 * 1024;` | Largest vault file (256 MiB). |
| `RecordEnvelope` | struct | `pub(crate) struct RecordEnvelope { pub(crate) id: ItemId, pub(crate) kind: ItemKind, pub(crate) sealed: Vec<u8> }` | One encrypted record as stored. `Debug, Clone, PartialEq, Eq`. |
| `VaultFile` | struct | `pub(crate) struct VaultFile { pub(crate) header: Header, pub(crate) slots: KeySlots, pub(crate) records: Vec<RecordEnvelope>, pub(crate) body_len: usize }` | A parsed file, structure only. `Debug`. |
| `record_ad` | function | `pub(crate) fn record_ad(header: &Header, id: &ItemId, kind: ItemKind) -> Vec<u8>` | Associated data of a record. |
| `encode_body` | function | `pub(crate) fn encode_body(header: &Header, slots: &KeySlots, records: &[RecordEnvelope]) -> VaultResult<Vec<u8>>` | Everything before the seal. |
| `seal` | function | `pub(crate) fn seal(key: &SecretKey, body: &[u8]) -> VaultResult<Vec<u8>>` | Computes the seal of a body. |
| `verify_seal` | function | `pub(crate) fn verify_seal(key: &SecretKey, file: &[u8], body_len: usize) -> VaultResult<()>` | Checks the seal at the end of a file. |
| `parse` | function | `pub(crate) fn parse(file: &[u8]) -> VaultResult<VaultFile>` | Structural parse, no key needed. |

None of these items is reachable from outside the crate.

## Details

### Layout

```text
header        HEADER_LEN (32) bytes
slot table    password slot ‖ recovery slot, SLOT_LEN (101) bytes each
records       record_count u32 ‖ record…
  record      record_id (16) ‖ record_type u8 ‖ sealed_len u32 ‖ sealed
seal          SEAL_LEN (40) bytes

record AD = header ‖ 0x01 ‖ record_id ‖ record_type
seal AD   = header ‖ 0x02 ‖ slot table ‖ record_count ‖ records
```

`sealed` is an [`aead`](aead.rs.md) message of the item's padded payload (see [`item`](item.rs.md)). Records are sorted by strictly ascending `record_id`, so the encoding is canonical and duplicates are impossible.

### `SEAL_LEN`, `MAX_RECORDS`, `MAX_SEALED_LEN`, `MAX_FILE_LEN`

Limits enforced identically by the writer and the reader. `MAX_FILE_LEN` is also the read limit in `vault::read_bounded`.

### `RecordEnvelope`

An item as it sits in memory and on disk: plaintext identifier and type, plus the sealed payload. [`vault`](vault.rs.md) keeps every item in this encrypted form and decrypts on demand.

### `VaultFile`

The output of `parse`. `body_len` is `file.len() - SEAL_LEN`, the length that the seal authenticates.

### `record_ad`

`header.to_bytes() ‖ 0x01 ‖ id ‖ kind.id()`, 50 bytes.

- **Security**: this binds the ciphertext to its vault (`vault_id` and the format version, through the header), its identifier and its type, so a record cannot be moved to another position or vault, or reinterpreted as another type. The context byte `0x01` separates it from the seal (`0x02`) and attachment chunks (`0x03`), all of which are encrypted under the same vault key.

### `encode_body`

1. Rejects more than `MAX_RECORDS` records.
2. Computes the total length and rejects a file that would exceed `MAX_FILE_LEN` once the seal is added.
3. Writes the header, both slots, the count and every record. Each record's `sealed_len` is checked against `MAX_SEALED_LEN`.

- **Parameters**: `records` must already be sorted by identifier with no duplicates. This function does not check that; [`vault`](vault.rs.md) keeps the order by binary-search insertion.
- **Returns**: the body, allocated with room for the seal.
- **Errors**:
  - `VaultError::InvalidItem { reason: "vault is too large" }`: too many records, the file would be too large, or a sealed record is too long.
- **Security**: the body holds only plaintext framing and ciphertext, so it needs no zeroization.

### `seal`

`aead::encrypt(key, &[], seal_ad(body))`: an AEAD encryption of the empty message, whose associated data is the header, the context byte `0x02`, and the rest of the body.

- **Returns**: the 40-byte seal.
- **Errors**: any error of `aead::encrypt`, converted to `VaultError::Crypto`.
- **Security**: per-record authentication cannot detect a record that was deleted, duplicated, reordered or replayed from an older file. The seal covers all of them, as well as the slot table and the record count. **Nonces**: the seal gets a fresh random nonce on every save, like every other message.

### `verify_seal`

Splits `file` at `body_len` and opens the seal against `seal_ad(body)`.

- **Errors**:
  - `VaultError::Corrupted { reason: "integrity check failed" }`: `body_len` is past the end of the file, or the seal fails with `Decryption` or `MalformedCiphertext`.
  - `VaultError::Crypto`: any other `CryptoError`, which `aead::decrypt` does not produce in practice.
- **Security**: [`vault`](vault.rs.md) calls this right after unwrapping the vault key and before it decrypts any record. Until the seal has been verified, the header is untrusted.

### `seal_ad` (private)

Inserts `0x02` after the first 32 bytes. With a body shorter than 32 bytes it uses the whole body as the "header" part, so it never panics.

### `parse`

1. `Header::parse`: magic, then version, then the other header fields.
2. Rejects files longer than `MAX_FILE_LEN`. This runs after the header check, so a large foreign file is still reported as `NotAVault`.
3. Reads two slots of `SLOT_LEN` bytes with `KeySlot::from_bytes` and checks their order with `KeySlots::from_slots`.
4. Reads `record_count` and rejects counts above `MAX_RECORDS`.
5. For each record, reads the identifier, the type (`ItemKind::from_id`) and `sealed_len`. It requires `OVERHEAD <= sealed_len <= MAX_SEALED_LEN` and an identifier strictly greater than the previous one, then copies `sealed`.
6. Requires exactly `SEAL_LEN` bytes to remain.

- **Returns**: a `VaultFile`.
- **Errors**:
  - Any error of `Header::parse` (`NotAVault`, `UnsupportedVersion`, `UnsupportedAlgorithm`, `Corrupted`).
  - `VaultError::Corrupted` with one of these reasons: `"file is too large"`, `"truncated header"`, `"key derivation parameters out of bounds"`, `"malformed key slot"`, `"key slots are out of order"`, `"too many records"`, `"unknown record type"` (from `ItemKind::from_id`), `"bad length"`, `"record length out of bounds"`, `"records not in ascending identifier order"`, `"unexpected end of file"` or `"wrong file length"`.
- **Security**:
  - Nothing here proves authenticity. That needs the vault key (`verify_seal`). The checks exist so that a foreign, newer or damaged file is rejected before the costly Argon2id derivation.
  - Every length is checked against the remaining input before allocating. The record vector is not pre-allocated from `record_count`, so memory stays proportional to the actual, bounded file size.
  - Out-of-bounds KDF parameters get their own reason because they are the likely sign of a downgrade or resource-exhaustion attempt.

### `parse_slot`, `corrupted`, `Reader` (private)

`parse_slot` maps `KeySlot::from_bytes` errors to `Corrupted`. `corrupted` is a `const fn` shorthand. `Reader` is a bounds-checked cursor (`take`, `array`, `u8`, `u32`) that returns `Corrupted { reason: "unexpected end of file" }` instead of panicking.

## Dependencies

- **Internal**:
  - [`aead`](aead.rs.md): `encrypt`, `decrypt`, `OVERHEAD`.
  - [`error`](error.rs.md): `CryptoError`, `VaultError`, `VaultResult`.
  - [`header`](header.rs.md): `Header`, `HEADER_LEN`.
  - [`item`](item.rs.md): `ItemId`, `ItemKind`, `ITEM_ID_LEN`, `MAX_PAYLOAD_LEN`.
  - [`keys`](keys.rs.md): `SecretKey`.
  - [`slots`](slots.rs.md): `KeySlot`, `KeySlots`, `SLOT_LEN`.
- **External**: none.

## Used by

- [`vault`](vault.rs.md): `parse`, `verify_seal`, `encode_body`, `seal`, `record_ad`, `RecordEnvelope`, `MAX_FILE_LEN`, `MAX_RECORDS`, `SEAL_LEN` (tests).

## Tests

- [`round_trips_and_verifies`](../../../../../crates/novavault-core/src/format.rs#L341): encode, seal, parse and verify a three-record file.
- [`empty_vault_round_trips`](../../../../../crates/novavault-core/src/format.rs#L353): an empty vault is `32 + 202 + 4 + 40` bytes and verifies.
- [`every_truncation_is_rejected`](../../../../../crates/novavault-core/src/format.rs#L363): every prefix fails to parse; one extra byte gives `Corrupted`.
- [`seal_detects_every_changed_byte`](../../../../../crates/novavault-core/src/format.rs#L375): a flipped bit in any byte of the file, header included, fails `verify_seal`.
- [`seal_detects_removed_and_replayed_records`](../../../../../crates/novavault-core/src/format.rs#L391): a file with a record dropped still parses but fails the original seal.
- [`rejects_unordered_and_duplicate_records`](../../../../../crates/novavault-core/src/format.rs#L409): reversed and duplicated records give `Corrupted`.
- [`rejects_bad_framing`](../../../../../crates/novavault-core/src/format.rs#L424): too many records, an unknown type, short and huge records, swapped slots and a weak KDF give `Corrupted`.
- [`oversized_file_is_rejected_after_the_header_check`](../../../../../crates/novavault-core/src/format.rs#L461): a file over 256 MiB gives `Corrupted`, or `NotAVault` if its magic is wrong.
- [`record_ad_is_domain_separated`](../../../../../crates/novavault-core/src/format.rs#L472): record AD is 50 bytes with context `0x01` and depends on the type; seal AD has context `0x02`.

## Notes

- **Doc comment narrower than the behaviour.** The doc comment of `verify_seal` says it fails when "some byte after the header was changed". The seal's associated data starts with the header itself, so a changed header byte also fails, as `seal_detects_every_changed_byte` shows. The comment understates what is checked.
- **Unreachable check.** In `parse`, `file.get(HEADER_LEN..)` cannot fail, because `Header::parse` has already required 32 bytes. Its `"truncated header"` error is never returned.
- **Spec order matches the code.** [vault-format.md §12](../../../../vault-format.md#12-validation-rules-for-readers) lists the size check as step 3, after the full header check. That is the order implemented here.
- **Rollback is not detected.** The seal does not detect a whole file being replaced by an older valid copy. That is a documented limitation ([threat-model.md](../../../../threat-model.md#cryptography-and-key-management)).
