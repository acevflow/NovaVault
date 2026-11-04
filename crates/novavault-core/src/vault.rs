//! An unlocked vault: create, open, read, edit and save a vault file.
//!
//! [`Vault`] ties the format pieces together. It holds the vault key for as
//! long as it lives (the key is zeroized when the `Vault` is dropped, which
//! is how a vault is locked) and keeps items encrypted in memory: an item
//! is decrypted only when it is read, and only for the caller that asked.
//!
//! Edits ([`Vault::insert`], [`Vault::update`], [`Vault::remove`]) change
//! the in-memory vault; [`Vault::save`] writes it to disk atomically.
//!
//! Opening follows the reader rules of the format specification: the file
//! structure (magic, version, header, key slots, record framing) is checked
//! before any key derivation, then the key slot is unwrapped, then the
//! whole-file seal is verified before any record can be read.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use secrecy::{ExposeSecret, SecretSlice};

use crate::aead;
use crate::atomic::write_atomic;
use crate::error::{CryptoError, Result, VaultError, VaultResult};
use crate::format::{self, MAX_FILE_LEN, RecordEnvelope};
use crate::header::Header;
use crate::item::{self, Item, ItemContent, ItemId, ItemSummary};
use crate::kdf::KdfParams;
use crate::keys::SecretKey;
use crate::recovery::RecoveryCode;
use crate::slots::KeySlots;

/// An unlocked vault.
///
/// Dropping a `Vault` locks it: the vault key is wiped from memory. Unsaved
/// edits are lost.
pub struct Vault {
    path: PathBuf,
    header: Header,
    slots: KeySlots,
    key: SecretKey,
    /// Encrypted records, sorted by identifier as in the file.
    records: Vec<RecordEnvelope>,
}

impl Vault {
    /// Creates a new, empty vault file at `path`, protected by the master
    /// `password` and a newly generated recovery code.
    ///
    /// The vault is written to disk before this returns. The recovery code
    /// must be shown to the user once; the vault does not store it.
    ///
    /// # Errors
    ///
    /// - [`VaultError::AlreadyExists`] if anything exists at `path`: an
    ///   existing vault is never overwritten.
    /// - [`VaultError::Io`] if the file cannot be written.
    /// - [`VaultError::Crypto`] for key derivation or randomness failures.
    pub fn create(
        path: impl AsRef<Path>,
        password: &SecretSlice<u8>,
        params: KdfParams,
    ) -> VaultResult<(Self, RecoveryCode)> {
        let path = path.as_ref().to_path_buf();
        // `symlink_metadata` also sees dangling symbolic links, which
        // `exists` would report as absent.
        if fs::symlink_metadata(&path).is_ok() {
            return Err(VaultError::AlreadyExists);
        }
        let header = Header::generate()?;
        let (slots, key, code) = KeySlots::create(password, params, &header.to_bytes())?;
        let vault = Self {
            path,
            header,
            slots,
            key,
            records: Vec::new(),
        };
        vault.save()?;
        Ok((vault, code))
    }

    /// Opens and unlocks the vault at `path` with the master password.
    ///
    /// # Errors
    ///
    /// - [`VaultError::Io`] if the file cannot be read.
    /// - [`VaultError::NotAVault`], [`VaultError::UnsupportedVersion`],
    ///   [`VaultError::UnsupportedAlgorithm`] or [`VaultError::Corrupted`]
    ///   if the file is not a valid vault of a supported version. These are
    ///   reported before the (slow) key derivation runs.
    /// - [`VaultError::InvalidCredentials`] if the password is wrong or the
    ///   password slot is corrupted.
    /// - [`VaultError::Corrupted`] if the file fails its integrity check
    ///   after unlocking.
    pub fn open_with_password(
        path: impl AsRef<Path>,
        password: &SecretSlice<u8>,
    ) -> VaultResult<Self> {
        Self::open(path.as_ref(), |slots, header| {
            slots.unlock_with_password(password, header)
        })
    }

    /// Opens and unlocks the vault at `path` with the recovery code.
    ///
    /// # Errors
    ///
    /// Same as [`Vault::open_with_password`], for the recovery slot.
    pub fn open_with_recovery_code(
        path: impl AsRef<Path>,
        code: &RecoveryCode,
    ) -> VaultResult<Self> {
        Self::open(path.as_ref(), |slots, header| {
            slots.unlock_with_recovery_code(code, header)
        })
    }

