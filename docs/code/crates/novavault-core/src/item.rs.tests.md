# [`crates/novavault-core/src/item.rs`](../../../../../crates/novavault-core/src/item.rs#L741) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `item.rs` checks the plaintext record payload codec against the two vectors published in the vault format specification (note payload, format versions 0 and 1), round trips for every item kind and both versions, attachment section encoding and limits, attachment name rules, padding, size limits, strict decoding, item kind and identifier parsing, and redacted `Debug` output. No encryption is involved: payloads are encrypted by the vault.

## Public API

| Name                                  | Kind   | Signature                                                                 | Description                                                                 |
| ------------------------------------- | ------ | ------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `s`                                   | helper | `fn s(text: &str) -> SecretString`                                        | Wraps a test string as a `SecretString`.                                    |
| `password_item`                       | helper | `fn password_item() -> ItemContent`                                       | Fake login `Bank` / `alice@example.com`.                                    |
| `note_item`                           | helper | `fn note_item() -> ItemContent`                                           | Note with title `t` and body `b`, the vector item.                          |
| `secret_item`                         | helper | `fn secret_item() -> ItemContent`                                         | Secret titled `API key ✓` with empty notes.                                 |
| `attachment`                          | helper | `fn attachment(byte: u8, name: &str) -> Attachment`                       | Attachment with id `[byte; 16]`, size `byte × 1000`, time `byte`.          |
| `encode`                              | helper | `fn encode(content: &ItemContent, attachments: &[Attachment]) -> Zeroizing<Vec<u8>>` | `encode_payload` at `FORMAT_VERSION` with times 1 and 2.         |
| `decode`                              | helper | `fn decode(kind: ItemKind, payload: &[u8]) -> VaultResult<Payload>`       | `decode_payload` at `FORMAT_VERSION`.                                       |
| `hex`                                 | helper | `fn hex(bytes: &[u8]) -> String`                                          | Lowercase hex encoding.                                                     |
| `padded`                              | helper | `fn padded(hex: &str) -> String`                                          | Appends `00` bytes up to the next multiple of `PAD_BLOCK`.                  |
| `assert_same`                         | helper | `fn assert_same(a: &ItemContent, b: &ItemContent)`                        | Same kind and same string fields in payload order.                          |
| `assert_malformed`                    | helper | `fn assert_malformed(kind: ItemKind, payload: &[u8])`                     | Asserts `decode` returns `Corrupted`.                                       |
| `known_note_encoding_v0`              | test   | `fn known_note_encoding_v0()`                                             | Version 0 note payload vector (64 bytes).                                   |
| `known_note_encoding_v1`              | test   | `fn known_note_encoding_v1()`                                             | Version 1 note payload vector with one attachment (128 bytes).              |
| `round_trips_every_kind`              | test   | `fn round_trips_every_kind()`                                             | All three kinds round-trip in versions 0 and 1.                             |
| `attachments_round_trip_in_order`     | test   | `fn attachments_round_trip_in_order()`                                    | Attachment metadata round-trips in the given order.                         |
| `version_zero_cannot_hold_attachments`| test   | `fn version_zero_cannot_hold_attachments()`                               | Encoding attachments for version 0 returns `InvalidItem`.                   |
| `attachment_limits_are_enforced`      | test   | `fn attachment_limits_are_enforced()`                                     | 1001 attachments and an invalid name are rejected; 1000 are accepted.       |
| `attachment_names_are_validated`      | test   | `fn attachment_names_are_validated()`                                     | Accepted and rejected names for `validate_attachment_name`.                 |
| `rejects_malformed_attachment_sections`| test  | `fn rejects_malformed_attachment_sections()`                              | Bad count, duplicate id, bad name and empty name return `Corrupted`.        |
| `padding_hides_exact_length`          | test   | `fn padding_hides_exact_length()`                                         | Two notes of different length encode to the same size.                      |
| `exact_block_needs_no_padding`        | test   | `fn exact_block_needs_no_padding()`                                       | A 64-byte payload gets no extra block and decodes.                          |
| `rejects_oversized_items`             | test   | `fn rejects_oversized_items()`                                            | A 1 MiB body exceeds `MAX_PAYLOAD_LEN` and returns `InvalidItem`.           |
| `rejects_malformed_payloads`          | test   | `fn rejects_malformed_payloads()`                                         | Truncation, extra or dirty padding, huge length and bad UTF-8 are rejected. |
| `kind_ids_are_strict`                 | test   | `fn kind_ids_are_strict()`                                                | Kind ids 1–3 round-trip; 0, 4 and 255 return `Corrupted`.                   |
| `ids_are_random_and_hex_formatted`    | test   | `fn ids_are_random_and_hex_formatted()`                                   | Generated ids differ; `Display` and `Debug` formats.                        |
| `ids_parse_from_strict_hex`           | test   | `fn ids_parse_from_strict_hex()`                                          | `from_hex` accepts 32 hex digits in either case and nothing else.           |
| `debug_shows_no_field_content`        | test   | `fn debug_shows_no_field_content()`                                       | `Debug` of items, contents, attachments and summaries hides all fields.    |

