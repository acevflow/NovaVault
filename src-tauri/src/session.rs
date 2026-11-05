//! The vault session: the one place the shell holds an unlocked vault.
//!
//! [`Session`] owns the [`Vault`] (and with it the vault key) for as long as
//! the vault is unlocked, and implements the logic of every Tauri command.
//! It has no Tauri types, so it can be unit-tested without a running app;
//! the command handlers in `commands.rs` only lock it and delegate.
//!
//! Locking drops the `Vault`, which zeroizes the key. Every edit is saved to
//! disk immediately (atomically, by the core). If a save fails the command
//! reports [`CommandError::Storage`]; the edit stays in memory and is
//! written by the next successful save.

use std::fs::{self, File};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::time::Instant;

use novavault_core::{ItemContent, KdfParams, RecoveryCode, Vault, VaultError};
use secrecy::{ExposeSecret, SecretSlice, SecretString};
use zeroize::Zeroizing;

use crate::dto::{
    AttachmentDto, Credential, ItemDetailsDto, ItemField, ItemInput, ItemSummaryDto, ItemUpdate,
    RecoveryCodeDto, RevealedValueDto, VaultStatusDto, parse_attachment_id, parse_item_id,
};
use crate::error::{CommandError, CommandResult};
use crate::settings::Settings;

/// File name of the settings, stored next to the vault file.
const SETTINGS_FILE_NAME: &str = "settings.json";

/// State of the single vault this app manages.
pub struct Session {
    /// Where the vault file lives. Chosen by the shell, never by the UI.
    path: PathBuf,
    /// Argon2id parameters for new key slots.
    params: KdfParams,
    /// The unlocked vault, or `None` while locked.
    vault: Option<Vault>,
    /// When the last command ran, for the inactivity auto-lock.
    last_activity: Instant,
    /// Where the settings file lives: next to the vault file.
    settings_path: PathBuf,
    /// The current settings, loaded at startup.
    settings: Settings,
}

impl Session {
    /// A locked session for the vault at `path`, creating key slots with
    /// `params`. Settings are read from `settings.json` in the same
    /// directory, with defaults if that file is missing or invalid.
    pub fn new(path: PathBuf, params: KdfParams, now: Instant) -> Self {
        let settings_path = path.with_file_name(SETTINGS_FILE_NAME);
        let settings = Settings::load(&settings_path);
        Self {
            path,
            params,
            vault: None,
            last_activity: now,
            settings_path,
            settings,
        }
    }

    /// The current settings.
    pub const fn settings(&self) -> Settings {
        self.settings
    }

    /// Sets the inactivity timeout and saves the settings.
    ///
    /// Requires an unlocked vault, so that someone at a locked app cannot
    /// lengthen the timeout before the owner unlocks it.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`], [`CommandError::InvalidInput`] if
    /// `minutes` is out of bounds, or [`CommandError::Storage`] if the
    /// file cannot be written (the previous value is then kept).
    pub fn set_auto_lock_minutes(&mut self, minutes: u32) -> CommandResult<()> {
        self.vault()?;
        let mut updated = self.settings;
        updated.set_auto_lock_minutes(minutes)?;
        updated
            .save(&self.settings_path)
            .map_err(|_| CommandError::Storage)?;
        self.settings = updated;
        Ok(())
    }

