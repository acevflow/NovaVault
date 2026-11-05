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
//!
//! # Calling convention
//!
//! Every handler takes `app: AppHandle`, which Tauri injects; the UI never
//! sends it. The other arguments come from the UI, under their camelCase
//! names in JavaScript (`currentCredential`, `newPassword`, `itemId`,
//! `attachmentId`, `onProgress`). Secret arguments deserialize straight
//! into `SecretString` (see `dto`), so they are wiped once the command
//! ends.
//!
//! A failed command returns a [`CommandError`]. Besides the errors listed
//! on each command, any command can return [`CommandError::Internal`] for
//! an unexpected failure, such as a worker thread panic or a failing random
//! number generator; the details are dropped, never sent to the UI.
//!
//! # Attachments
//!
//! The attachment commands open the native file dialog themselves, in Rust,
//! so the UI never sends a file system path (a compromised webview cannot
//! make the shell read or write a file the user did not pick) and never
//! receives one. The dialog is shown *before* the session is locked for the
//! transfer, so an open dialog never holds up the auto-lock. The transfer
//! then runs with the session locked, reporting progress over a channel.
//! Locking the vault by hand or on an OS event asks a running transfer to
//! stop at its next chunk (see [`AppState::with_session_interrupting_transfers`]).

use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use secrecy::SecretString;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::clipboard::ClipboardManager;
use crate::dto::{
    AttachmentDto, CopyResultDto, Credential, ItemDetailsDto, ItemField, ItemInput, ItemSummaryDto,
    ItemUpdate, RecoveryCodeDto, RevealedValueDto, TransferProgressDto, VaultStatusDto,
};
use crate::error::{CommandError, CommandResult};
use crate::session::Session;
use crate::settings::Settings;

/// Minimum time between two progress messages, so a fast transfer does
/// not flood the webview.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// Shared app state: the single vault session, managed by Tauri.
///
/// One mutex serializes every command, the idle timer and the OS lock
/// events. Commands are short and user-driven, so contention is rare, and
/// a lock can never run halfway through another command.
pub struct AppState {
    /// The vault session; reached only through [`AppState::with_session`].
    session: Mutex<Session>,
    /// Set while a lock request waits for the session, so a running
    /// attachment transfer stops at its next chunk instead of delaying the
    /// lock until it finishes.
    interrupt_transfer: AtomicBool,
}

impl AppState {
    /// Wraps `session` for use as Tauri managed state.
    pub fn new(session: Session) -> Self {
        Self {
            session: Mutex::new(session),
            interrupt_transfer: AtomicBool::new(false),
        }
    }

    /// Like [`AppState::with_session`], but first asks a running
    /// attachment transfer to stop, so `action` (a lock) runs within one
    /// chunk's time rather than after the whole transfer.
    pub fn with_session_interrupting_transfers<T>(
        &self,
        action: impl FnOnce(&mut Session) -> T,
    ) -> T {
        self.interrupt_transfer.store(true, Ordering::SeqCst);
        let result = self.with_session(action);
        self.interrupt_transfer.store(false, Ordering::SeqCst);
        result
    }

