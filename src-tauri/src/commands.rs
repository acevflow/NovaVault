//! Tauri command handlers: the whole IPC surface of the app.
//!
//! Each handler only moves its arguments to a blocking worker thread, locks
//! the [`Session`] and delegates to it. Argon2id runs for a noticeable time
//! on unlock, create and password changes, so no command runs on the main
//! thread or an async worker.
//!
//! The UI may call only the commands declared in `build.rs`, which
//! generates one permission per command, and
//! `capabilities/default.json` grants exactly those, so anything else is
//! rejected by Tauri before it reaches Rust code.

use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use secrecy::SecretString;
use tauri::{AppHandle, Manager};

use crate::dto::{
    Credential, ItemDetailsDto, ItemField, ItemInput, ItemSummaryDto, ItemUpdate, RecoveryCodeDto,
    RevealedValueDto, VaultStatusDto,
};
use crate::error::{CommandError, CommandResult};
use crate::session::Session;
use crate::settings::Settings;

/// Shared app state: the single vault session.
pub struct AppState {
    session: Mutex<Session>,
}

impl AppState {
    /// Wraps `session` for use as Tauri managed state.
    pub fn new(session: Session) -> Self {
        Self {
            session: Mutex::new(session),
        }
    }

    /// Runs `action` with exclusive access to the session.
    pub fn with_session<T>(&self, action: impl FnOnce(&mut Session) -> T) -> T {
        // A poisoned lock only means an earlier command panicked. The
        // session is a single `Option<Vault>` plus a timestamp, so it cannot
        // be left half-updated; recovering keeps lock and auto-lock working.
        let mut session = self.session.lock().unwrap_or_else(PoisonError::into_inner);
        action(&mut session)
    }
}

/// Runs a command's logic on a blocking thread, as user activity.
async fn run<T: Send + 'static>(
    app: AppHandle,
    action: impl FnOnce(&mut Session) -> CommandResult<T> + Send + 'static,
) -> CommandResult<T> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<AppState>().with_session(|session| {
            session.touch(Instant::now());
            action(session)
        })
    })
    .await
    .map_err(|_| CommandError::Internal)?
}

/// Reports whether a vault exists and whether it is unlocked.
#[tauri::command]
pub async fn vault_status(app: AppHandle) -> CommandResult<VaultStatusDto> {
    run(app, |session| session.status()).await
}

/// Creates a new vault protected by `password` and leaves it unlocked.
/// Returns the recovery code, which the UI must show exactly once.
#[tauri::command]
pub async fn create_vault(
    app: AppHandle,
    password: SecretString,
) -> CommandResult<RecoveryCodeDto> {
    run(app, move |session| session.create(&password)).await
}

/// Unlocks the vault with the master password or the recovery code.
#[tauri::command]
pub async fn unlock(app: AppHandle, credential: Credential) -> CommandResult<()> {
    run(app, move |session| session.unlock(&credential)).await
}

/// Locks the vault, wiping the vault key from memory.
#[tauri::command]
pub async fn lock(app: AppHandle) -> CommandResult<()> {
    run(app, |session| {
        session.lock();
        Ok(())
    })
    .await
}

/// Changes the master password of the unlocked vault, after checking the
/// current password or recovery code.
#[tauri::command]
pub async fn change_master_password(
    app: AppHandle,
    current_credential: Credential,
    new_password: SecretString,
) -> CommandResult<()> {
    run(app, move |session| {
        session.change_master_password(&current_credential, &new_password)
    })
    .await
}

/// Replaces the recovery code, after checking the current password or
/// recovery code, and returns the new one, to show once.
#[tauri::command]
pub async fn regenerate_recovery_code(
    app: AppHandle,
    current_credential: Credential,
) -> CommandResult<RecoveryCodeDto> {
    run(app, move |session| {
        session.regenerate_recovery_code(&current_credential)
    })
    .await
}

/// Returns the app settings. Works while locked: they are not secret.
#[tauri::command]
pub async fn get_settings(app: AppHandle) -> CommandResult<Settings> {
    run(app, |session| Ok(session.settings())).await
}

/// Sets the inactivity auto-lock timeout, in minutes (1 to 60).
#[tauri::command]
pub async fn set_auto_lock_timeout(app: AppHandle, minutes: u32) -> CommandResult<()> {
    run(app, move |session| session.set_auto_lock_minutes(minutes)).await
}

/// Lists the items of the unlocked vault, without secret fields.
#[tauri::command]
pub async fn list_items(app: AppHandle) -> CommandResult<Vec<ItemSummaryDto>> {
    run(app, |session| session.list_items()).await
}

/// Adds an item and returns its identifier.
#[tauri::command]
pub async fn create_item(app: AppHandle, item: ItemInput) -> CommandResult<String> {
    run(app, move |session| session.create_item(item)).await
}

/// Returns item `id` without its secret values.
#[tauri::command]
pub async fn get_item(app: AppHandle, id: String) -> CommandResult<ItemDetailsDto> {
    run(app, move |session| session.get_item(&id)).await
}

/// Returns one secret field of item `id`, because the user asked to see it.
#[tauri::command]
pub async fn reveal_item_field(
    app: AppHandle,
    id: String,
    field: ItemField,
) -> CommandResult<RevealedValueDto> {
    run(app, move |session| session.reveal_field(&id, field)).await
}

/// Updates item `id`; secret fields sent as `null` keep their values.
#[tauri::command]
pub async fn update_item(app: AppHandle, id: String, item: ItemUpdate) -> CommandResult<()> {
    run(app, move |session| session.update_item(&id, item)).await
}

/// Deletes item `id`.
#[tauri::command]
pub async fn delete_item(app: AppHandle, id: String) -> CommandResult<()> {
    run(app, move |session| session.delete_item(&id)).await
}