    /// Sets the clipboard clear delay and saves the settings.
    ///
    /// Requires an unlocked vault, like [`Session::set_auto_lock_minutes`].
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`], [`CommandError::InvalidInput`] if
    /// `seconds` is out of bounds, or [`CommandError::Storage`] if the
    /// file cannot be written (the previous value is then kept).
    pub fn set_clipboard_clear_seconds(&mut self, seconds: u32) -> CommandResult<()> {
        self.vault()?;
        let mut updated = self.settings;
        updated.set_clipboard_clear_seconds(seconds)?;
        updated
            .save(&self.settings_path)
            .map_err(|_| CommandError::Storage)?;
        self.settings = updated;
        Ok(())
    }

    /// Records user activity at `now`, restarting the inactivity timer.
    pub fn touch(&mut self, now: Instant) {
        self.last_activity = now;
    }

    /// Whether a vault is currently unlocked.
    pub const fn is_unlocked(&self) -> bool {
        self.vault.is_some()
    }

    /// Whether a vault file exists and whether it is unlocked, so the UI
    /// can choose between the create and unlock screens.
    ///
    /// # Errors
    ///
    /// [`CommandError::Storage`] if the vault location cannot be checked.
    pub fn status(&self) -> CommandResult<VaultStatusDto> {
        Ok(VaultStatusDto {
            exists: self.path.try_exists().map_err(|_| CommandError::Storage)?,
            unlocked: self.is_unlocked(),
        })
    }

    /// Creates a new vault protected by `password` and leaves it unlocked.
    ///
    /// Returns the recovery code, which the UI must show once; it is not
    /// stored anywhere.
    ///
    /// # Errors
    ///
    /// [`CommandError::InvalidInput`] for an empty password,
    /// [`CommandError::VaultExists`] if a vault already exists, and
    /// [`CommandError::Storage`] if the file cannot be written.
    pub fn create(&mut self, password: &SecretString) -> CommandResult<RecoveryCodeDto> {
        let password = password_bytes(password)?;
        // Drop any open vault first so its key is wiped before another is
        // derived, and a failure below leaves the session locked.
        self.lock();
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|_| CommandError::Storage)?;
        }
        let (vault, code) = Vault::create(&self.path, &password, self.params)?;
        self.vault = Some(vault);
        Ok(RecoveryCodeDto::from(&code))
    }

    /// Unlocks the vault with the master password or the recovery code.
    ///
    /// An already unlocked vault is locked first, so a failed attempt
    /// always leaves the session locked.
    ///
    /// # Errors
    ///
    /// [`CommandError::VaultNotFound`] if no vault exists,
    /// [`CommandError::InvalidRecoveryCode`] for a malformed code,
    /// [`CommandError::InvalidCredentials`] for a wrong secret, and the
    /// file errors of opening a vault.
    pub fn unlock(&mut self, credential: &Credential) -> CommandResult<()> {
        self.lock();
        if !self.path.try_exists().map_err(|_| CommandError::Storage)? {
            return Err(CommandError::VaultNotFound);
        }
        let vault = match credential {
            Credential::Password(password) => {
                Vault::open_with_password(&self.path, &password_bytes(password)?)?
            }
            Credential::RecoveryCode(code) => {
                let code = RecoveryCode::parse(code.expose_secret())?;
                Vault::open_with_recovery_code(&self.path, &code)?
            }
        };
        self.vault = Some(vault);
        Ok(())
    }

    /// Locks the vault, wiping the vault key. Returns whether it was
    /// unlocked.
    pub fn lock(&mut self) -> bool {
        self.vault.take().is_some()
    }

    /// Locks the vault if it has been unlocked and idle for at least the
    /// configured auto-lock timeout at `now`. Returns whether it locked.
    pub fn lock_if_idle(&mut self, now: Instant) -> bool {
        self.is_unlocked()
            && now.saturating_duration_since(self.last_activity)
                >= self.settings.auto_lock_timeout()
            && self.lock()
    }

    /// Changes the master password after checking `current` (the current
    /// password, or the recovery code for the forgotten-password flow).
    /// The recovery code keeps working.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`], [`CommandError::InvalidCredentials`] or
    /// [`CommandError::InvalidRecoveryCode`] if `current` is wrong,
    /// [`CommandError::InvalidInput`] for an empty password, or a save
    /// failure.
    pub fn change_master_password(
        &mut self,
        current: &Credential,
        new_password: &SecretString,
    ) -> CommandResult<()> {
        self.verify(current)?;
        let new_password = password_bytes(new_password)?;
        let params = self.params;
        self.vault_mut()?.change_password(&new_password, params)?;
        Ok(())
    }

    /// Replaces the recovery code after checking `current`; the previous
    /// code stops working. Returns the new code, which the UI must show
    /// once.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`], the errors of a wrong `current` (see
    /// [`Session::change_master_password`]) or a save failure.
    pub fn regenerate_recovery_code(
        &mut self,
        current: &Credential,
    ) -> CommandResult<RecoveryCodeDto> {
        self.verify(current)?;
        let params = self.params;
        let code = self.vault_mut()?.regenerate_recovery_code(params)?;
        Ok(RecoveryCodeDto::from(&code))
    }

    /// Re-authenticates the user before a key slot change, so someone at
    /// an unlocked but unattended app cannot take the vault over.
    fn verify(&self, current: &Credential) -> CommandResult<()> {
        let vault = self.vault()?;
        match current {
            Credential::Password(password) => {
                vault.verify_password(&password_bytes(password)?)?;
            }
            Credential::RecoveryCode(code) => {
                vault.verify_recovery_code(&RecoveryCode::parse(code.expose_secret())?)?;
            }
        }
        Ok(())
    }

    /// Lists every item's identifier, type, title and timestamps.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`] or [`CommandError::Corrupted`].
    pub fn list_items(&self) -> CommandResult<Vec<ItemSummaryDto>> {
        let summaries = self.vault()?.list()?;
        Ok(summaries.into_iter().map(ItemSummaryDto::from).collect())
    }

    /// Adds an item, saves, and returns the new item's identifier.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`], [`CommandError::InvalidItem`] or a save
    /// failure.
    pub fn create_item(&mut self, item: ItemInput) -> CommandResult<String> {
        let vault = self.vault_mut()?;
        let id = vault.insert(ItemContent::from(item))?;
        vault.save()?;
        Ok(id.to_string())
    }

    /// Returns item `id` with its plain fields, and for each secret field
    /// only whether it is set.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`], [`CommandError::InvalidInput`] for a
    /// malformed identifier, [`CommandError::ItemNotFound`] or
    /// [`CommandError::Corrupted`].
    pub fn get_item(&self, id: &str) -> CommandResult<ItemDetailsDto> {
        let item = self.vault()?.get(parse_item_id(id)?)?;
        Ok(ItemDetailsDto::from(&item))
    }

    /// Returns the value of one secret field of item `id`.
    ///
    /// # Errors
    ///
    /// [`CommandError::InvalidInput`] if `field` is not a secret field of
    /// this item's kind (plain fields come with [`Session::get_item`]),
    /// and the errors of [`Session::get_item`].
    pub fn reveal_field(&self, id: &str, field: ItemField) -> CommandResult<RevealedValueDto> {
        if !field.is_secret() {
            return Err(CommandError::InvalidInput);
        }
        let value = self.field_text(id, field)?;
        Ok(RevealedValueDto::new(SecretString::from(value.as_str())))
    }

    /// A copy of the text of `field` in item `id`, in zeroizing storage.
    ///
    /// # Errors
    ///
    /// [`CommandError::InvalidInput`] if items of this kind have no such
    /// field, and the errors of [`Session::get_item`].
    pub fn field_text(&self, id: &str, field: ItemField) -> CommandResult<Zeroizing<String>> {
        let item = self.vault()?.get(parse_item_id(id)?)?;
        Ok(Zeroizing::new(field.value_in(item.content())?.to_owned()))
    }

    /// Updates item `id` and saves. Secret fields left out of `update`
    /// keep their stored values.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`], [`CommandError::InvalidInput`] for a
    /// malformed identifier or a kind that does not match the stored item,
    /// [`CommandError::ItemNotFound`], [`CommandError::InvalidItem`] or a
    /// save failure.
    pub fn update_item(&mut self, id: &str, update: ItemUpdate) -> CommandResult<()> {
        let vault = self.vault_mut()?;
        let id = parse_item_id(id)?;
        let content = update.apply_to(vault.get(id)?.into_content())?;
        vault.update(id, content)?;
        vault.save()?;
        Ok(())
    }

    /// Deletes item `id` and saves.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`], [`CommandError::InvalidInput`],
    /// [`CommandError::ItemNotFound`] or a save failure.
    pub fn delete_item(&mut self, id: &str) -> CommandResult<()> {
        let vault = self.vault_mut()?;
        let id = parse_item_id(id)?;
        vault.remove(id)?;
        vault.save()?;
        Ok(())
    }

    /// Checks that the vault is unlocked and item `id` exists, before a
    /// file dialog is shown for it.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`], [`CommandError::InvalidInput`] or
    /// [`CommandError::ItemNotFound`].
    pub fn check_item(&self, id: &str) -> CommandResult<()> {
        self.vault()?.get(parse_item_id(id)?)?;
        Ok(())
    }

    /// The file name and size of attachment `attachment_id` of item
    /// `item_id`, to suggest a name when exporting it.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`], [`CommandError::InvalidInput`] for a
    /// malformed identifier, [`CommandError::ItemNotFound`] or
    /// [`CommandError::AttachmentNotFound`].
    pub fn attachment_info(
        &self,
        item_id: &str,
        attachment_id: &str,
    ) -> CommandResult<(String, u64)> {
        let attachment = self
            .vault()?
            .attachment(parse_item_id(item_id)?, parse_attachment_id(attachment_id)?)?;
        Ok((attachment.name().to_owned(), attachment.size()))
    }

    /// Encrypts the file at `source` (picked by the user in a native
    /// dialog, never sent by the UI) as a new attachment of item `item_id`,
    /// saves, and returns the attachment. The attachment is named after the
    /// file; a name that is not valid UTF-8 is stored with replacement
    /// characters.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`], [`CommandError::InvalidInput`],
    /// [`CommandError::ItemNotFound`], [`CommandError::FileAccess`] if the
    /// file cannot be read or the attachment written,
    /// [`CommandError::InvalidItem`] if the name is unusable or the item
    /// has too many attachments, [`CommandError::Cancelled`], or a save
    /// failure.
    pub fn add_attachment(
        &mut self,
        item_id: &str,
        source: &Path,
        progress: impl FnMut(u64) -> ControlFlow<()>,
    ) -> CommandResult<AttachmentDto> {
        let vault = self.vault_mut()?;
        let id = parse_item_id(item_id)?;
        let name = source
            .file_name()
            .ok_or(CommandError::InvalidInput)?
            .to_string_lossy();
        let mut file = File::open(source).map_err(|_| CommandError::FileAccess)?;
        let attachment = vault
            .add_attachment(id, &name, &mut file, progress)
            .map_err(file_error)?;
        vault.save()?;
        Ok(AttachmentDto::from(&attachment))
    }

    /// Decrypts attachment `attachment_id` of item `item_id` to
    /// `destination` (picked by the user in a native dialog). Nothing is
    /// left at `destination` if the data fails authentication.
    ///
    /// # Errors
    ///
    /// The errors of [`Session::attachment_info`],
    /// [`CommandError::Corrupted`] if the encrypted data is missing or
    /// damaged, [`CommandError::FileAccess`] if `destination` cannot be
    /// written or is one of the vault's own files, or
    /// [`CommandError::Cancelled`].
    pub fn export_attachment(
        &self,
        item_id: &str,
        attachment_id: &str,
        destination: &Path,
        progress: impl FnMut(u64) -> ControlFlow<()>,
    ) -> CommandResult<()> {
        let vault = self.vault()?;
        let item = parse_item_id(item_id)?;
        let attachment = parse_attachment_id(attachment_id)?;
        vault
            .export_attachment(item, attachment, destination, progress)
            .map_err(|error| match error {
                // The only invalid input left is a destination inside the
                // vault's storage: report it as a file the shell will not
                // write.
                VaultError::InvalidItem { .. } => CommandError::FileAccess,
                other => file_error(other),
            })
    }

    /// Removes attachment `attachment_id` from item `item_id`, saves, and
    /// deletes the attachment's encrypted data.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`], [`CommandError::InvalidInput`],
    /// [`CommandError::ItemNotFound`], [`CommandError::AttachmentNotFound`]
    /// or a save failure.
    pub fn delete_attachment(&mut self, item_id: &str, attachment_id: &str) -> CommandResult<()> {
        let vault = self.vault_mut()?;
        vault.remove_attachment(parse_item_id(item_id)?, parse_attachment_id(attachment_id)?)?;
        vault.save()?;
        Ok(())
    }

    /// The unlocked vault. Every data command goes through this (or
    /// [`Session::vault_mut`]), which makes it the one check that the vault
    /// is unlocked.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`] while locked.
    fn vault(&self) -> CommandResult<&Vault> {
        self.vault.as_ref().ok_or(CommandError::Locked)
    }

    /// Mutable access to the unlocked vault; see [`Session::vault`].
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`] while locked.
    fn vault_mut(&mut self) -> CommandResult<&mut Vault> {
        self.vault.as_mut().ok_or(CommandError::Locked)
    }
}

impl core::fmt::Debug for Session {
    /// Shows only whether the vault is unlocked: the vault and the paths
    /// stay out of any debug output.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Session")
            .field("unlocked", &self.is_unlocked())
            .finish_non_exhaustive()
    }
}

/// Maps an attachment transfer error: I/O failures there concern the
/// user's file or the attachment data, not the vault file.
fn file_error(error: VaultError) -> CommandError {
    match error {
        VaultError::Io(_) => CommandError::FileAccess,
        other => other.into(),
    }
}

/// The password as bytes for key derivation, in zeroizing storage.
///
/// # Errors
///
/// [`CommandError::InvalidInput`] for an empty password, which would make
/// the vault openable by anyone.
fn password_bytes(password: &SecretString) -> CommandResult<SecretSlice<u8>> {
    let text = password.expose_secret();
    if text.is_empty() {
        return Err(CommandError::InvalidInput);
    }
    Ok(SecretSlice::from(text.as_bytes().to_vec()))
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use super::*;

    /// A uniquely named directory under the system temp directory, removed
    /// on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let name = format!(
                "novavault-shell-test-{}",
                novavault_core::ItemId::generate().unwrap()
            );
            let path = std::env::temp_dir().join(name);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const PASSWORD: &str = "correct horse battery staple";

    fn s(text: &str) -> SecretString {
        SecretString::from(text)
    }

    /// Lowest accepted cost, so debug-build tests stay fast.
    fn fast_params() -> KdfParams {
        KdfParams::new(KdfParams::MIN_MEMORY_KIB, KdfParams::MIN_ITERATIONS, 1).unwrap()
    }

    /// A session whose vault lives in a not-yet-existing subdirectory, to
    /// check that `create` makes it.
    fn session(dir: &TempDir) -> Session {
        Session::new(
            dir.path().join("data").join("vault.nvault"),
            fast_params(),
            Instant::now(),
        )
    }

    fn note(title: &str, body: &str) -> ItemInput {
        ItemInput::Note {
            title: title.into(),
            body: s(body),
        }
    }

    fn note_update(title: &str, body: Option<&str>) -> ItemUpdate {
        ItemUpdate::Note {
            title: title.into(),
            body: body.map(s),
        }
    }

    fn login(password: &str, notes: &str) -> ItemInput {
        ItemInput::Password {
            title: "Bank".into(),
            username: "alice".into(),
            password: s(password),
            url: "https://bank.example".into(),
            notes: s(notes),
        }
    }

    fn revealed(session: &Session, id: &str, field: ItemField) -> CommandResult<String> {
        let dto = session.reveal_field(id, field)?;
        Ok(serde_json::to_value(dto).unwrap()["value"]
            .as_str()
            .unwrap()
            .to_owned())
    }

    fn with_password(password: &str) -> Credential {
        Credential::Password(s(password))
    }

    fn code_text(code: &RecoveryCodeDto) -> String {
        serde_json::to_value(code).unwrap()["recoveryCode"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn titles(session: &Session) -> Vec<String> {
        let mut titles: Vec<String> = session
            .list_items()
            .unwrap()
            .iter()
            .map(|item| item.title.clone())
            .collect();
        titles.sort();
        titles
    }

    #[test]
    fn create_lock_and_unlock_with_either_secret() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        let code = code_text(&session.create(&s(PASSWORD)).unwrap());
        assert!(session.is_unlocked());

        assert!(session.lock());
        assert!(!session.lock(), "locking twice reports nothing to lock");
        session.unlock(&with_password(PASSWORD)).unwrap();
        assert!(session.is_unlocked());

        session.lock();
        // Codes are accepted in any case and with or without dashes.
        let typed = code.to_lowercase().replace('-', " ");
        session
            .unlock(&Credential::RecoveryCode(s(&typed)))
            .unwrap();
        assert!(session.is_unlocked());
    }

    #[test]
    fn wrong_or_malformed_secrets_leave_the_session_locked() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();

        assert_eq!(
            session.unlock(&with_password("wrong")),
            Err(CommandError::InvalidCredentials)
        );
        assert!(!session.is_unlocked());
        assert_eq!(
            session.unlock(&Credential::RecoveryCode(s("not a code"))),
            Err(CommandError::InvalidRecoveryCode)
        );
        let other = RecoveryCode::generate().unwrap();
        assert_eq!(
            session.unlock(&Credential::RecoveryCode(other.to_display())),
            Err(CommandError::InvalidCredentials)
        );
        assert!(!session.is_unlocked());
    }

    #[test]
    fn status_reports_existence_and_lock_state() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        let status = |session: &Session| {
            let status = session.status().unwrap();
            (status.exists, status.unlocked)
        };
        assert_eq!(status(&session), (false, false));
        session.create(&s(PASSWORD)).unwrap();
        assert_eq!(status(&session), (true, true));
        session.lock();
        assert_eq!(status(&session), (true, false));
    }

    #[test]
    fn missing_and_existing_vaults_are_reported() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        assert_eq!(
            session.unlock(&with_password(PASSWORD)),
            Err(CommandError::VaultNotFound)
        );
        session.create(&s(PASSWORD)).unwrap();
        assert_eq!(
            session.create(&s("another")).err(),
            Some(CommandError::VaultExists)
        );
        // The original vault is untouched.
        session.unlock(&with_password(PASSWORD)).unwrap();
    }

    #[test]
    fn empty_passwords_are_rejected() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        assert_eq!(
            session.create(&s("")).err(),
            Some(CommandError::InvalidInput)
        );
        session.create(&s(PASSWORD)).unwrap();
        assert_eq!(
            session.change_master_password(&with_password(PASSWORD), &s("")),
            Err(CommandError::InvalidInput)
        );
    }

    #[test]
    fn data_commands_require_an_unlocked_vault() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();
        let id = session.create_item(note("a", "b")).unwrap();
        session.lock();

        assert_eq!(session.list_items().err(), Some(CommandError::Locked));
        assert_eq!(
            session.create_item(note("a", "b")),
            Err(CommandError::Locked)
        );
        assert_eq!(
            session.update_item(&id, note_update("a", Some("b"))),
            Err(CommandError::Locked)
        );
        assert_eq!(session.delete_item(&id), Err(CommandError::Locked));
        assert_eq!(
            session.change_master_password(&with_password(PASSWORD), &s("new")),
            Err(CommandError::Locked)
        );
        assert_eq!(
            session
                .regenerate_recovery_code(&with_password(PASSWORD))
                .err(),
            Some(CommandError::Locked)
        );
    }

    #[test]
    fn item_edits_are_saved_immediately() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();
        let keep = session.create_item(note("Diary", "first")).unwrap();
        let drop_me = session
            .create_item(ItemInput::Secret {
                title: "API".into(),
                value: s("sk-123"),
                notes: s(""),
            })
            .unwrap();
        session
            .update_item(&keep, note_update("Journal", Some("second")))
            .unwrap();
        session.delete_item(&drop_me).unwrap();

        // Re-open from disk: every edit was persisted.
        session.lock();
        session.unlock(&with_password(PASSWORD)).unwrap();
        assert_eq!(titles(&session), ["Journal"]);
        let listed = session.list_items().unwrap();
        assert_eq!(listed[0].id, keep);
        assert_eq!(listed[0].kind, "note");
    }

    #[test]
    fn listing_never_serializes_secret_fields() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();
        session
            .create_item(ItemInput::Password {
                title: "Bank".into(),
                username: "alice".into(),
                password: s("hunter2"),
                url: "https://bank.example".into(),
                notes: s("PIN elsewhere"),
            })
            .unwrap();
        let json = serde_json::to_string(&session.list_items().unwrap()).unwrap();
        assert!(json.contains("\"title\":\"Bank\""), "{json}");
        for secret in ["alice", "hunter2", "bank.example", "PIN"] {
            assert!(!json.contains(secret), "{json}");
        }
    }

    #[test]
    fn item_details_have_plain_fields_and_flags_only() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();
        let id = session.create_item(login("hunter2", "")).unwrap();
        let json = serde_json::to_value(session.get_item(&id).unwrap()).unwrap();
        assert_eq!(json["id"], id.as_str());
        assert_eq!(json["kind"], "password");
        assert_eq!(json["title"], "Bank");
        assert_eq!(json["username"], "alice");
        assert_eq!(json["url"], "https://bank.example");
        assert_eq!(json["hasPassword"], true);
        assert_eq!(json["hasNotes"], false);
        assert!(!json.to_string().contains("hunter2"), "{json}");

        let note_id = session.create_item(note("Diary", "")).unwrap();
        let json = serde_json::to_value(session.get_item(&note_id).unwrap()).unwrap();
        assert_eq!(json["kind"], "note");
        assert_eq!(json["hasBody"], false);
    }

    #[test]
    fn secret_fields_are_revealed_one_at_a_time() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();
        let login_id = session.create_item(login("hunter2", "PIN")).unwrap();
        let note_id = session.create_item(note("Diary", "dear diary")).unwrap();
        let secret_id = session
            .create_item(ItemInput::Secret {
                title: "API".into(),
                value: s("sk-123"),
                notes: s("rotate yearly"),
            })
            .unwrap();

        let cases = [
            (&login_id, ItemField::Password, "hunter2"),
            (&login_id, ItemField::Notes, "PIN"),
            (&note_id, ItemField::Body, "dear diary"),
            (&secret_id, ItemField::Value, "sk-123"),
            (&secret_id, ItemField::Notes, "rotate yearly"),
        ];
        for (id, field, expected) in cases {
            assert_eq!(revealed(&session, id, field).unwrap(), expected);
        }
        // Plain fields are not revealed, and fields of another kind do not
        // exist.
        for (id, field) in [
            (&login_id, ItemField::Username),
            (&login_id, ItemField::Url),
            (&login_id, ItemField::Body),
            (&note_id, ItemField::Password),
            (&secret_id, ItemField::Body),
        ] {
            assert_eq!(
                revealed(&session, id, field),
                Err(CommandError::InvalidInput),
                "{field:?}"
            );
        }
        let username = session.field_text(&login_id, ItemField::Username).unwrap();
        assert_eq!(username.as_str(), "alice");

        session.lock();
        assert_eq!(
            revealed(&session, &login_id, ItemField::Password),
            Err(CommandError::Locked)
        );
        assert_eq!(
            session.get_item(&login_id).err(),
            Some(CommandError::Locked)
        );
    }

    #[test]
    fn updates_keep_secret_fields_that_are_left_out() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();
        let id = session.create_item(login("hunter2", "PIN")).unwrap();
        let update = ItemUpdate::Password {
            title: "Bank (joint)".into(),
            username: "bob".into(),
            password: None,
            url: "https://bank.example/login".into(),
            notes: Some(s("")),
        };
        session.update_item(&id, update).unwrap();

        // Re-open from disk: the merged item was saved.
        session.lock();
        session.unlock(&with_password(PASSWORD)).unwrap();
        let json = serde_json::to_value(session.get_item(&id).unwrap()).unwrap();
        assert_eq!(json["title"], "Bank (joint)");
        assert_eq!(json["username"], "bob");
        assert_eq!(json["hasNotes"], false);
        assert_eq!(
            revealed(&session, &id, ItemField::Password).unwrap(),
            "hunter2"
        );
    }

    #[test]
    fn updates_cannot_change_the_kind_of_an_item() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();
        let id = session.create_item(login("hunter2", "")).unwrap();
        assert_eq!(
            session.update_item(&id, note_update("x", Some("y"))),
            Err(CommandError::InvalidInput)
        );
        assert_eq!(
            revealed(&session, &id, ItemField::Password).unwrap(),
            "hunter2"
        );
    }

    #[test]
    fn unknown_or_malformed_item_ids_are_reported() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();
        let missing = "00".repeat(16);
        assert_eq!(
            session.update_item(&missing, note_update("a", Some("b"))),
            Err(CommandError::ItemNotFound)
        );
        assert_eq!(
            session.delete_item(&missing),
            Err(CommandError::ItemNotFound)
        );
        assert_eq!(
            session.delete_item("not-an-id"),
            Err(CommandError::InvalidInput)
        );
    }

    #[test]
    fn changing_the_master_password() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        let code = code_text(&session.create(&s(PASSWORD)).unwrap());
        session.create_item(note("Diary", "x")).unwrap();
        session
            .change_master_password(&with_password(PASSWORD), &s("new password"))
            .unwrap();
        session.lock();

        assert_eq!(
            session.unlock(&with_password(PASSWORD)),
            Err(CommandError::InvalidCredentials)
        );
        session.unlock(&with_password("new password")).unwrap();
        assert_eq!(titles(&session), ["Diary"]);
        session.lock();
        session.unlock(&Credential::RecoveryCode(s(&code))).unwrap();
    }

    #[test]
    fn regenerating_the_recovery_code_invalidates_the_old_one() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        let old = code_text(&session.create(&s(PASSWORD)).unwrap());
        let new = code_text(
            &session
                .regenerate_recovery_code(&with_password(PASSWORD))
                .unwrap(),
        );
        assert_ne!(old, new);
        session.lock();

        assert_eq!(
            session.unlock(&Credential::RecoveryCode(s(&old))),
            Err(CommandError::InvalidCredentials)
        );
        session.unlock(&Credential::RecoveryCode(s(&new))).unwrap();
        session.lock();
        session.unlock(&with_password(PASSWORD)).unwrap();
    }

    #[test]
    fn key_slot_changes_require_the_current_secret() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        let code = code_text(&session.create(&s(PASSWORD)).unwrap());

        assert_eq!(
            session.change_master_password(&with_password("wrong"), &s("new password")),
            Err(CommandError::InvalidCredentials)
        );
        assert_eq!(
            session
                .regenerate_recovery_code(&with_password("wrong"))
                .err(),
            Some(CommandError::InvalidCredentials)
        );
        assert_eq!(
            session
                .regenerate_recovery_code(&Credential::RecoveryCode(s("typo")))
                .err(),
            Some(CommandError::InvalidRecoveryCode)
        );
        // Nothing changed: the vault stays unlocked and both secrets work.
        assert!(session.is_unlocked());
        session.lock();
        session.unlock(&with_password(PASSWORD)).unwrap();

        // Forgotten-password flow: the recovery code authorizes the change.
        session
            .change_master_password(&Credential::RecoveryCode(s(&code)), &s("new password"))
            .unwrap();
        session.lock();
        session.unlock(&with_password("new password")).unwrap();
    }

    #[test]
    fn idle_sessions_lock_after_the_timeout() {
        let dir = TempDir::new();
        let start = Instant::now();
        let timeout = Duration::from_secs(300);
        let mut session = Session::new(dir.path().join("vault.nvault"), fast_params(), start);
        assert_eq!(session.settings().auto_lock_timeout(), timeout);
        // Nothing to lock yet.
        assert!(!session.lock_if_idle(start + timeout));

        session.create(&s(PASSWORD)).unwrap();
        session.touch(start);
        assert!(!session.lock_if_idle(start + timeout - Duration::from_secs(1)));
        // Activity restarts the timer.
        session.touch(start + Duration::from_secs(200));
        assert!(!session.lock_if_idle(start + timeout));
        assert!(session.is_unlocked());
        assert!(session.lock_if_idle(start + Duration::from_secs(500)));
        assert!(!session.is_unlocked());
        // A clock reading before the last activity never panics or locks.
        assert!(!session.lock_if_idle(start));
    }

    #[test]
    fn clipboard_delay_is_configurable_and_persisted() {
        let dir = TempDir::new();
        let path = dir.path().join("vault.nvault");
        let mut session = Session::new(path.clone(), fast_params(), Instant::now());
        assert_eq!(
            session.set_clipboard_clear_seconds(10),
            Err(CommandError::Locked),
            "changing the delay needs an unlocked vault"
        );
        session.create(&s(PASSWORD)).unwrap();
        assert_eq!(
            session.set_clipboard_clear_seconds(1),
            Err(CommandError::InvalidInput)
        );
        session.set_clipboard_clear_seconds(10).unwrap();
        let reloaded = Session::new(path, fast_params(), Instant::now());
        assert_eq!(reloaded.settings().clipboard_clear_seconds(), 10);
    }

    #[test]
    fn auto_lock_timeout_is_configurable_and_persisted() {
        let dir = TempDir::new();
        let start = Instant::now();
        let path = dir.path().join("vault.nvault");
        let mut session = Session::new(path.clone(), fast_params(), start);
        assert_eq!(
            session.set_auto_lock_minutes(1),
            Err(CommandError::Locked),
            "changing the timeout needs an unlocked vault"
        );
        session.create(&s(PASSWORD)).unwrap();
        assert_eq!(
            session.set_auto_lock_minutes(0),
            Err(CommandError::InvalidInput)
        );
        session.set_auto_lock_minutes(1).unwrap();
        session.touch(start);
        assert!(session.lock_if_idle(start + Duration::from_secs(60)));

        // A new session (next app start) reads the saved value.
        let reloaded = Session::new(path, fast_params(), start);
        assert_eq!(
            reloaded.settings().auto_lock_timeout(),
            Duration::from_secs(60)
        );
    }

    fn keep_going(_: u64) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    /// Writes `contents` to a file named `name` in `dir` and returns its path.
    fn user_file(dir: &TempDir, name: &str, contents: &[u8]) -> PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn attachments_are_added_exported_and_deleted() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();
        let id = session.create_item(note("Taxes", "")).unwrap();
        let data: Vec<u8> = (0..200_000u32).map(|n| (n % 251) as u8).collect();
        let source = user_file(&dir, "return 2026.pdf", &data);

        let mut reported = 0;
        let added = session
            .add_attachment(&id, &source, |done| {
                reported = done;
                ControlFlow::Continue(())
            })
            .unwrap();
        assert_eq!(reported, data.len() as u64);
        let json = serde_json::to_value(&added).unwrap();
        assert_eq!(json["name"], "return 2026.pdf");
        assert_eq!(json["size"], data.len());
        let attachment_id = json["id"].as_str().unwrap().to_owned();

        // Persisted: visible after locking and unlocking again.
        session.lock();
        session.unlock(&with_password(PASSWORD)).unwrap();
        let details = serde_json::to_value(session.get_item(&id).unwrap()).unwrap();
        assert_eq!(details["attachments"][0]["id"], attachment_id.as_str());
        assert_eq!(
            session.attachment_info(&id, &attachment_id).unwrap(),
            ("return 2026.pdf".to_owned(), data.len() as u64)
        );

        let destination = dir.path().join("exported.pdf");
        session
            .export_attachment(&id, &attachment_id, &destination, keep_going)
            .unwrap();
        assert_eq!(fs::read(&destination).unwrap(), data);

        session.delete_attachment(&id, &attachment_id).unwrap();
        assert_eq!(
            session.delete_attachment(&id, &attachment_id),
            Err(CommandError::AttachmentNotFound)
        );
        session.lock();
        session.unlock(&with_password(PASSWORD)).unwrap();
        let details = serde_json::to_value(session.get_item(&id).unwrap()).unwrap();
        assert_eq!(details["attachments"], serde_json::json!([]));
    }

    #[test]
    fn deleting_an_item_deletes_its_attachment_data() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();
        let id = session.create_item(note("x", "")).unwrap();
        let source = user_file(&dir, "a.txt", b"secret file");
        session.add_attachment(&id, &source, keep_going).unwrap();
        let blobs = dir.path().join("data").join("vault.nvault.attachments");
        assert_eq!(fs::read_dir(&blobs).unwrap().count(), 1);
        session.delete_item(&id).unwrap();
        assert_eq!(fs::read_dir(&blobs).unwrap().count(), 0);
    }

    #[test]
    fn attachment_commands_report_their_errors() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();
        let id = session.create_item(note("x", "")).unwrap();
        let source = user_file(&dir, "a.txt", b"data");
        let missing_item = "00".repeat(16);
        let missing_attachment = "11".repeat(16);

        assert_eq!(session.check_item(&id), Ok(()));
        assert_eq!(
            session.check_item(&missing_item),
            Err(CommandError::ItemNotFound)
        );
        assert_eq!(
            session
                .add_attachment(&id, &dir.path().join("absent.txt"), keep_going)
                .err(),
            Some(CommandError::FileAccess)
        );
        assert_eq!(
            session.attachment_info(&id, &missing_attachment).err(),
            Some(CommandError::AttachmentNotFound)
        );
        assert_eq!(
            session.attachment_info(&id, "bad").err(),
            Some(CommandError::InvalidInput)
        );
        let cancelled = session.add_attachment(&id, &source, |_| ControlFlow::Break(()));
        assert_eq!(cancelled.err(), Some(CommandError::Cancelled));

        // Exporting over the vault itself is refused.
        let added = session.add_attachment(&id, &source, keep_going).unwrap();
        let vault_path = dir.path().join("data").join("vault.nvault");
        let before = fs::read(&vault_path).unwrap();
        assert_eq!(
            session.export_attachment(&id, &added.id, &vault_path, keep_going),
            Err(CommandError::FileAccess)
        );
        assert_eq!(fs::read(&vault_path).unwrap(), before);

        session.lock();
        assert_eq!(session.check_item(&id), Err(CommandError::Locked));
        assert_eq!(
            session.add_attachment(&id, &source, keep_going).err(),
            Some(CommandError::Locked)
        );
        assert_eq!(
            session.export_attachment(&id, &added.id, &dir.path().join("out"), keep_going),
            Err(CommandError::Locked)
        );
        assert_eq!(
            session.delete_attachment(&id, &added.id),
            Err(CommandError::Locked)
        );
    }

    #[test]
    fn attachment_dtos_never_contain_paths() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();
        let id = session.create_item(note("x", "")).unwrap();
        let source = user_file(&dir, "a.txt", b"data");
        let added =
            serde_json::to_string(&session.add_attachment(&id, &source, keep_going).unwrap())
                .unwrap();
        let details = serde_json::to_string(&session.get_item(&id).unwrap()).unwrap();
        let temp = dir.path().to_string_lossy().into_owned();
        for json in [added, details] {
            assert!(!json.contains(&temp), "{json}");
            assert!(!json.contains("nvault"), "{json}");
        }
    }

    #[test]
    fn debug_shows_only_lock_state() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();
        let printed = format!("{session:?}");
        assert!(printed.contains("unlocked: true"), "{printed}");
        assert!(!printed.contains("vault.nvault"), "{printed}");
    }
}