    /// Whether a lock request is waiting for a transfer to stop.
    fn transfer_interrupted(&self) -> bool {
        self.interrupt_transfer.load(Ordering::SeqCst)
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
///
/// Blocking, because Argon2id and file I/O must not stall Tauri's async
/// workers or the main thread. Every command, even a read-only one, resets
/// the inactivity timer: it shows that someone is using the app.
///
/// # Errors
///
/// Whatever `action` returns, or [`CommandError::Internal`] if the worker
/// thread panics.
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
    // A join error carries the panic payload; dropped so no text reaches
    // the UI.
    .map_err(|_| CommandError::Internal)?
}

/// Reports whether a vault exists and whether it is unlocked, so the UI
/// can choose between the create and unlock screens.
///
/// # Returns
///
/// A [`VaultStatusDto`].
///
/// # Errors
///
/// [`CommandError::Storage`] if the vault location cannot be checked.
#[tauri::command]
pub async fn vault_status(app: AppHandle) -> CommandResult<VaultStatusDto> {
    run(app, |session| session.status()).await
}

/// Creates a new vault protected by `password` and leaves it unlocked.
///
/// # Arguments
///
/// - `password`: the new master password; must not be empty.
///
/// # Returns
///
/// The recovery code, which the UI must show exactly once: it is stored
/// nowhere else.
///
/// # Errors
///
/// [`CommandError::InvalidInput`] for an empty password,
/// [`CommandError::VaultExists`] if a vault already exists, or
/// [`CommandError::Storage`] if the file cannot be written.
#[tauri::command]
pub async fn create_vault(
    app: AppHandle,
    password: SecretString,
) -> CommandResult<RecoveryCodeDto> {
    run(app, move |session| session.create(&password)).await
}

/// Unlocks the vault with the master password or the recovery code.
///
/// The vault key stays in Rust; the UI only learns that unlocking worked.
///
/// # Arguments
///
/// - `credential`: the master password or the recovery code.
///
/// # Errors
///
/// [`CommandError::VaultNotFound`] if no vault exists,
/// [`CommandError::InvalidRecoveryCode`] for a malformed code,
/// [`CommandError::InvalidCredentials`] for a wrong secret, or
/// [`CommandError::Storage`], [`CommandError::UnsupportedVault`] or
/// [`CommandError::Corrupted`] if the vault file cannot be opened. On any
/// error the vault is left locked.
#[tauri::command]
pub async fn unlock(app: AppHandle, credential: Credential) -> CommandResult<()> {
    run(app, move |session| session.unlock(&credential)).await
}

/// Locks the vault, wiping the vault key from memory, and clears a copied
/// field from the clipboard. A running attachment transfer is stopped.
///
/// Locking an already locked vault succeeds.
///
/// # Errors
///
/// Only [`CommandError::Internal`].
#[tauri::command]
pub async fn lock(app: AppHandle) -> CommandResult<()> {
    // Clear first: it does not need the session, so a copied secret goes
    // away even while the lock waits for a transfer to stop.
    app.state::<ClipboardManager>().clear();
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<AppState>()
            .with_session_interrupting_transfers(Session::lock)
    })
    .await
    .map_err(|_| CommandError::Internal)?;
    Ok(())
}

/// Changes the master password of the unlocked vault, after checking the
/// current password or recovery code. Only the vault key is re-wrapped;
/// the recovery code keeps working.
///
/// # Arguments
///
/// - `current_credential`: the current password, or the recovery code for
///   the forgotten-password flow. Asked again so that someone at an
///   unattended, unlocked app cannot take the vault over.
/// - `new_password`: the new master password; must not be empty.
///
/// # Errors
///
/// [`CommandError::Locked`], [`CommandError::InvalidCredentials`] or
/// [`CommandError::InvalidRecoveryCode`] if `current_credential` is wrong,
/// [`CommandError::InvalidInput`] for an empty password, or
/// [`CommandError::Storage`] if the vault file cannot be saved.
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
/// recovery code. The previous code stops working.
///
/// # Arguments
///
/// - `current_credential`: the current password or recovery code, as for
///   [`change_master_password`].
///
/// # Returns
///
/// The new recovery code, which the UI must show exactly once.
///
/// # Errors
///
/// [`CommandError::Locked`], the errors of a wrong `current_credential`
/// (see [`change_master_password`]), or [`CommandError::Storage`] if the
/// vault file cannot be saved.
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
///
/// # Returns
///
/// The current [`Settings`].
///
/// # Errors
///
/// Only [`CommandError::Internal`].
#[tauri::command]
pub async fn get_settings(app: AppHandle) -> CommandResult<Settings> {
    run(app, |session| Ok(session.settings())).await
}

/// Sets the inactivity auto-lock timeout and saves it.
///
/// Requires an unlocked vault, so that someone at a locked app cannot
/// lengthen the timeout before the owner unlocks it.
///
/// # Arguments
///
/// - `minutes`: the new timeout, 1 to 60.
///
/// # Errors
///
/// [`CommandError::Locked`], [`CommandError::InvalidInput`] if `minutes`
/// is out of bounds, or [`CommandError::Storage`] if the settings file
/// cannot be written (the previous value is then kept).
#[tauri::command]
pub async fn set_auto_lock_timeout(app: AppHandle, minutes: u32) -> CommandResult<()> {
    run(app, move |session| session.set_auto_lock_minutes(minutes)).await
}

