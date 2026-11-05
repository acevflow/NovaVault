# [`src-tauri/Cargo.toml`](../../../src-tauri/Cargo.toml)

## Purpose

The Cargo manifest of `novavault`, the thin Tauri 2 shell binary around [`novavault-core`](../crates/novavault-core/Cargo.toml.md). It declares the shell's direct dependencies, the per-platform crates for the OS lock monitors and the build-time dependency on `tauri-build`, and inherits its package metadata and lint levels from the workspace manifest at the repository root ([`Cargo.toml`](../../../Cargo.toml)).

## Public API

### `[package]`

| Key            | Type      | Value                                                            | Purpose                                         |
| -------------- | --------- | ---------------------------------------------------------------- | ----------------------------------------------- |
| `name`         | string    | `novavault`                                                      | Package and binary name (`cargo test -p novavault`). |
| `description`  | string    | `NovaVault desktop app: thin Tauri 2 shell around novavault-core.` | One-line summary.                             |
| `version`      | inherited | `0.1.0`                                                          | Shared workspace version.                       |
| `edition`      | inherited | `2024`                                                           | Rust edition.                                   |
| `rust-version` | inherited | `1.85`                                                           | Minimum supported Rust version.                 |
| `license`      | inherited | `GPL-3.0-or-later`                                               | Project license.                                |
| `repository`   | inherited | `https://github.com/acevflow/NovaVault`                          | Source location.                                |
| `publish`      | inherited | `false`                                                          | Never published to crates.io.                   |

### `[build-dependencies]`

| Crate         | Requirement | Features | Locked  | Purpose                                                                                          |
| ------------- | ----------- | -------- | ------- | ------------------------------------------------------------------------------------------------ |
| `tauri-build` | `2`         | none     | 2.6.3   | Used by [`build.rs`](build.rs.md): embeds the config and capabilities and generates per-command permissions. |

### `[dependencies]` (all platforms)

| Crate                 | Requirement | Features                                      | Locked   | Purpose                                                                      |
| --------------------- | ----------- | --------------------------------------------- | -------- | ---------------------------------------------------------------------------- |
| `arboard`             | `3.7.0`     | `default-features = false`; `wayland-data-control` | 3.7.0 | Clipboard copy and clear from Rust ([`clipboard`](src/clipboard.rs.md)).  |
| `novavault-core`      | path `../crates/novavault-core` | defaults              | 0.1.0    | All cryptography, the vault format and the key slots.                        |
| `secrecy`             | `0.10.3`    | `serde`                                       | 0.10.3   | Deserializes IPC secrets straight into `SecretString` ([`dto`](src/dto.rs.md)). |
| `serde`               | `1.0.229`   | `derive`                                      | 1.0.229  | IPC (de)serialization of DTOs and settings.                                  |
| `serde_json`          | `1.0.154`   | defaults                                      | 1.0.154  | Reads and writes `settings.json` ([`settings`](src/settings.rs.md)).         |
| `tauri`               | `2`         | none                                          | 2.11.6   | The app shell: windows, IPC, capabilities, CSP.                              |
| `tauri-plugin-dialog` | `2.7.3`     | defaults                                      | 2.7.3    | Native open and save dialogs for attachments, called from Rust only.         |
| `thiserror`           | `2.0.21`    | defaults                                      | 2.0.21   | Derives `CommandError` ([`error`](src/error.rs.md)).                         |
| `zeroize`             | `1.9.1`     | `derive`                                      | 1.9.1    | Wipes response DTOs and the copied clipboard value.                          |

### Per-target dependencies (OS lock monitors)

| Target                    | Crate              | Requirement | Features                                                                                                              | Purpose                                                       |
| ------------------------- | ------------------ | ----------- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| `cfg(target_os = "windows")` | `windows`       | `0.61.3`    | `Win32_Foundation`, `Win32_Graphics_Gdi`, `Win32_System_LibraryLoader`, `Win32_System_RemoteDesktop`, `Win32_UI_WindowsAndMessaging` | Hidden window for suspend and session-lock messages ([`os_lock/windows`](src/os_lock/windows.rs.md)). |
| `cfg(target_os = "macos")`   | `block2`        | `0.6.2`     | defaults                                                                                                              | Objective-C blocks for notification observers.                |
| `cfg(target_os = "macos")`   | `objc2-app-kit` | `0.3.2`     | `NSWorkspace`                                                                                                         | Sleep and session notifications ([`os_lock/macos`](src/os_lock/macos.rs.md)). |
| `cfg(target_os = "macos")`   | `objc2-foundation` | `0.3.2`  | `NSDistributedNotificationCenter`, `NSNotification`, `NSOperation`, `NSString`, `block2`                              | Notification centers, including the screen-lock notification. |
| `cfg(target_os = "linux")`   | `futures-util`  | `0.3.34`    | `default-features = false`                                                                                            | Merges the D-Bus signal streams ([`os_lock/linux`](src/os_lock/linux.rs.md)). |
| `cfg(target_os = "linux")`   | `zbus`          | `5.13.2`    | `default-features = false`; `tokio`                                                                                   | D-Bus client for logind and screen saver signals.             |

