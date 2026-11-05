//! The plaintext vault file header.
//!
//! The header is the first [`HEADER_LEN`] bytes of every vault file. It is
//! stored in the clear so a reader can identify the file and its algorithms
//! before attempting to decrypt anything, and it is authenticated by being
//! part of the associated data of every record and of the file's integrity
//! seal. Key slots authenticate it too, but with the version field zeroed
//! (see [`Header::slot_ad`]). Integers are little-endian.
//!
//! ```text
//! offset  size  field
//!      0     8  magic           "NOVAVLT\0"
//!      8     2  format_version  u16, 1 in this version (0 is migrated)
//!     10     1  aead_id         1 = XChaCha20-Poly1305
//!     11     1  flags           must be 0
//!     12     1  slot_count      must be 2
//!     13     3  reserved        must be 0
//!     16    16  vault_id        random
//! ```
//!
//! Parsing is strict and ordered: the magic is checked first, then the
//! version, and only then the rest, so a file from a newer NovaVault is
//! reported as an unsupported version rather than as corruption.

use crate::error::{CryptoError, Result, VaultError, VaultResult};

/// Bytes identifying a NovaVault vault file.
pub const MAGIC: [u8; 8] = *b"NOVAVLT\0";

/// The format version written by this build.
pub const FORMAT_VERSION: u16 = 1;

/// The oldest format version this build reads. Older-version files are
/// migrated to [`FORMAT_VERSION`] when they are opened.
pub const MIN_READ_VERSION: u16 = 0;

/// Length in bytes of the encoded header.
pub const HEADER_LEN: usize = 32;

/// Number of key slots in a vault: one password slot and one recovery
/// slot.
pub const SLOT_COUNT: u8 = 2;

/// Length in bytes of a [`VaultId`].
pub const VAULT_ID_LEN: usize = 16;

/// Length of the prefix (magic and version) a reader checks before
/// anything else.
const VERSION_END: usize = MAGIC.len() + 2;

/// Authenticated encryption algorithm used for every ciphertext in a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AeadId {
    /// XChaCha20-Poly1305 (draft-irtf-cfrg-xchacha), the only algorithm in
    /// format versions 0 and 1.
    XChaCha20Poly1305,
}

impl AeadId {
    /// The on-disk identifier of the algorithm.
    pub const fn id(self) -> u8 {
        match self {
            Self::XChaCha20Poly1305 => 1,
        }
    }

    /// Parses an on-disk algorithm identifier.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::UnsupportedAlgorithm`] for an unknown identifier.
    pub const fn from_id(id: u8) -> VaultResult<Self> {
        match id {
            1 => Ok(Self::XChaCha20Poly1305),
            _ => Err(VaultError::UnsupportedAlgorithm { id }),
        }
    }
}

/// Random identifier of one vault.
///
/// It is bound into the associated data of every ciphertext (through the
/// header), so ciphertexts cannot be moved from one vault to another. It is
/// not secret.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VaultId([u8; VAULT_ID_LEN]);

impl VaultId {
    /// Generates a new random identifier from the OS CSPRNG.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::Random`] if the OS random number generator fails.
    pub fn generate() -> Result<Self> {
        let mut bytes = [0u8; VAULT_ID_LEN];
        getrandom::fill(&mut bytes).map_err(|_| CryptoError::Random)?;
        Ok(Self(bytes))
    }

    /// Builds an identifier from its bytes.
    pub const fn from_bytes(bytes: [u8; VAULT_ID_LEN]) -> Self {
        Self(bytes)
    }

    /// The identifier's bytes.
    pub const fn as_bytes(&self) -> &[u8; VAULT_ID_LEN] {
        &self.0
    }
}

impl core::fmt::Debug for VaultId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VaultId(")?;
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        write!(f, ")")
    }
}

/// The decoded vault header.
///
/// Every field except the version and the vault identifier is fixed, and
/// the version changes only when a file is migrated to a newer format. That
/// matters: the header is associated data for every record and the seal, so
/// a migration re-encrypts every record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    version: u16,
    vault_id: VaultId,
}

