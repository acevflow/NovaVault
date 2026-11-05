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

A short summary. The full design is described in the [architecture](docs/architecture.md) and [threat model](docs/threat-model.md) documents.

- **Local only.** The vault is a single file on disk. NovaVault does not need network access for any core feature.
- **Rust owns the secrets.** All cryptography and data access live in the Rust core. The React UI never receives the vault key or any raw key material; it talks to the core through a minimal set of Tauri commands.
- **Audited primitives only.** No custom cryptography. NovaVault relies on audited crates from the RustCrypto project (or `age`).
- **Key derivation:** Argon2id.
- **Authenticated encryption:** XChaCha20-Poly1305 or AES-GCM.
- **Two key slots.** A random 256-bit vault key encrypts all data. It is stored wrapped twice: once under a key derived from the master password and once under a key derived from the recovery code. Either slot opens the vault, and changing the master password only re-wraps the vault key.
- **Memory hygiene.** Secrets are held in `secrecy` types and wiped with `zeroize`. Secrets are never logged.
- **Hardened shell.** Strict Content Security Policy and a minimal Tauri capability set.
- **Open format.** The vault format is versioned and documented as a public specification ([format version 0](docs/vault-format.md), pre-release).

Found a vulnerability? Please read [SECURITY.md](SECURITY.md) and report it privately.

## Tech stack

| Layer       | Technology                                     |
| ----------- | ---------------------------------------------- |
| Core        | Rust (`crates/novavault-core`)                 |
| Shell       | [Tauri 2](https://tauri.app/) (`src-tauri`)    |
| UI          | React + TypeScript (`ui`)                      |
| Platforms   | Windows, macOS, Linux                          |

## Building from source

> [!NOTE]
> You can create, unlock and lock a vault, and manage passwords, notes and secrets, but NovaVault is not ready for real secrets yet. Per-OS setup is detailed in the [development guide](docs/development.md).

Prerequisites:

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- [Node.js](https://nodejs.org/) (LTS)
- The [Tauri 2 system prerequisites](https://tauri.app/start/prerequisites/) for your operating system

Build and run:

```sh
git clone https://github.com/acevflow/NovaVault.git
cd NovaVault
npm ci                # Tauri CLI
npm ci --prefix ui    # UI dependencies
npm run check         # full local check: format, lint, test (Rust and UI)
npm run dev           # start the app in development mode
npm run build         # build a release bundle
```

## Documentation

Project documentation lives in [`docs/`](docs/README.md):

- [Architecture](docs/architecture.md): how the Rust core, the Tauri layer and the UI fit together, and why all cryptography lives in Rust.
- [Threat model](docs/threat-model.md): assets, adversaries, threats and mitigations.
- [Vault format specification](docs/vault-format.md): the versioned, public on-disk format (version 0, pre-release).
- [Development guide](docs/development.md): setup per OS, commands and project layout.
- [Clipboard behaviour](docs/clipboard.md): how copy and auto-clear work on Windows, macOS, X11 and Wayland.

Algorithms, parameters and the justification for every cryptography-related crate are in [Architecture: Cryptography and dependencies](docs/architecture.md#cryptography-and-dependencies).

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening an issue or pull request, and follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

NovaVault is free software, released under the [GNU General Public License v3.0 or later](LICENSE).
