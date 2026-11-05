//! Streaming encryption of file attachments.
//!
//! Attachments can be far larger than a vault file and must not be loaded
//! into memory whole, so each one is stored in its own file (an
//! *attachment blob*) and encrypted in fixed-size chunks, one
//! XChaCha20-Poly1305 sealed message per chunk:
//!
//! ```text
//! blob header (48 bytes, plaintext)
//!   magic "NVATTCH\0" (8) ‖ version u16 ‖ aead_id u8 ‖ reserved (5, zero)
//!   ‖ vault_id (16) ‖ attachment_id (16)
//! chunk 0 ‖ chunk 1 ‖ … ‖ final chunk
//!   chunk = nonce (24) ‖ ciphertext ‖ tag (16)
//! ```
//!
//! Every chunk except the last holds exactly [`CHUNK_LEN`] bytes of
//! plaintext. The last chunk holds between 0 and [`CHUNK_LEN`] bytes and is
//! always present, so an empty file is one empty chunk. Each chunk's
//! associated data is
//!
//! ```text
//! blob header ‖ 0x03 ‖ chunk_index u64 ‖ is_final u8
//! ```
//!
//! This is the shape of the STREAM construction (Hoang, Reyhanitabar,
//! Rogaway and Vizár) built on the AEAD the rest of the vault already uses,
//! with random 192-bit nonces instead of counter-derived ones. The index
//! stops chunks from being reordered, dropped or duplicated; the final flag
//! stops truncation, because a reader treats a chunk as final exactly when
//! the file ends after it, and a chunk that was not final when written then
//! fails to authenticate. The blob header binds every chunk to its vault
//! and attachment, so blobs cannot be swapped. The size recorded in the
//! owning item must also match, so the metadata shown to the user always
//! describes the data that is exported.
//!
//! Only one chunk of plaintext is held in memory at a time, in a buffer
//! that is wiped when dropped. Decryption authenticates each chunk before
//! writing it out, but a damaged blob is only detected when the bad chunk
//! is reached, so callers must discard the output on error (the vault
//! writes exports through an atomic temporary file for that reason).

use std::io::{self, BufRead, BufReader, Read, Write};
use std::ops::ControlFlow;

use secrecy::ExposeSecret;
use zeroize::Zeroizing;

use crate::aead::{self, OVERHEAD};
use crate::error::{VaultError, VaultResult};
use crate::header::{AeadId, VaultId};
use crate::item::{ATTACHMENT_ID_LEN, AttachmentId};
use crate::keys::SecretKey;

/// Bytes identifying an attachment blob.
pub const BLOB_MAGIC: [u8; 8] = *b"NVATTCH\0";

/// The blob format version written and accepted by this build.
pub const BLOB_VERSION: u16 = 1;

/// Length in bytes of the plaintext blob header.
pub const BLOB_HEADER_LEN: usize = 48;

/// Plaintext bytes in every chunk but the last (64 KiB).
pub const CHUNK_LEN: usize = 64 * 1024;

/// Length of a full sealed chunk.
pub const SEALED_CHUNK_LEN: usize = CHUNK_LEN + OVERHEAD;

/// File extension of attachment blobs.
pub const BLOB_EXTENSION: &str = "nvatt";

/// Associated data context byte of an attachment chunk (records use `0x01`
/// and the vault's integrity seal `0x02`).
const CHUNK_CONTEXT: u8 = 0x03;

/// The header of one attachment blob: which vault and attachment it
/// belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlobHeader {
    /// The owning vault, so a blob cannot be copied into another vault.
    pub(crate) vault_id: VaultId,
    /// The attachment, so blobs cannot be swapped within one vault.
    pub(crate) attachment_id: AttachmentId,
}

