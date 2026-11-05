# [`crates/novavault-core/src/attachment.rs`](../../../../../crates/novavault-core/src/attachment.rs)

## Purpose

Streams the encryption and decryption of file attachments. Attachments can be much larger than a vault file, so each one is stored in its own *blob* file and encrypted in 64 KiB chunks, one XChaCha20-Poly1305 sealed message per chunk. The construction is STREAM (Hoang, Reyhanitabar, Rogaway and Vizár) built on the vault's existing AEAD with random nonces. Only one chunk of plaintext is in memory at a time.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `BLOB_MAGIC` | constant | `pub const BLOB_MAGIC: [u8; 8] = *b"NVATTCH\0";` | Blob signature. |
| `BLOB_VERSION` | constant | `pub const BLOB_VERSION: u16 = 1;` | Blob format version written and accepted. |
| `BLOB_HEADER_LEN` | constant | `pub const BLOB_HEADER_LEN: usize = 48;` | Plaintext blob header length. |
| `CHUNK_LEN` | constant | `pub const CHUNK_LEN: usize = 64 * 1024;` | Plaintext bytes per full chunk. |
| `SEALED_CHUNK_LEN` | constant | `pub const SEALED_CHUNK_LEN: usize = CHUNK_LEN + OVERHEAD;` | Bytes of a full sealed chunk. |
| `BLOB_EXTENSION` | constant | `pub const BLOB_EXTENSION: &str = "nvatt";` | Blob file extension. |
| `blob_len` | function | `pub const fn blob_len(len: u64) -> u64` | Blob size of a `len`-byte file. |
| `BlobHeader` | struct (crate) | `pub(crate) struct BlobHeader { pub(crate) vault_id: VaultId, pub(crate) attachment_id: AttachmentId }` | Which vault and attachment a blob belongs to. `Debug, Clone, Copy, PartialEq, Eq`. |
| `BlobHeader::to_bytes` | function (crate) | `pub(crate) fn to_bytes(self) -> [u8; BLOB_HEADER_LEN]` | 48-byte encoding. |
| `encrypt_stream` | function (crate) | `pub(crate) fn encrypt_stream(key: &SecretKey, header: BlobHeader, source: &mut impl Read, sink: &mut impl Write, mut progress: impl FnMut(u64) -> ControlFlow<()>) -> VaultResult<u64>` | Encrypts a stream into a blob. |
| `decrypt_stream` | function (crate) | `pub(crate) fn decrypt_stream(key: &SecretKey, header: BlobHeader, expected_len: u64, source: impl Read, sink: &mut impl Write, mut progress: impl FnMut(u64) -> ControlFlow<()>) -> VaultResult<()>` | Verifies and decrypts a blob into a stream. |

The module is `pub`, so the constants and `blob_len` are reachable as `novavault_core::attachment::*`. Nothing here is re-exported from the crate root.

## Details

### Blob layout

```text
blob header (48 bytes, plaintext)
  magic "NVATTCH\0" (8) ‖ version u16 ‖ aead_id u8 ‖ reserved (5, zero)
  ‖ vault_id (16) ‖ attachment_id (16)
chunk 0 ‖ chunk 1 ‖ … ‖ final chunk
  chunk    = nonce (24) ‖ ciphertext ‖ tag (16)
  chunk AD = blob header ‖ 0x03 ‖ chunk_index u64 ‖ is_final u8
```

Every chunk except the last holds exactly `CHUNK_LEN` plaintext bytes. The last chunk is always present. It holds 1 to `CHUNK_LEN` bytes, or 0 bytes for an empty file, which is a single empty chunk.

- **Security**:
  - **Nonces**: each chunk is sealed with `aead::encrypt` and so gets a fresh random 192-bit nonce. No counter or nonce state is kept, unlike counter-based STREAM. Nonces are not derived from the index; the index and the final flag go into the associated data instead.
  - **Ordering and truncation**: the index stops reordering, dropping and duplicating chunks. The final flag stops truncation: a reader treats a chunk as final exactly when the file ends after it, so a cut after a full chunk makes that chunk fail authentication.
  - **Binding**: the blob header (with `vault_id` and `attachment_id`) is in every chunk's associated data, so blobs cannot be swapped between attachments or vaults, even with a rewritten header.
  - **Domain separation**: the context byte `0x03` separates chunks from records (`0x01`) and the seal (`0x02`), which share the vault key.

### `BLOB_MAGIC`, `BLOB_VERSION`, `BLOB_HEADER_LEN`

- **Format versioning**: `BLOB_VERSION` is independent of the vault's `FORMAT_VERSION`. Only version 1 exists and is accepted. A compile-time assertion keeps `BLOB_HEADER_LEN == 16 + 16 + ATTACHMENT_ID_LEN`.

### `CHUNK_LEN`, `SEALED_CHUNK_LEN`, `BLOB_EXTENSION`

