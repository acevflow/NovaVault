//! Byte-level framing of a vault file and its integrity seal.
//!
//! ```text
//! header        HEADER_LEN bytes (see `header`)
//! slot table    password slot ‖ recovery slot, SLOT_LEN bytes each
//! records       record_count u32 ‖ record…
//!   record      record_id (16) ‖ record_type u8 ‖ sealed_len u32 ‖ sealed
//! seal          SEAL_LEN bytes
//! ```
//!
//! `sealed` is an [`aead`] message (`nonce ‖ ciphertext ‖ tag`) of the
//! item's payload. Records are sorted by strictly ascending `record_id`, so
//! the encoding is canonical and duplicates are rejected.
//!
//! Each record is authenticated on its own, which stops a ciphertext from
//! being altered or moved to another record or vault. It does not stop a
//! whole record from being deleted, duplicated or replayed from an older
//! file, so the file ends with a seal: an AEAD encryption of the empty
//! message under the vault key, whose associated data is everything before
//! it. Associated data is domain-separated by a context byte after the
//! header:
//!
//! ```text
//! record AD = header ‖ 0x01 ‖ record_id ‖ record_type
//! seal AD   = header ‖ 0x02 ‖ slot table ‖ records
//! ```
//!
//! (Attachment chunks use the context byte `0x03`; see `attachment`.)
//!
//! This module only frames and checks bytes; unlocking and decryption are
//! done by the vault. Parsing is bounds-checked before every allocation.

use crate::aead::{self, OVERHEAD};
use crate::error::{CryptoError, VaultError, VaultResult};
use crate::header::{HEADER_LEN, Header};
use crate::item::{ITEM_ID_LEN, ItemId, ItemKind, MAX_PAYLOAD_LEN};
use crate::keys::SecretKey;
use crate::slots::{KeySlot, KeySlots, SLOT_LEN};

/// Length in bytes of the integrity seal at the end of the file.
pub const SEAL_LEN: usize = OVERHEAD;

/// Largest number of records in a vault.
pub const MAX_RECORDS: u32 = 100_000;

/// Largest sealed record: the largest payload plus the AEAD overhead.
pub const MAX_SEALED_LEN: usize = MAX_PAYLOAD_LEN + OVERHEAD;

/// Largest vault file this build reads or writes (256 MiB).
pub const MAX_FILE_LEN: usize = 256 * 1024 * 1024;

/// Associated data context byte of a record.
const RECORD_CONTEXT: u8 = 0x01;

/// Associated data context byte of the seal.
const SEAL_CONTEXT: u8 = 0x02;

/// Length of a record's plaintext framing before the sealed payload.
const RECORD_PREFIX_LEN: usize = ITEM_ID_LEN + 1 + 4;

/// One encrypted record as stored in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordEnvelope {
    /// The item's identifier.
    pub(crate) id: ItemId,
    /// The item's type.
    pub(crate) kind: ItemKind,
    /// The sealed payload: `nonce ‖ ciphertext ‖ tag`.
    pub(crate) sealed: Vec<u8>,
}

/// A vault file split into its parts, with only its structure validated.
#[derive(Debug)]
pub(crate) struct VaultFile {
    /// The decoded header.
    pub(crate) header: Header,
    /// The key slot table.
    pub(crate) slots: KeySlots,
    /// The records, in ascending identifier order.
    pub(crate) records: Vec<RecordEnvelope>,
    /// Length of everything before the seal, which the seal authenticates.
    pub(crate) body_len: usize,
}

/// Associated data binding a record to its vault, identifier and type.
pub(crate) fn record_ad(header: &Header, id: &ItemId, kind: ItemKind) -> Vec<u8> {
    let mut ad = Vec::with_capacity(HEADER_LEN + 1 + ITEM_ID_LEN + 1);
    ad.extend_from_slice(&header.to_bytes());
    ad.push(RECORD_CONTEXT);
    ad.extend_from_slice(id.as_bytes());
    ad.push(kind.id());
    ad
}