Each per-target version equals the version in `Cargo.lock`.

### `[lints]`

| Key         | Value  | Effect                                                                                     |
| ----------- | ------ | ------------------------------------------------------------------------------------------ |
| `workspace` | `true` | Applies `unsafe_code = "deny"`, `missing_docs = "warn"`, `clippy::unwrap_used = "deny"`, `clippy::expect_used = "deny"`. |

## Details

### Dependency versions

Plain requirements are caret requirements; `tauri` and `tauri-build` are written as `2`, so any 2.x is allowed. The exact versions are pinned by the workspace `Cargo.lock` (the **Locked** column), and CI builds with `--locked`. `thiserror` also appears in `Cargo.lock` at 1.0.69, pulled in by other dependencies.

### Security-relevant settings

- **No extra `tauri` features.** `features = []` adds nothing to Tauri's default features, so optional, non-default APIs such as `tray-icon`, `protocol-asset` or `devtools` are not compiled in.
- **`tauri-plugin-dialog` is compiled in but not granted.** The plugin is registered in [`main.rs`](src/main.rs.md) so Rust commands can open native dialogs. The webview is given none of its permissions (see [`default.json.md`](capabilities/default.json.md)), so the UI cannot open a dialog or learn a path; the user's choice in the dialog is the only way a file path enters the shell.
- **`arboard` with `default-features = false`.** Drops image support, which the app never needs, and keeps the dependency tree smaller. `wayland-data-control` is added back so copy and clear work natively on Wayland rather than only through XWayland. Copying in Rust keeps copied values out of the webview.
- **`secrecy` with `serde`.** Lets command inputs deserialize directly into `SecretString`, so the master password, recovery codes and secret item fields are never held in a plain `String` in the shell. `secrecy` does not serialize secrets by default; the output DTOs that carry one do it explicitly.
- **`zeroize` with `derive`.** The two response DTOs in [`dto.rs`](src/dto.rs.md) that carry item data use `#[derive(Zeroize, ZeroizeOnDrop)]` so response values are wiped once serialized.
- **`zbus` with `default-features = false` and `tokio`.** Avoids zbus' own async executor and runs on Tauri's Tokio runtime. It talks only to the local system and session D-Bus.
- **Per-target OS crates.** Each platform compiles only its own monitor's crates. The crate comment says the Windows and macOS versions match those already pulled in by Tauri, so no new crates are added there.
- **`unsafe_code = "deny"` (inherited).** `deny` can be overridden per module: [`os_lock/windows.rs`](src/os_lock/windows.rs.md) and [`os_lock/macos.rs`](src/os_lock/macos.rs.md) use `#![allow(unsafe_code)]` for FFI with a justification per block. No other shell file uses `unsafe`.
- **`unwrap_used`/`expect_used` denied (inherited).** The root `clippy.toml` exempts tests.

## Dependencies

- **Internal**: the workspace manifest [`Cargo.toml`](../../../Cargo.toml) (package fields, lints) and [`clippy.toml`](../../../clippy.toml) (test exemptions); [`novavault-core`](../crates/novavault-core/Cargo.toml.md).
- **External**: the crates above. Every security-relevant one is justified in [`docs/dependencies.md`](../../dependencies.md#shell-src-tauri).

## Used by

- The workspace manifest lists `src-tauri` in `members`.
- The Tauri CLI (`tauri dev`, `tauri build`) builds this package, using [`tauri.conf.json`](tauri.conf.json.md).

## Tests

_Not applicable: a manifest has no tests. `cargo test -p novavault` builds the package and runs its unit tests (see the `*.rs.tests.md` pages under [`src/`](src/))._

## Notes

- **Inconsistency:** the comments above `arboard`, `tauri-plugin-dialog` and the OS lock crates say "Justified in docs/architecture.md". The per-crate justifications are in [`docs/dependencies.md`](../../dependencies.md); `docs/architecture.md` mentions `arboard` and `tauri-plugin-dialog` but not `windows`, `objc2-*`, `block2`, `zbus` or `futures-util`.
- `tauri-build` is not listed in `docs/dependencies.md`. It runs only at build time.
- `docs/dependencies.md` lists `tauri` with "defaults"; the manifest writes `features = []`, which is the same thing.
