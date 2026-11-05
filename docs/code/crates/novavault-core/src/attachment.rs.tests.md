# [`crates/novavault-core/src/attachment.rs`](../../../../../crates/novavault-core/src/attachment.rs#L311) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `attachment.rs` checks the chunked, STREAM-shaped encryption of attachment blobs: the published blob header vector, round trips at every chunk boundary, streaming a 32 MiB file through bounded buffers, and detection of truncation, tampering, chunk reordering, blob swapping, size mismatches, wrong keys, foreign or newer blobs, plus cancellation through the progress callback. Everything runs in memory; no files are written.

## Public API

| Name                                             | Kind   | Signature                                                       | Description                                                                 |
| ------------------------------------------------ | ------ | --------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `key`                                            | helper | `fn key() -> SecretKey`                                         | Fixed test key `[0x42; 32]`.                                                |
| `header`                                         | helper | `fn header() -> BlobHeader`                                     | Blob header with `vault_id = 01 × 16`, `attachment_id = 02 × 16`.           |
| `keep_going`                                     | helper | `fn keep_going(_: u64) -> ControlFlow<()>`                      | Progress callback that never cancels.                                       |
| `byte_at`                                        | helper | `fn byte_at(position: u64) -> u8`                               | Deterministic pseudo-random byte for a position.                            |
| `content`                                        | helper | `fn content(len: usize) -> Vec<u8>`                             | `len` bytes of `byte_at` content.                                           |
| `encrypt`                                        | helper | `fn encrypt(plaintext: &[u8]) -> Vec<u8>`                       | Encrypts into a blob and checks the returned length.                        |
| `decrypt`                                        | helper | `fn decrypt(blob: &[u8], expected_len: u64) -> VaultResult<Vec<u8>>` | Decrypts a blob, expecting `expected_len` bytes.                       |
| `assert_corrupted`                               | helper | `fn assert_corrupted(result: VaultResult<Vec<u8>>, what: &str)` | Asserts `Corrupted`, labelled with `what`.                                  |
| `chunk_at`                                       | helper | `fn chunk_at(index: usize) -> usize`                            | Offset of sealed chunk `index` in a blob.                                   |
| `Generated`                                      | struct | `struct Generated { position: u64, len: u64 }`                  | `Read` source of `byte_at` content in reads of at most 10 007 bytes.        |
| `Verifier`                                       | struct | `struct Verifier { position: u64 }`                             | `Write` sink that checks every byte against `byte_at`.                      |
| `known_header_encoding`                          | test   | `fn known_header_encoding()`                                    | Blob header vector from the vault format specification.                     |
| `round_trips_at_chunk_boundaries`                | test   | `fn round_trips_at_chunk_boundaries()`                          | Seven lengths around chunk boundaries round-trip with the predicted size.   |
| `large_file_streams_through_bounded_buffers`     | test   | `fn large_file_streams_through_bounded_buffers()`               | A 32 MiB + 12 345-byte file streams through both directions.                |
| `every_truncation_is_detected`                   | test   | `fn every_truncation_is_detected()`                             | Cuts in the header, at and inside every chunk return `Corrupted`.           |
| `truncation_at_a_chunk_boundary_fails_even_with_a_matching_size` | test | `fn truncation_at_a_chunk_boundary_fails_even_with_a_matching_size()` | The final flag alone catches a cut after a full chunk.  |
| `every_region_of_every_chunk_is_authenticated`   | test   | `fn every_region_of_every_chunk_is_authenticated()`             | Flipped bits in header fields, nonces, ciphertexts and tags are caught.     |
| `reordered_dropped_duplicated_and_appended_chunks_are_detected` | test | `fn reordered_dropped_duplicated_and_appended_chunks_are_detected()` | Chunk order, count and trailing data are authenticated.     |
| `blobs_cannot_be_moved_between_attachments_or_vaults` | test | `fn blobs_cannot_be_moved_between_attachments_or_vaults()`     | Another attachment or vault id fails, even with a rewritten header.         |
| `size_must_match_the_record`                     | test   | `fn size_must_match_the_record()`                               | Any expected size other than the real one returns `Corrupted`.              |
| `wrong_key_is_detected`                          | test   | `fn wrong_key_is_detected()`                                    | Another key returns `Corrupted`.                                            |
| `foreign_and_newer_blobs_are_reported`           | test   | `fn foreign_and_newer_blobs_are_reported()`                     | Bad magic, newer version, unknown algorithm and reserved bytes.             |
| `progress_can_cancel_both_directions`            | test   | `fn progress_can_cancel_both_directions()`                      | `ControlFlow::Break` returns `Cancelled` when encrypting and decrypting.    |