    fn open(
        path: &Path,
        unlock: impl FnOnce(&KeySlots, &[u8]) -> Result<SecretKey>,
    ) -> VaultResult<Self> {
        let file = read_bounded(path)?;
        let parsed = format::parse(&file)?;
        let key =
            unlock(&parsed.slots, &parsed.header.to_bytes()).map_err(|error| match error {
                CryptoError::Decryption => VaultError::InvalidCredentials,
                other => other.into(),
            })?;
        format::verify_seal(&key, &file, parsed.body_len)?;
        Ok(Self {
            path: path.to_path_buf(),
            header: parsed.header,
            slots: parsed.slots,
            key,
            records: parsed.records,
        })
    }

    /// The path the vault is saved to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Number of items in the vault.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the vault has no items.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Lists every item's identifier, type, title and timestamps, in
    /// identifier order.
    ///
    /// Each item is decrypted to read its title; secret fields are dropped
    /// (and wiped) immediately.
    ///
    /// # Errors
    ///
    /// [`VaultError::Corrupted`] if a record fails to decrypt or decode.
    pub fn list(&self) -> VaultResult<Vec<ItemSummary>> {
        self.records
            .iter()
            .map(|record| self.decrypt(record).map(ItemSummary::from))
            .collect()
    }

    /// Decrypts and returns the item `id`.
    ///
    /// # Errors
    ///
    /// - [`VaultError::ItemNotFound`] if there is no such item.
    /// - [`VaultError::Corrupted`] if its record fails to decrypt or decode.
    pub fn get(&self, id: ItemId) -> VaultResult<Item> {
        self.decrypt(&self.records[self.position(id)?])
    }

    /// Adds a new item and returns its identifier.
    ///
    /// Its creation and update times are set to now.
    ///
    /// # Errors
    ///
    /// - [`VaultError::InvalidItem`] if the item or the vault would be too
    ///   large.
    /// - [`VaultError::Crypto`] for randomness or encryption failures.
    pub fn insert(&mut self, content: ItemContent) -> VaultResult<ItemId> {
        if self.records.len() >= format::MAX_RECORDS as usize {
            return Err(VaultError::InvalidItem {
                reason: "vault has too many items",
            });
        }
        let (id, index) = loop {
            let id = ItemId::generate()?;
            // A collision of 128-bit random identifiers is practically
            // impossible, but retrying costs nothing.
            if let Err(index) = self.search(id) {
                break (id, index);
            }
        };
        let now = now_ms();
        let record = self.encrypt(id, now, now, &content)?;
        self.records.insert(index, record);
        Ok(id)
    }

    /// Replaces the content of item `id`, keeping its identifier and
    /// creation time and setting its update time to now.
    ///
    /// The content may be of a different [`crate::ItemKind`] than before.
    ///
    /// # Errors
    ///
    /// - [`VaultError::ItemNotFound`] if there is no such item.
    /// - [`VaultError::Corrupted`] if the existing record cannot be read.
    /// - [`VaultError::InvalidItem`] if the item is too large.
    pub fn update(&mut self, id: ItemId, content: ItemContent) -> VaultResult<()> {
        let index = self.position(id)?;
        let existing = self.decrypt(&self.records[index])?;
        // Never move the update time backwards, even if the clock did.
        let updated = now_ms().max(existing.updated_at_ms());
        let record = self.encrypt(id, existing.created_at_ms(), updated, &content)?;
        self.records[index] = record;
        Ok(())
    }

    /// Removes item `id`.
    ///
    /// # Errors
    ///
    /// [`VaultError::ItemNotFound`] if there is no such item.
    pub fn remove(&mut self, id: ItemId) -> VaultResult<()> {
        let index = self.position(id)?;
        self.records.remove(index);
        Ok(())
    }

    /// Writes the vault to its path, atomically replacing the previous file.
    ///
    /// # Errors
    ///
    /// - [`VaultError::Io`] if the file cannot be written; the previous file
    ///   is then left intact.
    /// - [`VaultError::InvalidItem`] if the vault exceeds the format's limits.
    pub fn save(&self) -> VaultResult<()> {
        let mut file = format::encode_body(&self.header, &self.slots, &self.records)?;
        let seal = format::seal(&self.key, &file)?;
        file.extend_from_slice(&seal);
        write_atomic(&self.path, &file)?;
        Ok(())
    }

