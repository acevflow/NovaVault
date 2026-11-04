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
> NovaVault is in early development and is **not ready for real secrets**. There is no released build yet, and the vault format may change without notice until version 1.0.

## Overview

NovaVault is a desktop application that keeps your sensitive data in a single encrypted vault file on your own machine. It has no server, no account, no sync service and no telemetry: core features never touch the network. Everything is protected by a master password, with an emergency recovery code as a second way in.

The goal is a small, auditable password manager whose security model is easy to explain and whose storage format is publicly documented.

## Features

Planned for the first release (MVP):

- **Vault lifecycle**: create, unlock and lock a vault with a master password.
- **Passwords, notes and secrets**: full create, read, update and delete.
- **Encrypted file attachments** stored inside the vault.
- **Emergency recovery code**: shown once at creation, can be regenerated at any time (invalidating the previous one).
- **Master password change** without re-encrypting the whole vault.
- **Auto-lock** on inactivity, system suspend and screen lock.
- **Clipboard auto-clear** after copying a secret.

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
> The project scaffold is in place, but no vault features are implemented yet: the app opens a placeholder window. Per-OS setup is detailed in the [development guide](docs/development.md).

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

## Project status and roadmap

NovaVault is **pre-alpha**. Current progress toward the MVP:

- [x] Repository, license and community files
- [x] Project scaffold: Cargo workspace, Tauri 2 shell (strict CSP, minimal capabilities), React + TypeScript UI, single check command
- [x] Core crypto primitives: Argon2id key derivation, XChaCha20-Poly1305 encryption, zeroizing key types
- [x] Core key slots: vault key wrapped under the master password and the recovery code, recovery code generation and parsing, password change and recovery code regeneration
- [x] Core vault file format: versioned header, key slots, encrypted password, note and secret records, whole-file integrity seal, atomic saves (attachments come with the attachments feature below)
- [x] Tauri command layer: create, unlock and lock a vault; list, create, update and delete items; change the master password and regenerate the recovery code (UI screens pending)
- [ ] Create, unlock and lock a vault (UI)
- [ ] Passwords, notes and secrets (CRUD, UI)
- [ ] Encrypted file attachments
- [ ] Emergency recovery code and master password change (UI)
- [x] Auto-lock on inactivity, suspend and screen lock
- [ ] Clipboard auto-clear
- [x] Public vault format specification (format version 0, pre-release; attachments to be specified)
- [x] CI builds and tests on Windows, macOS and Linux
- [ ] First release

See [CHANGELOG.md](CHANGELOG.md) for what has changed so far.

## Documentation

Project documentation lives in [`docs/`](docs/README.md):

- [Architecture](docs/architecture.md): how the Rust core, the Tauri layer and the UI fit together, and why all cryptography lives in Rust.
- [Threat model](docs/threat-model.md): assets, adversaries, threats and mitigations.
- [Vault format specification](docs/vault-format.md): the versioned, public on-disk format (version 0, pre-release).
- [Development guide](docs/development.md): setup per OS, commands and project layout.

Planned: **Cryptography and dependencies**, covering algorithms, parameters and the justification for every cryptography-related crate.

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening an issue or pull request, and follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

NovaVault is free software, released under the [GNU General Public License v3.0 or later](LICENSE).