impl Header {
    /// Builds the header of a new vault with a random [`VaultId`].
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::Random`] if the OS random number generator fails.
    pub fn generate() -> Result<Self> {
        Ok(Self::new(VaultId::generate()?))
    }

    /// Builds a header for the vault `vault_id` at the current
    /// [`FORMAT_VERSION`].
    pub const fn new(vault_id: VaultId) -> Self {
        Self {
            version: FORMAT_VERSION,
            vault_id,
        }
    }

    /// The same header with another format version. Crate-internal: used to
    /// migrate older files and to build them in tests.
    pub(crate) const fn with_version(self, version: u16) -> Self {
        Self { version, ..self }
    }

    /// The file's format version.
    pub const fn version(&self) -> u16 {
        self.version
    }

    /// The vault's identifier.
    pub const fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    /// The algorithm used for every ciphertext in the file.
    pub const fn aead(&self) -> AeadId {
        AeadId::XChaCha20Poly1305
    }

    /// Encodes the header to its [`HEADER_LEN`]-byte form.
    pub fn to_bytes(&self) -> [u8; HEADER_LEN] {
        let mut bytes = [0u8; HEADER_LEN];
        bytes[..MAGIC.len()].copy_from_slice(&MAGIC);
        bytes[8..10].copy_from_slice(&self.version.to_le_bytes());
        bytes[10] = self.aead().id();
        // bytes[11] (flags) and bytes[13..16] (reserved) stay zero.
        bytes[12] = SLOT_COUNT;
        bytes[16..].copy_from_slice(self.vault_id.as_bytes());
        bytes
    }

    /// The header as authenticated by the key slots: [`Header::to_bytes`]
    /// with the `format_version` field zeroed.
    ///
    /// Key slots can only be rewritten with their secret, and a migration
    /// has at most one of the two. Leaving the version out of the slots'
    /// associated data lets a migration change it without touching either
    /// slot. The version stays authenticated: it is part of every record's
    /// associated data and of the integrity seal, which are checked right
    /// after unlocking. For a version 0 file this is exactly its header, so
    /// slots written by version 0 remain valid.
    pub fn slot_ad(&self) -> [u8; HEADER_LEN] {
        let mut bytes = self.to_bytes();
        bytes[8..10].fill(0);
        bytes
    }

    /// Parses the header at the start of `file`.
    ///
    /// `file` may be longer than [`HEADER_LEN`]; only the header bytes are
    /// read.
    ///
    /// # Errors
    ///
    /// In the order they are checked:
    ///
    /// - [`VaultError::NotAVault`] if `file` does not start with [`MAGIC`].
    /// - [`VaultError::UnsupportedVersion`] if the version is not between
    ///   [`MIN_READ_VERSION`] and [`FORMAT_VERSION`].
    /// - [`VaultError::Corrupted`] if the header is truncated.
    /// - [`VaultError::UnsupportedAlgorithm`] for an unknown algorithm.
    /// - [`VaultError::Corrupted`] for non-zero flags or reserved bytes, or a
    ///   slot count other than [`SLOT_COUNT`].
    pub fn parse(file: &[u8]) -> VaultResult<Self> {
        const TRUNCATED: VaultError = VaultError::Corrupted {
            reason: "truncated header",
        };
        if !file.starts_with(&MAGIC) {
            return Err(VaultError::NotAVault);
        }
        let version = file.get(MAGIC.len()..VERSION_END).ok_or(TRUNCATED)?;
        let version = u16::from_le_bytes([version[0], version[1]]);
        if !(MIN_READ_VERSION..=FORMAT_VERSION).contains(&version) {
            return Err(VaultError::UnsupportedVersion { found: version });
        }

        let header: &[u8; HEADER_LEN] = file
            .get(..HEADER_LEN)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(TRUNCATED)?;
        AeadId::from_id(header[10])?;
        if header[11] != 0 {
            return Err(VaultError::Corrupted {
                reason: "unknown header flags",
            });
        }
        if header[12] != SLOT_COUNT {
            return Err(VaultError::Corrupted {
                reason: "unexpected key slot count",
            });
        }
        if header[13..16] != [0u8; 3] {
            return Err(VaultError::Corrupted {
                reason: "non-zero reserved header bytes",
            });
        }
        let mut vault_id = [0u8; VAULT_ID_LEN];
        vault_id.copy_from_slice(&header[16..]);
        Ok(Self::new(VaultId::from_bytes(vault_id)).with_version(version))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Header {
        let mut id = [0u8; VAULT_ID_LEN];
        for (index, byte) in id.iter_mut().enumerate() {
            *byte = index as u8;
        }
        Header::new(VaultId::from_bytes(id))
    }

    /// Hex of [`sample`]'s encoding, as published in docs/vault-format.md.
    const SAMPLE_HEX: &str = concat!(
        "4e4f5641564c5400",                 // magic "NOVAVLT\0"
        "0100",                             // format_version 1
        "01",                               // aead_id XChaCha20-Poly1305
        "00",                               // flags
        "02",                               // slot_count
        "000000",                           // reserved
        "000102030405060708090a0b0c0d0e0f", // vault_id
    );

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn known_encoding() {
        assert_eq!(hex(&sample().to_bytes()), SAMPLE_HEX);
    }

    #[test]
    fn round_trips_with_trailing_data() {
        let mut file = sample().to_bytes().to_vec();
        file.extend_from_slice(b"rest of the file");
        assert_eq!(Header::parse(&file).unwrap(), sample());
    }

    #[test]
    fn reads_version_zero_for_migration() {
        let old = sample().with_version(0);
        let parsed = Header::parse(&old.to_bytes()).unwrap();
        assert_eq!(parsed.version(), 0);
        assert_eq!(parsed, old);
    }

    #[test]
    fn slot_ad_ignores_only_the_version() {
        let current = sample();
        let old = current.with_version(0);
        assert_eq!(current.slot_ad(), old.slot_ad());
        // A version 0 header is its own slot associated data.
        assert_eq!(old.slot_ad(), old.to_bytes());
        assert_ne!(current.slot_ad(), current.to_bytes());
        let other = Header::new(VaultId::from_bytes([0xEE; VAULT_ID_LEN]));
        assert_ne!(current.slot_ad(), other.slot_ad());
    }

    #[test]
    fn generated_ids_are_distinct() {
        assert_ne!(VaultId::generate().unwrap(), VaultId::generate().unwrap());
    }

    #[test]
    fn rejects_wrong_magic() {
        for input in [&b""[..], b"NOVA", b"NOVAVLT!", b"PK\x03\x04 zip file"] {
            assert!(matches!(Header::parse(input), Err(VaultError::NotAVault)));
        }
        let mut bytes = sample().to_bytes();
        bytes[0] ^= 0x20;
        assert!(matches!(Header::parse(&bytes), Err(VaultError::NotAVault)));
    }

    #[test]
    fn rejects_newer_versions_before_anything_else() {
        for version in [2u16, 0x0100, u16::MAX] {
            let mut bytes = sample().to_bytes();
            bytes[8..10].copy_from_slice(&version.to_le_bytes());
            // A file from a newer build may have a different layout, so only
            // the version must be reported, even if the rest looks invalid.
            bytes[10] = 0xFF;
            assert!(matches!(
                Header::parse(&bytes),
                Err(VaultError::UnsupportedVersion { found }) if found == version
            ));
            // The version is reported even when the rest is missing.
            assert!(matches!(
                Header::parse(&bytes[..VERSION_END]),
                Err(VaultError::UnsupportedVersion { .. })
            ));
        }
    }

    #[test]
    fn rejects_truncation() {
        let bytes = sample().to_bytes();
        for len in MAGIC.len()..HEADER_LEN {
            assert!(
                matches!(
                    Header::parse(&bytes[..len]),
                    Err(VaultError::Corrupted { .. })
                ),
                "length {len}"
            );
        }
    }

    #[test]
    fn rejects_unknown_algorithm() {
        for id in [0u8, 2, 0xFF] {
            let mut bytes = sample().to_bytes();
            bytes[10] = id;
            assert!(matches!(
                Header::parse(&bytes),
                Err(VaultError::UnsupportedAlgorithm { id: found }) if found == id
            ));
        }
    }

    #[test]
    fn rejects_flags_slot_count_and_reserved() {
        for index in [11usize, 12, 13, 14, 15] {
            let mut bytes = sample().to_bytes();
            bytes[index] ^= 0x01;
            assert!(
                matches!(Header::parse(&bytes), Err(VaultError::Corrupted { .. })),
                "byte {index}"
            );
        }
    }
}
