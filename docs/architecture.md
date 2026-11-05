# Architecture

This document describes how NovaVault is put together: its components, the layers they form, how data moves between them, and why every cryptographic operation lives in Rust.

> [!NOTE]
> NovaVault is pre-alpha. Every MVP feature described here is implemented: the core library (cryptographic primitives, key slots, recovery codes, the vault file format with passwords, notes, secrets and encrypted attachments), the Tauri command layer with auto-lock and clipboard handling, and the UI (see [Core library](#core-library), [Tauri command surface](#tauri-command-surface) and [UI](#ui)). It has not had an external security review and there is no release yet.

## Goals

- **Local only.** The vault is a file on the user's disk, plus a directory of encrypted attachment blobs next to it. Core features never use the network.
- **Small trusted core.** Everything that touches keys or plaintext secrets is concentrated in one Rust library that can be reviewed and tested on its own.
- **Untrusted presentation layer.** The UI is treated as the most exposed part of the app and is given as little power as possible.
- **Simplicity.** A solo maintainer must be able to understand and audit the whole system.

## Components

| Component | Path | Language | Responsibility |
| --------- | ---- | -------- | -------------- |
| Core | `crates/novavault-core/` | Rust | Cryptography, key derivation, key slots, vault format (read/write/validate), record and attachment storage, and the unlocked `Vault`, which owns the vault key. No dependency on Tauri or any UI. |
| Shell | `src-tauri/` | Rust | Thin Tauri 2 application: window lifecycle, the session (the single unlocked `Vault`, last activity time, settings), the Tauri command handlers, OS integration (auto-lock triggers on suspend and screen lock, clipboard, file dialogs), Content Security Policy and capability configuration. Delegates all vault logic to the core. |
| UI | `ui/` | React + TypeScript | Presentation and user input only. Renders forms and lists, sends user intents to the shell through Tauri commands and displays the results. Holds no keys. |

## Layers and trust boundary

```text
┌──────────────────────────────────────────────────────────┐
│ UI (webview)  React + TypeScript                         │  untrusted
│   forms, lists, navigation, inactivity timer events      │
└───────────────────────────┬──────────────────────────────┘
                            │  Tauri IPC: minimal commands,
                            │  validated inputs, minimal outputs
════════════════════════════╪═══════════════ trust boundary ═
┌───────────────────────────┴──────────────────────────────┐
│ Shell (src-tauri)  Rust                                  │  trusted
│   command handlers, session, OS hooks, clipboard, CSP    │
├──────────────────────────────────────────────────────────┤
│ Core (novavault-core)  Rust                              │  trusted
│   Vault: vault key (secrecy + zeroize)                   │
│   key slots · Argon2id · AEAD · vault format · records   │
└───────────────────────────┬──────────────────────────────┘
                            │  file I/O
          ┌─────────────────┴──────────────────┐
          │  vault file  +  attachment blobs   │  encrypted at rest
          └────────────────────────────────────┘
```

The trust boundary is the Tauri IPC bridge. Everything above it is assumed to be potentially hostile (for example, if malicious content ever managed to execute script in the webview). Everything below it is the trusted computing base and is kept as small as possible.

## Rust/UI boundary

The webview reaches Rust only through the nineteen commands listed under [Tauri command surface](#tauri-command-surface). What crosses the boundary:

| Direction | Data |
| --------- | ---- |
| UI → Rust | The master password, recovery codes and secret item fields, deserialized straight into `SecretString`; plain item fields (title, user name, URL); item and attachment identifiers as 32 hexadecimal digits; field names; settings values; a progress `Channel` for attachment transfers. |
| Rust → UI | Whether a vault exists and is unlocked; item identifiers, kinds, titles, user names, URLs and timestamps; for each secret field only whether it is set; attachment identifiers, names, sizes and times; transfer progress in bytes; settings; typed errors with fixed messages. |
| Rust → UI, on explicit request only | One secret field value per `reveal_item_field` call; a new recovery code from `create_vault` and `regenerate_recovery_code`. |
| Never | The vault key, wrapping keys, KDF output, salts, ciphertext, file paths, and values copied to the clipboard (the shell copies them itself). |

The vault location and the attachment files are chosen in Rust (fixed path, native dialogs), so no command takes a path. The core's `SecretKey` exposes its bytes only inside the crate, so even the shell cannot read the vault key.

## Core library

`crates/novavault-core` is organized in layers, each using only the ones below it:

| Module | Responsibility |
| ------ | -------------- |
| `vault` | `Vault`, the unlocked vault: `create`, `open_with_password`, `open_with_recovery_code` (both migrate older format versions), `list`, `get`, `insert`, `update`, `remove`, `save`, `add_attachment`, `export_attachment`, `remove_attachment`, `attachment`, `attachments_dir`, `change_password`, `regenerate_recovery_code` (each rewrites one key slot and saves, restoring the previous slot if the save fails), `verify_password`, `verify_recovery_code` (re-authentication against the in-memory slots). |
| `item` | The item model (`PasswordItem`, `NoteItem`, `SecretItem`, `ItemId`, `Attachment`, `AttachmentId`) and the padded payload encoding. |
| `attachment` | Streaming, chunked encryption and decryption of attachment blobs. |
| `format` (private) | Byte framing of the file, record associated data and the whole-file integrity seal. Pure functions, no I/O. |
| `atomic` (private) | Atomic file replacement (temporary file, flush, rename), also as a streaming writer that deletes its temporary file unless committed. |
| `header` | The plaintext versioned header. |
| `slots`, `recovery` | Key slots and recovery codes. |
| `kdf`, `aead`, `keys`, `error` | Primitives, key types and typed errors (`CryptoError`, `VaultError`). |

### Vault file

The byte-level format is specified in [vault-format.md](vault-format.md). In short:

```text
header (32) ‖ password slot (101) ‖ recovery slot (101) ‖ record_count ‖ records… ‖ seal (40)
```

- **Records** hold one item each. Only the random record identifier and the item type are in plaintext; the title, timestamps and fields are encrypted together under the vault key, padded to 64-byte buckets. Each record's associated data binds it to the vault header, its identifier and its type.
- **Integrity seal.** The file ends with an AEAD tag under the vault key over everything before it, so removing, duplicating, reordering or replaying an individual record is detected, not only modifying one.
- **Opening** checks the whole structure (magic, version, header fields, slots, record framing, sizes) before running Argon2id, so a foreign, newer or truncated file is rejected quickly with a precise `VaultError`. A wrong secret or damaged slot is reported as the single `InvalidCredentials` error. The seal is verified before any record can be read.
- **Saving** encodes the file in memory and replaces the old one atomically: a temporary file in the same directory is written, flushed and renamed over the vault, so a crash leaves either the old or the new file, never a mix. On Unix the file is created owner-only (`0600`) and the directory is synced after the rename.
- **Migration.** A vault in an older format version is migrated when it is unlocked: every record is re-encrypted for the new version and the file rewritten atomically, with a backup kept until the new file reads back correctly. Key slots authenticate the header with its version field zeroed, so a migration never needs to rewrap a slot (it could not: it has only one of the two secrets). The version is still authenticated by the records and the seal.
- **No new dependencies** were needed: the encoding is a small hand-written binary format and atomic writes use the standard library.

### Attachments

Attachment contents cannot live in the vault file: it is read and rewritten whole on every save and capped at 256 MiB. Each attachment is instead encrypted into its own *blob* file in `<vault file>.attachments/`, named by its random identifier. The owning item's encrypted payload lists each attachment's identifier, name, size and time added, so names and sizes are never in plaintext.

```text
blob = header (48: magic, version, algorithm, vault_id, attachment_id) ‖ chunk 0 ‖ … ‖ final chunk
chunk i = XChaCha20-Poly1305(vault key, 64 KiB of plaintext, ad = header ‖ 0x03 ‖ i ‖ is_final)
```

- **Streaming.** Only one 64 KiB chunk of plaintext is in memory at a time, in a buffer wiped on drop, for files of any size.
- **Integrity.** The chunk index and final flag in each chunk's associated data detect reordered, dropped, duplicated and truncated chunks (the STREAM construction); the header binds every chunk to its vault and attachment, so blobs cannot be swapped. The size in the record must match. Blobs are never rewritten, so there is no older blob to roll back to.
- **Export without partial plaintext.** Decrypted data goes to a temporary file next to the destination, renamed into place only once every chunk has authenticated and the size matches. On any failure or cancellation the temporary file is deleted. Exports to the vault file or into the attachments directory are refused.
- **Lifecycle.** A new blob is complete on disk before an item refers to it. Blobs of removed attachments or items are deleted only after the vault file that no longer lists them has been saved. Blobs left behind by a crash or a failed save are deleted the next time the vault is opened.
- **Why not the `aead-stream` crate.** It implements the same STREAM idea with counter-derived nonces. Building it from the vault's existing `aead::encrypt`/`decrypt` with random 192-bit nonces adds about a hundred lines and no dependency, keeps a single AEAD code path, and needs no nonce bookkeeping. No new cryptographic primitive is involved; the composition is specified in [vault-format.md](vault-format.md#9-attachments).

## Session state

- While a vault is unlocked, the core holds the 256-bit vault key in a `secrecy` wrapper inside the Rust process, owned by a `Vault` value. The key is zeroized when the `Vault` is dropped, which is how a vault is locked. Quitting the app does not lock the vault first; the key's memory is released with the process.
- Records stay encrypted in memory. A record is decrypted on demand for a single request and dropped (zeroized) once the response has been built; secret fields are `SecretString`s and other text fields are wiped on drop. On open, every record is decrypted once (and dropped) to check that attachment identifiers are unique and to find unreferenced blobs. The core does not keep a long-lived plaintext cache of the vault.
- Locking discards the session. Any subsequent command that needs vault data fails with a `locked` error until the user unlocks again.
- In the shell, `Session` (`src-tauri/src/session.rs`) is the single owner of the unlocked `Vault`, behind a mutex in Tauri's managed state. It holds the vault path, the Argon2id parameters for new key slots and the time of the last command. It has no Tauri types, so the logic behind every command is unit-tested without a running app.
- An attachment transfer holds the session mutex for its whole duration, so other commands wait for it. A manual lock or an OS lock event sets a flag before waiting for the mutex; the transfer checks it after each chunk and stops, so locking is delayed by at most one chunk. The inactivity timer does not interrupt a transfer: a transfer counts as activity, and the session is touched again when it ends.
- There is one vault per installation, at `vault.nvault` in the app data directory (for example `%APPDATA%\io.github.acevflow.novavault` on Windows, `~/Library/Application Support/io.github.acevflow.novavault` on macOS and `~/.local/share/io.github.acevflow.novavault` on Linux). The path is chosen by the shell and never sent by the UI.

## Auto-lock

Three triggers lock the vault automatically. All of them wipe the vault key (by dropping the `Vault`), clear a copied field from the clipboard (see [Clipboard](#clipboard)) and then reload the main webview. Reloading throws away everything the UI holds in memory and returns it to its start screen without the UI having to subscribe to events, so the webview needs no event permission.

| Trigger | Implementation |
| ------- | -------------- |
| Inactivity | A background thread checks every 5 seconds and locks once the auto-lock timeout has passed since the last command. Every command counts as activity. The timeout defaults to 5 minutes and can be set from 1 to 60 minutes (`set_auto_lock_timeout`); it is stored in `settings.json` next to the vault file, which is not secret and falls back to the default if missing or invalid. |
| System suspend | Windows: `WM_POWERBROADCAST` / `PBT_APMSUSPEND`. macOS: `NSWorkspaceWillSleepNotification`. Linux: logind `PrepareForSleep(true)`. |
| Screen lock | Windows: `WM_WTSSESSION_CHANGE` / `WTS_SESSION_LOCK`. macOS: `com.apple.screenIsLocked` and `NSWorkspaceSessionDidResignActiveNotification` (fast user switching). Linux: logind `Session.Lock`, and `ActiveChanged(true)` from `org.freedesktop.ScreenSaver` or `org.gnome.ScreenSaver`. |

- The OS triggers sit behind one `LockMonitor` trait (`src-tauri/src/os_lock/`) with one implementation per OS, selected at compile time. Windows uses a dedicated thread with a hidden, never-shown top-level window: power broadcasts reach only top-level windows. macOS registers notification observers on the main thread. Linux listens to D-Bus signals on Tauri's async runtime.
- Locking on suspend runs synchronously inside the notification handler on Windows and macOS, so the key is wiped before the system sleeps.
- If a monitor cannot start (for example, no logind on Linux), the app prints a fixed message and keeps running with the inactivity timer and manual locking.
- **Scoped `unsafe`.** Windows and macOS offer these notifications only through FFI. The workspace denies `unsafe_code`, and only `os_lock/windows.rs` and `os_lock/macos.rs` opt out. Each `unsafe` block is a single FFI call with a `SAFETY` comment. The core crate keeps `#![forbid(unsafe_code)]`.

## Data flows

### Create a vault

1. The UI collects the master password (rated at least “Strong” by the UI's strength check) and sends it with a `create_vault` command.
2. The shell rejects an empty password, locks any open vault and creates the app data directory if needed. The core refuses to create a vault if anything already exists at the vault path.
3. The core generates a random vault identifier, a random 256-bit vault key and a recovery code from the OS CSPRNG.
4. It derives a wrapping key from the master password and another from the recovery code with Argon2id (default parameters), and wraps the vault key under each one, producing two key slots.
5. It writes the header, key slots, an empty record set and the integrity seal to disk atomically (see [vault-format.md](vault-format.md)). The vault stays unlocked.
6. The recovery code is returned to the UI exactly once, in display form (`{ recoveryCode }`). The user must retype it to confirm they have saved it before the flow completes. The core does not store the code itself.

### Unlock

1. The UI sends the master password or the recovery code with an `unlock` command.
2. The shell locks any open vault first, so a failed attempt leaves the app locked, and returns `vault_not_found` if there is no vault file. A recovery code is then parsed and checked (alphabet, length, checksum) before the vault file is read, so a typo is reported as `invalid_recovery_code` without running Argon2id.
3. The core reads the file and checks its whole structure (magic, version, header, key slots, record framing) before any key derivation.
4. It derives the wrapping key from the secret with the slot's stored Argon2id parameters and unwraps the vault key. A wrong secret or a damaged slot yields the single `invalid_credentials` error.
5. It verifies the whole-file integrity seal, migrates a file in an older format version, decrypts each record once to check attachment identifiers, and deletes unreferenced attachment blobs. The `Vault` is then stored in the session.
6. The command returns nothing. The UI then calls `list_items` to show the vault.

### Read and edit a record

1. The UI lists items with `list_items` (identifiers, types, titles and timestamps only) and searches and filters that list itself.
2. Selecting an item calls `get_item`. The core decrypts the record, verifying its authentication tag, and the shell returns the plain fields (title, user name, URL, timestamps) and, for each secret field, only whether it is set.
3. A secret field's value (password, notes, note text, secret value) is sent only when the user clicks **Reveal**, one field at a time, with `reveal_item_field`. The UI keeps it in component state and drops it when the user hides it or leaves the item.
4. **Copy** calls `copy_item_field`: the shell reads the field and places it on the clipboard itself, so the value never passes through the UI (see [Clipboard](#clipboard)).
5. Edits are sent with `update_item`. Secret fields the user did not change are sent as `null` and keep their stored value, so the edit form never needs secrets the user did not reveal. The core re-encrypts the record with a fresh nonce and writes the file atomically.

### Clipboard

Copying runs on one worker thread in the shell (`src-tauri/src/clipboard.rs`) that owns the clipboard through `arboard` for the lifetime of the app; on X11 and Wayland the copying process must keep serving the contents. The worker clears the clipboard after the configured delay (`set_clipboard_clear_delay`, 5 to 300 seconds, default 30), when the vault locks and when the app exits, but only if the clipboard still holds the copied value. Each copy carries the platform's clipboard-history opt-out. Per-OS behaviour, including X11 and Wayland, is described in [clipboard.md](clipboard.md).

### Lock

Triggered by the user, by the inactivity timer, or by OS events (suspend, screen lock) observed in the shell. The core zeroizes the vault key and drops the session, the shell clears a copied field from the clipboard, and the UI clears its in-memory view state and returns to the unlock screen. A manual or OS-triggered lock first stops a running attachment transfer at its next chunk.

### Attachments

1. **Add.** The UI calls `add_attachment` with the item id and a progress channel. The shell checks that the vault is unlocked and the item exists, then shows the native open dialog itself, without holding the session lock, so an open dialog never delays the auto-lock. It then opens the chosen file and the core streams it into a new blob, reporting progress after each chunk; the shell forwards it to the UI at most every 100 ms. The item is updated and the vault saved. The UI receives the attachment's id, name, size and time, never the path.
2. **Export.** `export_attachment` reads the attachment's name, shows the native save dialog with that name suggested (the dialog asks before replacing a file), then decrypts the blob to a temporary file next to the chosen destination and moves it into place only when the whole blob has authenticated.
3. **Delete.** `delete_attachment` removes the attachment from the item, saves the vault, then deletes the blob. Deleting an item deletes all its blobs the same way.

### Change master password or regenerate the recovery code

With the vault unlocked, the user first re-enters the current master password (or the recovery code, if the password is forgotten); the core checks it by unwrapping the matching in-memory key slot, so an unattended unlocked app cannot be taken over. The core then derives a new wrapping key from the new secret and re-wraps the existing vault key, replacing only the corresponding key slot. Records and attachments are not re-encrypted. Regenerating the recovery code replaces the recovery slot, which invalidates the previous code.

## Tauri command surface

- Commands are coarse-grained and intent-based, never generic ("read bytes", "decrypt blob").
- No command ever returns the vault key, wrapping keys, KDF output, or raw ciphertext.
- Inputs are validated in Rust; errors returned to the UI are typed and contain no secret material.
- Every handler (`src-tauri/src/commands.rs`) moves its arguments to a blocking worker thread, because Argon2id takes a noticeable time, then locks the session, records the activity and delegates to it. Three steps run outside the session lock: `lock` clears the clipboard first and then asks a running transfer to stop before it takes the lock; `copy_item_field` writes the clipboard after releasing it; the attachment commands show their native dialog before taking it.
- **Allow-list.** `src-tauri/build.rs` declares the commands, so Tauri generates one `allow-<command>` permission per command and rejects any call the window's capability does not grant. `capabilities/default.json` grants exactly these nineteen and no core or plugin permission. A test checks that the two lists match.
- **No paths from the UI.** The vault location is fixed by the shell, and attachment commands open the native file dialogs from Rust (`tauri-plugin-dialog`, whose own commands are not granted to the webview). The UI can therefore never make the shell read or write a file the user did not pick, and no response contains a path.

### Commands

Arguments are camelCase in JavaScript (`invoke('change_master_password', { currentCredential, newPassword })`). `credential` and `currentCredential` share one shape: `{ method: "password" | "recoveryCode", secret }`.

| Command | Arguments | Returns | Notes |
| ------- | --------- | ------- | ----- |
| `vault_status` | none | `{ exists, unlocked }` | Lets the UI choose the create or unlock screen at startup. |
| `create_vault` | `password` | `{ recoveryCode }` | Creates the vault and leaves it unlocked. The code (`XXXX-XXXX-…`) is returned only here; the UI must show it once and have the user confirm it was saved. |
| `unlock` | `credential`: `{ method: "password" \| "recoveryCode", secret }` | nothing | Locks any open vault first, so a failed attempt leaves the app locked. Recovery codes are accepted in any case, with or without dashes. |
| `lock` | none | nothing | Wipes the vault key. |
| `change_master_password` | `currentCredential`, `newPassword` | nothing | Checks `currentCredential` first, then re-wraps the vault key in the password slot. The recovery code is accepted as the current credential (forgotten-password flow). |
| `regenerate_recovery_code` | `currentCredential` | `{ recoveryCode }` | Checks `currentCredential` first, then replaces the recovery slot; the previous code stops working. |
| `get_settings` | none | `{ autoLockMinutes, clipboardClearSeconds }` | Works while locked; settings are not secret. |
| `set_auto_lock_timeout` | `minutes` | nothing | Requires an unlocked vault. `invalid_input` outside 1 to 60. |
| `set_clipboard_clear_delay` | `seconds` | nothing | Requires an unlocked vault. `invalid_input` outside 5 to 300. |
| `list_items` | none | `[{ id, kind, title, createdAtMs, updatedAtMs }]` | No secret fields. `kind` is `"password"`, `"note"` or `"secret"`; `id` is 32 hex digits. |
| `get_item` | `id` | item details (below) | Plain fields and `has*` flags only; never a secret value. |
| `reveal_item_field` | `id`, `field` | `{ value }` | One secret field: `"password"`, `"notes"`, `"body"` or `"value"`. `invalid_input` for a plain field or a field the item's kind does not have. Call only when the user asks to see the value. |
| `copy_item_field` | `id`, `field` | `{ clearAfterSeconds }` | Copies any field (also `"username"` and `"url"`) to the clipboard from Rust; the value is not returned. `clipboard_unavailable` if the clipboard cannot be written. |
| `create_item` | `item` | `id` | |
| `update_item` | `id`, `item` | nothing | Secret fields set to `null` (or left out) keep their stored value. The kind must match the stored item (`invalid_input` otherwise). |
| `delete_item` | `id` | nothing | Also deletes the item's attachment blobs, after the vault is saved. |
| `add_attachment` | `itemId`, `onProgress` (a `Channel`) | attachment or `null` | Shows the native open dialog and encrypts the chosen file into the item. `null` if the user closes the dialog. Progress messages are `{ done, total }` in bytes. |
| `export_attachment` | `itemId`, `attachmentId`, `onProgress` | `true`, or `false` if the dialog was closed | Shows the native save dialog with the file name suggested and decrypts the attachment there. Nothing is written at the destination if the data fails authentication. |
| `delete_attachment` | `itemId`, `attachmentId` | nothing | Removes the attachment from the item, saves, then deletes its blob. |

`item` is tagged by `kind`: `{ kind: "password", title, username, password, url, notes }`, `{ kind: "note", title, body }` or `{ kind: "secret", title, value, notes }`. For `create_item` every field is required (use empty strings). Secret fields are deserialized straight into `SecretString`.

Item details from `get_item` are `{ id, kind, title, createdAtMs, updatedAtMs, attachments, … }` with, per kind: `password`: `username`, `url`, `hasPassword`, `hasNotes`; `note`: `hasBody`; `secret`: `hasValue`, `hasNotes`. `attachments` is a list of `{ id, name, size, addedAtMs }`.

Every edit is saved to disk at once (atomically). If a save fails, the command returns `storage`; the edit stays in memory and is written by the next successful save.

### Errors

A failed command rejects with `{ code, message }`. The message is a fixed string. Details from the core (such as I/O error text, which can contain paths) are dropped, and no error ever contains a command argument.

| Code | Meaning |
| ---- | ------- |
| `locked` | The vault is locked. |
| `vault_not_found` | No vault exists yet (offer to create one). |
| `vault_exists` | `create_vault` was called but a vault already exists. |
| `invalid_credentials` | Wrong password or recovery code, or damaged key slot (intentionally indistinguishable). |
| `invalid_recovery_code` | The recovery code is malformed (length, character or checksum): a typo, caught before key derivation. |
| `invalid_input` | Malformed argument: an empty password, a malformed item or attachment id, a setting out of range, a plain field passed to `reveal_item_field`, a field the item's kind does not have, a different kind in `update_item`, or a picked file with no file name. |
| `item_not_found` | No item has that id. |
| `attachment_not_found` | The item has no attachment with that id. |
| `file_access` | The picked file could not be read, the export destination could not be written, or the destination is one of the vault's own files. |
| `cancelled` | An attachment transfer was stopped because the vault was locked. |
| `invalid_item` | The item is too large, the vault is full, the item already has 1000 attachments, the file name cannot be used as an attachment name, or a size would overflow. |
| `unsupported_vault` | Not a vault file, or a newer format version or unknown algorithm. |
| `corrupted` | The vault file or an attachment blob failed a structural or integrity check, or a blob is missing. |
| `storage` | The vault file, its directory or the settings file could not be read, written or checked. |
| `clipboard_unavailable` | The system clipboard could not be opened or written. |
| `internal` | Unexpected failure, for example of the system random number generator. |

## UI

The UI (`ui/src`) is a React + TypeScript app built with Vite. It holds no keys and calls only the commands listed above.

| Path | Contents |
| ---- | -------- |
| `api/` | Typed wrappers over every command (`commands.ts`), DTO types (`types.ts`) and `CommandError` with the error codes and user-facing messages (`errors.ts`). This is the only code that calls `invoke`. |
| `app/` | The router (a reducer over a `Route` union, not a router library: there are few screens and no URLs) and the app shell (skip link, navigation, lock button). |
| `screens/` | Create vault, unlock, settings, and `items/`: the vault screen with the item list (title search and type filter in `filterItems.ts`), item details with their attachments (`AttachmentList`), the create and edit form and the delete confirmation. |
| `recovery/` | `RecoveryCodeFlow`: shows a new recovery code once, prints it, and requires retyping it. Used at creation and on regeneration. |
| `password/` | Lazy-loaded zxcvbn strength estimate, strength meter and new-password fields. A master password must rate at least 3 of 4 (“Strong”). |
| `components/` | Accessible base components: button, text, text area and password fields, alert, focus-managed headings, `SecretField` (hidden until revealed) and `CopyButton`. |
| `format/` | Text formatting helpers (durations, dates, file sizes and transfer percentages). |
| `theme/` | Light and dark themes from CSS tokens, following the system unless the user picks one. The choice (not secret) is kept in `localStorage`. |

Routing starts from `vault_status`: no vault leads to the create flow, a locked vault to the unlock screen. A `locked` error from any command, the Lock button and every auto-lock (which reloads the webview) all end on the unlock screen.

Secrets in the UI: the master password, the recovery code while it is shown or retyped, and re-authentication secrets live only in component state. They are cleared after use and after failed attempts, never logged and never written to storage. JavaScript cannot wipe strings, so this limits how long they stay reachable rather than guaranteeing erasure (see the threat model). There is no copy button for the recovery code: it is not stored in the vault, so the shell cannot copy it, and it should be written down rather than pasted.

Secret item fields are masked by default. A revealed value lives only in the state of its `SecretField` and is dropped when hidden, when another item is selected and on lock. The edit form starts secret fields empty and sends `null` for any the user did not change; **Reveal current …** loads the stored value only on request. Copy buttons never receive the value.

Attachments are listed under the item's details with their size and date. **Attach file**, **Export** and **Remove** (after confirmation) call the attachment commands; a progress bar with the percentage and byte counts follows each transfer, and the other attachment actions are disabled meanwhile. Closing a native dialog cancels silently. The UI only ever holds attachment names, sizes and progress, never contents or paths.

Printing uses `window.print()` with a print stylesheet that hides everything except the recovery sheet.

## Shell hardening

- **Content Security Policy:** the production policy (`csp` in `tauri.conf.json`) is strict, with no remote origins, no inline scripts or styles and no `eval`; IPC is the only connection allowed. The development policy (`devCsp`) additionally allows the Vite dev server and inline code for hot reloading.
- **Webview hardening:** `withGlobalTauri` is off (the UI imports `@tauri-apps/api` instead of using a global object) and `freezePrototype` is on, so script cannot tamper with built-in prototypes.
- **Capabilities:** only the nineteen vault commands are granted (see [Tauri command surface](#tauri-command-surface)). No core or plugin permissions: no events, shell, filesystem, dialog or HTTP from the webview. Transfer progress uses a Tauri IPC channel passed as a command argument, which needs no extra permission.
- **No network:** core features do not open network connections.

## Why all cryptography lives in Rust

- **The webview is the largest attack surface.** It parses and renders content and runs a large JavaScript engine. Keeping keys out of it means a script-injection bug in the UI cannot directly exfiltrate the vault key.
- **Memory control.** JavaScript strings are immutable and garbage-collected, so secrets cannot be reliably wiped. Rust with `zeroize` and `secrecy` gives deterministic clearing and makes accidental copies or logging harder.
- **Audited primitives.** The RustCrypto crates (or `age`) provide reviewed implementations of Argon2id and AEAD ciphers. Using them from one place avoids mixing implementations across languages.
- **One testable core.** Crypto, format and key-slot logic can be unit-tested with known vectors without a UI or a running app, and reused by future tools (for example a format validator).
- **Smaller review scope.** Auditors and contributors can focus on one crate to verify the security properties of the whole application.

## Cryptography and dependencies

**Final choice.** NovaVault uses exactly one authenticated encryption algorithm, **XChaCha20-Poly1305** (`aead_id` `0x01` in every vault file and attachment blob), and exactly one key derivation function, **Argon2id** version 0x13. No other algorithm is implemented and readers reject any other `aead_id`. AES-GCM was considered and rejected (see [below](#why-xchacha20-poly1305-rather-than-aes-gcm)). Changing either choice would require a new format version.

The primitives live in `crates/novavault-core` (`kdf`, `aead`, `keys` and `error` modules). Everything else in the core is built on these few functions: recovery codes, key slots, records, the integrity seal and attachment chunks use only `derive_key`, `encrypt` and `decrypt`, and add no dependencies.

### Algorithms and parameters

| Purpose | Choice | Parameters |
| ------- | ------ | ---------- |
| Key derivation | Argon2id, version 0x13 (RFC 9106) | Defaults: 64 MiB memory, 3 iterations, 4 lanes, 16-byte random salt, 32-byte output (RFC 9106 second recommended option). Accepted range: memory 19 MiB to 4 GiB, iterations 2 to 64, lanes 1 to 16. |
| Authenticated encryption | XChaCha20-Poly1305 | 256-bit key, 192-bit random nonce per message, 128-bit tag. Sealed message layout: `nonce (24) ‖ ciphertext ‖ tag (16)`. |
| Randomness | OS CSPRNG via `getrandom` | Keys, salts and nonces. |

- **KDF bounds.** Argon2id parameters are stored in each key slot, so on unlock they are untrusted input. The lower bound (the OWASP minimum of 19 MiB and 2 iterations) stops a tampered file from silently downgrading protection; the upper bound stops a crafted file from forcing a huge allocation or an extremely long unlock. Bounds are checked again inside `derive_key`, so unchecked values cannot reach Argon2.
- **Nonces.** Every encryption draws a fresh random nonce. Callers cannot supply one; the internal function that accepts a fixed nonce exists only for known-answer tests.
- **Errors.** Every decryption failure (wrong key, modified nonce, ciphertext or tag, different associated data) returns the same `CryptoError::Decryption`, so the error reveals nothing about the cause. Error variants never contain secret data.
- **Key material in memory.** `SecretKey` holds its 32 bytes in a `secrecy::SecretBox`, which zeroizes them on drop and redacts them in `Debug`. It does not implement `Clone`, and its bytes are only readable inside the crate. Derived keys and decrypted plaintext are written straight into zeroizing storage, and the cipher's internal key copy is wiped too (the `zeroize` feature of `chacha20poly1305`).

### Why XChaCha20-Poly1305 rather than AES-GCM

- **Random nonces are safe.** With a 192-bit nonce the chance of a repeat is negligible even after far more messages than a vault will ever encrypt under one key. AES-GCM has a 96-bit nonce, for which NIST SP 800-38D caps random nonces at 2^32 messages per key. A nonce repeat in either cipher is catastrophic, so the extended nonce removes a whole class of bugs (counters, persisted nonce state) from the design.
- **Constant time everywhere.** ChaCha20 and Poly1305 are fast and constant-time in plain software. AES-GCM is only fast and side-channel resistant with hardware AES and carry-less multiply instructions, which would be an assumption about every user's machine on Windows, macOS and Linux.
- **Simplicity.** No nonce-misuse workaround or per-message key derivation layer is needed, and the same construction is used by other well-reviewed tools (for example libsodium's `crypto_aead_xchacha20poly1305_ietf`).
- **Trade-off accepted.** XChaCha20-Poly1305 is specified by an IETF draft (draft-irtf-cfrg-xchacha) rather than a finished RFC. It is a straightforward composition of RFC 8439 ChaCha20-Poly1305 with HChaCha20, and the implementation is tested against the draft's vector.

### Dependencies

Every cryptography-related dependency, with its locked version and the reason it was chosen, is listed in [dependencies.md](dependencies.md). Key handling, memory handling, auto-lock and per-OS behaviour are described in the [security design](security-design.md).

### Test vectors

- Argon2id: RFC 9106 §5.3 (with secret and associated data) and the reference implementation's `password`/`somesalt` vector, both run through the same parameter wiring as production derivation.
- XChaCha20-Poly1305: draft-irtf-cfrg-xchacha-03 Appendix A.3.1.
- Tamper detection: every single-bit flip of a sealed message, truncation, appended bytes, a wrong key and wrong associated data are all rejected.
- Vault format: known encodings of the header, of item payloads (versions 0 and 1) and of an attachment blob header (published in [vault-format.md](vault-format.md#13-test-vectors)); every truncation of a vault file; a flipped byte in every field of every section; a removed or replayed record; unsupported versions, algorithms and foreign files; a rewritten version field; migration from version 0 with either secret.
- Attachments: round trips at every chunk boundary and a 32 MiB file streamed through bounded buffers; truncation at chunk boundaries and inside chunks; flipped bytes in every region of every chunk; reordered, dropped, duplicated and appended chunks; blobs moved between attachments and vaults; size mismatches; missing blobs; no export file left behind after any failure or cancellation.

## Related documents

- [Threat model](threat-model.md)
- [Security design](security-design.md)
- [Cryptography dependencies](dependencies.md)
- [Vault format specification](vault-format.md)
- [Development guide](development.md)
- [Clipboard behaviour](clipboard.md)
