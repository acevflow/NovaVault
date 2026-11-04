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

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use novavault_core::{ItemContent, KdfParams, RecoveryCode, Vault};
use secrecy::{ExposeSecret, SecretSlice, SecretString};

use crate::dto::{
    Credential, ItemInput, ItemSummaryDto, RecoveryCodeDto, VaultStatusDto, parse_item_id,
};
use crate::error::{CommandError, CommandResult};

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
}

impl Session {
    /// A locked session for the vault at `path`, creating key slots with
    /// `params`.
    pub fn new(path: PathBuf, params: KdfParams, now: Instant) -> Self {
        Self {
            path,
            params,
            vault: None,
            last_activity: now,
        }
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

    /// Locks the vault if it has been unlocked and idle for at least
    /// `timeout` at `now`. Returns whether it locked.
    pub fn lock_if_idle(&mut self, now: Instant, timeout: Duration) -> bool {
        self.is_unlocked()
            && now.saturating_duration_since(self.last_activity) >= timeout
            && self.lock()
    }

    /// Changes the master password. The recovery code keeps working.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`], [`CommandError::InvalidInput`] for an
    /// empty password, or a save failure.
    pub fn change_master_password(&mut self, new_password: &SecretString) -> CommandResult<()> {
        let new_password = password_bytes(new_password)?;
        let params = self.params;
        self.vault_mut()?.change_password(&new_password, params)?;
        Ok(())
    }

    /// Replaces the recovery code; the previous one stops working. Returns
    /// the new code, which the UI must show once.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`] or a save failure.
    pub fn regenerate_recovery_code(&mut self) -> CommandResult<RecoveryCodeDto> {
        let params = self.params;
        let code = self.vault_mut()?.regenerate_recovery_code(params)?;
        Ok(RecoveryCodeDto::from(&code))
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

    /// Replaces the content of item `id` and saves.
    ///
    /// # Errors
    ///
    /// [`CommandError::Locked`], [`CommandError::InvalidInput`] for a
    /// malformed identifier, [`CommandError::ItemNotFound`],
    /// [`CommandError::InvalidItem`] or a save failure.
    pub fn update_item(&mut self, id: &str, item: ItemInput) -> CommandResult<()> {
        let vault = self.vault_mut()?;
        let id = parse_item_id(id)?;
        vault.update(id, ItemContent::from(item))?;
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

    fn vault(&self) -> CommandResult<&Vault> {
        self.vault.as_ref().ok_or(CommandError::Locked)
    }

    fn vault_mut(&mut self) -> CommandResult<&mut Vault> {
        self.vault.as_mut().ok_or(CommandError::Locked)
    }
}

impl core::fmt::Debug for Session {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Session")
            .field("unlocked", &self.is_unlocked())
            .finish_non_exhaustive()
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
            session.change_master_password(&s("")),
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
            session.update_item(&id, note("a", "b")),
            Err(CommandError::Locked)
        );
        assert_eq!(session.delete_item(&id), Err(CommandError::Locked));
        assert_eq!(
            session.change_master_password(&s("new")),
            Err(CommandError::Locked)
        );
        assert_eq!(
            session.regenerate_recovery_code().err(),
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
            .update_item(&keep, note("Journal", "second"))
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
    fn unknown_or_malformed_item_ids_are_reported() {
        let dir = TempDir::new();
        let mut session = session(&dir);
        session.create(&s(PASSWORD)).unwrap();
        let missing = "00".repeat(16);
        assert_eq!(
            session.update_item(&missing, note("a", "b")),
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
        session.change_master_password(&s("new password")).unwrap();
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
        let new = code_text(&session.regenerate_recovery_code().unwrap());
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
    fn idle_sessions_lock_after_the_timeout() {
        let dir = TempDir::new();
        let start = Instant::now();
        let timeout = Duration::from_secs(300);
        let mut session = Session::new(dir.path().join("vault.nvault"), fast_params(), start);
        // Nothing to lock yet.
        assert!(!session.lock_if_idle(start + timeout, timeout));

        session.create(&s(PASSWORD)).unwrap();
        session.touch(start);
        assert!(!session.lock_if_idle(start + timeout - Duration::from_secs(1), timeout));
        // Activity restarts the timer.
        session.touch(start + Duration::from_secs(200));
        assert!(!session.lock_if_idle(start + timeout, timeout));
        assert!(session.is_unlocked());
        assert!(session.lock_if_idle(start + Duration::from_secs(500), timeout));
        assert!(!session.is_unlocked());
        // A clock reading before the last activity never panics or locks.
        assert!(!session.lock_if_idle(start, timeout));
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
