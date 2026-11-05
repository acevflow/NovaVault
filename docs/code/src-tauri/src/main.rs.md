# [`src-tauri/src/main.rs`](../../../../src-tauri/src/main.rs)

## Purpose

The entry point of the NovaVault desktop app, a thin Tauri 2 shell around `novavault-core`. It declares the shell's modules and builds the Tauri app:

- registers the dialog plugin and the 19 commands;
- creates the session for the fixed vault location;
- starts the clipboard worker, the idle timer and the OS lock monitor;
- clears the clipboard on exit.

All cryptography and vault logic live in the core, and the webview never receives key material.

## Public API

_Not applicable: this is a binary crate root and exposes no public items. Its private items are described below._

## Details

### Modules

`mod` declarations, all private:

- [`autolock`](autolock.rs.md): locking on inactivity and OS events, and resetting the UI.
- [`clipboard`](clipboard.rs.md): copying item fields and clearing them again.
- [`commands`](commands.rs.md): the Tauri command handlers (the whole IPC surface).
- [`dto`](dto.rs.md): command inputs and outputs.
- [`error`](error.rs.md): the typed, secret-free errors returned to the UI.
- [`os_lock`](os_lock/mod.rs.md): per-OS suspend and screen-lock monitors.
- [`session`](session.rs.md): the unlocked vault and the logic behind every command.
- [`settings`](settings.rs.md): the non-secret settings file.

`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` hides the console window on Windows release builds.

### `VAULT_FILE_NAME`

`const VAULT_FILE_NAME: &str = "vault.nvault";`. The vault lives at `<app data dir>/vault.nvault`, and `settings.json` sits next to it.

- **Security**: the location is fixed by the shell. The UI never sends a path, so a compromised webview cannot point the app at arbitrary files.

### `main`

`fn main() -> ExitCode`

1. Builds `tauri::Builder::default()`:
   1. `.plugin(tauri_plugin_dialog::init())`: native file dialogs. The plugin is used from Rust only; the capability grants the webview none of its commands.
   2. `.setup(...)`, on the main thread:
      1. resolves `app.path().app_data_dir()?.join("vault.nvault")`;
      2. creates `Session::new(path, KdfParams::default(), Instant::now())`;
      3. manages `AppState::new(session)` and `ClipboardManager::start(SystemClipboard::open)`. Both are managed before the timers start, because the lock path reads them;
      4. calls `autolock::start_idle_timer`;
      5. calls `start_os_lock_monitor`.
   3. `.invoke_handler(tauri::generate_handler![...])` with the 19 commands:

      | Group | Commands |
      | ----- | -------- |
      | Vault | `vault_status`, `create_vault`, `unlock`, `lock`, `change_master_password`, `regenerate_recovery_code` |
      | Settings | `get_settings`, `set_auto_lock_timeout`, `set_clipboard_clear_delay` |
      | Items | `list_items`, `get_item`, `reveal_item_field`, `copy_item_field`, `create_item`, `update_item`, `delete_item` |
      | Attachments | `add_attachment`, `export_attachment`, `delete_attachment` |

      The comment says to keep this list in sync with `COMMANDS` in `build.rs`.
   4. `.build(tauri::generate_context!())`.
2. If the build fails, prints `NovaVault failed: {error}` and returns `ExitCode::FAILURE`. Nothing is open or copied at that point, and Tauri's startup errors carry no vault secrets.
3. `app.run(...)`: on `RunEvent::Exit`, calls `ClipboardManager::shutdown()` if the state exists. A copied secret is cleared before the clipboard is released, so an X11 clipboard manager cannot take it over.
4. Returns `ExitCode::SUCCESS`.

- **Errors**: the `setup` closure propagates a failure to resolve the app data directory with `?`.
- **Security**: the session starts locked. The webview's IPC surface is exactly the registered commands, further restricted by the capability.

### `start_os_lock_monitor`

`fn start_os_lock_monitor(app: tauri::AppHandle)`

Builds a `LockSink` that calls `autolock::lock_and_reset_ui(&app, reason)`, and calls `os_lock::platform_monitor().start(sink)`. An error is printed as `NovaVault: {error}`, for example `NovaVault: OS lock events are unavailable: not supported on this platform`. It is not fatal: the inactivity timer and manual locking still protect the vault.

- **Edge cases and invariants**: it runs inside `setup`, so it is on the main thread. The macOS monitor requires that.

### Platform differences

- On Windows release builds, the binary uses the GUI subsystem.
- `platform_monitor` picks the OS monitor at compile time (see [`os_lock/mod.rs.md`](os_lock/mod.rs.md#platform-differences)).
- The app data directory is resolved by Tauri per platform. This file does not choose it.

## Dependencies

- **Internal**: every module of the shell (listed above).
- **External**:
  - `tauri`: the builder, the run loop and the managed state;
  - `tauri-plugin-dialog`: the file dialogs used by the attachment commands;
  - `novavault_core`: `KdfParams::default()`, the Argon2id parameters for new key slots.

## Used by

_Not applicable: this is the binary entry point._

## Tests

- [`capability_grants_exactly_the_declared_commands`](../../../../src-tauri/src/main.rs#L132): extracts the quoted names from `const COMMANDS` in [`build.rs`](../../../../src-tauri/build.rs) and maps each to `allow-<name with dashes>`. Checks that there are 19 of them and that they equal the `permissions` of [`capabilities/default.json`](../../../../src-tauri/capabilities/default.json), sorted. A new command can then never be callable by accident.

## Notes

- The test compares `build.rs` with the capability, but not with `generate_handler!` in this file. A command added to `build.rs` and the capability but missing from `generate_handler!` would fail at runtime, not in the test. The comment above `generate_handler!` is the only safeguard.
- Open question: what the user sees when `setup` fails (for example, no app data directory)? This file assumes Tauri reports it through the `build` error. That depends on Tauri's behaviour and is not tested here.