impl BlobHeader {
    /// Encodes the header to its [`BLOB_HEADER_LEN`]-byte form.
    pub(crate) fn to_bytes(self) -> [u8; BLOB_HEADER_LEN] {
        let mut bytes = [0u8; BLOB_HEADER_LEN];
        bytes[..8].copy_from_slice(&BLOB_MAGIC);
        bytes[8..10].copy_from_slice(&BLOB_VERSION.to_le_bytes());
        bytes[10] = AeadId::XChaCha20Poly1305.id();
        // bytes[11..16] are reserved and stay zero.
        bytes[16..32].copy_from_slice(self.vault_id.as_bytes());
        bytes[32..].copy_from_slice(self.attachment_id.as_bytes());
        bytes
    }

    /// Checks that `bytes` is the header of the blob `self` describes.
    ///
    /// # Errors
    ///
    /// - [`VaultError::Corrupted`] if the magic or reserved bytes are wrong
    ///   or the blob belongs to another vault or attachment.
    /// - [`VaultError::UnsupportedVersion`] or
    ///   [`VaultError::UnsupportedAlgorithm`] for an unknown version or
    ///   algorithm.
    fn check(self, bytes: &[u8; BLOB_HEADER_LEN]) -> VaultResult<()> {
        if bytes[..8] != BLOB_MAGIC {
            return Err(corrupted("not an attachment file"));
        }
        let version = u16::from_le_bytes([bytes[8], bytes[9]]);
        if version != BLOB_VERSION {
            return Err(VaultError::UnsupportedVersion { found: version });
        }
        AeadId::from_id(bytes[10])?;
        if bytes[11..16] != [0u8; 5] {
            return Err(corrupted("non-zero reserved attachment header bytes"));
        }
        // Compared in full, not field by field: everything else is fixed.
        if *bytes != self.to_bytes() {
            return Err(corrupted("attachment belongs to another vault or item"));
        }
        Ok(())
    }
}

/// Associated data of chunk `index` of the blob with `header`:
/// `header ‖ 0x03 ‖ index u64 ‖ is_final u8`. The index and the final flag
/// are what make reordering and truncation detectable.
fn chunk_ad(header: &[u8; BLOB_HEADER_LEN], index: u64, is_final: bool) -> Vec<u8> {
    let mut ad = Vec::with_capacity(BLOB_HEADER_LEN + 1 + 8 + 1);
    ad.extend_from_slice(header);
    ad.push(CHUNK_CONTEXT);
    ad.extend_from_slice(&index.to_le_bytes());
    ad.push(u8::from(is_final));
    ad
}

/// Encrypts everything `source` yields into a blob written to `sink`.
///
/// `progress` is called after each chunk with the number of plaintext
/// bytes encrypted so far; returning [`ControlFlow::Break`] stops with
/// [`VaultError::Cancelled`].
///
/// Returns the plaintext length.
///
/// # Errors
///
/// [`VaultError::Io`] if reading or writing fails, [`VaultError::Crypto`]
/// for encryption or randomness failures, or [`VaultError::Cancelled`]. On
/// error `sink` holds a partial blob, which the caller must discard.
pub(crate) fn encrypt_stream(
    key: &SecretKey,
    header: BlobHeader,
    source: &mut impl Read,
    sink: &mut impl Write,
    mut progress: impl FnMut(u64) -> ControlFlow<()>,
) -> VaultResult<u64> {
    let header = header.to_bytes();
    sink.write_all(&header)?;

    // Two buffers: whether a full chunk is the last one is only known once
    // the next read comes back empty.
    let mut current = Zeroizing::new(vec![0u8; CHUNK_LEN]);
    let mut next = Zeroizing::new(vec![0u8; CHUNK_LEN]);
    let mut current_len = read_full(source, &mut current)?;
    let mut index: u64 = 0;
    let mut total: u64 = 0;
    loop {
        let (is_final, next_len) = if current_len < CHUNK_LEN {
            (true, 0)
        } else {
            let next_len = read_full(source, &mut next)?;
            (next_len == 0, next_len)
        };
        let plaintext = &current[..current_len];
        let sealed = aead::encrypt(key, plaintext, &chunk_ad(&header, index, is_final))?;
        sink.write_all(&sealed)?;
        total = total
            .checked_add(u64::try_from(current_len).map_err(|_| too_large())?)
            .ok_or_else(too_large)?;
        if progress(total).is_break() {
            return Err(VaultError::Cancelled);
        }
        if is_final {
            sink.flush()?;
            return Ok(total);
        }
        core::mem::swap(&mut current, &mut next);
        current_len = next_len;
        index = index.checked_add(1).ok_or_else(too_large)?;
    }
}

