# Tauri command API reference

Every command the Tauri shell exposes to the UI, with its arguments, return value and errors. This is the whole IPC surface of NovaVault: the UI has no other way to reach the vault.

The reference is written from the source. If it disagrees with the code, the code wins; please open an issue.

| Source | What it defines |
| ------ | --------------- |
| [`src-tauri/src/commands.rs`](../src-tauri/src/commands.rs) | The command handlers. |
| [`src-tauri/src/dto.rs`](../src-tauri/src/dto.rs) | The JSON shapes of arguments and return values. |
| [`src-tauri/src/error.rs`](../src-tauri/src/error.rs) | The error object and its codes. |
| [`src-tauri/src/settings.rs`](../src-tauri/src/settings.rs) | The settings object and its bounds. |
| [`src-tauri/build.rs`](../src-tauri/build.rs), [`src-tauri/capabilities/default.json`](../src-tauri/capabilities/default.json) | The command allow-list. |
| [`ui/src/api/`](../ui/src/api/) | The typed TypeScript wrappers the UI uses (`commands.ts`, `types.ts`, `errors.ts`). |

For why the surface looks like this (coarse commands, no key material, no paths), see [Architecture: Tauri command surface](architecture.md#tauri-command-surface).

## Contents

- [Calling convention](#calling-convention)
- [Errors](#errors)
- [Shared types](#shared-types)
- Vault: [`vault_status`](#vault_status), [`create_vault`](#create_vault), [`unlock`](#unlock), [`lock`](#lock), [`change_master_password`](#change_master_password), [`regenerate_recovery_code`](#regenerate_recovery_code)
- Settings: [`get_settings`](#get_settings), [`set_auto_lock_timeout`](#set_auto_lock_timeout), [`set_clipboard_clear_delay`](#set_clipboard_clear_delay)
- Items: [`list_items`](#list_items), [`get_item`](#get_item), [`reveal_item_field`](#reveal_item_field), [`copy_item_field`](#copy_item_field), [`create_item`](#create_item), [`update_item`](#update_item), [`delete_item`](#delete_item)
- Attachments: [`add_attachment`](#add_attachment), [`export_attachment`](#export_attachment), [`delete_attachment`](#delete_attachment)
- [Adding a command](#adding-a-command)

## Calling convention

- **Invocation.** The UI calls a command with `invoke(name, args)` from `@tauri-apps/api/core`. In NovaVault only `ui/src/api/commands.ts` does this, through one helper that turns every rejection into a typed `CommandError`.
- **Argument names.** Rust parameters are snake_case; the UI sends them in camelCase (`current_credential` → `currentCredential`, `item_id` → `itemId`, `on_progress` → `onProgress`). Every handler also takes an `AppHandle`, which Tauri injects; the UI never sends it.
- **JSON field names** in arguments and return values are camelCase too (`createdAtMs`, `clearAfterSeconds`).
- **Secrets in arguments** (passwords, recovery codes, secret item fields) deserialize straight into `SecretString`, so Rust wipes them when the command ends. They are never logged and never echoed back in an error.
- **Threading.** Every handler runs its work on a blocking worker thread, because Argon2id and file I/O take a noticeable time, and holds the single session lock while it runs. Commands therefore run one at a time.
- **Activity.** Every command, including read-only ones such as `vault_status` and `get_settings`, counts as user activity and restarts the inactivity auto-lock timer.
- **Allow-list.** Only the 19 commands below are registered and granted to the main window. Tauri rejects anything else before it reaches Rust code. No core or plugin permission (events, file system, dialog, shell, HTTP) is granted to the webview.
- **No events.** The shell emits no events. When the vault locks itself (inactivity, suspend, screen lock), the UI finds out from the next command, which fails with `locked`.
- **Persistence.** Every successful change is saved to disk at once, atomically.

## Errors

A failed command rejects with a JSON object:

```json
{ "code": "locked", "message": "the vault is locked" }
```

The UI branches on `code` and shows its own wording. `message` is a fixed English string, useful for diagnostics; it never contains a command argument, a file path or details from the core. The codes are a contract with `ui/src/api/errors.ts`: renaming one is a breaking change on both sides.

| Code | Message | Meaning |
| ---- | ------- | ------- |
| `locked` | the vault is locked | The command needs an unlocked vault. |
| `vault_not_found` | no vault exists yet | There is no vault file to unlock. |
| `vault_exists` | a vault already exists | `create_vault` was called but a vault file already exists. |
| `invalid_credentials` | wrong password or recovery code | Wrong password or recovery code, or a damaged key slot. These cases are intentionally indistinguishable. |
| `invalid_recovery_code` | the recovery code is not valid; check it for typos | Malformed recovery code (wrong length, invalid character or failed checksum), rejected before any key derivation. |
| `invalid_input` | invalid input | A malformed argument, for example an empty password, a malformed identifier or an out-of-range setting. Each command lists its cases. |
| `item_not_found` | item not found | No item has the given identifier. |
| `attachment_not_found` | attachment not found | The item has no attachment with the given identifier. |
| `file_access` | the file could not be read or written | A file dialog failed, the picked file could not be read, or the export destination could not be written. |
| `cancelled` | the operation was cancelled | An attachment transfer was stopped because the vault was locked. |
| `invalid_item` | the item cannot be stored | The item cannot be stored, for example because it is too large. |
| `unsupported_vault` | the vault file is not supported by this version of NovaVault | Not a vault file, or a format version or algorithm this build does not support. |
| `corrupted` | the vault file is corrupted | The vault file or an attachment blob failed a structural or integrity check. |
| `storage` | the vault file could not be read or written | The vault file, its directory or the settings file could not be read, written or checked. |
| `clipboard_unavailable` | the clipboard is not available | The system clipboard could not be opened or written. |
| `internal` | internal error | An unexpected failure, such as a worker thread panic or a failing random number generator. Details are dropped. |

Two rules apply to every command and are not repeated below:

- Any command can fail with `internal`.
- If the arguments do not match the expected shape (a missing argument, a wrong type, an unknown `kind` or `field`), Tauri rejects the call before the handler runs, with a plain string rather than a `{ code, message }` object. The UI's wrapper reports that as `internal`.

## Shared types

TypeScript notation; the same names are used in `ui/src/api/types.ts`. Identifiers are 32 lower-case hexadecimal digits (upper case is accepted on input). Timestamps are milliseconds since the Unix epoch, UTC.

### Credential

The secret that opens the vault, used by `unlock`, `change_master_password` and `regenerate_recovery_code`.

```ts
{ method: "password", secret: string }
{ method: "recoveryCode", secret: string }
```

A recovery code is accepted in any case, with or without dashes and whitespace. It is displayed as 8 groups of 4 symbols, for example `ABCD-EFGH-JKLM-NPQR-STUV-WXYZ-2345-6789`; see [Security design: Recovery code](security-design.md#recovery-code).

### ItemInput

The content of a new item (`create_item`), tagged by `kind`. Every field is required; use an empty string for an empty field.

```ts
{ kind: "password", title: string, username: string, password: string, url: string, notes: string }
{ kind: "note",     title: string, body: string }
{ kind: "secret",   title: string, value: string, notes: string }
```

### ItemUpdate

New content for an existing item (`update_item`). Same shape as `ItemInput`, except that the secret fields (`password`, `notes`, `body`, `value`) may be `null` or left out, which keeps the stored value. Plain fields (`title`, `username`, `url`) are always replaced.

### ItemField

One field of an item, for `reveal_item_field` and `copy_item_field`.

| Value | Kind | Secret |
| ----- | ---- | ------ |
| `"username"` | password | no |
| `"url"` | password | no |
| `"password"` | password | yes |
| `"notes"` | password, secret | yes |
| `"body"` | note | yes |
| `"value"` | secret | yes |

### ItemSummary

One entry of `list_items`. Contains no secret field.

```ts
{ id: string, kind: "password" | "note" | "secret", title: string, createdAtMs: number, updatedAtMs: number }
```

### ItemDetails

One item as returned by `get_item`: every plain field and, for each secret field, only whether it is non-empty.

```ts
{
  id: string, title: string, createdAtMs: number, updatedAtMs: number,
  attachments: Attachment[],
  // plus, depending on kind:
  kind: "password", username: string, url: string, hasPassword: boolean, hasNotes: boolean
  kind: "note",     hasBody: boolean
  kind: "secret",   hasValue: boolean, hasNotes: boolean
}
```

### Attachment

```ts
{ id: string, name: string, size: number, addedAtMs: number }
```

`name` is the original file name and `size` is in bytes. Attachments are listed in the order they were added. No response ever contains a file system path.

### TransferProgress

Sent over the `onProgress` channel of the attachment commands, at most every 100 ms plus once when the transfer completes.

```ts
{ done: number, total: number }
```

Both are in bytes. `total` is the file size when the transfer started; it is never below `done`, even if the file grew while it was read.

### Settings

```ts
{ autoLockMinutes: number, clipboardClearSeconds: number }
```

| Field | Range | Default |
| ----- | ----- | ------- |
| `autoLockMinutes` | 1 to 60 | 5 |
| `clipboardClearSeconds` | 5 to 300 | 30 |

### Other return values

| Type | JSON |
| ---- | ---- |
| `VaultStatus` | `{ exists: boolean, unlocked: boolean }` |
| `RecoveryCodeResponse` | `{ recoveryCode: string }` |
| `RevealedValue` | `{ value: string }` |
| `CopyResult` | `{ clearAfterSeconds: number }` |

## Vault

### vault_status

Reports whether a vault exists and whether it is unlocked, so the UI can choose between the create and unlock screens.

- **Arguments:** none.
- **Returns:** `VaultStatus`.
- **Errors:** `storage` if the vault location cannot be checked.

### create_vault

Creates a new vault protected by `password` and leaves it unlocked. A vault that was open is locked first, so its key is wiped before a new one is derived.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `password` | `string` | The new master password. Must not be empty. |

- **Returns:** `RecoveryCodeResponse`. The recovery code is stored nowhere else: the UI must show it exactly once and have the user confirm they saved it.
- **Errors:**
  - `invalid_input`: empty password.
  - `vault_exists`: a vault file already exists.
  - `storage`: the vault directory or file cannot be written.

### unlock

Unlocks the vault with the master password or the recovery code. The vault key stays in Rust; the UI only learns that unlocking worked. An unlocked vault is locked first, so on any error the vault is left locked.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `credential` | `Credential` | The master password or the recovery code. |

- **Returns:** nothing (`null`).
- **Errors:**
  - `vault_not_found`: no vault exists.
  - `invalid_input`: empty password.
  - `invalid_recovery_code`: malformed recovery code.
  - `invalid_credentials`: wrong password or recovery code.
  - `storage`, `unsupported_vault` or `corrupted`: the vault file cannot be opened.

### lock

Locks the vault, wiping the vault key from memory, and clears a field this app copied to the clipboard. The clipboard is cleared first, without waiting for the session. A running attachment transfer is asked to stop at its next chunk, so the lock does not wait for the whole transfer. Locking an already locked vault succeeds.

- **Arguments:** none.
- **Returns:** nothing.
- **Errors:** only `internal`.

### change_master_password

Changes the master password of the unlocked vault, after checking the current password or recovery code. Only the vault key is re-wrapped; the data is not re-encrypted and the recovery code keeps working.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `currentCredential` | `Credential` | The current password, or the recovery code for the forgotten-password flow. Asked again so that someone at an unattended, unlocked app cannot take the vault over. |
| `newPassword` | `string` | The new master password. Must not be empty. |

- **Returns:** nothing.
- **Errors:**
  - `locked`.
  - `invalid_credentials` or `invalid_recovery_code`: `currentCredential` is wrong or malformed.
  - `invalid_input`: empty current or new password.
  - `storage`: the vault file cannot be saved.

### regenerate_recovery_code

Replaces the recovery code, after checking the current password or recovery code. The previous code stops working.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `currentCredential` | `Credential` | The current password or recovery code, as for `change_master_password`. |

- **Returns:** `RecoveryCodeResponse`. As with `create_vault`, the UI must show the new code exactly once.
- **Errors:**
  - `locked`.
  - `invalid_credentials`, `invalid_recovery_code` or `invalid_input`: as for `change_master_password`.
  - `storage`: the vault file cannot be saved.

## Settings

### get_settings

Returns the app settings. Works while locked, because settings are not secret.

- **Arguments:** none.
- **Returns:** `Settings`.
- **Errors:** only `internal`.

### set_auto_lock_timeout

Sets the inactivity auto-lock timeout and saves it. Requires an unlocked vault, so that someone at a locked app cannot lengthen the timeout before the owner unlocks it.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `minutes` | `number` (unsigned integer) | The new timeout, 1 to 60. |

- **Returns:** nothing.
- **Errors:**
  - `locked`.
  - `invalid_input`: `minutes` is out of range.
  - `storage`: the settings file cannot be written; the previous value is kept.

### set_clipboard_clear_delay

Sets how long a copied field stays on the clipboard, and saves it. Requires an unlocked vault, like `set_auto_lock_timeout`.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `seconds` | `number` (unsigned integer) | The new delay, 5 to 300. |

- **Returns:** nothing.
- **Errors:**
  - `locked`.
  - `invalid_input`: `seconds` is out of range.
  - `storage`: the settings file cannot be written; the previous value is kept.

## Items

### list_items

Lists the items of the unlocked vault, without secret fields.

- **Arguments:** none.
- **Returns:** `ItemSummary[]`.
- **Errors:** `locked` or `corrupted`.

### get_item

Returns one item without its secret values. Secret values are fetched one at a time with `reveal_item_field`.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `id` | `string` | The item identifier. |

- **Returns:** `ItemDetails`.
- **Errors:**
  - `locked`.
  - `invalid_input`: malformed identifier.
  - `item_not_found`.
  - `corrupted`.

### reveal_item_field

Returns one secret field of an item, because the user asked to see it. This is the only command that sends an item secret to the webview, and only one field per call; call it only on an explicit user action.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `id` | `string` | The item identifier. |
| `field` | `ItemField` | A secret field of that item's kind: `"password"`, `"notes"`, `"body"` or `"value"`. |

- **Returns:** `RevealedValue`.
- **Errors:**
  - `invalid_input`: `field` is a plain field (`"username"`, `"url"`) or not a field of this kind of item.
  - The errors of `get_item`.

### copy_item_field

Copies one field of an item to the system clipboard and schedules it to be cleared after the configured delay. Any field can be copied, plain or secret. The value goes from the vault to the clipboard inside Rust and never comes back to the UI.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `id` | `string` | The item identifier. |
| `field` | `ItemField` | The field to copy. |

- **Returns:** `CopyResult`: the number of seconds until the clipboard is cleared.
- **Errors:**
  - `locked`.
  - `invalid_input`: malformed identifier, or a field this kind of item does not have.
  - `item_not_found`.
  - `corrupted`.
  - `clipboard_unavailable`: the clipboard cannot be written.

See [Clipboard behaviour](clipboard.md) for what happens on each OS.

### create_item

Adds an item and saves the vault.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `item` | `ItemInput` | The new item's content, tagged by `kind`. |

- **Returns:** `string`, the new item's identifier.
- **Errors:**
  - `locked`.
  - `invalid_item`: the item cannot be stored (for example, it is too large or the vault is full).
  - `storage`: the vault file cannot be saved.

### update_item

Replaces an item's content and saves the vault. Secret fields sent as `null` or left out keep their stored values, so the edit form never needs a secret the user did not reveal.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `id` | `string` | The item identifier. |
| `item` | `ItemUpdate` | The new content. Its `kind` must match the stored item's. |

- **Returns:** nothing.
- **Errors:**
  - `locked`.
  - `invalid_input`: malformed identifier, or a `kind` different from the stored item's.
  - `item_not_found`.
  - `invalid_item`: the updated item cannot be stored.
  - `storage`: the vault file cannot be saved.

### delete_item

Deletes an item, with its attachments, and saves the vault. The attachments' encrypted data is deleted after the vault is saved.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `id` | `string` | The item identifier. |

- **Returns:** nothing.
- **Errors:**
  - `locked`.
  - `invalid_input`: malformed identifier.
  - `item_not_found`.
  - `storage`: the vault file cannot be saved.

## Attachments

The attachment commands open the native file dialog themselves, from Rust. The UI never sends a file system path and never receives one, so a compromised webview cannot make the shell read or write a file the user did not pick. The dialog is shown before the session lock is taken, so an open dialog never holds up the auto-lock; the transfer then runs with the session locked, and a lock (manual or from an OS event) stops it at its next chunk with `cancelled`.

`onProgress` is a Tauri `Channel<TransferProgress>` created by the UI (`new Channel()` from `@tauri-apps/api/core`). Progress is cosmetic: a closed channel does not fail the transfer.

### add_attachment

Asks the user for a file in a native open dialog, encrypts it as a new attachment of the item, and saves the vault.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `itemId` | `string` | The item to attach the file to. |
| `onProgress` | `Channel<TransferProgress>` | Receives progress updates. |

- **Returns:** `Attachment`, or `null` if the user closed the dialog.
- **Errors**, checked before the dialog opens:
  - `locked`.
  - `invalid_input`: malformed identifier.
  - `item_not_found`.
- **Errors** after a file is picked:
  - `file_access`: the dialog fails or the file cannot be read.
  - `invalid_input`: the picked path has no file name.
  - `invalid_item`: the item has too many attachments, or the file name cannot be used as an attachment name.
  - `cancelled`: a lock stopped the transfer.
  - `storage`: the vault file cannot be saved.

### export_attachment

Asks the user where to save an attachment in a native save dialog, suggesting its file name, and decrypts it there. The dialog asks before replacing an existing file.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `itemId` | `string` | The item that holds the attachment. |
| `attachmentId` | `string` | The attachment to export. |
| `onProgress` | `Channel<TransferProgress>` | Receives progress updates. |

- **Returns:** `true` once the file is written, or `false` if the user closed the dialog.
- **Errors**, checked before the dialog opens:
  - `locked`.
  - `invalid_input`: malformed identifier.
  - `item_not_found`.
  - `attachment_not_found`.
- **Errors** after a destination is picked:
  - `file_access`: the dialog fails, the destination cannot be written, or it is one of the vault's own files.
  - `corrupted`: the encrypted data is missing or fails authentication. Nothing is left at the destination in that case.
  - `cancelled`: a lock stopped the transfer.

### delete_attachment

Removes an attachment from an item, saves the vault, then deletes the attachment's encrypted data.

| Argument | Type | Description |
| -------- | ---- | ----------- |
| `itemId` | `string` | The item that holds the attachment. |
| `attachmentId` | `string` | The attachment to remove. |

- **Returns:** nothing.
- **Errors:**
  - `locked`.
  - `invalid_input`: malformed identifier.
  - `item_not_found`.
  - `attachment_not_found`.
  - `storage`: the vault file cannot be saved.

## Adding a command

A new command must be added in four places, or Tauri rejects it at runtime:

1. The handler in `src-tauri/src/commands.rs`, with `///` docs listing its arguments, return value and errors.
2. `COMMANDS` in `src-tauri/build.rs`, which generates its `allow-<command>` permission.
3. `generate_handler!` in `src-tauri/src/main.rs`.
4. `src-tauri/capabilities/default.json`. A test fails if the capability and `COMMANDS` differ.

Then add a typed wrapper in `ui/src/api/commands.ts` (and its name to `CommandName`), and document the command here.