## Details

### Fixtures

All values are fake: `password_item` is title `Bank`, user name `alice@example.com`, password `correct horse battery staple`, URL `https://bank.example`, notes `PIN is elsewhere`. `secret_item` has a non-ASCII title (`API key ✓`), value `sk-test-123` and empty notes, which covers multi-byte UTF-8 and empty fields.

### `known_note_encoding_v0`

`encode_payload(0, 1, 2, &note_item(), &[])` must equal (hex, padded to 64 bytes with 38 zero bytes):

| Field            | Hex                |
| ---------------- | ------------------ |
| `created_at_ms`  | `0100000000000000` |
| `updated_at_ms`  | `0200000000000000` |
| title `t`        | `01000000` `74`    |
| body `b`         | `01000000` `62`    |
| padding          | `00` × 38          |

### `known_note_encoding_v1`

The same note at `FORMAT_VERSION` 1 with one attachment (`attachment_id = aa × 16`, `size = 3`, `added_at_ms = 4`, name `a.txt`). The payload is `2 × PAD_BLOCK` (128) bytes:

| Field              | Hex                                 |
| ------------------ | ----------------------------------- |
| `created_at_ms`    | `0100000000000000`                  |
| `updated_at_ms`    | `0200000000000000`                  |
| title `t`          | `01000000` `74`                     |
| body `b`           | `01000000` `62`                     |
| `attachment_count` | `01000000`                          |
| `attachment_id`    | `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`  |
| `size`             | `0300000000000000`                  |
| `added_at_ms`      | `0400000000000000`                  |
| name `a.txt`       | `05000000` `612e747874`             |
| padding            | `00` × 57                           |