/// Sets how long a copied field stays on the clipboard, and saves it.
///
/// Requires an unlocked vault, like [`set_auto_lock_timeout`].
///
/// # Arguments
///
/// - `seconds`: the new delay, 5 to 300.
///
/// # Errors
///
/// [`CommandError::Locked`], [`CommandError::InvalidInput`] if `seconds`
/// is out of bounds, or [`CommandError::Storage`] if the settings file
/// cannot be written (the previous value is then kept).
#[tauri::command]
pub async fn set_clipboard_clear_delay(app: AppHandle, seconds: u32) -> CommandResult<()> {
    run(app, move |session| {
        session.set_clipboard_clear_seconds(seconds)
    })
    .await
}

/// Copies one field of item `id` to the clipboard and schedules it to be
/// cleared.
///
/// # Arguments
///
/// - `id`: the item identifier.
/// - `field`: the field to copy, secret or plain.
///
/// # Returns
///
/// How long the clipboard will hold the value. The value itself never
/// comes back to the UI.
///
/// # Errors
///
/// [`CommandError::Locked`], [`CommandError::InvalidInput`] for a
/// malformed identifier or a field this kind of item does not have,
/// [`CommandError::ItemNotFound`], [`CommandError::Corrupted`], or
/// [`CommandError::ClipboardUnavailable`] if the clipboard cannot be
/// written.
#[tauri::command]
pub async fn copy_item_field(
    app: AppHandle,
    id: String,
    field: ItemField,
) -> CommandResult<CopyResultDto> {
    let worker = app.clone();
    // The value goes from the session straight to the clipboard thread, in
    // zeroizing storage; it is never serialized for the webview.
    let (text, settings) = run(app, move |session| {
        Ok((session.field_text(&id, field)?, session.settings()))
    })
    .await?;
    // Outside the session lock: the clipboard may be slow to answer.
    tauri::async_runtime::spawn_blocking(move || {
        worker
            .state::<ClipboardManager>()
            .copy(text, settings.clipboard_clear_delay())
    })
    .await
    .map_err(|_| CommandError::Internal)??;
    Ok(CopyResultDto {
        clear_after_seconds: settings.clipboard_clear_seconds(),
    })
}

/// Lists the items of the unlocked vault, without secret fields.
///
/// # Returns
///
/// One [`ItemSummaryDto`] per item: identifier, kind, title and
/// timestamps.
///
/// # Errors
///
/// [`CommandError::Locked`] or [`CommandError::Corrupted`].
#[tauri::command]
pub async fn list_items(app: AppHandle) -> CommandResult<Vec<ItemSummaryDto>> {
    run(app, |session| session.list_items()).await
}

/// Adds an item and saves the vault.
///
/// # Arguments
///
/// - `item`: the new item's content, tagged by `kind`.
///
/// # Returns
///
/// The new item's identifier.
///
/// # Errors
///
/// [`CommandError::Locked`], [`CommandError::InvalidItem`] if the item
/// cannot be stored, or [`CommandError::Storage`] if the vault file cannot
/// be saved.
#[tauri::command]
pub async fn create_item(app: AppHandle, item: ItemInput) -> CommandResult<String> {
    run(app, move |session| session.create_item(item)).await
}

/// Returns item `id` without its secret values.
///
/// # Arguments
///
/// - `id`: the item identifier.
///
/// # Returns
///
/// The item's plain fields, attachments and, for each secret field, only
/// whether it is set. Secrets come one at a time from
/// [`reveal_item_field`].
///
/// # Errors
///
/// [`CommandError::Locked`], [`CommandError::InvalidInput`] for a
/// malformed identifier, [`CommandError::ItemNotFound`] or
/// [`CommandError::Corrupted`].
#[tauri::command]
pub async fn get_item(app: AppHandle, id: String) -> CommandResult<ItemDetailsDto> {
    run(app, move |session| session.get_item(&id)).await
}

/// Returns one secret field of item `id`, because the user asked to see it.
///
/// This is the only command that sends an item secret to the webview, and
/// only one field per call.
///
/// # Arguments
///
/// - `id`: the item identifier.
/// - `field`: a secret field of that item's kind.
///
/// # Returns
///
/// The field's value.
///
/// # Errors
///
/// [`CommandError::InvalidInput`] if `field` is a plain field or not a
/// field of this kind of item, and the errors of [`get_item`].
#[tauri::command]
pub async fn reveal_item_field(
    app: AppHandle,
    id: String,
    field: ItemField,
) -> CommandResult<RevealedValueDto> {
    run(app, move |session| session.reveal_field(&id, field)).await
}