## Details

### Helpers

`byte_at(p)` is `(p × 2 654 435 761 >> 13) as u8` (wrapping multiplication by Knuth's multiplicative-hash constant), so content is deterministic but not periodic at chunk sizes, and a misplaced chunk changes the bytes. `chunk_at(i)` is `BLOB_HEADER_LEN + i × SEALED_CHUNK_LEN` (48 + i × 65 576). The key `[0x42; 32]` and the alternative `[0x43; 32]` are fake.

### `known_header_encoding`

`header().to_bytes()` must be, in hex:

| Field         | Hex                                |
| ------------- | ---------------------------------- |
| magic         | `4e56415454434800` (`NVATTCH\0`)   |
| version 1     | `0100`                             |
| aead_id       | `01`                               |
| reserved      | `0000000000`                       |
| vault_id      | `01010101010101010101010101010101` |
| attachment_id | `02020202020202020202020202020202` |

The same vector is published in [`docs/vault-format.md` §13](../../../../vault-format.md#13-test-vectors).

### `round_trips_at_chunk_boundaries`

For lengths 0, 1, `CHUNK_LEN - 1`, `CHUNK_LEN`, `CHUNK_LEN + 1`, `2 × CHUNK_LEN` and `3 × CHUNK_LEN + 7` (with `CHUNK_LEN` = 65 536): the blob length equals `blob_len(len)`, and decryption returns the original content. This covers the empty file (one empty chunk) and exact multiples (no extra empty chunk).

### `Generated`, `Verifier` and `large_file_streams_through_bounded_buffers`

`Generated` yields `byte_at` content in reads capped at 10 007 bytes (an odd size, so `read_full` must loop to fill a chunk). `Verifier` asserts every written byte equals `byte_at` at its position and stores nothing.

1. Encrypts `32 × 1024 × 1024 + 12 345` bytes from `Generated` into a `Vec`. The progress callback runs once per chunk (`len.div_ceil(CHUNK_LEN)` = 513 times) and never reports more than `len`. The returned length is `len` and the blob length is `blob_len(len)`.
2. Decrypts into a `Verifier`. Progress values strictly increase and end at `len`; the verifier saw exactly `len` bytes.

The plaintext is never held whole, only the ciphertext `Vec` is.

### `every_truncation_is_detected`

For a `2 × CHUNK_LEN + 100`-byte file (three chunks), decrypts prefixes cut at: 0, 1, 47 and 48 (inside and at the end of the header); for each chunk its start, start + 1, start + 39, start + 40 and start + 1000; the last byte of chunks 0 and 1; and one byte before the end. Cuts at or past the blob length are skipped. Each returns `Corrupted`.

### `truncation_at_a_chunk_boundary_fails_even_with_a_matching_size`

Encrypts `2 × CHUNK_LEN` bytes, cuts after the first full chunk and passes `CHUNK_LEN` as the expected size, as if the recorded size had also been tampered with. Decryption still returns `Corrupted`: the first chunk was sealed with `is_final = 0`, and the reader now authenticates it as final.

### `every_region_of_every_chunk_is_authenticated`

For a three-chunk blob, XORs the low bit of: header bytes 0 (magic), 8 (version), 10 (algorithm), 16 (vault id), 32 (attachment id) and 47; for each chunk, byte 0 and 23 of the nonce, byte 24 (first ciphertext byte) and byte 100; the last byte (tag) of chunks 0 and 1; and the blob's last byte. Every case must return an error (not only `Corrupted`: changing the version gives `UnsupportedVersion`).

### `reordered_dropped_duplicated_and_appended_chunks_are_detected`

For a `3 × CHUNK_LEN + 5`-byte file (four chunks, the last holding 5 bytes), reassembles the original header with chunks in these orders:

| Case        | Order          |
| ----------- | -------------- |
| swapped     | 1, 0, 2, 3     |
| dropped     | 0, 2, 3        |
| duplicated  | 0, 1, 1, 2, 3  |
| final moved | 0, 1, 3        |

Each is decrypted twice, with the real size and with `order.len() × CHUNK_LEN`, and must return `Corrupted` both times. Then appending a copy of chunk 0, and appending one `0` byte, to the intact blob also return `Corrupted`.

### `blobs_cannot_be_moved_between_attachments_or_vaults`

Encrypts `contract` (8 bytes) and tries to decrypt it as attachment `03 × 16` and as vault `09 × 16`:

1. With the original header: `Corrupted` from the header check, and nothing is written to the output.
2. With the header rewritten to match the other attachment or vault: still `Corrupted`, because the header is part of every chunk's associated data.

### `size_must_match_the_record`

A 1000-byte blob decrypted with expected sizes 0, 999, 1001 and `u64::MAX` returns `Corrupted` ("larger than recorded" or "smaller than recorded").

### `wrong_key_is_detected`

A one-byte blob decrypted with `[0x43; 32]` returns `Corrupted`.

### `foreign_and_newer_blobs_are_reported`

On a one-byte blob: byte 0 set to `X` gives `Corrupted`; version byte 8 set to `2` gives `UnsupportedVersion { found: 2 }`; algorithm byte 10 set to `7` gives `UnsupportedAlgorithm { id: 7 }`; reserved byte 12 set to `1` gives `Corrupted`.

### `progress_can_cancel_both_directions`

1. Encrypting `3 × CHUNK_LEN` bytes with a callback that breaks once at least `CHUNK_LEN` bytes are done returns `Cancelled`.
2. Decrypting the full blob with a callback that always breaks returns `Cancelled`, and exactly one chunk (`CHUNK_LEN` bytes) of authenticated plaintext was written to the sink first. That is why callers must discard partial output.

## Dependencies

- **Internal**: [`attachment`](attachment.rs.md) (`super::*`): `encrypt_stream`, `decrypt_stream`, `BlobHeader`, `blob_len` and the constants; [`header`](header.rs.md): `VaultId`; [`item`](item.rs.md): `AttachmentId`; [`keys`](keys.rs.md): `SecretKey`; [`error`](error.rs.md): `VaultError`.
- **External**: none beyond the standard library (`std::io::{Read, Write}`, `std::ops::ControlFlow`).

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`crates/novavault-core/src/attachment.rs`](../../../../../crates/novavault-core/src/attachment.rs), documented in [`attachment.rs.md`](attachment.rs.md).

How to run:

```sh
cargo test -p novavault-core attachment::tests
cargo test -p novavault-core attachment::tests::large_file_streams_through_bounded_buffers
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- `large_file_streams_through_bounded_buffers` encrypts and decrypts 32 MiB and keeps the whole 32 MiB ciphertext in memory; it is the slowest test in this module in debug builds.
- The tampering and truncation tests use chosen offsets, not every byte, to keep the run time reasonable on multi-chunk blobs. `docs/vault-format.md` §13 says the core's tests check "every truncation" and, for blobs, "flipped bytes in every region of every chunk"; for blobs that means representative offsets in each region, not every byte.
- The blob header vector must stay identical to `docs/vault-format.md` §13 by hand. It matches at the time of writing.