/// Encodes everything but the seal. `records` must be sorted by identifier
/// with no duplicates.
///
/// # Errors
///
/// Returns [`VaultError::InvalidItem`] if the vault exceeds [`MAX_RECORDS`]
/// or [`MAX_FILE_LEN`].
pub(crate) fn encode_body(
    header: &Header,
    slots: &KeySlots,
    records: &[RecordEnvelope],
) -> VaultResult<Vec<u8>> {
    const TOO_LARGE: VaultError = VaultError::InvalidItem {
        reason: "vault is too large",
    };
    let count = u32::try_from(records.len())
        .ok()
        .filter(|&count| count <= MAX_RECORDS)
        .ok_or(TOO_LARGE)?;
    let records_len: usize = records
        .iter()
        .map(|record| RECORD_PREFIX_LEN + record.sealed.len())
        .sum();
    let body_len = HEADER_LEN + 2 * SLOT_LEN + 4 + records_len;
    if body_len + SEAL_LEN > MAX_FILE_LEN {
        return Err(TOO_LARGE);
    }

    let mut body = Vec::with_capacity(body_len + SEAL_LEN);
    body.extend_from_slice(&header.to_bytes());
    body.extend_from_slice(&slots.password_slot().to_bytes());
    body.extend_from_slice(&slots.recovery_slot().to_bytes());
    body.extend_from_slice(&count.to_le_bytes());
    for record in records {
        // Sealed payloads come from `aead::encrypt` of a bounded payload, so
        // they always fit; the check keeps the writer as strict as the reader.
        let sealed_len = u32::try_from(record.sealed.len())
            .ok()
            .filter(|_| record.sealed.len() <= MAX_SEALED_LEN)
            .ok_or(TOO_LARGE)?;
        body.extend_from_slice(record.id.as_bytes());
        body.push(record.kind.id());
        body.extend_from_slice(&sealed_len.to_le_bytes());
        body.extend_from_slice(&record.sealed);
    }
    Ok(body)
}

/// Computes the seal of `body` (the output of [`encode_body`]).
///
/// # Errors
///
/// Any error of [`aead::encrypt`].
pub(crate) fn seal(key: &SecretKey, body: &[u8]) -> VaultResult<Vec<u8>> {
    Ok(aead::encrypt(key, &[], &seal_ad(body))?)
}

/// Checks the seal at the end of `file`, whose structure was validated by
/// [`parse`].
///
/// # Errors
///
/// Returns [`VaultError::Corrupted`] if the seal does not authenticate the
/// file under `key`: some byte after the header was changed, or records
/// were added, removed or reordered.
pub(crate) fn verify_seal(key: &SecretKey, file: &[u8], body_len: usize) -> VaultResult<()> {
    const FAILED: VaultError = VaultError::Corrupted {
        reason: "integrity check failed",
    };
    let (body, seal) = file.split_at_checked(body_len).ok_or(FAILED)?;
    match aead::decrypt(key, seal, &seal_ad(body)) {
        Ok(_) => Ok(()),
        Err(CryptoError::Decryption | CryptoError::MalformedCiphertext) => Err(FAILED),
        Err(other) => Err(other.into()),
    }
}

/// `header ‖ SEAL_CONTEXT ‖ rest of body`.
fn seal_ad(body: &[u8]) -> Vec<u8> {
    let (header, rest) = body.split_at(HEADER_LEN.min(body.len()));
    let mut ad = Vec::with_capacity(body.len() + 1);
    ad.extend_from_slice(header);
    ad.push(SEAL_CONTEXT);
    ad.extend_from_slice(rest);
    ad
}

/// Parses the structure of a vault file without decrypting anything.
///
/// The header is checked first (magic, then version), then the size, the
/// key slots, the record framing and the seal's length. Nothing here
/// proves authenticity: that needs the vault key, see [`verify_seal`].
///
/// # Errors
///
/// Any error of [`Header::parse`], or [`VaultError::Corrupted`] for any
/// other structural problem.
pub(crate) fn parse(file: &[u8]) -> VaultResult<VaultFile> {
    let header = Header::parse(file)?;
    if file.len() > MAX_FILE_LEN {
        return Err(corrupted("file is too large"));
    }
    let mut reader = Reader {
        rest: file
            .get(HEADER_LEN..)
            .ok_or(corrupted("truncated header"))?,
    };

    let password = parse_slot(reader.take(SLOT_LEN)?)?;
    let recovery = parse_slot(reader.take(SLOT_LEN)?)?;
    let slots = KeySlots::from_slots(password, recovery)
        .map_err(|_| corrupted("key slots are out of order"))?;

    let count = reader.u32()?;
    if count > MAX_RECORDS {
        return Err(corrupted("too many records"));
    }
    // No pre-allocation from `count`: every record consumes file bytes, so
    // memory use stays proportional to the (bounded) file size.
    let mut records: Vec<RecordEnvelope> = Vec::new();
    for _ in 0..count {
        let id = ItemId::from_bytes(reader.array()?);
        let kind = ItemKind::from_id(reader.u8()?)?;
        let sealed_len = usize::try_from(reader.u32()?).map_err(|_| corrupted("bad length"))?;
        if !(OVERHEAD..=MAX_SEALED_LEN).contains(&sealed_len) {
            return Err(corrupted("record length out of bounds"));
        }
        if records.last().is_some_and(|previous| previous.id >= id) {
            return Err(corrupted("records not in ascending identifier order"));
        }
        let sealed = reader.take(sealed_len)?.to_vec();
        records.push(RecordEnvelope { id, kind, sealed });
    }

    if reader.rest.len() != SEAL_LEN {
        return Err(corrupted("wrong file length"));
    }
    Ok(VaultFile {
        header,
        slots,
        records,
        body_len: file.len() - SEAL_LEN,
    })
}

