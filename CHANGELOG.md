# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- README with project overview, features, security model summary, tech stack, build instructions and roadmap.
- Security policy (`SECURITY.md`) with private vulnerability reporting instructions, response expectations and scope.
- Code of Conduct based on the Contributor Covenant 2.1.
- Contributing guidelines covering development setup, branch and commit conventions, code style, testing, the pull request process and rules for security-sensitive changes.
- This changelog.
- Project documentation in `docs/`, linked from the README:
  - Documentation index (`docs/README.md`).
  - Architecture overview: components, layers, trust boundary, data flows and why all cryptography lives in Rust.
  - Threat model: assets, adversaries, in-scope and out-of-scope threats, mitigations and residual risks.
  - Vault format specification skeleton (format version 0, draft): header, key slots, encrypted records, attachments and recovery code.
  - Development guide: per-OS setup, commands and project layout.
- GitHub issue forms for bug reports and feature requests; blank issues are disabled and vulnerability reports are directed to private security advisories and `SECURITY.md`.
- Pull request template with a security impact section and a contribution checklist.
- CI workflow running Rust format, clippy and tests and UI lint, typecheck and tests on Windows, macOS and Linux, with caching; steps are skipped until the corresponding component exists. Actions are pinned to commit SHAs.
- Dependabot configuration for Cargo, npm and GitHub Actions updates.
- CI status badge in the README.
- Cargo workspace with shared lints (unsafe code denied; `unwrap()`/`expect()` denied outside tests) and an empty, documented `novavault-core` library crate.
- Minimal Tauri 2 desktop shell (`src-tauri`) with one window, a strict production Content Security Policy, frozen prototypes, no global Tauri API and a default capability that grants no permissions.
- App icons generated from a new square emblem (`assets/icon.svg`).
- React + TypeScript UI scaffold in `ui/` built with Vite: strict TypeScript, ESLint (type-checked strict rules, React hooks, jsx-a11y), Prettier and Vitest with Testing Library.
- Single full local check, `npm run check`, covering formatting, linting, typechecking, tests and builds for the UI and the Rust workspace; documented in `docs/development.md`.
- Core cryptographic primitives in `novavault-core`:
  - Argon2id key derivation (`derive_key`) with validated `KdfParams`: RFC 9106 defaults (64 MiB, 3 iterations, 4 lanes) and enforced minimum and maximum bounds.
  - XChaCha20-Poly1305 `encrypt`/`decrypt` with a fresh random 192-bit nonce per message and a single generic error for every authentication failure.
  - Zeroizing `SecretKey` (via `secrecy`) and `Salt` types; random bytes come from the OS CSPRNG.
  - Typed `CryptoError` (via `thiserror`) with no secret data in any variant.
  - Unit tests with RFC 9106, Argon2 reference and XChaCha20-Poly1305 draft test vectors, plus tamper-detection tests.
- Emergency recovery codes (`RecoveryCode`) in `novavault-core`: 150 random bits from the OS CSPRNG encoded with a 32-symbol alphabet without look-alikes (no `0`, `1`, `O`, `I`), a 2-symbol checksum that catches every single-symbol typo and adjacent swap, a grouped display form (`XXXX-XXXX-…`, 8 groups of 4) and case- and separator-insensitive parsing with typed errors.
- Key slots (`KeySlot`, `SlotType`) in `novavault-core`: the vault key wrapped with XChaCha20-Poly1305 under an Argon2id key derived from the master password or the recovery code, with a fresh salt and nonce per wrap, the vault header and slot metadata authenticated as associated data, and a strict fixed 101-byte encoding.
- Key slot table (`KeySlots`) in `novavault-core`: create a vault (random 256-bit vault key, password slot, recovery slot and a new recovery code), unlock with the master password or the recovery code, change the master password by re-wrapping only the password slot, and regenerate the recovery code so the previous one stops working.
- Vault file format version 0 in `novavault-core`:
  - Versioned 32-byte plaintext header (`Header`): magic bytes, format version, AEAD identifier, reserved flags, slot count and a random vault identifier; magic and version are checked before anything else.
  - Item model (`Item`, `ItemContent`, `ItemId`, `ItemKind`): passwords, notes and secrets, each with an id, title, creation and update timestamps and an encrypted payload. Only the id and type are in plaintext; payloads are padded to 64-byte buckets and secret fields are zeroizing `SecretString`s.
  - `Vault`: create a vault, open it with the master password or the recovery code, list, read, insert, update and remove items, and save with atomic writes (temporary file, flush, rename; owner-only permissions on Unix).
  - A whole-file integrity seal that detects removed, duplicated, reordered or replayed records.
  - Typed `VaultError` for I/O, foreign files, unsupported versions and algorithms, corruption and wrong credentials, with no secret data in any variant.
  - Round-trip, version-mismatch, truncation, per-field corruption and record-replay tests, and known-encoding vectors for the header and payload.
- `Vault::change_password` and `Vault::regenerate_recovery_code` in `novavault-core`: rewrite only the matching key slot and save the vault; if the save fails the previous slot is kept, so memory always matches the file.
- Tauri command layer in `src-tauri`:
  - A vault session that is the single owner of the unlocked vault; the vault key never crosses IPC and is wiped on lock. One vault per installation, at a fixed path in the app data directory.
  - Commands `create_vault` (returns the recovery code once), `unlock` (master password or recovery code), `lock`, `change_master_password`, `regenerate_recovery_code`, `list_items` (no secret fields), `create_item`, `update_item` and `delete_item`, run off the main thread; every edit is saved immediately.
  - Typed, serializable command errors (`{ code, message }`) with fixed messages that never contain secrets, paths or command arguments.
  - The capability grants only these nine commands; commands are declared in `build.rs` so any other call is rejected.
  - Auto-lock after five minutes without a command, before system suspend and on screen lock, with one `LockMonitor` per OS (Win32 power and session notifications, AppKit notifications, logind and screen saver D-Bus signals). Auto-lock reloads the webview to discard the UI's in-memory view.
  - Unit tests for the session, errors, DTOs, the capability allow-list and the Windows message mapping.
- Command surface, error codes, auto-lock design and the shell's dependencies documented in `docs/architecture.md`; IPC and auto-lock residual risks in `docs/threat-model.md`.
- `vault_status` command reporting whether a vault exists and whether it is unlocked, so the UI can choose the create or unlock screen at startup.
- Cryptography and dependencies section in `docs/architecture.md`: algorithms, parameters, the XChaCha20-Poly1305 choice and the justification for every cryptography-related crate.

### Changed

- Vault format specification: AEAD fixed to XChaCha20-Poly1305; Argon2id bounds and salt, nonce and wrapped-key sizes filled in.
- Vault format specification: key slot byte layout, wrapping associated data (`header ‖ slot_metadata`), slot operations and the recovery code alphabet, length, checksum, display form and parsing rules are now specified, with a recovery code test vector.
- Vault format specification finalized for format version 0: file layout, header, record types and encoding, payload encoding and padding, integrity seal, reader validation order and test vectors. Attachments are no longer part of version 0 and will be specified with a later format version.
- Architecture document describes the core library modules and the vault file; the threat model records the payload padding and the integrity seal.
- CI builds the UI before the Rust checks (the Tauri shell embeds the built UI) and checks UI formatting; Dependabot also tracks the root npm package (Tauri CLI).

[Unreleased]: https://github.com/acevflow/NovaVault/commits/main