/// Updates item `id` and saves the vault.
///
/// # Arguments
///
/// - `id`: the item identifier.
/// - `item`: the new content. Its `kind` must match the stored item's;
///   secret fields sent as `null` (or left out) keep their values, so the
///   edit form never needs a secret the user did not reveal.
///
/// # Errors
///
/// [`CommandError::Locked`], [`CommandError::InvalidInput`] for a
/// malformed identifier or a different kind,
/// [`CommandError::ItemNotFound`], [`CommandError::InvalidItem`] or
/// [`CommandError::Storage`].
#[tauri::command]
pub async fn update_item(app: AppHandle, id: String, item: ItemUpdate) -> CommandResult<()> {
    run(app, move |session| session.update_item(&id, item)).await
}

/// Deletes item `id`, with its attachments, and saves the vault.
///
/// # Arguments
///
/// - `id`: the item identifier.
///
/// # Errors
///
/// [`CommandError::Locked`], [`CommandError::InvalidInput`] for a
/// malformed identifier, [`CommandError::ItemNotFound`] or
/// [`CommandError::Storage`].
#[tauri::command]
pub async fn delete_item(app: AppHandle, id: String) -> CommandResult<()> {
    run(app, move |session| session.delete_item(&id)).await
}

/// Asks the user for a file in a native dialog, encrypts it as a new
/// attachment of item `item_id`, and saves the vault.
///
/// # Arguments
///
/// - `item_id`: the item to attach the file to.
/// - `on_progress`: receives [`TransferProgressDto`] updates, throttled.
///
/// # Returns
///
/// The new attachment, or `None` if the user closed the dialog.
///
/// # Errors
///
/// [`CommandError::Locked`], [`CommandError::InvalidInput`] or
/// [`CommandError::ItemNotFound`] before the dialog opens;
/// [`CommandError::FileAccess`] if the dialog fails or the file cannot be
/// read, [`CommandError::InvalidItem`] if the item has too many
/// attachments or the name is unusable, [`CommandError::Cancelled`] if a
/// lock stopped the transfer, or [`CommandError::Storage`].
#[tauri::command]
pub async fn add_attachment(
    app: AppHandle,
    item_id: String,
    on_progress: Channel<TransferProgressDto>,
) -> CommandResult<Option<AttachmentDto>> {
    // Fail before showing a dialog if the vault is locked or the item gone.
    let id = item_id.clone();
    run(app.clone(), move |session| session.check_item(&id)).await?;
    let Some(source) = pick_file(app.clone()).await? else {
        return Ok(None);
    };
    let reporter_app = app.clone();
    run(app, move |session| {
        // Only for the progress bar: the size actually encrypted is
        // whatever the file holds when it is read.
        let total = std::fs::metadata(&source).map_or(0, |metadata| metadata.len());
        let mut reporter = ProgressReporter::new(reporter_app, on_progress, total);
        let result = session.add_attachment(&item_id, &source, |done| reporter.report(done));
        // The transfer was activity: do not count its duration as idle.
        session.touch(Instant::now());
        result.map(Some)
    })
    .await
}

/// Asks the user where to save attachment `attachment_id` of item
/// `item_id`, suggesting its file name, and decrypts it there.
///
/// # Arguments
///
/// - `item_id`: the item that holds the attachment.
/// - `attachment_id`: the attachment to export.
/// - `on_progress`: receives [`TransferProgressDto`] updates, throttled.
///
/// # Returns
///
/// `true` once the file is written, or `false` if the user closed the
/// dialog.
///
/// # Errors
///
/// [`CommandError::Locked`], [`CommandError::InvalidInput`],
/// [`CommandError::ItemNotFound`] or [`CommandError::AttachmentNotFound`]
/// before the dialog opens; [`CommandError::FileAccess`] if the dialog
/// fails or the destination cannot be written (or is one of the vault's
/// own files), [`CommandError::Corrupted`] if the encrypted data is
/// missing or damaged, or [`CommandError::Cancelled`] if a lock stopped
/// the transfer. Nothing is left at the destination if the data fails
/// authentication.
#[tauri::command]
pub async fn export_attachment(
    app: AppHandle,
    item_id: String,
    attachment_id: String,
    on_progress: Channel<TransferProgressDto>,
) -> CommandResult<bool> {
    let (item, attachment) = (item_id.clone(), attachment_id.clone());
    let (name, size) = run(app.clone(), move |session| {
        session.attachment_info(&item, &attachment)
    })
    .await?;
    let Some(destination) = save_file(app.clone(), name).await? else {
        return Ok(false);
    };
    let reporter_app = app.clone();
    run(app, move |session| {
        let mut reporter = ProgressReporter::new(reporter_app, on_progress, size);
        let result = session.export_attachment(&item_id, &attachment_id, &destination, |done| {
            reporter.report(done)
        });
        session.touch(Instant::now());
        result.map(|()| true)
    })
    .await
}