64 KiB plaintext chunks, so a full sealed chunk is 65 576 bytes. Blobs are named `<attachment_id>.nvatt` by [`vault`](vault.rs.md).

### `blob_len`

`48 + len + 40 × chunks`, where `chunks` is 1 for an empty file and `⌈len / CHUNK_LEN⌉` otherwise. A length that is an exact multiple of `CHUNK_LEN` ends with a full final chunk, not an extra empty one. The function is used by tests and for size estimates.

### `BlobHeader` and `BlobHeader::to_bytes`

`to_bytes` writes the magic, `BLOB_VERSION`, `AeadId::XChaCha20Poly1305.id()`, five zero bytes, the vault identifier and the attachment identifier.

### `BlobHeader::check` (private)

Checks a read header against the expected one, in this order:

1. The magic, otherwise `Corrupted { reason: "not an attachment file" }`.
2. The version, otherwise `UnsupportedVersion { found }`.
3. The algorithm, otherwise `UnsupportedAlgorithm { id }`.
4. The reserved bytes, otherwise `Corrupted { reason: "non-zero reserved attachment header bytes" }`.
5. A full comparison with `self.to_bytes()`, otherwise `Corrupted { reason: "attachment belongs to another vault or item" }`.

### `encrypt_stream`

1. Writes the blob header to `sink`.
2. Allocates two `Zeroizing` buffers of `CHUNK_LEN` (`current` and `next`).
3. Fills `current` with `read_full`. If it is short, it is the final chunk. If it is full, it reads `next`: when `next` comes back empty, `current` is final.
4. Seals `current[..len]` with `chunk_ad(header, index, is_final)` and writes the result.
5. Adds the chunk length to the running total and calls `progress(total)`. A `Break` returns `Cancelled`.
6. On the final chunk, flushes `sink` and returns the total. Otherwise it swaps the buffers and moves to the next index.

- **Parameters**:
  - `key`: the vault key.
  - `header`: the blob's identity.
  - `source`: the plaintext reader.
  - `sink`: the blob writer.
  - `progress`: called after each chunk with the bytes encrypted so far.
- **Returns**: the plaintext length.
- **Errors**:
  - `VaultError::Io`: reading or writing failed.
  - `VaultError::Crypto`: randomness or encryption failed.
  - `VaultError::Cancelled`: `progress` returned `Break`.
  - `VaultError::InvalidItem { reason: "attachment is too large" }`: the byte count or the chunk index would overflow `u64`.
  - On any error, `sink` holds a partial blob that the caller must discard. [`vault`](vault.rs.md) writes through an `AtomicFile`.
- **Security**:
  - **Zeroization**: plaintext is only ever in the two zeroizing buffers, so at most two chunks (128 KiB) are in memory at once. The sealed output is ciphertext.
  - `progress` sees only byte counts.

### `decrypt_stream`

1. Wraps `source` in a `BufReader` of `SEALED_CHUNK_LEN`.
2. Reads 48 header bytes (a short read is `Corrupted { reason: "truncated attachment" }`) and runs `check`.
3. Repeats: reads up to `SEALED_CHUNK_LEN` bytes. Fewer than `OVERHEAD` bytes is `Corrupted { reason: "truncated attachment" }`. The chunk is final if it is short or if `fill_buf` shows the end of the input.
4. Opens the chunk with `chunk_ad(header_bytes, index, is_final)`. Failure is `Corrupted { reason: "attachment failed authentication" }`.
5. Adds the chunk length to the total. Exceeding `expected_len` is `Corrupted { reason: "attachment is larger than recorded" }`.
6. Writes the authenticated plaintext to `sink` and calls `progress(total)`. A `Break` returns `Cancelled`.
7. After the final chunk, a total different from `expected_len` is `Corrupted { reason: "attachment is smaller than recorded" }`. Otherwise it flushes `sink`.

- **Parameters**:
  - `key`: the vault key.
  - `header`: the expected identity.
  - `expected_len`: the size recorded in the item's encrypted payload.
  - `source`: the blob.
  - `sink`: the plaintext writer.
  - `progress`: called after each chunk.
- **Returns**: `()` once every chunk has authenticated and the size matches.
- **Errors**:
  - `VaultError::Corrupted`: truncated, extended, reordered, tampered, wrong key, wrong vault or attachment, or a size mismatch.
  - `VaultError::UnsupportedVersion` and `VaultError::UnsupportedAlgorithm`: a blob from a newer build.
  - `VaultError::Io`: read or write failure.
  - `VaultError::Cancelled`.
  - `VaultError::InvalidItem`: `u64` overflow.
