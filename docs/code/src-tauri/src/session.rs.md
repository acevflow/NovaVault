# [`src-tauri/src/session.rs`](../../../../src-tauri/src/session.rs)

## Purpose

Defines `Session`, the one place the shell holds an unlocked vault. It owns the core [`Vault`](../../crates/novavault-core/src/vault.rs.md), and with it the vault key, for as long as the vault is unlocked. It also implements the logic behind every Tauri command. It has no Tauri types, so it can be tested without a running app. The handlers in [`commands.rs`](commands.rs.md) only lock it and delegate. Locking drops the `Vault`, which wipes the key.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `Session` | struct | `pub struct Session` | Vault path, KDF parameters, optional unlocked vault, last activity, settings. Custom `Debug`. |
| `Session::new` | function | `pub fn new(path: PathBuf, params: KdfParams, now: Instant) -> Self` | A locked session; loads the settings. |
| `Session::settings` | method | `pub const fn settings(&self) -> Settings` | The current settings. |
| `Session::set_auto_lock_minutes` | method | `pub fn set_auto_lock_minutes(&mut self, minutes: u32) -> CommandResult<()>` | Changes and saves the timeout (unlocked only). |
| `Session::set_clipboard_clear_seconds` | method | `pub fn set_clipboard_clear_seconds(&mut self, seconds: u32) -> CommandResult<()>` | Changes and saves the clear delay (unlocked only). |
| `Session::touch` | method | `pub fn touch(&mut self, now: Instant)` | Records activity. |
| `Session::is_unlocked` | method | `pub const fn is_unlocked(&self) -> bool` | Whether a vault is held. |
| `Session::status` | method | `pub fn status(&self) -> CommandResult<VaultStatusDto>` | Exists / unlocked. |
| `Session::create` | method | `pub fn create(&mut self, password: &SecretString) -> CommandResult<RecoveryCodeDto>` | Creates a vault and leaves it unlocked. |
| `Session::unlock` | method | `pub fn unlock(&mut self, credential: &Credential) -> CommandResult<()>` | Opens the vault with either credential. |
| `Session::lock` | method | `pub fn lock(&mut self) -> bool` | Drops the vault; returns whether it was unlocked. |
| `Session::lock_if_idle` | method | `pub fn lock_if_idle(&mut self, now: Instant) -> bool` | Locks if idle for the timeout. |
| `Session::change_master_password` | method | `pub fn change_master_password(&mut self, current: &Credential, new_password: &SecretString) -> CommandResult<()>` | Re-authenticates, then re-wraps the password slot. |
| `Session::regenerate_recovery_code` | method | `pub fn regenerate_recovery_code(&mut self, current: &Credential) -> CommandResult<RecoveryCodeDto>` | Re-authenticates, then replaces the recovery slot. |
| `Session::list_items` | method | `pub fn list_items(&self) -> CommandResult<Vec<ItemSummaryDto>>` | Item summaries. |
| `Session::create_item` | method | `pub fn create_item(&mut self, item: ItemInput) -> CommandResult<String>` | Inserts and saves; returns the identifier. |
| `Session::get_item` | method | `pub fn get_item(&self, id: &str) -> CommandResult<ItemDetailsDto>` | Item details without secret values. |
| `Session::reveal_field` | method | `pub fn reveal_field(&self, id: &str, field: ItemField) -> CommandResult<RevealedValueDto>` | One secret field's value. |
| `Session::field_text` | method | `pub fn field_text(&self, id: &str, field: ItemField) -> CommandResult<Zeroizing<String>>` | Any field's text, in zeroizing storage. |
| `Session::update_item` | method | `pub fn update_item(&mut self, id: &str, update: ItemUpdate) -> CommandResult<()>` | Merges, updates and saves. |
| `Session::delete_item` | method | `pub fn delete_item(&mut self, id: &str) -> CommandResult<()>` | Removes and saves. |
| `Session::check_item` | method | `pub fn check_item(&self, id: &str) -> CommandResult<()>` | Unlocked and item exists. |
| `Session::attachment_info` | method | `pub fn attachment_info(&self, item_id: &str, attachment_id: &str) -> CommandResult<(String, u64)>` | Attachment name and size. |
| `Session::add_attachment` | method | `pub fn add_attachment(&mut self, item_id: &str, source: &Path, progress: impl FnMut(u64) -> ControlFlow<()>) -> CommandResult<AttachmentDto>` | Encrypts a user-picked file and saves. |
| `Session::export_attachment` | method | `pub fn export_attachment(&self, item_id: &str, attachment_id: &str, destination: &Path, progress: impl FnMut(u64) -> ControlFlow<()>) -> CommandResult<()>` | Decrypts to a user-picked destination. |
| `Session::delete_attachment` | method | `pub fn delete_attachment(&mut self, item_id: &str, attachment_id: &str) -> CommandResult<()>` | Removes, saves, deletes the blob. |
| `impl Debug for Session` | trait impl | `fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result` | Prints only `unlocked`. |