/// Removes attachment `attachment_id` from item `item_id`, saves the vault
/// and deletes the attachment's encrypted data.
///
/// # Arguments
///
/// - `item_id`: the item that holds the attachment.
/// - `attachment_id`: the attachment to remove.
///
/// # Errors
///
/// [`CommandError::Locked`], [`CommandError::InvalidInput`],
/// [`CommandError::ItemNotFound`], [`CommandError::AttachmentNotFound`] or
/// [`CommandError::Storage`].
#[tauri::command]
pub async fn delete_attachment(
    app: AppHandle,
    item_id: String,
    attachment_id: String,
) -> CommandResult<()> {
    run(app, move |session| {
        session.delete_attachment(&item_id, &attachment_id)
    })
    .await
}

/// Shows the native "open file" dialog on a blocking thread. The session
/// is not locked meanwhile, so the auto-lock keeps working.
///
/// # Errors
///
/// [`CommandError::FileAccess`] if the picked entry is not a file system
/// path, or [`CommandError::Internal`].
async fn pick_file(app: AppHandle) -> CommandResult<Option<PathBuf>> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = app.dialog().file().set_title("Attach a file");
        if let Some(window) = app.get_webview_window("main") {
            dialog = dialog.set_parent(&window);
        }
        dialog.blocking_pick_file().map(|path| path.into_path())
    })
    .await
    .map_err(|_| CommandError::Internal)?
    .transpose()
    .map_err(|_| CommandError::FileAccess)
}

/// Shows the native "save file" dialog, suggesting `file_name`, on a
/// blocking thread. The dialog asks before replacing an existing file.
///
/// # Errors
///
/// As for [`pick_file`].
async fn save_file(app: AppHandle, file_name: String) -> CommandResult<Option<PathBuf>> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = app
            .dialog()
            .file()
            .set_title("Export attachment")
            .set_file_name(file_name);
        if let Some(window) = app.get_webview_window("main") {
            dialog = dialog.set_parent(&window);
        }
        dialog.blocking_save_file().map(|path| path.into_path())
    })
    .await
    .map_err(|_| CommandError::Internal)?
    .transpose()
    .map_err(|_| CommandError::FileAccess)
}

/// Forwards transfer progress to the UI, throttled, and stops the transfer
/// when a lock is waiting.
struct ProgressReporter {
    /// To read [`AppState::transfer_interrupted`].
    app: AppHandle,
    /// The UI's progress channel.
    channel: Channel<TransferProgressDto>,
    /// Expected size in bytes, for the progress bar only.
    total: u64,
    /// When the last message was sent; `None` before the first one.
    last_sent: Option<Instant>,
}

impl ProgressReporter {
    /// A reporter for a transfer of about `total` bytes.
    fn new(app: AppHandle, channel: Channel<TransferProgressDto>, total: u64) -> Self {
        Self {
            app,
            channel,
            total,
            last_sent: None,
        }
    }

    /// Called by the core after each chunk with the bytes done so far.
    fn report(&mut self, done: u64) -> ControlFlow<()> {
        if self.app.state::<AppState>().transfer_interrupted() {
            return ControlFlow::Break(());
        }
        let now = Instant::now();
        let due = self
            .last_sent
            .is_none_or(|last| now.saturating_duration_since(last) >= PROGRESS_INTERVAL);
        if due || done >= self.total {
            // A file that grew while it was read reports more than its
            // initial size; never send a total below the bytes done.
            let progress = TransferProgressDto {
                done,
                total: self.total.max(done),
            };
            // Progress is cosmetic: a closed channel must not fail the
            // transfer.
            let _ = self.channel.send(progress);
            self.last_sent = Some(now);
        }
        ControlFlow::Continue(())
    }
}