/// Decrypts the blob read from `source` into `sink`, checking that it is
/// the blob `header` describes and holds exactly `expected_len` bytes.
///
/// Each chunk is authenticated before it is written. `progress` is called
/// after each chunk with the number of plaintext bytes written so far;
/// returning [`ControlFlow::Break`] stops with [`VaultError::Cancelled`].
///
/// # Errors
///
/// - [`VaultError::Corrupted`] if the blob is truncated, extended,
///   reordered, tampered with, belongs to another vault or attachment, or
///   has another length than `expected_len`.
/// - [`VaultError::UnsupportedVersion`] or
///   [`VaultError::UnsupportedAlgorithm`] for a blob from a newer build.
/// - [`VaultError::Io`] if reading or writing fails.
/// - [`VaultError::Cancelled`].
///
/// On error `sink` may hold authenticated plaintext of the chunks before
/// the failure; the caller must discard it.
pub(crate) fn decrypt_stream(
    key: &SecretKey,
    header: BlobHeader,
    expected_len: u64,
    source: impl Read,
    sink: &mut impl Write,
    mut progress: impl FnMut(u64) -> ControlFlow<()>,
) -> VaultResult<()> {
    let mut source = BufReader::with_capacity(SEALED_CHUNK_LEN, source);
    let mut header_bytes = [0u8; BLOB_HEADER_LEN];
    if read_full(&mut source, &mut header_bytes)? != BLOB_HEADER_LEN {
        return Err(corrupted("truncated attachment"));
    }
    header.check(&header_bytes)?;

    let mut sealed = vec![0u8; SEALED_CHUNK_LEN];
    let mut index: u64 = 0;
    let mut total: u64 = 0;
    loop {
        let sealed_len = read_full(&mut source, &mut sealed)?;
        if sealed_len < OVERHEAD {
            return Err(corrupted("truncated attachment"));
        }
        // A short read means the file ended; a full chunk is the last one
        // only if nothing follows it.
        let is_final = sealed_len < SEALED_CHUNK_LEN || source.fill_buf()?.is_empty();
        let plaintext = aead::decrypt(
            key,
            &sealed[..sealed_len],
            &chunk_ad(&header_bytes, index, is_final),
        )
        .map_err(|_| corrupted("attachment failed authentication"))?;
        let plaintext = plaintext.expose_secret();
        total = total
            .checked_add(u64::try_from(plaintext.len()).map_err(|_| too_large())?)
            .ok_or_else(too_large)?;
        if total > expected_len {
            return Err(corrupted("attachment is larger than recorded"));
        }
        sink.write_all(plaintext)?;
        if progress(total).is_break() {
            return Err(VaultError::Cancelled);
        }
        if is_final {
            break;
        }
        index = index.checked_add(1).ok_or_else(too_large)?;
    }
    if total != expected_len {
        return Err(corrupted("attachment is smaller than recorded"));
    }
    sink.flush()?;
    Ok(())
}

/// Reads until `buffer` is full or the input ends; returns the bytes read.
fn read_full(source: &mut impl Read, buffer: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match source.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(filled)
}

/// Shorthand for [`VaultError::Corrupted`] with a static `reason`.
const fn corrupted(reason: &'static str) -> VaultError {
    VaultError::Corrupted { reason }
}

/// The error for a byte or chunk count that would overflow `u64`.
const fn too_large() -> VaultError {
    VaultError::InvalidItem {
        reason: "attachment is too large",
    }
}

