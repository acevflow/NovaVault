//! An unlocked vault: create, open, read, edit and save a vault file.
//!
//! [`Vault`] ties the format pieces together. It holds the vault key for as
//! long as it lives (the key is zeroized when the `Vault` is dropped, which
//! is how a vault is locked) and keeps items encrypted in memory: an item
//! is decrypted only when it is read, and only for the caller that asked.
//!
//! Edits ([`Vault::insert`], [`Vault::update`], [`Vault::remove`] and the
//! attachment operations) change the in-memory vault; [`Vault::save`]
//! writes it to disk atomically.
//!
//! Opening follows the reader rules of the format specification: the file
//! structure (magic, version, header, key slots, record framing) is checked
//! before any key derivation, then the key slot is unwrapped, then the
//! whole-file seal is verified before any record can be read. A vault in an
//! older format version is then migrated (see [`Vault::open_with_password`]).
//!
//! # Attachments
//!
//! Attachment contents are too large for the vault file, which is read and
//! rewritten whole, so each one is encrypted into its own blob file (see
//! [`crate::attachment`]) in the directory [`Vault::attachments_dir`] next
//! to the vault file. The item's encrypted payload records each blob's
//! identifier, name and size. Blobs are never rewritten: adding a file
//! creates a new blob, and the blobs of removed attachments are deleted
//! only after the vault file no longer refers to them has been saved.
//! Blobs that no saved item refers to (left by a crash or a failed save)
//! are deleted when the vault is opened.

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use secrecy::{ExposeSecret, SecretSlice};

use crate::aead;
use crate::atomic::{AtomicFile, write_atomic};
use crate::attachment::{self, BLOB_EXTENSION, BlobHeader};
use crate::error::{CryptoError, Result, VaultError, VaultResult};
use crate::format::{self, MAX_FILE_LEN, RecordEnvelope};
use crate::header::{FORMAT_VERSION, Header};
use crate::item::{
    self, Attachment, AttachmentId, Item, ItemContent, ItemId, ItemSummary,
    MAX_ATTACHMENTS_PER_ITEM, Payload, validate_attachment_name,
};
use crate::kdf::KdfParams;
use crate::keys::SecretKey;
use crate::recovery::RecoveryCode;
use crate::slots::KeySlots;

