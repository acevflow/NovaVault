//! App settings that live outside the vault.
//!
//! Settings are not secret and must be readable while the vault is locked
//! (the inactivity timeout applies before any unlock), so they are kept in
//! a small JSON file next to the vault rather than inside it.
//!
//! The file is advisory: if it is missing, unreadable or invalid, the
//! defaults apply and the app still starts. Anyone who can write it can
//! already replace the app or the vault file, so it is not authenticated.

use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandResult};

/// Shortest accepted inactivity timeout, in minutes.
pub const MIN_AUTO_LOCK_MINUTES: u32 = 1;

/// Longest accepted inactivity timeout, in minutes. A longer timeout would
/// leave an unattended vault open for too long to be a useful default.
pub const MAX_AUTO_LOCK_MINUTES: u32 = 60;

/// Inactivity timeout used until the user picks another one.
pub const DEFAULT_AUTO_LOCK_MINUTES: u32 = 5;

/// The persisted settings.
///
/// JSON (file and IPC): `{ "autoLockMinutes": 5 }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    /// Minutes without a command before the vault locks itself.
    auto_lock_minutes: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_lock_minutes: DEFAULT_AUTO_LOCK_MINUTES,
        }
    }
}

impl Settings {
    /// The inactivity timeout as a duration.
    pub fn auto_lock_timeout(self) -> Duration {
        Duration::from_secs(u64::from(self.auto_lock_minutes) * 60)
    }

    /// Sets the inactivity timeout.
    ///
    /// # Errors
    ///
    /// [`CommandError::InvalidInput`] if `minutes` is outside
    /// [`MIN_AUTO_LOCK_MINUTES`]..=[`MAX_AUTO_LOCK_MINUTES`].
    pub fn set_auto_lock_minutes(&mut self, minutes: u32) -> CommandResult<()> {
        if !(MIN_AUTO_LOCK_MINUTES..=MAX_AUTO_LOCK_MINUTES).contains(&minutes) {
            return Err(CommandError::InvalidInput);
        }
        self.auto_lock_minutes = minutes;
        Ok(())
    }

    /// Reads the settings at `path`, falling back to the defaults if the
    /// file is missing, unreadable, malformed or out of bounds.
    pub fn load(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(|settings| {
                (MIN_AUTO_LOCK_MINUTES..=MAX_AUTO_LOCK_MINUTES)
                    .contains(&settings.auto_lock_minutes)
            })
            .unwrap_or_default()
    }

    /// Writes the settings to `path`, replacing the file in one rename so a
    /// crash never leaves a half-written file behind.
    ///
    /// # Errors
    ///
    /// Any I/O error from writing or renaming the file.
    pub fn save(self, path: &Path) -> io::Result<()> {
        let bytes = serde_json::to_vec_pretty(&self).map_err(io::Error::other)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, bytes)?;
        fs::rename(&temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path() -> std::path::PathBuf {
        let name = format!(
            "novavault-settings-test-{}",
            novavault_core::ItemId::generate().unwrap()
        );
        std::env::temp_dir().join(name).join("settings.json")
    }

    #[test]
    fn bounds_are_enforced() {
        let mut settings = Settings::default();
        assert_eq!(settings.auto_lock_minutes, DEFAULT_AUTO_LOCK_MINUTES);
        for bad in [0, MAX_AUTO_LOCK_MINUTES + 1, u32::MAX] {
            assert_eq!(
                settings.set_auto_lock_minutes(bad),
                Err(CommandError::InvalidInput)
            );
        }
        assert_eq!(settings.auto_lock_minutes, DEFAULT_AUTO_LOCK_MINUTES);
        for good in [MIN_AUTO_LOCK_MINUTES, 15, MAX_AUTO_LOCK_MINUTES] {
            settings.set_auto_lock_minutes(good).unwrap();
            assert_eq!(settings.auto_lock_minutes, good);
        }
        assert_eq!(settings.auto_lock_timeout(), Duration::from_secs(3600));
    }

    #[test]
    fn saves_and_loads() {
        let path = temp_path();
        let mut settings = Settings::default();
        settings.set_auto_lock_minutes(30).unwrap();
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), settings);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn invalid_files_fall_back_to_defaults() {
        let path = temp_path();
        assert_eq!(Settings::load(&path), Settings::default(), "missing file");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        for content in [
            "not json",
            r#"{"autoLockMinutes":0}"#,
            r#"{"autoLockMinutes":999}"#,
            r#"{"autoLockMinutes":"5"}"#,
            r#"{"autoLockMinutes":5,"extra":true}"#,
        ] {
            fs::write(&path, content).unwrap();
            assert_eq!(Settings::load(&path), Settings::default(), "{content}");
        }
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }
}