    /// Changes the master password and saves the vault.
    ///
    /// Only the password key slot is rewritten (fresh salt and nonce, the
    /// given Argon2id `params`); the vault key and every record stay the
    /// same, and the recovery code keeps working. The vault may have been
    /// unlocked with the recovery code, which is the forgotten-password
    /// flow.
    ///
    /// Unsaved item edits are written too, because the whole file is saved.
    ///
    /// # Errors
    ///
    /// - [`VaultError::Crypto`] for key derivation or randomness failures.
    /// - Any error of [`Vault::save`].
    ///
    /// On error the in-memory vault keeps the previous password slot, so it
    /// still matches the file on disk.
    pub fn change_password(
        &mut self,
        new_password: &SecretSlice<u8>,
        params: KdfParams,
    ) -> VaultResult<()> {
        let mut slots = self.slots.clone();
        slots.change_password(&self.key, new_password, params, &self.header.to_bytes())?;
        self.replace_slots_and_save(slots)
    }

    /// Replaces the recovery code with a newly generated one, saves the
    /// vault and returns the new code, which must be shown to the user once.
    ///
    /// Only the recovery key slot is rewritten, so the previous code stops
    /// working as soon as this returns. The master password is unaffected.
    ///
    /// # Errors
    ///
    /// Same as [`Vault::change_password`]; on error the previous code still
    /// opens the vault.
    pub fn regenerate_recovery_code(&mut self, params: KdfParams) -> VaultResult<RecoveryCode> {
        let mut slots = self.slots.clone();
        let code = slots.regenerate_recovery_code(&self.key, params, &self.header.to_bytes())?;
        self.replace_slots_and_save(slots)?;
        Ok(code)
    }

    /// Installs `slots` and saves; restores the previous slots if the save
    /// fails, so memory never claims a credential the file does not have.
    fn replace_slots_and_save(&mut self, slots: KeySlots) -> VaultResult<()> {
        let previous = core::mem::replace(&mut self.slots, slots);
        if let Err(error) = self.save() {
            self.slots = previous;
            return Err(error);
        }
        Ok(())
    }

    fn search(&self, id: ItemId) -> core::result::Result<usize, usize> {
        self.records.binary_search_by_key(&id, |record| record.id)
    }

    fn position(&self, id: ItemId) -> VaultResult<usize> {
        self.search(id).map_err(|_| VaultError::ItemNotFound)
    }

    fn encrypt(
        &self,
        id: ItemId,
        created_at_ms: u64,
        updated_at_ms: u64,
        content: &ItemContent,
    ) -> VaultResult<RecordEnvelope> {
        let kind = content.kind();
        let payload = item::encode_payload(created_at_ms, updated_at_ms, content)?;
        let ad = format::record_ad(&self.header, &id, kind);
        let sealed = aead::encrypt(&self.key, &payload, &ad)?;
        Ok(RecordEnvelope { id, kind, sealed })
    }

    fn decrypt(&self, record: &RecordEnvelope) -> VaultResult<Item> {
        let ad = format::record_ad(&self.header, &record.id, record.kind);
        // The seal was verified on open, so a failure here means the
        // in-memory record was damaged or the writer had a bug.
        let payload =
            aead::decrypt(&self.key, &record.sealed, &ad).map_err(|_| VaultError::Corrupted {
                reason: "record failed authentication",
            })?;
        let (created, updated, content) =
            item::decode_payload(record.kind, payload.expose_secret())?;
        Ok(Item::new(record.id, created, updated, content))
    }
}

impl core::fmt::Debug for Vault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Vault")
            .field("path", &self.path)
            .field("items", &self.records.len())
            .finish_non_exhaustive()
    }
}

