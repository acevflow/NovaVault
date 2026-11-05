# [`src-tauri/src/commands.rs`](../../../../src-tauri/src/commands.rs)

## Purpose

The Tauri command handlers: the whole IPC surface between the webview and Rust. Each handler moves its arguments to a blocking worker thread, takes the single session mutex and delegates to [`Session`](session.rs.md). Only the 19 commands declared in `build.rs` are callable, and `capabilities/default.json` grants exactly those (checked by a test in [`main.rs`](main.rs.md#tests)). The file also defines `AppState`, the managed state that serializes every command, the idle timer and the OS lock events. The attachment commands open the native file dialogs themselves, so no file system path ever crosses the boundary.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `AppState` | struct | `pub struct AppState` | The session mutex and the transfer interrupt flag. |
| `AppState::new` | function | `pub fn new(session: Session) -> Self` | Wraps a session. |
| `AppState::with_session_interrupting_transfers` | method | `pub fn with_session_interrupting_transfers<T>(&self, action: impl FnOnce(&mut Session) -> T) -> T` | Asks a transfer to stop, then runs `action`. |
| `AppState::with_session` | method | `pub fn with_session<T>(&self, action: impl FnOnce(&mut Session) -> T) -> T` | Runs `action` with exclusive access. |
| `vault_status` | command | `pub async fn vault_status(app: AppHandle) -> CommandResult<VaultStatusDto>` | Exists / unlocked. |
| `create_vault` | command | `pub async fn create_vault(app: AppHandle, password: SecretString) -> CommandResult<RecoveryCodeDto>` | Creates a vault. |
| `unlock` | command | `pub async fn unlock(app: AppHandle, credential: Credential) -> CommandResult<()>` | Unlocks. |
| `lock` | command | `pub async fn lock(app: AppHandle) -> CommandResult<()>` | Locks and clears the clipboard. |
| `change_master_password` | command | `pub async fn change_master_password(app: AppHandle, current_credential: Credential, new_password: SecretString) -> CommandResult<()>` | Re-wraps the password slot. |
| `regenerate_recovery_code` | command | `pub async fn regenerate_recovery_code(app: AppHandle, current_credential: Credential) -> CommandResult<RecoveryCodeDto>` | Replaces the recovery slot. |
| `get_settings` | command | `pub async fn get_settings(app: AppHandle) -> CommandResult<Settings>` | Current settings. |
| `set_auto_lock_timeout` | command | `pub async fn set_auto_lock_timeout(app: AppHandle, minutes: u32) -> CommandResult<()>` | Sets the timeout. |
| `set_clipboard_clear_delay` | command | `pub async fn set_clipboard_clear_delay(app: AppHandle, seconds: u32) -> CommandResult<()>` | Sets the clear delay. |
| `copy_item_field` | command | `pub async fn copy_item_field(app: AppHandle, id: String, field: ItemField) -> CommandResult<CopyResultDto>` | Copies a field in Rust. |
| `list_items` | command | `pub async fn list_items(app: AppHandle) -> CommandResult<Vec<ItemSummaryDto>>` | Item summaries. |
| `create_item` | command | `pub async fn create_item(app: AppHandle, item: ItemInput) -> CommandResult<String>` | Adds an item. |
| `get_item` | command | `pub async fn get_item(app: AppHandle, id: String) -> CommandResult<ItemDetailsDto>` | Item details, no secrets. |
| `reveal_item_field` | command | `pub async fn reveal_item_field(app: AppHandle, id: String, field: ItemField) -> CommandResult<RevealedValueDto>` | One secret value. |
| `update_item` | command | `pub async fn update_item(app: AppHandle, id: String, item: ItemUpdate) -> CommandResult<()>` | Updates an item. |
| `delete_item` | command | `pub async fn delete_item(app: AppHandle, id: String) -> CommandResult<()>` | Deletes an item. |
| `add_attachment` | command | `pub async fn add_attachment(app: AppHandle, item_id: String, on_progress: Channel<TransferProgressDto>) -> CommandResult<Option<AttachmentDto>>` | Picks and encrypts a file. |
| `export_attachment` | command | `pub async fn export_attachment(app: AppHandle, item_id: String, attachment_id: String, on_progress: Channel<TransferProgressDto>) -> CommandResult<bool>` | Picks a destination and decrypts. |
| `delete_attachment` | command | `pub async fn delete_attachment(app: AppHandle, item_id: String, attachment_id: String) -> CommandResult<()>` | Removes an attachment. |

Every command is `#[tauri::command]`.

## Details

### Calling convention and boundary rules

- **Arguments**: `app: AppHandle` is injected by Tauri; the UI never sends it. The other parameters are sent by the UI under camelCase names (`currentCredential`, `newPassword`, `itemId`, `attachmentId`, `onProgress`). Arguments that do not match the expected shape are rejected by Tauri before the handler runs.
- **Secrets in**: passwords, recovery codes and secret item fields deserialize straight into `SecretString` (see [`dto`](dto.rs.md)) and are wiped when the command ends.
- **Errors**: a failed command returns a [`CommandError`](error.rs.md), serialized as `{ code, message }` with a fixed message. On top of the errors listed per command, every command can return `Internal`, for example when the worker thread panics. The panic payload is dropped.
- **Never crosses to the UI**, from any command:
  - the vault key, derived keys and key slots;
  - the master password or recovery code once sent (the exception is a newly generated recovery code, below);
  - any file system path (vault, settings, blobs, picked files);
  - attachment contents;
  - core error details;
  - item secret values, except through `reveal_item_field`.
- **Activity**: every command that goes through `run` touches the session first, so it counts as activity for the inactivity timer, read-only ones included.

### `AppState`

| Field | Type | Meaning |
| ----- | ---- | ------- |
| `session` | `Mutex<Session>` | The single session. One mutex serializes every command, the idle timer and the OS lock events, so a lock can never run halfway through another command. |
| `interrupt_transfer` | `AtomicBool` | Set while a lock waits for the session, so a running transfer stops at its next chunk. |

#### `AppState::with_session`

Locks the mutex and runs `action`. A poisoned mutex is recovered (`PoisonError::into_inner`). The source argues that the session cannot be left half-updated, and recovering keeps locking working after a panic.

#### `AppState::with_session_interrupting_transfers`

Sets `interrupt_transfer`, runs `with_session(action)`, then clears the flag. Used by the `lock` command and by OS-event locks in [`autolock`](autolock.rs.md#lock_and_reset_ui).

#### `transfer_interrupted` (private)

Reads the flag. Called by `ProgressReporter::report` after each chunk.

### `run` (private)

`async fn run<T: Send + 'static>(app: AppHandle, action: impl FnOnce(&mut Session) -> CommandResult<T> + Send + 'static) -> CommandResult<T>`

On `tauri::async_runtime::spawn_blocking`: takes the session via `with_session`, calls `session.touch(Instant::now())`, then runs `action`. A join error (panic) becomes `Internal`. The work is blocking because Argon2id and file I/O must not stall Tauri's async workers or the main thread.

### `vault_status`

- **Arguments**: none.
- **Returns**: `VaultStatusDto`, `{ exists, unlocked }`.
- **Errors**: `Storage` if the vault location cannot be checked.
- **Boundary**: two booleans. The vault path is not returned.

### `create_vault`

- **Arguments**: `password` (JS `password`, `string`) → `SecretString`. Must not be empty.
- **Returns**: `RecoveryCodeDto`, `{ recoveryCode }`. The UI must show it once, because it is stored nowhere else.
- **Errors**: `InvalidInput` (empty password), `VaultExists`, `Storage`.
- **Boundary**: the recovery code is the one credential deliberately sent to the UI. The password, the vault key and both key slots stay in Rust.

### `unlock`

- **Arguments**: `credential` (JS `credential`, `{ method: "password" | "recoveryCode", secret }`) → `Credential`.
- **Returns**: nothing (`null`).
- **Errors**:
  - `VaultNotFound`;
  - `InvalidInput` (empty password, not listed in the doc comment);
  - `InvalidRecoveryCode`;
  - `InvalidCredentials`;
  - `Storage`, `UnsupportedVault` or `Corrupted`.

  On any error the vault is locked.
- **Boundary**: the UI learns only success or a fixed error code. The vault key stays in the core's `Vault`.

### `lock`

1. Calls `ClipboardManager::clear()` first. It does not need the session, so a copied secret goes away even while the lock waits for a transfer to stop.
2. On a blocking thread, runs `with_session_interrupting_transfers(Session::lock)`.

- **Arguments**: none.
- **Returns**: nothing. Locking an already locked vault succeeds.
- **Errors**: only `Internal` (join error).
- **Boundary**: nothing is returned. The command does not reload the webview; see [`autolock`](autolock.rs.md#notes).
- **Edge cases and invariants**: does not go through `run`, so it does not touch the activity timestamp.

### `change_master_password`

- **Arguments**:
  - `current_credential` (JS `currentCredential`) → `Credential`: the current password, or the recovery code for the forgotten-password flow. It is asked again so that someone at an unattended, unlocked app cannot take the vault over.
  - `new_password` (JS `newPassword`) → `SecretString`, non-empty.
- **Returns**: nothing.
- **Errors**:
  - `Locked`;
  - `InvalidCredentials` or `InvalidRecoveryCode`;
  - `InvalidInput` for an empty current or new password;
  - `Storage`.
- **Boundary**: nothing is returned. Only the password slot is re-wrapped, and the recovery code keeps working.

### `regenerate_recovery_code`

- **Arguments**: `current_credential` (JS `currentCredential`) → `Credential`.
- **Returns**: `RecoveryCodeDto` with the new code, to show once. The previous code stops working.
- **Errors**: `Locked`, `InvalidCredentials`, `InvalidRecoveryCode`, `InvalidInput` (empty current password), `Storage`.
- **Boundary**: as for `create_vault`, the new code is the only secret returned.

### `get_settings`

- **Arguments**: none.
- **Returns**: `Settings`, `{ autoLockMinutes, clipboardClearSeconds }`. Works while locked, because the settings are not secret.
- **Errors**: only `Internal`.

### `set_auto_lock_timeout`

- **Arguments**: `minutes` (JS `minutes`, unsigned integer) → `u32`, 1 to 60.
- **Returns**: nothing.
- **Errors**: `Locked` (setting it needs an unlocked vault), `InvalidInput` (out of range), `Storage` (the previous value is kept).

### `set_clipboard_clear_delay`

- **Arguments**: `seconds` (JS `seconds`) → `u32`, 5 to 300.
- **Returns**: nothing.
- **Errors**: `Locked`, `InvalidInput`, `Storage`.

### `copy_item_field`

1. Through `run`, gets `session.field_text(&id, field)` (a `Zeroizing<String>`) and the settings.
2. Outside the session lock, because the clipboard may be slow to answer, calls `ClipboardManager::copy(text, settings.clipboard_clear_delay())` on another blocking thread.
3. Returns the delay in seconds.

- **Arguments**:
  - `id` (JS `id`) → `String`, 32 hexadecimal digits;
  - `field` (JS `field`, `"username" | "password" | "url" | "notes" | "body" | "value"`) → `ItemField`, plain or secret.
- **Returns**: `CopyResultDto`, `{ clearAfterSeconds }`.
- **Errors**:
  - `Locked`;
  - `InvalidInput` (identifier, or a field the kind lacks);
  - `ItemNotFound`, `Corrupted`;
  - `ClipboardUnavailable`;
  - `Internal` if the clipboard worker is gone.
- **Boundary**: the value goes from the session to the clipboard thread in zeroizing storage and is never serialized for the webview. See [`clipboard.rs.md`](clipboard.rs.md) for per-OS behaviour.

### `list_items`

- **Arguments**: none.
- **Returns**: `Vec<ItemSummaryDto>`. Each entry is `{ id, kind, title, createdAtMs, updatedAtMs }`.
- **Errors**: `Locked`, `Corrupted`.
- **Boundary**: no secret field, user name or URL.

### `create_item`

- **Arguments**: `item` (JS `item`, tagged by `kind`) → `ItemInput`. Its secret fields are `SecretString`.
- **Returns**: the new item identifier (`string`).
- **Errors**: `Locked`, `InvalidItem`, `Storage`.

### `get_item`

- **Arguments**: `id` → `String`.
- **Returns**: `ItemDetailsDto`, with the plain fields, attachments (`{ id, name, size, addedAtMs }`) and `has…` flags.
- **Errors**: `Locked`, `InvalidInput`, `ItemNotFound`, `Corrupted`.
- **Boundary**: no secret values and no attachment paths.

### `reveal_item_field`

- **Arguments**: `id` → `String`; `field` → `ItemField`, which must be a secret field of the item's kind.
- **Returns**: `RevealedValueDto`, `{ value }`.
- **Errors**: `InvalidInput` (plain field, or a field the kind lacks), plus those of `get_item`.
- **Boundary**: the only command that sends an item secret to the webview, one field per call. Once in JavaScript, the value is outside Rust's zeroization.

### `update_item`

- **Arguments**: `id` → `String`; `item` → `ItemUpdate`, whose `kind` must match the stored item. A secret field that is `null` or absent keeps its stored value.
- **Returns**: nothing.
- **Errors**: `Locked`, `InvalidInput` (identifier or kind), `ItemNotFound`, `InvalidItem`, `Storage`.
- **Boundary**: the edit form never needs to receive a secret it did not reveal.

### `delete_item`

- **Arguments**: `id` → `String`.
- **Returns**: nothing.
- **Errors**: `Locked`, `InvalidInput`, `ItemNotFound`, `Storage`.
- **Edge cases and invariants**: the item's attachment blobs are deleted by the save.

### `add_attachment`

1. `run(check_item)`: fails before showing a dialog if the vault is locked or the item is missing.
2. `pick_file`: a native open dialog on a blocking thread, with the session mutex not held, so the auto-lock keeps working while the dialog is open. Closing it returns `Ok(None)`.
3. `run`:
   1. reads the file size for the progress bar only (0 if unreadable);
   2. calls `Session::add_attachment` with a `ProgressReporter`;
   3. touches the session again afterwards, so the transfer's duration does not count as idle time.

- **Arguments**:
  - `item_id` (JS `itemId`) → `String`;
  - `on_progress` (JS `onProgress`, a `Channel<TransferProgress>` created by the UI) → `Channel<TransferProgressDto>`.
- **Returns**: `Option<AttachmentDto>`: the new attachment, or `null` if the dialog was closed.
- **Errors**:
  - before the dialog: `Locked`, `InvalidInput`, `ItemNotFound`;
  - after it:
    - `FileAccess` (dialog failure, or the file cannot be read);
    - `InvalidInput` (path without a file name; not listed in the doc comment);
    - `InvalidItem`;
    - `Cancelled` (a lock stopped the transfer);
    - `Storage`.
- **Boundary**: the UI never sends or receives a path, so a compromised webview cannot make the shell read a file the user did not pick. The plaintext file contents go from disk to the core's encryptor and never to the webview. Only byte counts are sent over the channel.
- **Edge cases and invariants**: the vault can lock between steps 1 and 3. Step 3 then fails with `Locked`.

### `export_attachment`

1. `run(attachment_info)`: gets the attachment's name and size, and fails before the dialog if anything is missing.
2. `save_file(name)`: a native save dialog suggesting that name. The dialog asks before replacing an existing file. Closing it returns `Ok(false)`.
3. `run`: calls `Session::export_attachment` with a `ProgressReporter` sized to the attachment, then touches the session.

- **Arguments**:
  - `item_id` (JS `itemId`) → `String`;
  - `attachment_id` (JS `attachmentId`) → `String`;
  - `on_progress` (JS `onProgress`) → `Channel<TransferProgressDto>`.
- **Returns**: `true` once written, or `false` if the dialog was closed.
- **Errors**:
  - before the dialog: `Locked`, `InvalidInput`, `ItemNotFound`, `AttachmentNotFound`;
  - after it:
    - `FileAccess` (dialog failure, an unwritable destination, or a vault file as destination);
    - `Corrupted` (missing or damaged blob; nothing is left at the destination);
    - `Cancelled`.
- **Boundary**: the file name goes from Rust to the native dialog, not through the webview. The destination path and the plaintext never reach the UI.

### `delete_attachment`

- **Arguments**: `item_id` (JS `itemId`) → `String`; `attachment_id` (JS `attachmentId`) → `String`.
- **Returns**: nothing.
- **Errors**: `Locked`, `InvalidInput`, `ItemNotFound`, `AttachmentNotFound`, `Storage`.

### `pick_file` and `save_file` (private)

Each shows a `tauri_plugin_dialog` file dialog on a blocking thread, parented to the `main` window if it exists:

- `pick_file` has the title "Attach a file";
- `save_file` has the title "Export attachment" and a suggested file name.

Both return `Ok(None)` if the user closes the dialog.

- **Errors**: `FileAccess` if the picked entry is not a file system path (`into_path` fails); `Internal` on a join error.

### `ProgressReporter` (private)

Holds the `AppHandle`, the channel, the expected `total` and the time of the last message. `report(done)` is called by the core after each chunk:

1. If `transfer_interrupted()`, returns `ControlFlow::Break`, so the core stops and reports `Cancelled`.
2. Sends `{ done, total: max(total, done) }` at most every `PROGRESS_INTERVAL` (100 ms), plus once when `done >= total`. A file that grew while it was read never reports a total below `done`.
3. Ignores send errors: progress is cosmetic, and a closed channel must not fail the transfer.

## Dependencies

- **Internal**:
  - [`session`](session.rs.md): all command logic;
  - [`dto`](dto.rs.md): argument and return types;
  - [`error`](error.rs.md): `CommandError` and `CommandResult`;
  - [`settings`](settings.rs.md): the `get_settings` return type and the clear delay;
  - [`clipboard`](clipboard.rs.md): `ClipboardManager`.
- **External**:
  - `tauri`: `AppHandle`, managed state, `ipc::Channel`, `async_runtime::spawn_blocking` and `#[tauri::command]`;
  - `tauri-plugin-dialog`: native file dialogs, used from Rust only;
  - `secrecy`: `SecretString` arguments.

## Used by

- [`main.rs`](main.rs.md): registers the 19 commands in `generate_handler!` and manages `AppState`.
- [`autolock.rs`](autolock.rs.md): `AppState::with_session` and `with_session_interrupting_transfers`.
- `ui/src/api/commands.ts`: the typed TypeScript wrappers.

## Tests

_Not applicable: this file has no tests. The command logic is tested through [`session.rs`](session.rs.md#tests), the allow-list through [`main.rs`](main.rs.md#tests), and the clipboard path through [`clipboard.rs`](clipboard.rs.md#tests). The handlers themselves, `ProgressReporter` and the dialog helpers are not unit-tested._

## Notes

- The module doc says Argon2id runs "on unlock, create and password changes". It also runs on `regenerate_recovery_code` (a new slot plus the `verify` step) and on the `verify` step of `change_master_password`.
- The module doc says every handler "only moves its arguments to a blocking worker thread, locks the Session and delegates". `lock` clears the clipboard outside the session and does not touch the activity time. `copy_item_field` calls the clipboard outside the session lock. Both are deliberate and commented in the code. [`docs/api-reference.md`](../../../api-reference.md#calling-convention) ("Every command … counts as user activity") is likewise not exact for `lock`.
- The doc comments of `unlock`, `regenerate_recovery_code` and `add_attachment` omit `InvalidInput` cases that the code can return: an empty password, an empty current password, and a picked path without a file name. [`docs/api-reference.md`](../../../api-reference.md) lists all three.
- If two locks overlap (for example a manual lock and a screen-lock event), the first to finish clears `interrupt_transfer` while the second may still be waiting. No transfer can start in between, because the vault is already locked, so this has no visible effect.