/// Parses one serialized key slot, reporting any problem as file corruption.
///
/// A slot read from disk that fails strict parsing means the file is
/// damaged or crafted, so it becomes [`VaultError::Corrupted`]. Out-of-bounds
/// KDF parameters get their own reason because they are the likely sign of a
/// downgrade or resource-exhaustion attempt.
fn parse_slot(bytes: &[u8]) -> VaultResult<KeySlot> {
    KeySlot::from_bytes(bytes).map_err(|error| match error {
        CryptoError::InvalidKdfParams { .. } => {
            corrupted("key derivation parameters out of bounds")
        }
        _ => corrupted("malformed key slot"),
    })
}

/// Shorthand for [`VaultError::Corrupted`] with a static `reason`.
const fn corrupted(reason: &'static str) -> VaultError {
    VaultError::Corrupted { reason }
}

/// Bounds-checked cursor over the file. Every read fails with
/// [`VaultError::Corrupted`] instead of panicking when the file is short.
struct Reader<'a> {
    /// The bytes not read yet.
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    /// Consumes and returns the next `len` bytes.
    fn take(&mut self, len: usize) -> VaultResult<&'a [u8]> {
        let (head, rest) = self
            .rest
            .split_at_checked(len)
            .ok_or(corrupted("unexpected end of file"))?;
        self.rest = rest;
        Ok(head)
    }

    /// Consumes the next `N` bytes as an array.
    fn array<const N: usize>(&mut self) -> VaultResult<[u8; N]> {
        self.take(N)?
            .try_into()
            .map_err(|_| corrupted("unexpected end of file"))
    }

    /// Consumes one byte.
    fn u8(&mut self) -> VaultResult<u8> {
        self.array::<1>().map(|[byte]| byte)
    }

    /// Consumes a little-endian `u32`.
    fn u32(&mut self) -> VaultResult<u32> {
        self.array().map(u32::from_le_bytes)
    }
}

#[cfg(test)]
mod tests {
    use secrecy::SecretSlice;

    use super::*;
    use crate::kdf::KdfParams;

    struct Fixture {
        header: Header,
        slots: KeySlots,
        key: SecretKey,
        records: Vec<RecordEnvelope>,
    }

    fn fixture() -> Fixture {
        let header = Header::generate().unwrap();
        let params =
            KdfParams::new(KdfParams::MIN_MEMORY_KIB, KdfParams::MIN_ITERATIONS, 1).unwrap();
        let password = SecretSlice::from(b"pw".to_vec());
        let (slots, key, _) = KeySlots::create(&password, params, &header.slot_ad()).unwrap();
        let mut records: Vec<RecordEnvelope> = (1u8..=3)
            .map(|n| {
                let id = ItemId::from_bytes([n; ITEM_ID_LEN]);
                let kind = ItemKind::Note;
                let sealed = aead::encrypt(&key, &[n; 64], &record_ad(&header, &id, kind)).unwrap();
                RecordEnvelope { id, kind, sealed }
            })
            .collect();
        records.sort_by_key(|record| record.id);
        Fixture {
            header,
            slots,
            key,
            records,
        }
    }

    fn sealed_file(f: &Fixture, records: &[RecordEnvelope]) -> Vec<u8> {
        let mut file = encode_body(&f.header, &f.slots, records).unwrap();
        let seal = seal(&f.key, &file).unwrap();
        file.extend_from_slice(&seal);
        file
    }

    #[test]
    fn round_trips_and_verifies() {
        let f = fixture();
        let file = sealed_file(&f, &f.records);
        let parsed = parse(&file).unwrap();
        assert_eq!(parsed.header, f.header);
        assert_eq!(parsed.slots, f.slots);
        assert_eq!(parsed.records, f.records);
        assert_eq!(parsed.body_len, file.len() - SEAL_LEN);
        verify_seal(&f.key, &file, parsed.body_len).unwrap();
    }

    #[test]
    fn empty_vault_round_trips() {
        let f = fixture();
        let file = sealed_file(&f, &[]);
        assert_eq!(file.len(), HEADER_LEN + 2 * SLOT_LEN + 4 + SEAL_LEN);
        let parsed = parse(&file).unwrap();
        assert!(parsed.records.is_empty());
        verify_seal(&f.key, &file, parsed.body_len).unwrap();
    }