## Details

### `Session`

| Field | Type | Meaning |
| ----- | ---- | ------- |
| `path` | `PathBuf` | The vault file. Chosen by the shell (`main.rs`), never by the UI. |
| `params` | `KdfParams` | Argon2id parameters for new key slots. |
| `vault` | `Option<Vault>` | The unlocked vault, or `None` while locked. |
| `last_activity` | `Instant` | Time of the last command, for the inactivity lock. |
| `settings_path` | `PathBuf` | `settings.json` in the vault's directory. |
| `settings` | `Settings` | The current settings. |

- **Security**: the vault key lives only inside `vault`. Dropping the `Vault` wipes it (see the core's [`Vault`](../../crates/novavault-core/src/vault.rs.md#vault)). No method returns the `Vault`, a key, or a path.

### `Session::new`

Sets `settings_path` to `path.with_file_name("settings.json")` and loads it with [`Settings::load`](settings.rs.md#settingsload) (defaults if unusable). The session starts locked, with `last_activity = now`.

### `Session::settings`

Returns a copy of the settings (`Settings` is `Copy`).

### `set_auto_lock_minutes` and `set_clipboard_clear_seconds`

1. `self.vault()?`: requires an unlocked vault, so someone at a locked app cannot lengthen the timeout before the owner unlocks it.
2. Copies the settings, applies the setter (bounds check), saves the copy to `settings_path`.
3. Only on success, replaces `self.settings`.

- **Errors**: `Locked`; `InvalidInput` out of bounds; `Storage` if the file cannot be written (the previous value is kept in memory and on disk).

### `Session::touch` and `Session::is_unlocked`

`touch` sets `last_activity`. `is_unlocked` is `vault.is_some()`.

### `Session::status`

Returns `VaultStatusDto { exists: path.try_exists()?, unlocked }`.

- **Errors**: `Storage` if the existence check fails, for example on a permission error.

### `Session::create`

1. Converts the password with `password_bytes` (rejects an empty one).
2. Calls `lock()`, so a vault that was open has its key wiped before a new one is derived, and a failure below leaves the session locked.
3. Creates the vault's parent directory.
4. Calls `Vault::create(path, password, params)`, which refuses an existing file and saves the new vault.
5. Keeps the vault unlocked and returns `RecoveryCodeDto::from(&code)`.

- **Errors**: `InvalidInput` (empty password, checked before locking); `Storage` if the directory cannot be created or the file written; `VaultExists` if a file exists at the path.
- **Security**: the recovery code is returned once and stored nowhere else. The `RecoveryCode` itself is wiped when it drops at the end of the call. The password bytes are in a `SecretSlice`, wiped on drop.
- **Edge cases and invariants**: calling `create` while unlocked with an existing vault locks the session and then fails with `VaultExists`.

### `Session::unlock`

1. Calls `lock()` first, so a failed attempt always leaves the session locked.
2. Checks `path.try_exists()`.
3. Opens the vault:
   - password: `password_bytes`, then `Vault::open_with_password`;
   - recovery code: `RecoveryCode::parse`, then `Vault::open_with_recovery_code`.
4. Stores the vault.

- **Errors**:
  - `Storage` if the existence check fails;
  - `VaultNotFound` if no file exists;
  - `InvalidInput` for an empty password;
  - `InvalidRecoveryCode` if the code does not parse, before any key derivation;
  - `InvalidCredentials` for a wrong secret;
  - otherwise the open errors (`Storage`, `UnsupportedVault`, `Corrupted`).
- **Security**: the key never leaves the core. The caller learns only success or a fixed error code.

### `Session::lock`

`vault.take().is_some()`. Dropping the `Vault` wipes the key. Returns `false` if it was already locked.

### `Session::lock_if_idle`

Locks only if the vault is unlocked and `now.saturating_duration_since(last_activity) >= settings.auto_lock_timeout()`. Returns whether it locked.

- **Edge cases and invariants**: a `now` earlier than `last_activity` gives a zero duration, so no panic and no lock.

### `Session::change_master_password`

1. `verify(current)`: re-authenticates with the current password or the recovery code (the forgotten-password flow).
2. Converts the new password with `password_bytes`.
3. Calls `Vault::change_password(new, params)`, which re-wraps only the password slot and saves.

- **Errors**:
  - `Locked`;
  - `InvalidCredentials` or `InvalidRecoveryCode` if `current` is wrong or malformed;
  - `InvalidInput` for an empty current or new password;
  - a save error, usually `Storage`.
- **Security**: re-authentication means someone at an unattended, unlocked app cannot take the vault over. The recovery code keeps working.

### `Session::regenerate_recovery_code`

`verify(current)`, then `Vault::regenerate_recovery_code(params)`, which replaces the recovery slot and saves. Returns the new code as `RecoveryCodeDto`. The previous code stops working.

- **Errors**: as for `change_master_password`, without the new-password check.

### `verify` (private)

Calls `Vault::verify_password` or `Vault::verify_recovery_code` on the unlocked vault. Errors come from `vault()` (`Locked`), `password_bytes` (`InvalidInput`), `RecoveryCode::parse` (`InvalidRecoveryCode`) and the core (`InvalidCredentials`).

### `Session::list_items`

`Vault::list()`, with each `ItemSummary` converted to `ItemSummaryDto`. No secret field, user name or URL is included.

- **Errors**: `Locked`, `Corrupted`.

### `Session::create_item`

`Vault::insert(ItemContent::from(item))`, then `Vault::save()`. Returns the new identifier as hexadecimal.

- **Errors**: `Locked`, `InvalidItem`, or a save error.

### `Session::get_item`

Parses the identifier, decrypts the item with `Vault::get` and returns `ItemDetailsDto::from(&item)`.

- **Errors**: `Locked`, `InvalidInput` (malformed identifier), `ItemNotFound`, `Corrupted`.
- **Security**: secret fields appear only as `has…` flags. The decrypted `Item` is wiped when it drops.

### `reveal_field`

Refuses plain fields (`!field.is_secret()` gives `InvalidInput`), then copies the value through `field_text` into a new `SecretString` wrapped in `RevealedValueDto`.

- **Errors**: `InvalidInput` for a plain field or a field the kind does not have, plus the errors of `get_item`.
- **Security**: the only path by which an item secret reaches the UI, one field per call.

### `Session::field_text`

Decrypts the item and copies `field.value_in(content)` into a `Zeroizing<String>`. It accepts plain and secret fields. It is used by `reveal_field` and by `copy_item_field`, which sends the text to the clipboard worker instead of the UI.

- **Errors**: as for `get_item`, plus `InvalidInput` for a field the kind does not have.

### `Session::update_item`

Parses the identifier, takes the stored content (`Vault::get(id)?.into_content()`), merges it with [`ItemUpdate::apply_to`](dto.rs.md#itemupdateapply_to), then calls `Vault::update` and `Vault::save`.

- **Errors**: `Locked`, `InvalidInput` (identifier or kind mismatch), `ItemNotFound`, `InvalidItem`, or a save error.

### `Session::delete_item`

`Vault::remove(id)` then `Vault::save()`. The save also deletes the item's attachment blobs (see the core's [`Vault::save`](../../crates/novavault-core/src/vault.rs.md#vaultsave)).

- **Errors**: `Locked`, `InvalidInput`, `ItemNotFound`, or a save error.

### `Session::check_item`

`Vault::get(parse_item_id(id)?)` with the result discarded. Used before a file dialog opens, so the user is not asked for a file that cannot be attached.

- **Errors**: `Locked`, `InvalidInput`, `ItemNotFound`, and possibly `Corrupted` from decrypting the item.

### `Session::attachment_info`

Returns `(name, size)` of the attachment, used to suggest a file name in the save dialog. Stays in Rust; `export_attachment` passes the name only to the native dialog.

- **Errors**: `Locked`, `InvalidInput`, `ItemNotFound`, `AttachmentNotFound`.

### `Session::add_attachment`

1. Requires an unlocked vault and parses `item_id`.
2. Takes the attachment name from `source.file_name()`, converted with `to_string_lossy` (invalid UTF-8 becomes replacement characters).
3. Opens `source`.
4. Calls `Vault::add_attachment(id, name, file, progress)`, which encrypts the file in chunks and calls `progress` after each one.
5. Saves and returns `AttachmentDto`.

- **Parameters**: `source` comes from a native dialog opened by Rust, never from the UI. `progress` returning `ControlFlow::Break` stops the transfer.
- **Errors**:
  - `Locked`;
  - `InvalidInput` for a malformed identifier or a path without a file name;
  - `ItemNotFound`;
  - `FileAccess` if the file cannot be opened, or for any I/O error during the transfer (`file_error`);
  - `InvalidItem` (unusable name, too many attachments);
  - `Cancelled`;
  - a save error.
- **Security**: neither the source path nor the blob path ever reaches a DTO (tested).

### `Session::export_attachment`

Parses both identifiers and calls `Vault::export_attachment(item, attachment, destination, progress)`. The core writes to a temporary file and only renames it into place once all data has authenticated.

- **Errors**:
  - `Locked`, `InvalidInput`, `ItemNotFound`, `AttachmentNotFound`;
  - `Corrupted` if the blob is missing or damaged;
  - `FileAccess` for I/O errors, and for `VaultError::InvalidItem`, which the core returns when `destination` is one of the vault's own files;
  - `Cancelled`.
- **Edge cases and invariants**: takes `&self`. The transfer only reads the vault.

### `Session::delete_attachment`

`Vault::remove_attachment`, then `Vault::save()`, which deletes the now-unreferenced blob.

- **Errors**: `Locked`, `InvalidInput`, `ItemNotFound`, `AttachmentNotFound`, or a save error.

### `vault` and `vault_mut` (private)

`self.vault.as_ref()` or `as_mut()`, with `CommandError::Locked` if `None`. Every data operation goes through one of them, so they are the single "is it unlocked" check.

### `impl Debug for Session`

`Session { unlocked: <bool>, .. }`. The vault, paths, settings and timestamps never appear in debug output.

### `file_error` and `password_bytes` (private)

- `file_error(error)`: `VaultError::Io` becomes `FileAccess` (the user's file or the attachment data, not the vault file). Anything else goes through the normal `From<VaultError>` mapping.
- `password_bytes(password)`: rejects an empty password with `InvalidInput`, because it would make the vault openable by anyone. Returns the UTF-8 bytes in a `SecretSlice<u8>`, wiped on drop.

## Dependencies

- **Internal**:
  - [`dto`](dto.rs.md): inputs, outputs and identifier parsing.
  - [`error`](error.rs.md): `CommandError` and `CommandResult`.
  - [`settings`](settings.rs.md): `Settings`.
- **External**:
  - `novavault_core`: `Vault`, `ItemContent`, `KdfParams`, `RecoveryCode` and `VaultError`.
  - `secrecy` and `zeroize`: secret containers for passwords and field text.

## Used by

- [`commands.rs`](commands.rs.md): `AppState` wraps the `Session` in a mutex, and every command delegates to it.
- [`autolock.rs`](autolock.rs.md): calls `lock_if_idle` and `lock`.
- [`main.rs`](main.rs.md): creates the session at startup.

## Tests

All tests use a fresh temporary directory, the lowest accepted Argon2id cost, and the fake password `correct horse battery staple`.

- [`create_lock_and_unlock_with_either_secret`](../../../../src-tauri/src/session.rs#L625): create leaves the vault unlocked, and a second `lock` returns `false`. Both the password and a lower-case recovery code with spaces instead of dashes unlock it.
- [`wrong_or_malformed_secrets_leave_the_session_locked`](../../../../src-tauri/src/session.rs#L646): a wrong password and a valid-but-wrong code give `InvalidCredentials`, a malformed code gives `InvalidRecoveryCode`, and the session stays locked.
- [`status_reports_existence_and_lock_state`](../../../../src-tauri/src/session.rs#L669): `(false, false)`, then `(true, true)`, then `(true, false)`.
- [`missing_and_existing_vaults_are_reported`](../../../../src-tauri/src/session.rs#L684): `VaultNotFound` before create, `VaultExists` on a second create, and the original vault still opens.
- [`empty_passwords_are_rejected`](../../../../src-tauri/src/session.rs#L701): empty passwords for `create` and for the new password of `change_master_password` give `InvalidInput`.
- [`data_commands_require_an_unlocked_vault`](../../../../src-tauri/src/session.rs#L716): list, create, update, delete, change password and regenerate code give `Locked`.
- [`item_edits_are_saved_immediately`](../../../../src-tauri/src/session.rs#L746): create, update and delete survive a lock and unlock.
- [`listing_never_serializes_secret_fields`](../../../../src-tauri/src/session.rs#L773): the listing JSON contains the title but not the user name, password, URL or notes.
- [`item_details_have_plain_fields_and_flags_only`](../../../../src-tauri/src/session.rs#L794): the details JSON has the plain fields and the `has…` flags, but not the password.
- [`secret_fields_are_revealed_one_at_a_time`](../../../../src-tauri/src/session.rs#L816):
  - each secret field of each kind is revealed;
  - plain fields and fields of another kind give `InvalidInput`;
  - `field_text` returns a plain field;
  - reveal and get give `Locked` after locking.
- [`updates_keep_secret_fields_that_are_left_out`](../../../../src-tauri/src/session.rs#L870): an omitted password is kept, an empty `notes` replaces the old notes, and both survive a reload.
- [`updates_cannot_change_the_kind_of_an_item`](../../../../src-tauri/src/session.rs#L898): a note update on a password item gives `InvalidInput` and leaves the item unchanged.
- [`unknown_or_malformed_item_ids_are_reported`](../../../../src-tauri/src/session.rs#L914): an unknown identifier gives `ItemNotFound`; a malformed one gives `InvalidInput`.
- [`changing_the_master_password`](../../../../src-tauri/src/session.rs#L934): the old password fails, the new one works, the data is intact, and the recovery code still works.
- [`regenerating_the_recovery_code_invalidates_the_old_one`](../../../../src-tauri/src/session.rs#L955): the new code differs from the old one, the old one fails, and the new code and the password work.
- [`key_slot_changes_require_the_current_secret`](../../../../src-tauri/src/session.rs#L977):
  - a wrong password or a malformed code is rejected and nothing changes;
  - the recovery code authorizes a password change.
- [`idle_sessions_lock_after_the_timeout`](../../../../src-tauri/src/session.rs#L1012):
  - the default timeout is 300 seconds;
  - there is no lock before the timeout or while locked;
  - `touch` restarts the timer;
  - an earlier clock reading neither locks nor panics.
- [`clipboard_delay_is_configurable_and_persisted`](../../../../src-tauri/src/session.rs#L1035): `Locked` while locked, `InvalidInput` out of bounds, and the value is persisted for a new session.
- [`auto_lock_timeout_is_configurable_and_persisted`](../../../../src-tauri/src/session.rs#L1055): the same checks, and a 1-minute timeout locks after 60 seconds.
- [`attachments_are_added_exported_and_deleted`](../../../../src-tauri/src/session.rs#L1094): a 200 000-byte file round-trips with progress, survives a reload and is deleted; a second delete gives `AttachmentNotFound`.
- [`deleting_an_item_deletes_its_attachment_data`](../../../../src-tauri/src/session.rs#L1143): the blob directory is empty after the item is deleted.
- [`attachment_commands_report_their_errors`](../../../../src-tauri/src/session.rs#L1157):
  - `check_item` errors; `FileAccess` for a missing source;
  - `AttachmentNotFound` and `InvalidInput` from `attachment_info`;
  - `Cancelled` from a progress `Break`;
  - `FileAccess` when exporting over the vault file, which is left unchanged;
  - `Locked` for every attachment operation while locked.
- [`attachment_dtos_never_contain_paths`](../../../../src-tauri/src/session.rs#L1215): neither the added-attachment JSON nor the details JSON contains the temporary directory or `nvault`.
- [`debug_shows_only_lock_state`](../../../../src-tauri/src/session.rs#L1233): `Debug` shows `unlocked: true` and not the vault file name.

## Notes

- The module doc says that if a save fails, "the edit stays in memory and is written by the next successful save". For `create_item`, the command then reports `Storage` even though the item exists in memory. If the UI retries, the item may end up stored twice.
- The doc comments of `Session::unlock` and of the `unlock` command do not list `InvalidInput`, which `password_bytes` returns for an empty password. [`docs/api-reference.md`](../../../api-reference.md#unlock) does list it. The same goes for an empty current password in `change_master_password` and `regenerate_recovery_code`: the code returns `InvalidInput`, and the API reference mentions it, but only implicitly in the doc comments.
- `check_item` decrypts the whole item to check that it exists. A damaged item therefore fails with `Corrupted` before the dialog, which its doc comment does not list.
- `unlock` checks existence and then opens the file, two separate steps. If the file disappears in between, the error is `Storage` instead of `VaultNotFound`.