/// Suffix appended to the vault file name to name its attachments
/// directory: `vault.nvault` keeps its blobs in `vault.nvault.attachments`.
pub const ATTACHMENTS_DIR_SUFFIX: &str = ".attachments";

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
    /// Blobs of removed attachments and items, deleted once a save no
    /// longer refers to them.
    pending_deletions: Vec<AttachmentId>,
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
        let (slots, key, code) = KeySlots::create(password, params, &header.slot_ad())?;
        let mut vault = Self {
            path,
            header,
            slots,
            key,
            records: Vec::new(),
            pending_deletions: Vec::new(),
        };
        vault.save()?;
        Ok((vault, code))
    }

    /// Opens and unlocks the vault at `path` with the master password.
    ///
    /// A vault in an older format version is migrated to the current one
    /// after it is unlocked: every record is re-encrypted for the new
    /// version and the file is rewritten atomically. The previous file is
    /// kept as `<file name>.v<version>.bak` until the new one has been
    /// written and verified. The key slots are not changed, so both the
    /// password and the recovery code keep working.
    ///
    /// Attachment blobs that no item refers to are deleted.
    ///
    /// # Errors
    ///
    /// - [`VaultError::Io`] if the file cannot be read (or, when migrating,
    ///   written).
    /// - [`VaultError::NotAVault`], [`VaultError::UnsupportedVersion`],
    ///   [`VaultError::UnsupportedAlgorithm`] or [`VaultError::Corrupted`]
    ///   if the file is not a valid vault of a supported version. These are
    ///   reported before the (slow) key derivation runs.
    /// - [`VaultError::InvalidCredentials`] if the password is wrong or the
    ///   password slot is corrupted.
    /// - [`VaultError::Corrupted`] if the file fails its integrity check
    ///   after unlocking, or a record cannot be read.
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
        let key = unlock(&parsed.slots, &parsed.header.slot_ad()).map_err(|error| match error {
            CryptoError::Decryption => VaultError::InvalidCredentials,
            other => other.into(),
        })?;
        format::verify_seal(&key, &file, parsed.body_len)?;
        let mut vault = Self {
            path: path.to_path_buf(),
            header: parsed.header,
            slots: parsed.slots,
            key,
            records: parsed.records,
            pending_deletions: Vec::new(),
        };
        if vault.header.version() < FORMAT_VERSION {
            vault.migrate(&file)?;
        }
        let referenced = vault.referenced_attachments()?;
        vault.prune_unreferenced_blobs(&referenced);
        Ok(vault)
    }

    /// The path the vault is saved to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The directory holding the vault's attachment blobs: the vault file's
    /// path with [`ATTACHMENTS_DIR_SUFFIX`] appended. It is created when the
    /// first file is attached. Back it up together with the vault file.
    pub fn attachments_dir(&self) -> PathBuf {
        let mut name = self
            .path
            .file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_default();
        name.push(ATTACHMENTS_DIR_SUFFIX);
        self.path.with_file_name(name)
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
        let record = self.encrypt(id, now, now, &content, &[])?;
        self.records.insert(index, record);
        Ok(id)
    }

    /// Replaces the content of item `id`, keeping its identifier, creation
    /// time and attachments and setting its update time to now.
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
        let created = existing.created_at_ms();
        let (_, attachments) = existing.into_parts();
        let record = self.encrypt(id, created, updated, &content, &attachments)?;
        self.records[index] = record;
        Ok(())
    }

    /// Removes item `id`. Its attachment blobs are deleted by the next
    /// successful [`Vault::save`].
    ///
    /// # Errors
    ///
    /// - [`VaultError::ItemNotFound`] if there is no such item.
    /// - [`VaultError::Corrupted`] if its record cannot be read.
    pub fn remove(&mut self, id: ItemId) -> VaultResult<()> {
        let index = self.position(id)?;
        let item = self.decrypt(&self.records[index])?;
        self.pending_deletions
            .extend(item.attachments().iter().map(Attachment::id));
        self.records.remove(index);
        Ok(())
    }

    /// Returns the metadata of attachment `attachment` of item `item`.
    ///
    /// # Errors
    ///
    /// - [`VaultError::ItemNotFound`] or [`VaultError::AttachmentNotFound`].
    /// - [`VaultError::Corrupted`] if the item's record cannot be read.
    pub fn attachment(&self, item: ItemId, attachment: AttachmentId) -> VaultResult<Attachment> {
        self.get(item)?
            .attachments()
            .iter()
            .find(|candidate| candidate.id() == attachment)
            .cloned()
            .ok_or(VaultError::AttachmentNotFound)
    }

    /// Encrypts everything `source` yields as a new attachment of item
    /// `item`, named `name`, and returns its metadata.
    ///
    /// The data is streamed: only one chunk is held in memory at a time.
    /// The blob is complete on disk before the item refers to it; the item
    /// change itself is in memory until the next [`Vault::save`] (a blob
    /// that is never saved is deleted the next time the vault is opened).
    /// The item's update time is set to now.
    ///
    /// `progress` is called after each chunk with the number of bytes
    /// encrypted so far; returning [`ControlFlow::Break`] stops with
    /// [`VaultError::Cancelled`] and leaves nothing behind.
    ///
    /// # Errors
    ///
    /// - [`VaultError::ItemNotFound`] if there is no such item.
    /// - [`VaultError::InvalidItem`] for an invalid name (see
    ///   [`validate_attachment_name`]) or if the item already has
    ///   [`MAX_ATTACHMENTS_PER_ITEM`] attachments.
    /// - [`VaultError::Io`] if reading `source` or writing the blob fails.
    /// - [`VaultError::Cancelled`], or [`VaultError::Crypto`] for
    ///   encryption or randomness failures.
    pub fn add_attachment(
        &mut self,
        item: ItemId,
        name: &str,
        source: &mut impl Read,
        progress: impl FnMut(u64) -> ControlFlow<()>,
    ) -> VaultResult<Attachment> {
        validate_attachment_name(name)?;
        let index = self.position(item)?;
        let existing = self.decrypt(&self.records[index])?;
        if existing.attachments().len() >= MAX_ATTACHMENTS_PER_ITEM {
            return Err(VaultError::InvalidItem {
                reason: "item has too many attachments",
            });
        }

        create_private_dir(&self.attachments_dir())?;
        let (id, path) = loop {
            let id = AttachmentId::generate()?;
            let path = self.blob_path(id);
            // As for items, a collision is practically impossible; checking
            // keeps a blob from ever being replaced.
            if fs::symlink_metadata(&path).is_err() {
                break (id, path);
            }
        };
        let header = BlobHeader {
            vault_id: self.header.vault_id(),
            attachment_id: id,
        };
        // An error or cancellation drops `blob`, which deletes the partial
        // temporary file.
        let mut blob = AtomicFile::create(&path)?;
        let size = attachment::encrypt_stream(&self.key, header, source, &mut blob, progress)?;
        blob.commit()?;

        let added = now_ms();
        let created = existing.created_at_ms();
        let updated = added.max(existing.updated_at_ms());
        let (content, mut attachments) = existing.into_parts();
        let new = Attachment::new(id, name.to_owned(), size, added);
        attachments.push(new.clone());
        match self.encrypt(item, created, updated, &content, &attachments) {
            Ok(record) => {
                self.records[index] = record;
                Ok(new)
            }
            Err(error) => {
                // Nothing refers to the blob; deleting it now saves the
                // next open from doing it.
                let _ = fs::remove_file(&path);
                Err(error)
            }
        }
    }

    /// Decrypts attachment `attachment` of item `item` into a new file at
    /// `destination`, replacing any file there.
    ///
    /// The plaintext is written to a temporary file next to `destination`
    /// and moved into place only after every chunk has been authenticated
    /// and the size matches the item's record. On any error, including
    /// cancellation, the temporary file is deleted, so a damaged blob never
    /// leaves partial plaintext behind.
    ///
    /// `progress` is called after each chunk with the number of bytes
    /// written so far; returning [`ControlFlow::Break`] stops with
    /// [`VaultError::Cancelled`].
    ///
    /// # Errors
    ///
    /// - [`VaultError::ItemNotFound`] or [`VaultError::AttachmentNotFound`].
    /// - [`VaultError::InvalidItem`] if `destination` is the vault file or
    ///   inside its attachments directory.
    /// - [`VaultError::Corrupted`] if the blob is missing, damaged or does
    ///   not match the record.
    /// - [`VaultError::Io`] if writing `destination` fails.
    /// - [`VaultError::Cancelled`].
    pub fn export_attachment(
        &self,
        item: ItemId,
        attachment: AttachmentId,
        destination: &Path,
        progress: impl FnMut(u64) -> ControlFlow<()>,
    ) -> VaultResult<()> {
        let metadata = self.attachment(item, attachment)?;
        if self.is_vault_storage(destination) {
            return Err(VaultError::InvalidItem {
                reason: "cannot export over the vault's own files",
            });
        }
        let blob = File::open(self.blob_path(attachment)).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                VaultError::Corrupted {
                    reason: "attachment data is missing",
                }
            } else {
                error.into()
            }
        })?;
        let header = BlobHeader {
            vault_id: self.header.vault_id(),
            attachment_id: attachment,
        };
        let mut output = AtomicFile::create(destination)?;
        attachment::decrypt_stream(
            &self.key,
            header,
            metadata.size(),
            blob,
            &mut output,
            progress,
        )?;
        output.commit()?;
        Ok(())
    }

    /// Removes attachment `attachment` from item `item` and sets the item's
    /// update time to now. The blob is deleted by the next successful
    /// [`Vault::save`].
    ///
    /// # Errors
    ///
    /// - [`VaultError::ItemNotFound`] or [`VaultError::AttachmentNotFound`].
    /// - [`VaultError::Corrupted`] if the item's record cannot be read.
    pub fn remove_attachment(&mut self, item: ItemId, attachment: AttachmentId) -> VaultResult<()> {
        let index = self.position(item)?;
        let existing = self.decrypt(&self.records[index])?;
        let created = existing.created_at_ms();
        let updated = now_ms().max(existing.updated_at_ms());
        let (content, mut attachments) = existing.into_parts();
        let position = attachments
            .iter()
            .position(|candidate| candidate.id() == attachment)
            .ok_or(VaultError::AttachmentNotFound)?;
        attachments.remove(position);
        self.records[index] = self.encrypt(item, created, updated, &content, &attachments)?;
        self.pending_deletions.push(attachment);
        Ok(())
    }

    /// Writes the vault to its path, atomically replacing the previous
    /// file, then deletes the blobs of removed attachments.
    ///
    /// # Errors
    ///
    /// - [`VaultError::Io`] if the file cannot be written; the previous file
    ///   is then left intact, and no blob is deleted.
    /// - [`VaultError::InvalidItem`] if the vault exceeds the format's limits.
    pub fn save(&mut self) -> VaultResult<()> {
        let mut file = format::encode_body(&self.header, &self.slots, &self.records)?;
        let seal = format::seal(&self.key, &file)?;
        file.extend_from_slice(&seal);
        write_atomic(&self.path, &file)?;
        for id in core::mem::take(&mut self.pending_deletions) {
            // Best effort: the saved vault no longer refers to the blob, so
            // one that cannot be deleted now is deleted on the next open.
            let _ = fs::remove_file(self.blob_path(id));
        }
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
        slots.change_password(&self.key, new_password, params, &self.header.slot_ad())?;
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
        let code = slots.regenerate_recovery_code(&self.key, params, &self.header.slot_ad())?;
        self.replace_slots_and_save(slots)?;
        Ok(code)
    }

    /// Checks that `password` is the current master password, by
    /// unwrapping the in-memory password slot (one Argon2id derivation).
    ///
    /// Used to re-authenticate the user before sensitive changes such as a
    /// password change or recovery code regeneration, so an unlocked but
    /// unattended vault cannot be taken over. Nothing is changed or saved.
    ///
    /// # Errors
    ///
    /// - [`VaultError::InvalidCredentials`] if the password is wrong.
    /// - [`VaultError::Crypto`] for key derivation failures.
    pub fn verify_password(&self, password: &SecretSlice<u8>) -> VaultResult<()> {
        // The unwrapped key is dropped (and wiped) at once: a successful
        // AEAD unwrap is the proof; the key itself is already held.
        self.slots
            .unlock_with_password(password, &self.header.slot_ad())
            .map(drop)
            .map_err(|error| match error {
                CryptoError::Decryption => VaultError::InvalidCredentials,
                other => other.into(),
            })
    }

    /// Checks that `code` is the current recovery code, by unwrapping the
    /// in-memory recovery slot. The counterpart of
    /// [`Vault::verify_password`] for users who unlocked with the recovery
    /// code because they forgot the password.
    ///
    /// # Errors
    ///
    /// Same as [`Vault::verify_password`].
    pub fn verify_recovery_code(&self, code: &RecoveryCode) -> VaultResult<()> {
        self.slots
            .unlock_with_recovery_code(code, &self.header.slot_ad())
            .map(drop)
            .map_err(|error| match error {
                CryptoError::Decryption => VaultError::InvalidCredentials,
                other => other.into(),
            })
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

    /// Upgrades a vault read from `original` (an older format version) to
    /// [`FORMAT_VERSION`] and saves it.
    ///
    /// The key slots stay as they are: their associated data leaves out
    /// the version (see [`Header::slot_ad`]). Every record is re-encrypted,
    /// because the record and seal associated data include the version.
    fn migrate(&mut self, original: &[u8]) -> VaultResult<()> {
        let old = self.header;
        let new = old.with_version(FORMAT_VERSION);
        let mut records = Vec::with_capacity(self.records.len());
        for record in &self.records {
            let payload = self.decrypt_payload(&old, record)?;
            let encoded = item::encode_payload(
                new.version(),
                payload.created_at_ms,
                payload.updated_at_ms,
                &payload.content,
                &payload.attachments,
            )?;
            let ad = format::record_ad(&new, &record.id, record.kind);
            let sealed = aead::encrypt(&self.key, &encoded, &ad)?;
            records.push(RecordEnvelope {
                id: record.id,
                kind: record.kind,
                sealed,
            });
        }

        let backup = self.backup_path(old.version());
        write_atomic(&backup, original)?;
        self.header = new;
        self.records = records;
        if let Err(error) = self.save() {
            // The original file is intact (saves are atomic), so the
            // backup is not needed.
            let _ = fs::remove_file(&backup);
            return Err(error);
        }
        // Keep the backup unless the new file reads back as valid.
        let written = read_bounded(&self.path)?;
        let parsed = format::parse(&written)?;
        format::verify_seal(&self.key, &written, parsed.body_len)?;
        if parsed.header != new || parsed.records != self.records {
            return Err(VaultError::Corrupted {
                reason: "migrated vault did not read back",
            });
        }
        // Best effort: a leftover backup is an encrypted copy of the old
        // file, which the same secrets open.
        let _ = fs::remove_file(&backup);
        Ok(())
    }

    /// `<vault file name>.v<version>.bak`, next to the vault file.
    fn backup_path(&self, version: u16) -> PathBuf {
        let mut name = self
            .path
            .file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_default();
        name.push(format!(".v{version}.bak"));
        self.path.with_file_name(name)
    }

    /// The identifiers of every attachment of every item.
    ///
    /// # Errors
    ///
    /// [`VaultError::Corrupted`] if a record cannot be read or two
    /// attachments share an identifier.
    fn referenced_attachments(&self) -> VaultResult<HashSet<AttachmentId>> {
        let mut referenced = HashSet::new();
        for record in &self.records {
            for attachment in self.decrypt(record)?.attachments() {
                if !referenced.insert(attachment.id()) {
                    return Err(VaultError::Corrupted {
                        reason: "two attachments share an identifier",
                    });
                }
            }
        }
        Ok(referenced)
    }

    /// Deletes blobs in the attachments directory that no item refers to,
    /// and temporary files left by interrupted writes. Only files named
    /// like blobs are touched; anything else in the directory is left
    /// alone. Best effort: failures are ignored, as the files are
    /// ciphertext and are retried on the next open.
    fn prune_unreferenced_blobs(&self, referenced: &HashSet<AttachmentId>) {
        let Ok(entries) = fs::read_dir(self.attachments_dir()) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let orphan = match parse_blob_name(name) {
                Some(id) => !referenced.contains(&id),
                None => is_blob_temp_name(name),
            };
            if orphan && entry.file_type().is_ok_and(|kind| kind.is_file()) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    /// Path of the blob of attachment `id`.
    fn blob_path(&self, id: AttachmentId) -> PathBuf {
        self.attachments_dir()
            .join(format!("{id}.{BLOB_EXTENSION}"))
    }

    /// Whether writing to `path` would replace the vault file or a file in
    /// its attachments directory. Compared after resolving links and
    /// relative parts, so another spelling of the same path is caught.
    fn is_vault_storage(&self, path: &Path) -> bool {
        let Some(resolved) = resolve(path) else {
            // The destination's directory does not exist; the write fails
            // later with a clear I/O error.
            return false;
        };
        resolve(&self.path).is_some_and(|vault| resolved == vault)
            || fs::canonicalize(self.attachments_dir()).is_ok_and(|dir| resolved.starts_with(dir))
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
        attachments: &[Attachment],
    ) -> VaultResult<RecordEnvelope> {
        let kind = content.kind();
        let payload = item::encode_payload(
            self.header.version(),
            created_at_ms,
            updated_at_ms,
            content,
            attachments,
        )?;
        let ad = format::record_ad(&self.header, &id, kind);
        let sealed = aead::encrypt(&self.key, &payload, &ad)?;
        Ok(RecordEnvelope { id, kind, sealed })
    }

    fn decrypt(&self, record: &RecordEnvelope) -> VaultResult<Item> {
        let payload = self.decrypt_payload(&self.header, record)?;
        Ok(Item::new(
            record.id,
            payload.created_at_ms,
            payload.updated_at_ms,
            payload.content,
            payload.attachments,
        ))
    }

    /// Decrypts and decodes `record` as written with `header`.
    fn decrypt_payload(&self, header: &Header, record: &RecordEnvelope) -> VaultResult<Payload> {
        let ad = format::record_ad(header, &record.id, record.kind);
        // The seal was verified on open, so a failure here means the
        // in-memory record was damaged or the writer had a bug.
        let payload =
            aead::decrypt(&self.key, &record.sealed, &ad).map_err(|_| VaultError::Corrupted {
                reason: "record failed authentication",
            })?;
        item::decode_payload(header.version(), record.kind, payload.expose_secret())
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

/// Creates `dir` (and its parents) if needed; on Unix a new directory is
/// accessible to its owner only.
fn create_private_dir(dir: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(dir)
}

/// The attachment identifier of a blob file name `<32 hex digits>.nvatt`.
fn parse_blob_name(name: &str) -> Option<AttachmentId> {
    let (id, extension) = name.split_once('.')?;
    if extension != BLOB_EXTENSION {
        return None;
    }
    AttachmentId::from_hex(id)
}

/// Whether `name` is a temporary file of an interrupted blob write:
/// `.<blob name>.<16 hex digits>.tmp` (see `atomic::AtomicFile`).
fn is_blob_temp_name(name: &str) -> bool {
    name.strip_prefix('.')
        .and_then(|rest| rest.strip_suffix(".tmp"))
        .and_then(|rest| rest.rsplit_once('.'))
        .is_some_and(|(blob, random)| {
            random.len() == 16
                && random.bytes().all(|byte| byte.is_ascii_hexdigit())
                && parse_blob_name(blob).is_some()
        })
}

/// `path` with its directory resolved (links and relative parts), or
/// `None` if the directory does not exist. The file itself need not exist.
fn resolve(path: &Path) -> Option<PathBuf> {
    let name: OsString = path.file_name()?.to_os_string();
    let dir = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    Some(fs::canonicalize(dir).ok()?.join(name))
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
    use crate::attachment::CHUNK_LEN;
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

        for version in [FORMAT_VERSION + 1, 0x0100, u16::MAX] {
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
    fn verify_password_accepts_only_the_current_password() {
        let dir = TempDir::new();
        let (mut vault, _code) = new_vault(&dir);
        vault.verify_password(&pw(PASSWORD)).unwrap();
        assert!(matches!(
            vault.verify_password(&pw("wrong")),
            Err(VaultError::InvalidCredentials)
        ));
        vault
            .change_password(&pw("new password"), fast_params())
            .unwrap();
        assert!(matches!(
            vault.verify_password(&pw(PASSWORD)),
            Err(VaultError::InvalidCredentials)
        ));
        vault.verify_password(&pw("new password")).unwrap();
    }

    #[test]
    fn verify_recovery_code_accepts_only_the_current_code() {
        let dir = TempDir::new();
        let (mut vault, old) = new_vault(&dir);
        vault.verify_recovery_code(&old).unwrap();
        let new = vault.regenerate_recovery_code(fast_params()).unwrap();
        assert!(matches!(
            vault.verify_recovery_code(&old),
            Err(VaultError::InvalidCredentials)
        ));
        vault.verify_recovery_code(&new).unwrap();
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

    // ----- Format version 0 migration -----

    /// The vault's current contents as a format version 0 file, written
    /// the way a version 0 build wrote it.
    fn version_zero_file(vault: &Vault) -> Vec<u8> {
        let old = vault.header.with_version(0);
        let records: Vec<RecordEnvelope> = vault
            .records
            .iter()
            .map(|record| {
                let payload = vault.decrypt_payload(&vault.header, record).unwrap();
                let encoded = item::encode_payload(
                    0,
                    payload.created_at_ms,
                    payload.updated_at_ms,
                    &payload.content,
                    &[],
                )
                .unwrap();
                let ad = format::record_ad(&old, &record.id, record.kind);
                RecordEnvelope {
                    id: record.id,
                    kind: record.kind,
                    sealed: aead::encrypt(&vault.key, &encoded, &ad).unwrap(),
                }
            })
            .collect();
        let mut file = format::encode_body(&old, &vault.slots, &records).unwrap();
        let seal = format::seal(&vault.key, &file).unwrap();
        file.extend_from_slice(&seal);
        file
    }

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .map(|entries| {
                entries
                    .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    #[test]
    fn version_zero_vaults_are_migrated_on_open() {
        let dir = TempDir::new();
        let (mut vault, code) = new_vault(&dir);
        let login_id = vault.insert(login()).unwrap();
        let note_id = vault.insert(note("from version 0")).unwrap();
        let created = vault.get(note_id).unwrap().created_at_ms();
        let v0 = version_zero_file(&vault);
        assert_eq!(&v0[8..10], &[0, 0]);
        let path = dir.path().join("old.nvault");
        fs::write(&path, &v0).unwrap();
        drop(vault);

        let mut migrated = open(&path).unwrap();
        // Rewritten as the current version, with no backup left behind.
        let on_disk = fs::read(&path).unwrap();
        assert_eq!(&on_disk[8..10], &FORMAT_VERSION.to_le_bytes());
        assert!(
            file_names(dir.path())
                .iter()
                .all(|name| !name.ends_with(".bak"))
        );
        // Items, fields and timestamps survive.
        assert_eq!(migrated.len(), 2);
        let item = migrated.get(note_id).unwrap();
        assert_eq!(note_body(&item), "from version 0");
        assert_eq!(item.created_at_ms(), created);
        assert!(item.attachments().is_empty());
        assert!(matches!(
            migrated.get(login_id).unwrap().content(),
            ItemContent::Password(_)
        ));
        // The key slots were kept: both secrets still open the vault.
        assert!(Vault::open_with_recovery_code(&path, &code).is_ok());
        // And the migrated vault can hold attachments.
        migrated
            .add_attachment(note_id, "new.txt", &mut &b"hi"[..], keep_going)
            .unwrap();
        migrated.save().unwrap();
        assert_eq!(
            open(&path)
                .unwrap()
                .get(note_id)
                .unwrap()
                .attachments()
                .len(),
            1
        );
    }

    #[test]
    fn version_zero_vaults_open_with_the_recovery_code_too() {
        let dir = TempDir::new();
        let (mut vault, code) = new_vault(&dir);
        let id = vault.insert(note("kept")).unwrap();
        let path = dir.path().join("old.nvault");
        fs::write(&path, version_zero_file(&vault)).unwrap();
        let migrated = Vault::open_with_recovery_code(&path, &code).unwrap();
        assert_eq!(note_body(&migrated.get(id).unwrap()), "kept");
        assert!(open(&path).is_ok());
    }

    #[test]
    fn a_wrong_secret_never_migrates() {
        let dir = TempDir::new();
        let (vault, _) = new_vault(&dir);
        let path = dir.path().join("old.nvault");
        let v0 = version_zero_file(&vault);
        fs::write(&path, &v0).unwrap();
        assert!(matches!(
            Vault::open_with_password(&path, &pw("wrong")),
            Err(VaultError::InvalidCredentials)
        ));
        assert_eq!(fs::read(&path).unwrap(), v0);
    }

    #[test]
    fn rewriting_the_version_field_is_detected() {
        // Slots do not authenticate the version, so changing it must be
        // caught by the seal and the records.
        let dir = TempDir::new();
        let (mut vault, _) = new_vault(&dir);
        vault.insert(note("x")).unwrap();
        vault.save().unwrap();
        let mut downgraded = fs::read(vault.path()).unwrap();
        downgraded[8..10].copy_from_slice(&0u16.to_le_bytes());
        assert!(matches!(
            open_bytes(&dir, &downgraded),
            Err(VaultError::Corrupted { .. })
        ));
    }

    // ----- Attachments -----

    fn keep_going(_: u64) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn content(len: usize) -> Vec<u8> {
        (0..len).map(|index| (index * 31 % 251) as u8).collect()
    }

    fn attach(vault: &mut Vault, item: ItemId, name: &str, data: &[u8]) -> Attachment {
        vault
            .add_attachment(item, name, &mut &data[..], keep_going)
            .unwrap()
    }

    fn export(
        vault: &Vault,
        item: ItemId,
        attachment: AttachmentId,
        dir: &TempDir,
    ) -> VaultResult<Vec<u8>> {
        let path = dir.path().join("exported.bin");
        vault.export_attachment(item, attachment, &path, keep_going)?;
        Ok(fs::read(path).unwrap())
    }

    fn blob_names(vault: &Vault) -> Vec<String> {
        file_names(&vault.attachments_dir())
    }

    #[test]
    fn attachments_round_trip_through_save_and_reopen() {
        let dir = TempDir::new();
        let (mut vault, code) = new_vault(&dir);
        let id = vault.insert(note("with files")).unwrap();
        let big = content(3 * CHUNK_LEN + 5);
        let empty = attach(&mut vault, id, "empty.txt", b"");
        let mut reported = Vec::new();
        let large = vault
            .add_attachment(id, "scan ✓.pdf", &mut &big[..], |done| {
                reported.push(done);
                ControlFlow::Continue(())
            })
            .unwrap();
        assert_eq!(reported, [65_536, 131_072, 196_608, big.len() as u64]);
        assert_eq!(large.size(), big.len() as u64);
        assert_eq!(large.name(), "scan ✓.pdf");
        vault.save().unwrap();

        // Blobs hold ciphertext only, named by identifier.
        assert_eq!(blob_names(&vault).len(), 2);
        let blob = fs::read(vault.blob_path(large.id())).unwrap();
        assert_eq!(blob.len() as u64, attachment::blob_len(big.len() as u64));
        let sample = &big[..64];
        assert!(!blob.windows(64).any(|window| window == sample));

        let reopened = Vault::open_with_recovery_code(vault.path(), &code).unwrap();
        let item = reopened.get(id).unwrap();
        let listed: Vec<(&str, u64)> = item
            .attachments()
            .iter()
            .map(|attachment| (attachment.name(), attachment.size()))
            .collect();
        assert_eq!(listed, [("empty.txt", 0), ("scan ✓.pdf", big.len() as u64)]);
        assert_eq!(export(&reopened, id, large.id(), &dir).unwrap(), big);
        assert_eq!(export(&reopened, id, empty.id(), &dir).unwrap(), b"");
        assert_eq!(
            reopened.attachment(id, large.id()).unwrap().name(),
            "scan ✓.pdf"
        );
    }

    #[test]
    fn editing_an_item_keeps_its_attachments() {
        let dir = TempDir::new();
        let (mut vault, _) = new_vault(&dir);
        let id = vault.insert(note("v1")).unwrap();
        let file = attach(&mut vault, id, "a.txt", b"data");
        vault.update(id, note("v2")).unwrap();
        vault.save().unwrap();
        let reopened = open(vault.path()).unwrap();
        let item = reopened.get(id).unwrap();
        assert_eq!(note_body(&item), "v2");
        assert_eq!(item.attachments()[0].id(), file.id());
        assert_eq!(export(&reopened, id, file.id(), &dir).unwrap(), b"data");
    }

    #[test]
    fn removed_attachments_are_deleted_only_after_a_save() {
        let dir = TempDir::new();
        let (mut vault, _) = new_vault(&dir);
        let id = vault.insert(note("x")).unwrap();
        let keep = attach(&mut vault, id, "keep.txt", b"keep");
        let gone = attach(&mut vault, id, "gone.txt", b"gone");
        vault.save().unwrap();

        vault.remove_attachment(id, gone.id()).unwrap();
        assert!(matches!(
            vault.remove_attachment(id, gone.id()),
            Err(VaultError::AttachmentNotFound)
        ));
        // The saved file still refers to the blob, so it must still exist.
        assert!(vault.blob_path(gone.id()).exists());
        vault.save().unwrap();
        assert!(!vault.blob_path(gone.id()).exists());
        assert!(vault.blob_path(keep.id()).exists());
        assert!(matches!(
            export(&vault, id, gone.id(), &dir),
            Err(VaultError::AttachmentNotFound)
        ));
    }

    #[test]
    fn removing_an_item_deletes_its_blobs_after_a_save() {
        let dir = TempDir::new();
        let (mut vault, _) = new_vault(&dir);
        let id = vault.insert(note("x")).unwrap();
        let other = vault.insert(note("y")).unwrap();
        let file = attach(&mut vault, id, "a.txt", b"a");
        let kept = attach(&mut vault, other, "b.txt", b"b");
        vault.save().unwrap();
        vault.remove(id).unwrap();
        assert!(vault.blob_path(file.id()).exists());
        vault.save().unwrap();
        assert_eq!(blob_names(&vault), [format!("{}.nvatt", kept.id())]);
    }

    #[test]
    fn unsaved_and_interrupted_blobs_are_pruned_on_open() {
        let dir = TempDir::new();
        let (mut vault, _) = new_vault(&dir);
        let id = vault.insert(note("x")).unwrap();
        let saved = attach(&mut vault, id, "saved.txt", b"saved");
        vault.save().unwrap();
        let unsaved = attach(&mut vault, id, "unsaved.txt", b"never saved");
        let attachments = vault.attachments_dir();
        let temp = format!(".{}.nvatt.0123456789abcdef.tmp", unsaved.id());
        fs::write(attachments.join(&temp), b"partial").unwrap();
        fs::write(attachments.join("notes.txt"), b"not ours").unwrap();
        drop(vault);

        let reopened = open(&dir.path().join("test.nvault")).unwrap();
        let mut expected = vec![format!("{}.nvatt", saved.id()), "notes.txt".to_owned()];
        expected.sort();
        assert_eq!(file_names(&attachments), expected);
        assert_eq!(reopened.get(id).unwrap().attachments().len(), 1);
    }

    #[test]
    fn cancelled_transfers_leave_nothing_behind() {
        let dir = TempDir::new();
        let (mut vault, _) = new_vault(&dir);
        let id = vault.insert(note("x")).unwrap();
        let big = content(2 * CHUNK_LEN + 1);
        let result = vault.add_attachment(id, "big.bin", &mut &big[..], |done| {
            if done > 0 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });
        assert!(matches!(result, Err(VaultError::Cancelled)));
        assert!(blob_names(&vault).is_empty());
        assert!(vault.get(id).unwrap().attachments().is_empty());

        let file = attach(&mut vault, id, "big.bin", &big);
        let destination = dir.path().join("out.bin");
        let result =
            vault.export_attachment(id, file.id(), &destination, |_| ControlFlow::Break(()));
        assert!(matches!(result, Err(VaultError::Cancelled)));
        assert!(!destination.exists());
        assert!(
            file_names(dir.path())
                .iter()
                .all(|name| !name.ends_with(".tmp"))
        );
    }

    #[test]
    fn damaged_blobs_never_produce_an_export() {
        let dir = TempDir::new();
        let (mut vault, _) = new_vault(&dir);
        let id = vault.insert(note("x")).unwrap();
        let data = content(2 * CHUNK_LEN + 10);
        let file = attach(&mut vault, id, "a.bin", &data);
        let other = attach(&mut vault, id, "b.bin", &data);
        vault.save().unwrap();
        let blob_path = vault.blob_path(file.id());
        let original = fs::read(&blob_path).unwrap();
        let destination = dir.path().join("exported.bin");
        fs::write(&destination, b"previous export").unwrap();

        let mut flipped_last = original.clone();
        let last = flipped_last.len() - 1;
        flipped_last[last] ^= 1;
        let mut flipped_first = original.clone();
        flipped_first[attachment::BLOB_HEADER_LEN + 30] ^= 1;
        let truncated =
            original[..attachment::BLOB_HEADER_LEN + attachment::SEALED_CHUNK_LEN].to_vec();
        let swapped = fs::read(vault.blob_path(other.id())).unwrap();
        for (what, bytes) in [
            ("last byte", flipped_last),
            ("first chunk", flipped_first),
            ("truncated", truncated),
            ("other attachment", swapped),
        ] {
            fs::write(&blob_path, &bytes).unwrap();
            let result = vault.export_attachment(id, file.id(), &destination, keep_going);
            assert!(
                matches!(result, Err(VaultError::Corrupted { .. })),
                "{what}: {result:?}"
            );
            assert_eq!(
                fs::read(&destination).unwrap(),
                b"previous export",
                "{what}"
            );
        }
        fs::remove_file(&blob_path).unwrap();
        assert!(matches!(
            vault.export_attachment(id, file.id(), &destination, keep_going),
            Err(VaultError::Corrupted { .. })
        ));
        assert!(
            file_names(dir.path())
                .iter()
                .all(|name| !name.ends_with(".tmp"))
        );
    }

    #[test]
    fn exports_never_overwrite_the_vault() {
        let dir = TempDir::new();
        let (mut vault, _) = new_vault(&dir);
        let id = vault.insert(note("x")).unwrap();
        let file = attach(&mut vault, id, "a.txt", b"plaintext");
        vault.save().unwrap();
        let before = fs::read(vault.path()).unwrap();
        let blob = vault.blob_path(file.id());
        let spelled_differently = dir.path().join(".").join("test.nvault");
        for target in [
            vault.path().to_path_buf(),
            spelled_differently,
            blob.clone(),
        ] {
            assert!(
                matches!(
                    vault.export_attachment(id, file.id(), &target, keep_going),
                    Err(VaultError::InvalidItem { .. })
                ),
                "{target:?}"
            );
        }
        assert_eq!(fs::read(vault.path()).unwrap(), before);
        assert!(blob.exists());
    }

    #[test]
    fn attachment_inputs_are_validated() {
        let dir = TempDir::new();
        let (mut vault, _) = new_vault(&dir);
        let id = vault.insert(note("x")).unwrap();
        for name in ["", "..", "dir/file", "dir\\file"] {
            assert!(matches!(
                vault.add_attachment(id, name, &mut &b"x"[..], keep_going),
                Err(VaultError::InvalidItem { .. })
            ));
        }
        let missing = ItemId::from_bytes([9; 16]);
        assert!(matches!(
            vault.add_attachment(missing, "a", &mut &b"x"[..], keep_going),
            Err(VaultError::ItemNotFound)
        ));
        assert!(matches!(
            vault.attachment(id, AttachmentId::from_bytes([1; 16])),
            Err(VaultError::AttachmentNotFound)
        ));
        // Nothing was written for the rejected inputs.
        assert!(blob_names(&vault).is_empty());
    }
}
