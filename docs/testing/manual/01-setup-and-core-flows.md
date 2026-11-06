# Manual test guide A: setup and core flows

Manual test procedure for running NovaVault and checking its core flows: vault creation, unlocking and locking, master password change, recovery code regeneration, and items (passwords, notes and secrets). It covers only features that exist in the code today. Attachments, auto-lock, clipboard auto-clear and appearance settings are not covered here.

Automated results are in the [Rust test report](../reports/rust.md) and the [UI test report](../reports/ui.md). This guide checks what those tests cannot: the real app, on a real OS, used by a person.

## Recording results

Fill in this table once per test run, then mark each case **Pass** or **Fail** and write what you saw in **Notes** (always for a failure: what happened instead, and any error message, word for word).

| | |
| --- | --- |
| Tester | |
| Date | |
| Commit tested | |
| Build | Development (`npm run dev`) / Release (installed bundle) |
| OS and version | |

Never use real passwords or personal data while testing. Use made-up values such as those suggested in the steps, and a vault that holds nothing of value.

## 1. Running the app

### 1.1 Prerequisites

Install Git, the Rust stable toolchain, Node.js LTS and the Tauri 2 system prerequisites for your OS, as described in the [development guide](../../development.md#prerequisites-all-platforms). Then get the code and install the dependencies from the repository root:

```sh
git clone https://github.com/acevflow/NovaVault.git
cd NovaVault
npm ci
npm ci --prefix ui
```

The commands in this section are the same on Windows (PowerShell or Command Prompt), macOS and Linux.

### 1.2 Development build

```sh
npm run dev
```

This starts the Vite dev server on `http://localhost:5173` and then compiles and opens the Tauri window. The first run compiles all Rust dependencies and can take several minutes. Changes to the UI reload in place; changes to Rust code rebuild and restart the app. Stop it with `Ctrl+C` in the terminal or by closing the window.

The development build uses a more permissive Content Security Policy (it allows the dev server), so security-related checks should also be repeated on a release build.

### 1.3 Release build

```sh
npm run build
```

This builds the UI into `ui/dist`, compiles the app in release mode and creates installers under `target/release/bundle/`. The unbundled executable is `target/release/novavault` (`novavault.exe` on Windows).

| OS | Bundles created | How to install and start |
| --- | --- | --- |
| Windows | `bundle/msi/*.msi` and `bundle/nsis/*-setup.exe` | Run either installer, then start **NovaVault** from the Start menu. |
| macOS | `bundle/macos/NovaVault.app` and `bundle/dmg/*.dmg` | Open the `.dmg` and drag NovaVault to Applications, or open `NovaVault.app` directly. The app is not signed, so the first start may need right-click, **Open**. |
| Linux | `bundle/deb/*.deb`, `bundle/rpm/*.rpm` and `bundle/appimage/*.AppImage` | Install the `.deb` or `.rpm` with your package manager, or make the AppImage executable (`chmod +x`) and run it. |

The bundles are not code-signed. Expect OS warnings (Windows SmartScreen, macOS Gatekeeper) and accept them only for builds you made yourself.

If a build fails on Windows with `os error 4551`, a Windows Application Control policy is blocking build scripts; see [Troubleshooting](../../development.md#troubleshooting) in the development guide.

### 1.4 Where the vault is stored

NovaVault manages one vault per user account. The shell chooses its location; the UI never sends a path. The data directory is the app data directory for the identifier `io.github.acevflow.novavault`:

| OS | Data directory |
| --- | --- |
| Windows | `%APPDATA%\io.github.acevflow.novavault\` |
| macOS | `~/Library/Application Support/io.github.acevflow.novavault/` |
| Linux | `~/.local/share/io.github.acevflow.novavault/` (or `$XDG_DATA_HOME/io.github.acevflow.novavault/`) |

It can contain:

| File | Contents |
| --- | --- |
| `vault.nvault` | The encrypted vault: key slots and all items. Created when the vault is created. |
| `vault.nvault.attachments/` | Encrypted attachment files, if any were added. |
| `settings.json` | Auto-lock timeout and clipboard clear delay. Not secret. Missing until a setting is changed; defaults are used then. |
| `.vault.nvault.<hex>.tmp` | Temporary file written during a save and renamed over `vault.nvault`. It should only exist for an instant; one left behind after the app has closed is worth a note. |

The development and release builds use the **same** data directory, so they open the same vault.

The theme preference (light, dark or system) is not in this directory: it is kept in the webview's local storage under the key `novavault.theme`.

## 2. Preconditions and clean state

### 2.1 Preconditions

Before every run:

- The app builds and starts on this machine (section 1).
- You use a test OS account or a machine with no vault you care about. If a vault already exists, back it up first (section 2.3).
- No other copy of NovaVault is running. Close every NovaVault window, and stop `npm run dev` with `Ctrl+C`.
- You have something to write on (paper or a plain text file kept only for testing) to record the test master passwords and recovery codes. Never reuse them anywhere.

Suggested test data:

| Purpose | Value |
| --- | --- |
| Master password | `correct-horse-battery-staple-7` |
| New master password (section 3.3) | `orbit-lantern-violet-canyon-42` |
| Weak password | `password1` |
| Accented title (search) | `Café Wi-Fi` |

### 2.2 Reset to a fresh install

A fresh install means the data directory does not exist, so the app opens on **Create your vault**.

1. Quit NovaVault.
2. Back up the data directory if it holds a vault you want to keep (section 2.3).
3. Delete the data directory:

   | OS | Command |
   | --- | --- |
   | Windows (PowerShell) | `Remove-Item -Recurse -Force "$env:APPDATA\io.github.acevflow.novavault"` |
   | macOS | `rm -rf ~/Library/Application\ Support/io.github.acevflow.novavault` |
   | Linux | `rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/io.github.acevflow.novavault"` |

4. Optional: to also reset the theme preference, open **Settings** in the unlocked vault and choose **Match system** under **Appearance** before step 1. Deleting the data directory does not reset it.
5. Start the app. **Expected:** the **Create your vault** screen.

Deleting the data directory deletes the vault and every attachment for good. NovaVault keeps no other copy.

### 2.3 Back up a test vault

Use this to keep a vault in a known state (for example with sample items) and return to it between cases.

1. Quit NovaVault, so no save is in progress.
2. Copy the whole data directory (not only `vault.nvault`: attachments and settings live next to it) to a folder outside it, for example `nv-backup-<date>`:

   | OS | Command |
   | --- | --- |
   | Windows (PowerShell) | `Copy-Item -Recurse "$env:APPDATA\io.github.acevflow.novavault" "$HOME\nv-backup"` |
   | macOS | `cp -R ~/Library/Application\ Support/io.github.acevflow.novavault ~/nv-backup` |
   | Linux | `cp -R "${XDG_DATA_HOME:-$HOME/.local/share}/io.github.acevflow.novavault" ~/nv-backup` |

3. Write down the master password and recovery code that open this backup; they are part of its state.

To restore, quit the app, delete the data directory (section 2.2, step 3), copy the backup back to the data directory path, and start the app. **Expected:** the **Unlock your vault** screen, and the recorded master password opens the vault.

A backup taken before a master password change or recovery code regeneration still opens with the old secrets: restoring it undoes the change.

### 2.4 Delete a test vault

When a run is finished, delete the test vault with section 2.2, step 3, and delete any backups and notes of test secrets you made.