/// Length in bytes of the blob of a `len`-byte file, for tests and size
/// estimates.
pub const fn blob_len(len: u64) -> u64 {
    let chunk = CHUNK_LEN as u64;
    // An exact multiple still needs its final chunk to be full, not an
    // extra empty one; only an empty file has an empty chunk.
    let chunks = if len == 0 { 1 } else { len.div_ceil(chunk) };
    BLOB_HEADER_LEN as u64 + len + chunks * OVERHEAD as u64
}

// Keep the documented header layout and the identifier length in step:
// a 16-byte fixed prefix (magic, version, algorithm, reserved), then the
// 16-byte vault identifier, then the attachment identifier.
const _: () = assert!(BLOB_HEADER_LEN == 16 + 16 + ATTACHMENT_ID_LEN);

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> SecretKey {
        SecretKey::from_bytes([0x42; 32])
    }

    fn header() -> BlobHeader {
        BlobHeader {
            vault_id: VaultId::from_bytes([1; 16]),
            attachment_id: AttachmentId::from_bytes([2; 16]),
        }
    }

    fn keep_going(_: u64) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    /// Deterministic, non-repeating test content.
    fn byte_at(position: u64) -> u8 {
        (position.wrapping_mul(2_654_435_761) >> 13) as u8
    }

    fn content(len: usize) -> Vec<u8> {
        (0..len as u64).map(byte_at).collect()
    }

    fn encrypt(plaintext: &[u8]) -> Vec<u8> {
        let mut blob = Vec::new();
        let len =
            encrypt_stream(&key(), header(), &mut &plaintext[..], &mut blob, keep_going).unwrap();
        assert_eq!(len, plaintext.len() as u64);
        blob
    }

    fn decrypt(blob: &[u8], expected_len: u64) -> VaultResult<Vec<u8>> {
        let mut out = Vec::new();
        decrypt_stream(&key(), header(), expected_len, blob, &mut out, keep_going)?;
        Ok(out)
    }

    fn assert_corrupted(result: VaultResult<Vec<u8>>, what: &str) {
        assert!(
            matches!(result, Err(VaultError::Corrupted { .. })),
            "{what}: {result:?}"
        );
    }

    /// Offset of sealed chunk `index` in a blob.
    fn chunk_at(index: usize) -> usize {
        BLOB_HEADER_LEN + index * SEALED_CHUNK_LEN
    }

    #[test]
    fn known_header_encoding() {
        let hex: String = header()
            .to_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            hex,
            concat!(
                "4e56415454434800",                 // magic "NVATTCH\0"
                "0100",                             // version 1
                "01",                               // aead_id
                "0000000000",                       // reserved
                "01010101010101010101010101010101", // vault_id
                "02020202020202020202020202020202", // attachment_id
            )
        );
    }

    #[test]
    fn round_trips_at_chunk_boundaries() {
        for len in [
            0,
            1,
            CHUNK_LEN - 1,
            CHUNK_LEN,
            CHUNK_LEN + 1,
            2 * CHUNK_LEN,
            3 * CHUNK_LEN + 7,
        ] {
            let plaintext = content(len);
            let blob = encrypt(&plaintext);
            assert_eq!(blob.len() as u64, blob_len(len as u64), "length {len}");
            assert_eq!(
                decrypt(&blob, len as u64).unwrap(),
                plaintext,
                "length {len}"
            );
        }
    }

    /// Generates `len` bytes of test content without holding them.
    struct Generated {
        position: u64,
        len: u64,
    }

    impl Read for Generated {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            // Odd-sized reads exercise `read_full`'s loop.
            let wanted = buffer.len().min(10_007) as u64;
            let count = wanted.min(self.len - self.position) as usize;
            for byte in &mut buffer[..count] {
                *byte = byte_at(self.position);
                self.position += 1;
            }
            Ok(count)
        }
    }

    /// Checks written bytes against the generator without storing them.
    struct Verifier {
        position: u64,
    }

    impl Write for Verifier {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            for &byte in buffer {
                assert_eq!(byte, byte_at(self.position), "byte {}", self.position);
                self.position += 1;
            }
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn large_file_streams_through_bounded_buffers() {
        // 32 MiB plus a partial chunk; neither side holds the whole file.
        let len = 32 * 1024 * 1024 + 12_345;
        let mut blob = Vec::new();
        let mut calls = 0u64;
        let written = encrypt_stream(
            &key(),
            header(),
            &mut Generated { position: 0, len },
            &mut blob,
            |done| {
                calls += 1;
                assert!(done <= len);
                ControlFlow::Continue(())
            },
        )
        .unwrap();
        assert_eq!(written, len);
        assert_eq!(calls, len.div_ceil(CHUNK_LEN as u64));
        assert_eq!(blob.len() as u64, blob_len(len));

        let mut verifier = Verifier { position: 0 };
        let mut last = 0;
        decrypt_stream(&key(), header(), len, &blob[..], &mut verifier, |done| {
            assert!(done > last);
            last = done;
            ControlFlow::Continue(())
        })
        .unwrap();
        assert_eq!(verifier.position, len);
        assert_eq!(last, len);
    }

    #[test]
    fn every_truncation_is_detected() {
        let len = 2 * CHUNK_LEN + 100;
        let blob = encrypt(&content(len));
        // Every chunk boundary, and points inside the header and chunks.
        let mut cuts = vec![0, 1, BLOB_HEADER_LEN - 1, BLOB_HEADER_LEN];
        for index in 0..3 {
            let start = chunk_at(index);
            cuts.extend([
                start,
                start + 1,
                start + OVERHEAD - 1,
                start + OVERHEAD,
                start + 1000,
            ]);
        }
        cuts.extend([chunk_at(1) - 1, chunk_at(2) - 1, blob.len() - 1]);
        for cut in cuts.into_iter().filter(|&cut| cut < blob.len()) {
            assert_corrupted(decrypt(&blob[..cut], len as u64), &format!("cut at {cut}"));
        }
    }

    #[test]
    fn truncation_at_a_chunk_boundary_fails_even_with_a_matching_size() {
        // The recorded size is no help if it was tampered with too: the
        // final flag alone must catch a blob cut after a full chunk.
        let blob = encrypt(&content(2 * CHUNK_LEN));
        assert_corrupted(
            decrypt(&blob[..chunk_at(1)], CHUNK_LEN as u64),
            "cut after the first chunk",
        );
    }

    #[test]
    fn every_region_of_every_chunk_is_authenticated() {
        let len = 2 * CHUNK_LEN + 100;
        let blob = encrypt(&content(len));
        let last = chunk_at(2);
        let mut offsets = vec![0, 8, 10, 16, 32, BLOB_HEADER_LEN - 1];
        for start in [chunk_at(0), chunk_at(1), last] {
            offsets.extend([start, start + 23, start + 24, start + 100]);
        }
        offsets.extend([chunk_at(1) - 1, last - 1, blob.len() - 1]);
        for offset in offsets {
            let mut tampered = blob.clone();
            tampered[offset] ^= 0x01;
            let result = decrypt(&tampered, len as u64);
            assert!(result.is_err(), "byte {offset} went undetected");
        }
    }

    #[test]
    fn reordered_dropped_duplicated_and_appended_chunks_are_detected() {
        let len = 3 * CHUNK_LEN + 5;
        let blob = encrypt(&content(len));
        let header = &blob[..BLOB_HEADER_LEN];
        let chunk = |index: usize| &blob[chunk_at(index)..chunk_at(index + 1).min(blob.len())];
        let assemble = |order: &[usize]| {
            let mut out = header.to_vec();
            for &index in order {
                out.extend_from_slice(chunk(index));
            }
            out
        };
        for (what, order) in [
            ("swapped", vec![1, 0, 2, 3]),
            ("dropped", vec![0, 2, 3]),
            ("duplicated", vec![0, 1, 1, 2, 3]),
            ("final moved", vec![0, 1, 3]),
        ] {
            for expected in [len as u64, (order.len() * CHUNK_LEN) as u64] {
                assert_corrupted(decrypt(&assemble(&order), expected), what);
            }
        }
        let mut appended = blob.clone();
        appended.extend_from_slice(chunk(0));
        assert_corrupted(decrypt(&appended, len as u64), "appended chunk");
        let mut extra_byte = blob.clone();
        extra_byte.push(0);
        assert_corrupted(decrypt(&extra_byte, len as u64), "appended byte");
    }

    #[test]
    fn blobs_cannot_be_moved_between_attachments_or_vaults() {
        let blob = encrypt(b"contract");
        let other_attachment = BlobHeader {
            attachment_id: AttachmentId::from_bytes([3; 16]),
            ..header()
        };
        let other_vault = BlobHeader {
            vault_id: VaultId::from_bytes([9; 16]),
            ..header()
        };
        for other in [other_attachment, other_vault] {
            let mut out = Vec::new();
            assert!(matches!(
                decrypt_stream(&key(), other, 8, &blob[..], &mut out, keep_going),
                Err(VaultError::Corrupted { .. })
            ));
            assert!(out.is_empty());

            // Rewriting the header to match does not help: it is
            // authenticated with every chunk.
            let mut relabelled = other.to_bytes().to_vec();
            relabelled.extend_from_slice(&blob[BLOB_HEADER_LEN..]);
            assert!(matches!(
                decrypt_stream(&key(), other, 8, &relabelled[..], &mut out, keep_going),
                Err(VaultError::Corrupted { .. })
            ));
        }
    }

    #[test]
    fn size_must_match_the_record() {
        let blob = encrypt(&content(1000));
        for expected in [0, 999, 1001, u64::MAX] {
            assert_corrupted(decrypt(&blob, expected), &format!("expected {expected}"));
        }
    }

    #[test]
    fn wrong_key_is_detected() {
        let blob = encrypt(b"x");
        let mut out = Vec::new();
        let other = SecretKey::from_bytes([0x43; 32]);
        assert!(matches!(
            decrypt_stream(&other, header(), 1, &blob[..], &mut out, keep_going),
            Err(VaultError::Corrupted { .. })
        ));
    }

    #[test]
    fn foreign_and_newer_blobs_are_reported() {
        let blob = encrypt(b"x");
        let mut not_blob = blob.clone();
        not_blob[0] = b'X';
        assert_corrupted(decrypt(&not_blob, 1), "magic");
        let mut newer = blob.clone();
        newer[8] = 2;
        assert!(matches!(
            decrypt(&newer, 1),
            Err(VaultError::UnsupportedVersion { found: 2 })
        ));
        let mut cipher = blob.clone();
        cipher[10] = 7;
        assert!(matches!(
            decrypt(&cipher, 1),
            Err(VaultError::UnsupportedAlgorithm { id: 7 })
        ));
        let mut reserved = blob;
        reserved[12] = 1;
        assert_corrupted(decrypt(&reserved, 1), "reserved");
    }

    #[test]
    fn progress_can_cancel_both_directions() {
        let plaintext = content(3 * CHUNK_LEN);
        let mut blob = Vec::new();
        let result = encrypt_stream(&key(), header(), &mut &plaintext[..], &mut blob, |done| {
            if done >= CHUNK_LEN as u64 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });
        assert!(matches!(result, Err(VaultError::Cancelled)));

        let blob = encrypt(&plaintext);
        let mut out = Vec::new();
        let result = decrypt_stream(
            &key(),
            header(),
            3 * CHUNK_LEN as u64,
            &blob[..],
            &mut out,
            |_| ControlFlow::Break(()),
        );
        assert!(matches!(result, Err(VaultError::Cancelled)));
        assert_eq!(out.len(), CHUNK_LEN);
    }
}