Both vectors match [`docs/vault-format.md` §13](../../../../vault-format.md#13-test-vectors).

### `round_trips_every_kind`

For versions 0 and `FORMAT_VERSION`, and each of the three fixtures, encodes with `created_at_ms = 1_700_000_000_000` and `updated_at_ms = u64::MAX` (the largest timestamp), checks the length is a multiple of 64, decodes with the same kind and version, and checks the timestamps, the fields (via `assert_same`) and that there are no attachments.

### `attachments_round_trip_in_order`

Encodes the password fixture with `attachment(3, "scan.pdf")` then `attachment(1, "photo ✓.jpg")` and decodes it. The `(id, name, size, added_at_ms)` tuples come back in the same order, which shows the codec does not sort attachments.

### `version_zero_cannot_hold_attachments`

`encode_payload(0, …)` with one attachment returns `InvalidItem`.

### `attachment_limits_are_enforced`

Builds `MAX_ATTACHMENTS_PER_ITEM + 1` (1001) attachments with distinct ids (the index in the first 8 bytes) and name `f`. All 1001 return `InvalidItem`; the first 1000 encode. An attachment named `a/b` returns `InvalidItem` from the encoder.

### `attachment_names_are_validated`

| Accepted                                                | Rejected                                                                                   |
| ------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `a`, `report 2026.pdf`, `é✓.txt`, `.hidden`, `x` × 255 | empty, `.`, `..`, `a/b`, `a\b`, `a` NUL `b`, `line` newline `break`, `x` × 256 |

So the limit is 255 bytes, dot files are allowed, and path separators, control characters and the two special directory names are rejected.

### `rejects_malformed_attachment_sections`

Encodes the note fixture with attachments `a` (id `01 × 16`) and `b` (id `02 × 16`). The count is at byte 26 (16 timestamp bytes, 5 for the title, 5 for the body), the first entry at 30, the second at `30 + 32 + 4 + 1 = 67`. Then:

| Case         | Change                                               |
| ------------ | ---------------------------------------------------- |
| count        | count set to 1001                                    |
| duplicate id | second id overwritten with `01 × 16`                 |
| bad name     | first name byte set to `/`                           |
| empty name   | first name length set to 0                           |

Each must decode to `Corrupted`. In the empty-name case the name fails `validate_attachment_name` right away, before the unread `a` byte could misalign the rest of the section.

### `padding_hides_exact_length`

Notes with empty titles and bodies `a` and `a much longer note body` encode to the same length (one 64-byte block).

### `exact_block_needs_no_padding`

An empty title and a 36-byte body give 16 + 2 × 4 + 36 + 4 (attachment count) = 64 bytes: the payload is exactly `PAD_BLOCK` long, with no extra zero block, and decodes.

### `rejects_oversized_items`

A note body of `MAX_PAYLOAD_LEN` (1 MiB) bytes, plus the framing, exceeds the limit: `InvalidItem`.

### `rejects_malformed_payloads`

From the note fixture with one attachment:

- every prefix shorter than the payload returns `Corrupted`;
- one extra block of 64 zero bytes returns `Corrupted` (padding must be shorter than a block);
- the last padding byte set to `1` returns `Corrupted`;
- the title length (bytes 16–19) set to `u32::MAX` returns `Corrupted`;
- the title's first byte (byte 20) set to `0xFF`, invalid UTF-8, returns `Corrupted`.

A source comment records that decoding as a different kind is deliberately not tested: zero padding reads as empty strings, so it may succeed, and the record type is authenticated as associated data.

### `kind_ids_are_strict`

`Password`, `Note` and `Secret` round-trip through `id`/`from_id`. Ids `0`, `4` and `0xFF` return `Corrupted`.

### `ids_are_random_and_hex_formatted`

Two generated `ItemId`s differ, as do two `AttachmentId`s. `ItemId::from_bytes([0xAB; 16])` displays as `ab` × 16 and debugs as `ItemId(<hex>)`; an `AttachmentId` debugs as `AttachmentId(<hex>)`.

### `ids_parse_from_strict_hex`

`from_hex` round-trips a generated id in lower and upper case, for both id types. It returns `None` for: empty, `abc`, `g` × 32, `a` × 33, `+f` × 16, ` a` × 16 and `é` × 16 (32 bytes of multi-byte UTF-8, the right byte length but not hex).

### `debug_shows_no_field_content`

Builds an `Item` from the password fixture with attachment `tax-return.pdf`, then formats the item, its content and its attachments with `{:?}`. None of `Bank`, `alice`, `correct horse`, `bank.example`, `PIN` or `tax-return` appears; the output contains `attachments: 1`. Converting to `ItemSummary` keeps the title `Bank` in the field, but the summary's `Debug` does not print it.

## Dependencies

- **Internal**: [`item`](item.rs.md) (`super::*`): the item types, `encode_payload`, `decode_payload`, `Payload`, `validate_attachment_name`, the private `fields` function and constants such as `ATTACHMENT_FIXED_LEN`, and the crate-private constructors `Attachment::new` and `Item::new`; [`header`](header.rs.md): `FORMAT_VERSION`; [`error`](error.rs.md): `VaultError`.
- **External**: `secrecy::SecretString` and `zeroize::Zeroizing` (via `super::*`): field and payload types.

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`crates/novavault-core/src/item.rs`](../../../../../crates/novavault-core/src/item.rs), documented in [`item.rs.md`](item.rs.md).

How to run:

```sh
cargo test -p novavault-core item::tests
cargo test -p novavault-core item::tests::known_note_encoding_v1
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- The two `known_note_encoding_*` vectors must stay identical to `docs/vault-format.md` §13 by hand; the test does not read the spec. They match at the time of writing.
- `known_note_encoding_v1` hard-codes version 1 in its doc comment but encodes at `FORMAT_VERSION`; if the format version is bumped, the test will follow the new version while still claiming to check version 1.
- `rejects_oversized_items` checks only a payload far above the limit, not the exact boundary.