- **Security**:
  - Each chunk is authenticated before any of its plaintext is written, so unauthenticated plaintext is never released.
  - A damaged chunk is only detected when it is reached, so on error `sink` may already hold the authenticated plaintext of earlier chunks. The caller must discard it; [`vault`](vault.rs.md) writes exports through an `AtomicFile` that is deleted unless committed.
  - The plaintext of each chunk is a `SecretSlice` from `aead::decrypt`, wiped on drop. The ciphertext buffer is not zeroized, and needs no zeroization.
  - Checking `expected_len` ties the metadata shown to the user to the data actually exported.

### `chunk_ad`, `read_full`, `corrupted`, `too_large` (private)

- `chunk_ad` builds the 58-byte associated data.
- `read_full` loops over `read` until the buffer is full or the input ends, retrying `Interrupted`.
- `corrupted` and `too_large` are error shorthands.

## Dependencies

- **Internal**:
  - [`aead`](aead.rs.md): `encrypt`, `decrypt`, `OVERHEAD`.
  - [`error`](error.rs.md): `VaultError`, `VaultResult`.
  - [`header`](header.rs.md): `AeadId`, `VaultId`.
  - [`item`](item.rs.md): `AttachmentId`, `ATTACHMENT_ID_LEN`.
  - [`keys`](keys.rs.md): `SecretKey`.
- **External**:
  - `secrecy`: `ExposeSecret` to read the decrypted chunk.
  - `zeroize`: `Zeroizing` for the plaintext buffers.

## Used by

- [`vault`](vault.rs.md): `encrypt_stream` in `add_attachment`, `decrypt_stream` in `export_attachment`, `BlobHeader`, and `BLOB_EXTENSION` for blob names. Its tests also use `CHUNK_LEN`, `BLOB_HEADER_LEN`, `SEALED_CHUNK_LEN` and `blob_len`.

## Tests

- [`known_header_encoding`](../../../../../crates/novavault-core/src/attachment.rs#L366): the blob header matches the vector in [vault-format.md §13](../../../../vault-format.md#13-test-vectors).
- [`round_trips_at_chunk_boundaries`](../../../../../crates/novavault-core/src/attachment.rs#L386): 0, 1, `CHUNK_LEN − 1`, `CHUNK_LEN`, `CHUNK_LEN + 1`, `2 × CHUNK_LEN` and `3 × CHUNK_LEN + 7` bytes round-trip with the size `blob_len` predicts.
- [`large_file_streams_through_bounded_buffers`](../../../../../crates/novavault-core/src/attachment.rs#L446): a 32 MiB file streams through both directions with one progress call per chunk.
- [`every_truncation_is_detected`](../../../../../crates/novavault-core/src/attachment.rs#L480): cuts inside the header, at every chunk boundary and inside chunks give `Corrupted`.
- [`truncation_at_a_chunk_boundary_fails_even_with_a_matching_size`](../../../../../crates/novavault-core/src/attachment.rs#L502): the final flag alone catches a cut after a full chunk.
- [`every_region_of_every_chunk_is_authenticated`](../../../../../crates/novavault-core/src/attachment.rs#L513): flipped bytes in the header and in every chunk's nonce, ciphertext and tag are detected.
- [`reordered_dropped_duplicated_and_appended_chunks_are_detected`](../../../../../crates/novavault-core/src/attachment.rs#L531): swapped, dropped, duplicated, moved-final and appended chunks, and one appended byte, give `Corrupted`.
- [`blobs_cannot_be_moved_between_attachments_or_vaults`](../../../../../crates/novavault-core/src/attachment.rs#L562): another attachment or vault is rejected without output, even with a rewritten header.
- [`size_must_match_the_record`](../../../../../crates/novavault-core/src/attachment.rs#L592): expected sizes of 0, 999, 1001 and `u64::MAX` for a 1000-byte file give `Corrupted`.
- [`wrong_key_is_detected`](../../../../../crates/novavault-core/src/attachment.rs#L600): another key gives `Corrupted`.
- [`foreign_and_newer_blobs_are_reported`](../../../../../crates/novavault-core/src/attachment.rs#L611): a wrong magic, version 2, algorithm 7 and a non-zero reserved byte each give their own error.
- [`progress_can_cancel_both_directions`](../../../../../crates/novavault-core/src/attachment.rs#L634): `Break` stops encryption and decryption with `Cancelled`; decryption had written one chunk.

## Notes

- **Two chunks in memory, not one.** The module documentation says "only one chunk of plaintext is held in memory at a time". `encrypt_stream` keeps two plaintext buffers, `current` and the look-ahead `next`, so up to two chunks can be held at once. Both are zeroizing. The spirit of the claim (bounded memory) holds.
- **Stale buffer contents.** `read_full` fills the buffers in place. After a short read, the unused tail of a buffer keeps bytes from the previous chunk until it is overwritten or dropped. They are never encrypted or written, and the buffer is wiped on drop.
- **Visible sizes.** A blob's size reveals the attachment's exact plaintext size to anyone with file access, as [vault-format.md §9.6](../../../../vault-format.md#96-integrity) acknowledges.