/// Reads at most one byte more than [`MAX_FILE_LEN`], so a huge file is
/// rejected (after its header is checked) without being loaded.
fn read_bounded(path: &Path) -> VaultResult<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_FILE_LEN as u64 + 1)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Milliseconds since the Unix epoch, or 0 if the clock is set before it.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use secrecy::SecretString;

    use super::*;
    use crate::header::HEADER_LEN;
    use crate::item::{ItemKind, NoteItem, PasswordItem, SecretItem};
    use crate::slots::SLOT_LEN;
    use crate::test_util::TempDir;

    /// Lowest accepted cost, so debug-build tests stay fast.
    fn fast_params() -> KdfParams {
        KdfParams::new(KdfParams::MIN_MEMORY_KIB, KdfParams::MIN_ITERATIONS, 1).unwrap()
    }

    fn pw(text: &str) -> SecretSlice<u8> {
        SecretSlice::from(text.as_bytes().to_vec())
    }

    fn s(text: &str) -> SecretString {
        SecretString::from(text)
    }

    const PASSWORD: &str = "correct horse battery staple";

    fn login() -> ItemContent {
        ItemContent::Password(PasswordItem {
            title: "Bank".into(),
            username: "alice".into(),
            password: s("hunter2"),
            url: "https://bank.example".into(),
            notes: s(""),
        })
    }

    fn note(body: &str) -> ItemContent {
        ItemContent::Note(NoteItem {
            title: "Diary".into(),
            body: s(body),
        })
    }

    fn token() -> ItemContent {
        ItemContent::Secret(SecretItem {
            title: "API".into(),
            value: s("sk-123"),
            notes: s("rotate yearly"),
        })
    }

    fn new_vault(dir: &TempDir) -> (Vault, RecoveryCode) {
        Vault::create(dir.path().join("test.nvault"), &pw(PASSWORD), fast_params()).unwrap()
    }

    fn open(path: &Path) -> VaultResult<Vault> {
        Vault::open_with_password(path, &pw(PASSWORD))
    }

    fn note_body(item: &Item) -> String {
        match item.content() {
            ItemContent::Note(note) => note.body.expose_secret().to_owned(),
            other => panic!("expected a note, got {other:?}"),
        }
    }

    #[test]
    fn round_trip_with_password_and_recovery_code() {
        let dir = TempDir::new();
        let (mut vault, code) = new_vault(&dir);
        assert!(vault.is_empty());
        let login_id = vault.insert(login()).unwrap();
        let note_id = vault.insert(note("dear diary")).unwrap();
        let token_id = vault.insert(token()).unwrap();
        vault.save().unwrap();

        let reopened = open(vault.path()).unwrap();
        assert_eq!(reopened.len(), 3);
        let mut titles: Vec<(ItemKind, String)> = reopened
            .list()
            .unwrap()
            .iter()
            .map(|summary| (summary.kind, summary.title.clone()))
            .collect();
        titles.sort_by(|a, b| a.1.cmp(&b.1));
        assert_eq!(
            titles,
            [
                (ItemKind::Secret, "API".to_owned()),
                (ItemKind::Password, "Bank".to_owned()),
                (ItemKind::Note, "Diary".to_owned()),
            ]
        );

        let item = reopened.get(login_id).unwrap();
        assert_eq!(item.id(), login_id);
        assert_eq!(item.created_at_ms(), item.updated_at_ms());
        match item.content() {
            ItemContent::Password(login) => {
                assert_eq!(login.username, "alice");
                assert_eq!(login.password.expose_secret(), "hunter2");
                assert_eq!(login.url, "https://bank.example");
            }
            other => panic!("expected a login, got {other:?}"),
        }
        match reopened.get(token_id).unwrap().content() {
            ItemContent::Secret(token) => {
                assert_eq!(token.value.expose_secret(), "sk-123");
                assert_eq!(token.notes.expose_secret(), "rotate yearly");
            }
            other => panic!("expected a secret, got {other:?}"),
        }

        let by_code = Vault::open_with_recovery_code(vault.path(), &code).unwrap();
        assert_eq!(note_body(&by_code.get(note_id).unwrap()), "dear diary");
    }

    #[test]
    fn edits_persist_after_save() {
        let dir = TempDir::new();
        let (mut vault, _) = new_vault(&dir);
        let keep = vault.insert(note("first")).unwrap();
        let drop_me = vault.insert(token()).unwrap();
        let created = vault.get(keep).unwrap().created_at_ms();

        vault.update(keep, note("second")).unwrap();
        vault.remove(drop_me).unwrap();
        // Unsaved edits are not on disk yet.
        assert_eq!(open(vault.path()).unwrap().len(), 0);
        vault.save().unwrap();

        let reopened = open(vault.path()).unwrap();
        assert_eq!(reopened.len(), 1);
        let item = reopened.get(keep).unwrap();
        assert_eq!(note_body(&item), "second");
        assert_eq!(item.created_at_ms(), created);
        assert!(item.updated_at_ms() >= created);
        assert!(matches!(
            reopened.get(drop_me),
            Err(VaultError::ItemNotFound)
        ));
    }

    #[test]
    fn unknown_items_are_reported() {
        let dir = TempDir::new();
        let (mut vault, _) = new_vault(&dir);
        let missing = ItemId::from_bytes([9; 16]);
        assert!(matches!(vault.get(missing), Err(VaultError::ItemNotFound)));
        assert!(matches!(
            vault.update(missing, note("x")),
            Err(VaultError::ItemNotFound)
        ));
        assert!(matches!(
            vault.remove(missing),
            Err(VaultError::ItemNotFound)
        ));
    }

    #[test]
    fn create_never_overwrites() {
        let dir = TempDir::new();
        let path = dir.path().join("existing.nvault");
        fs::write(&path, b"precious").unwrap();
        assert!(matches!(
            Vault::create(&path, &pw(PASSWORD), fast_params()),
            Err(VaultError::AlreadyExists)
        ));
        assert_eq!(fs::read(&path).unwrap(), b"precious");
    }

    #[test]
    fn wrong_secrets_are_rejected_generically() {
        let dir = TempDir::new();
        let (vault, _) = new_vault(&dir);
        assert!(matches!(
            Vault::open_with_password(vault.path(), &pw("wrong")),
            Err(VaultError::InvalidCredentials)
        ));
        let other_code = RecoveryCode::generate().unwrap();
        assert!(matches!(
            Vault::open_with_recovery_code(vault.path(), &other_code),
            Err(VaultError::InvalidCredentials)
        ));
    }

    #[test]
    fn missing_file_is_an_io_error() {
        let dir = TempDir::new();
        assert!(matches!(
            open(&dir.path().join("absent.nvault")),
            Err(VaultError::Io(_))
        ));
    }

    /// Writes `bytes` next to the vault and tries to open them.
    fn open_bytes(dir: &TempDir, bytes: &[u8]) -> VaultResult<Vault> {
        let path = dir.path().join("modified.nvault");
        fs::write(&path, bytes).unwrap();
        open(&path)
    }

    #[test]
    fn unsupported_versions_and_foreign_files_are_rejected() {
        let dir = TempDir::new();
        let (vault, _) = new_vault(&dir);
        let file = fs::read(vault.path()).unwrap();

        for version in [1u16, 2, u16::MAX] {
            let mut newer = file.clone();
            newer[8..10].copy_from_slice(&version.to_le_bytes());
            assert!(matches!(
                open_bytes(&dir, &newer),
                Err(VaultError::UnsupportedVersion { found }) if found == version
            ));
        }
        for foreign in [&b""[..], b"hello", b"PK\x03\x04", &[0u8; 512]] {
            assert!(matches!(
                open_bytes(&dir, foreign),
                Err(VaultError::NotAVault)
            ));
        }
        let mut other_cipher = file.clone();
        other_cipher[10] = 2;
        assert!(matches!(
            open_bytes(&dir, &other_cipher),
            Err(VaultError::UnsupportedAlgorithm { id: 2 })
        ));
        let mut flagged = file.clone();
        flagged[11] = 1;
        assert!(matches!(
            open_bytes(&dir, &flagged),
            Err(VaultError::Corrupted { .. })
        ));
    }

    #[test]
    fn corruption_is_always_detected() {
        let dir = TempDir::new();
        let (mut vault, _) = new_vault(&dir);
        vault.insert(note("secret")).unwrap();
        vault.save().unwrap();
        let file = fs::read(vault.path()).unwrap();

        let slots = HEADER_LEN;
        let record = slots + 2 * SLOT_LEN + 4;
        let sealed = record + 16 + 1 + 4;
        let seal = file.len() - format::SEAL_LEN;

        // Truncation is caught by the structural checks.
        for len in [
            0,
            9,
            HEADER_LEN - 1,
            HEADER_LEN,
            record,
            seal,
            file.len() - 1,
        ] {
            assert!(open_bytes(&dir, &file[..len]).is_err(), "length {len}");
        }

        // One flipped byte in every field of every section. Byte flips are
        // sampled per field because each open runs Argon2id.
        let password_slot = [
            ("password slot type", slots),
            ("password slot salt", slots + 1),
            ("password slot memory", slots + 17),
            ("password slot nonce", slots + 29),
            ("password slot tag", slots + SLOT_LEN - 1),
        ];
        let rest = [
            ("vault id", 16),
            ("recovery slot salt", slots + SLOT_LEN + 1),
            ("recovery slot tag", slots + 2 * SLOT_LEN - 1),
            ("record count", record - 4),
            ("record id", record),
            ("record type", record + 16),
            ("record length", record + 17),
            ("record nonce", sealed),
            ("record ciphertext", sealed + 24),
            ("record tag", seal - 1),
            ("seal nonce", seal),
            ("seal tag", file.len() - 1),
        ];
        for (field, index) in password_slot.into_iter().chain(rest) {
            let mut tampered = file.clone();
            tampered[index] ^= 0x01;
            let result = open_bytes(&dir, &tampered);
            assert!(
                matches!(
                    result,
                    Err(VaultError::Corrupted { .. } | VaultError::InvalidCredentials)
                ),
                "{field} (byte {index}): {result:?}"
            );
        }
    }

    #[test]
    fn replaying_an_old_record_is_detected() {
        let dir = TempDir::new();
        let (mut vault, _) = new_vault(&dir);
        let id = vault.insert(note("old balance: 10")).unwrap();
        vault.save().unwrap();
        let old = fs::read(vault.path()).unwrap();
        vault.update(id, note("new balance: 99")).unwrap();
        vault.save().unwrap();
        let new = fs::read(vault.path()).unwrap();
        assert_eq!(old.len(), new.len());

        // Splice the old, validly encrypted record into the new file.
        let record = HEADER_LEN + 2 * SLOT_LEN + 4;
        let seal = new.len() - format::SEAL_LEN;
        let mut spliced = new.clone();
        spliced[record..seal].copy_from_slice(&old[record..seal]);
        assert!(matches!(
            open_bytes(&dir, &spliced),
            Err(VaultError::Corrupted { .. })
        ));
    }

    #[test]
    fn changing_the_password_keeps_data_and_recovery_code() {
        let dir = TempDir::new();
        let (mut vault, code) = new_vault(&dir);
        let id = vault.insert(note("kept")).unwrap();
        vault
            .change_password(&pw("new password"), fast_params())
            .unwrap();

        assert!(matches!(
            open(vault.path()),
            Err(VaultError::InvalidCredentials)
        ));
        let reopened = Vault::open_with_password(vault.path(), &pw("new password")).unwrap();
        // Unsaved edits are saved together with the new slot.
        assert_eq!(note_body(&reopened.get(id).unwrap()), "kept");
        assert!(Vault::open_with_recovery_code(vault.path(), &code).is_ok());
    }

    #[test]
    fn regenerating_the_recovery_code_invalidates_the_old_one() {
        let dir = TempDir::new();
        let (mut vault, old_code) = new_vault(&dir);
        let id = vault.insert(note("kept")).unwrap();
        let new_code = vault.regenerate_recovery_code(fast_params()).unwrap();

        assert!(matches!(
            Vault::open_with_recovery_code(vault.path(), &old_code),
            Err(VaultError::InvalidCredentials)
        ));
        let reopened = Vault::open_with_recovery_code(vault.path(), &new_code).unwrap();
        assert_eq!(note_body(&reopened.get(id).unwrap()), "kept");
        assert!(open(vault.path()).is_ok());
    }

    #[test]
    fn failed_save_keeps_previous_slots() {
        let dir = TempDir::new();
        let (mut vault, code) = new_vault(&dir);
        // Make the save fail: the vault's directory no longer exists.
        let moved = dir.path().join("moved.nvault");
        fs::rename(vault.path(), &moved).unwrap();
        vault.path = dir.path().join("missing").join("test.nvault");

        assert!(vault.change_password(&pw("new"), fast_params()).is_err());
        assert!(vault.regenerate_recovery_code(fast_params()).is_err());

        // Saving to the original location works with the original secrets.
        vault.path = moved;
        vault.save().unwrap();
        assert!(open(vault.path()).is_ok());
        assert!(Vault::open_with_recovery_code(vault.path(), &code).is_ok());
    }

    #[test]
    fn debug_hides_contents() {
        let dir = TempDir::new();
        let (mut vault, _) = new_vault(&dir);
        vault.insert(login()).unwrap();
        let printed = format!("{vault:?}");
        assert!(printed.contains("items: 1"), "{printed}");
        assert!(!printed.contains("hunter2") && !printed.contains("Bank"));
    }
}
