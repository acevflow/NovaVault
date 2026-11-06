<div align="center">

<img src="assets/logo.svg" alt="NovaVault" width="420">

**An open-source, local-first encrypted vault for your passwords, notes, secrets and files.**

[![CI](https://github.com/acevflow/NovaVault/actions/workflows/ci.yml/badge.svg)](https://github.com/acevflow/NovaVault/actions/workflows/ci.yml)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](LICENSE)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)
![Status](https://img.shields.io/badge/status-pre--alpha-orange.svg)
![Built with Tauri 2](https://img.shields.io/badge/built%20with-Tauri%202-24C8DB.svg)
![Core in Rust](https://img.shields.io/badge/core-Rust-B7410E.svg)
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.1-4baaaa.svg)](CODE_OF_CONDUCT.md)

</div>

---

> [!WARNING]
> NovaVault is pre-alpha and **not ready for real secrets**. It has not been independently audited, there is no released build yet (you have to build it from source), and the vault format may change until version 1.0.

## Overview

NovaVault is a desktop application that keeps your sensitive data in one encrypted vault on your own machine. It has no server, no account, no sync service and no telemetry, and core features never touch the network. A master password protects the vault, and an emergency recovery code gives you a second way in.

The goal is a small, auditable password manager whose security model is easy to explain and whose storage format is publicly documented.

## Features

Every MVP feature is implemented and runs from a source build on Windows, macOS and Linux:

- **Vault lifecycle.** Create, unlock and lock a vault with a master password. New passwords get live strength feedback (zxcvbn) and must be rated at least "Strong".
- **Emergency recovery code.** Shown once when the vault is created, with a print-only sheet and a retype step to confirm you saved it. It unlocks the vault if you forget the master password, and you can regenerate it in Settings, which invalidates the previous code.
- **Master password change.** Only the password key slot is re-wrapped, so the vault data is not re-encrypted.
- **Passwords, notes and secrets.** Create, view, edit and delete them, with a search by title (ignoring case and accents) and a type filter. Secret fields stay hidden until you reveal them, and the UI fetches them from the Rust core only on demand.
- **Encrypted file attachments.** Files are encrypted as a stream in authenticated 64 KiB chunks, so large files never sit in memory as a whole. You attach, export and remove them through native file dialogs, with a progress bar.
- **Auto-lock** after inactivity (1 to 60 minutes, default 5), before system suspend and on screen lock.
- **Clipboard auto-clear.** Copying is handled in Rust, so the value never reaches the UI. The clipboard is cleared after a configurable delay (default 30 seconds), on lock and on exit, and clipboard history and cloud sync are asked to skip the value. See [clipboard behaviour](docs/clipboard.md) for per-OS details.
- **Light and dark themes**, following the system by default.

Planned:

- [ ] First release with packaged installers.

Explicitly out of scope: cloud sync, browser extension or autofill, mobile apps and multi-user sharing.

## Security model

A short summary. The full design is in [security design](docs/security-design.md), [architecture](docs/architecture.md) and the [threat model](docs/threat-model.md), which also lists known limitations.

- **Local only.** The vault is a single file (plus one encrypted blob per attachment) in the app data directory. Core features never open a network connection.
- **Rust owns the secrets.** All cryptography and data access live in the Rust core (`crates/novavault-core`). The React UI never receives the vault key, a wrapping key or any key derivation output, and it talks to the core only through a small set of Tauri commands (see the [API reference](docs/api-reference.md)). Secret field values cross into the UI only when you reveal them, and copying happens in Rust.
- **Two audited primitives, no custom cryptography.** Argon2id (RFC 9106: 64 MiB, 3 iterations, 4 lanes) for key derivation and XChaCha20-Poly1305 with a fresh random 192-bit nonce for every encryption, both from RustCrypto crates. Every cryptography-related dependency is justified in [dependencies.md](docs/dependencies.md).
- **Two key slots.** A random 256-bit vault key encrypts all data. It is stored wrapped twice, once under a key derived from the master password and once under a key derived from the 150-bit recovery code, and either slot opens the vault. Changing the master password or regenerating the recovery code rewrites only its own slot.
- **Tamper detection.** Records, key slots and attachment chunks are authenticated with associated data that binds them to the vault and their position, and a whole-file integrity seal detects removed, duplicated, reordered or replayed records. Saves are atomic.
- **Memory hygiene.** Keys and secret values are held in `secrecy` types and wiped with `zeroize`, and secrets are never logged. Locking wipes the vault key.
- **Hardened shell.** Strict production Content Security Policy, frozen prototypes, no global Tauri API, and a capability that grants only NovaVault's own commands. File dialogs are opened from Rust, so the UI never handles file paths.
- **Open format.** The vault format is versioned and documented as a public specification ([format version 1](docs/vault-format.md), pre-release), with test vectors.

Found a vulnerability? Please read [SECURITY.md](SECURITY.md) and report it privately.

## Building from source

There is no released build yet. To try NovaVault, build it from source. The [development guide](docs/development.md) has the full per-OS setup, every command and troubleshooting tips.

### Prerequisites

All platforms:

- [Git](https://git-scm.com/)
- [Rust](https://www.rust-lang.org/tools/install) stable via `rustup`, with `rustfmt` and `clippy`
- [Node.js](https://nodejs.org/) LTS with `npm`

Plus the [Tauri 2 system prerequisites](https://tauri.app/start/prerequisites/) for your OS:

- **Windows:** [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the *Desktop development with C++* workload, Microsoft Edge WebView2 (preinstalled on Windows 10 1803+ and Windows 11), and the default MSVC Rust toolchain.
- **macOS:** Xcode Command Line Tools (`xcode-select --install`).
- **Linux:** WebKitGTK 4.1 and build tools. On Debian or Ubuntu:

  ```sh
  sudo apt update
  sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
    libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
  ```

  Fedora and Arch Linux packages are listed in the [development guide](docs/development.md#linux).

### Build and run

The commands are the same on every OS and shell:

```sh
git clone https://github.com/acevflow/NovaVault.git
cd NovaVault
npm ci                # Tauri CLI
npm ci --prefix ui    # UI dependencies
npm run check         # full local check: format, lint, typecheck, test and build (UI and Rust)
npm run dev           # run the app in development mode
npm run build         # build a release bundle for your OS
```

The vault (`vault.nvault`), its attachments folder and a non-secret `settings.json` are stored in the app data directory for `io.github.acevflow.novavault`:

| OS | Location |
| -- | -------- |
| Windows | `%APPDATA%\io.github.acevflow.novavault\` |
| macOS | `~/Library/Application Support/io.github.acevflow.novavault/` |
| Linux | `~/.local/share/io.github.acevflow.novavault/` (or `$XDG_DATA_HOME`) |

## Project structure

| Path | Contents |
| ---- | -------- |
| [`crates/novavault-core/`](crates/novavault-core) | Rust library: cryptography, vault format, key slots, recovery codes and attachments. UI-independent. |
| [`src-tauri/`](src-tauri) | Thin Tauri 2 shell: the vault session, commands, auto-lock, clipboard, CSP and capabilities. |
| [`ui/`](ui) | React + TypeScript front end (Vite, ESLint, Prettier, Vitest). |
| [`docs/`](docs/README.md) | Architecture, threat model, security design, vault format, API reference, development guide and per-file code reference. |
| [`assets/`](assets) | Logo and app icon sources. |
| [`scripts/check.mjs`](scripts/check.mjs) | The full local check run by `npm run check`. |
| [`.github/`](.github) | CI workflow (Windows, macOS, Linux), Dependabot, issue and pull request templates. |

The [development guide](docs/development.md#project-layout) has a more detailed layout.

## Documentation

Project documentation lives in [`docs/`](docs/README.md):

- [Architecture](docs/architecture.md): components, trust boundary, data flows, and why all cryptography lives in Rust.
- [Threat model](docs/threat-model.md): assets, adversaries, mitigations, out-of-scope threats and known limitations.
- [Security design](docs/security-design.md): key hierarchy, recovery code, memory handling, auto-lock and clipboard, with per-OS notes.
- [Cryptography dependencies](docs/dependencies.md): every cryptography-related crate, its version and why it was chosen.
- [Vault format specification](docs/vault-format.md): the versioned, public on-disk format (version 1, pre-release) with test vectors.
- [API reference](docs/api-reference.md): every Tauri command the UI can call, with arguments, results and error codes.
- [Development guide](docs/development.md): per-OS setup, commands, project layout and troubleshooting.
- [Clipboard behaviour](docs/clipboard.md): how copy and auto-clear work on Windows, macOS, X11 and Wayland.

### Testing

`npm run check` runs the full local check: formatting, linting, type checking, tests and builds for the UI and the Rust workspace. CI runs the same checks on Windows, macOS and Linux.

- [Test report](docs/testing/test-report.md): results of the latest automated runs, failures, bugs found and untested areas.
- [Manual test plan](docs/testing/manual-test-plan.md): the manual test guides, a results table to fill in and a bug report template.

### Code reference

Every hand-written source file and significant configuration file has its own reference page under [`docs/code/`](docs/code/README.md), covering its purpose, public API with exact signatures, security notes, dependencies and tests. The [code reference index](docs/code/README.md) groups the pages by layer: core, Tauri shell, UI and configuration.

Notable changes are recorded in [CHANGELOG.md](CHANGELOG.md).

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening an issue or pull request, and follow the [Code of Conduct](CODE_OF_CONDUCT.md).

Please do not report security vulnerabilities in public issues; follow [SECURITY.md](SECURITY.md) instead.

## License

NovaVault is free software, released under the [GNU General Public License v3.0 or later](LICENSE).