    #[test]
    fn every_truncation_is_rejected() {
        let f = fixture();
        let file = sealed_file(&f, &f.records);
        for len in 0..file.len() {
            assert!(parse(&file[..len]).is_err(), "length {len}");
        }
        let mut longer = file.clone();
        longer.push(0);
        assert!(matches!(parse(&longer), Err(VaultError::Corrupted { .. })));
    }

    #[test]
    fn seal_detects_every_changed_byte() {
        let f = fixture();
        let file = sealed_file(&f, &f.records);
        let body_len = file.len() - SEAL_LEN;
        // The seal needs no key derivation, so every byte can be checked.
        for index in 0..file.len() {
            let mut tampered = file.clone();
            tampered[index] ^= 0x01;
            assert!(
                verify_seal(&f.key, &tampered, body_len).is_err(),
                "byte {index}"
            );
        }
    }

    #[test]
    fn seal_detects_removed_and_replayed_records() {
        let f = fixture();
        let file = sealed_file(&f, &f.records);
        let body_len = file.len() - SEAL_LEN;
        let seal = &file[body_len..];

        // Drop the last record but keep the original seal.
        let mut fewer = encode_body(&f.header, &f.slots, &f.records[..2]).unwrap();
        let fewer_len = fewer.len();
        fewer.extend_from_slice(seal);
        assert!(parse(&fewer).is_ok());
        assert!(matches!(
            verify_seal(&f.key, &fewer, fewer_len),
            Err(VaultError::Corrupted { .. })
        ));
    }

    #[test]
    fn rejects_unordered_and_duplicate_records() {
        let f = fixture();
        let mut reversed = f.records.clone();
        reversed.reverse();
        let mut duplicated = f.records.clone();
        duplicated.push(f.records[2].clone());
        for records in [reversed, duplicated] {
            assert!(matches!(
                parse(&sealed_file(&f, &records)),
                Err(VaultError::Corrupted { .. })
            ));
        }
    }

    #[test]
    fn rejects_bad_framing() {
        let f = fixture();
        let file = sealed_file(&f, &f.records);
        let count_at = HEADER_LEN + 2 * SLOT_LEN;
        let first_record = count_at + 4;

        let mut too_many = file.clone();
        too_many[count_at..first_record].copy_from_slice(&(MAX_RECORDS + 1).to_le_bytes());
        let mut unknown_type = file.clone();
        unknown_type[first_record + ITEM_ID_LEN] = 9;
        let mut short_sealed = file.clone();
        short_sealed[first_record + ITEM_ID_LEN + 1..first_record + RECORD_PREFIX_LEN]
            .copy_from_slice(&((OVERHEAD - 1) as u32).to_le_bytes());
        let mut huge_sealed = file.clone();
        huge_sealed[first_record + ITEM_ID_LEN + 1..first_record + RECORD_PREFIX_LEN]
            .copy_from_slice(&u32::MAX.to_le_bytes());
        let mut swapped_slots = file.clone();
        swapped_slots[HEADER_LEN..HEADER_LEN + 2 * SLOT_LEN].rotate_left(SLOT_LEN);
        let mut weak_kdf = file.clone();
        weak_kdf[HEADER_LEN + 17..HEADER_LEN + 21].copy_from_slice(&1024u32.to_le_bytes());

        for (name, input) in [
            ("record count", too_many),
            ("record type", unknown_type),
            ("short record", short_sealed),
            ("huge record", huge_sealed),
            ("slot order", swapped_slots),
            ("weak KDF", weak_kdf),
        ] {
            assert!(
                matches!(parse(&input), Err(VaultError::Corrupted { .. })),
                "{name}"
            );
        }
    }

    #[test]
    fn oversized_file_is_rejected_after_the_header_check() {
        let f = fixture();
        // A zeroed allocation is mapped lazily, so this stays cheap.
        let mut file = vec![0u8; MAX_FILE_LEN + 1];
        file[..HEADER_LEN].copy_from_slice(&f.header.to_bytes());
        assert!(matches!(parse(&file), Err(VaultError::Corrupted { .. })));
        file[0] = b'X';
        assert!(matches!(parse(&file), Err(VaultError::NotAVault)));
    }

    #[test]
    fn record_ad_is_domain_separated() {
        let f = fixture();
        let id = ItemId::from_bytes([7; ITEM_ID_LEN]);
        let ad = record_ad(&f.header, &id, ItemKind::Secret);
        assert_eq!(ad.len(), HEADER_LEN + 1 + ITEM_ID_LEN + 1);
        assert_eq!(ad[HEADER_LEN], RECORD_CONTEXT);
        assert_ne!(ad, record_ad(&f.header, &id, ItemKind::Note));
        assert_eq!(seal_ad(&f.header.to_bytes())[HEADER_LEN], SEAL_CONTEXT);
    }
}
