# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Vault and cryptography (`novavault-core`)

- Argon2id key derivation (`derive_key`) with validated `KdfParams`: RFC 9106 defaults (64 MiB, 3 iterations, 4 lanes) and enforced minimum and maximum bounds.
- XChaCha20-Poly1305 `encrypt`/`decrypt` with a fresh random 192-bit nonce per message and a single generic error for every authentication failure.
- Zeroizing `SecretKey` (via `secrecy`) and `Salt` types; random bytes come from the OS CSPRNG. Typed `CryptoError` (via `thiserror`) with no secret data in any variant.
- Emergency recovery codes (`RecoveryCode`): 150 random bits from the OS CSPRNG encoded with a 32-symbol alphabet without look-alikes (no `0`, `1`, `O`, `I`), a 2-symbol checksum that catches every single-symbol typo and adjacent swap, a grouped display form (`XXXX-XXXX-…`, 8 groups of 4) and case- and separator-insensitive parsing with typed errors.
- Key slots (`KeySlot`, `SlotType`): the vault key wrapped with XChaCha20-Poly1305 under an Argon2id key derived from the master password or the recovery code, with a fresh salt and nonce per wrap, the vault header and slot metadata authenticated as associated data, and a strict fixed 101-byte encoding.
- Key slot table (`KeySlots`): create a vault (random 256-bit vault key, password slot, recovery slot and a new recovery code), unlock with the master password or the recovery code, change the master password by re-wrapping only the password slot, and regenerate the recovery code so the previous one stops working.
- Vault file format:
  - Versioned 32-byte plaintext header (`Header`): magic bytes, format version, AEAD identifier, reserved flags, slot count and a random vault identifier; magic and version are checked before anything else.
  - Item model (`Item`, `ItemContent`, `ItemId`, `ItemKind`): passwords, notes and secrets, each with an id, title, creation and update timestamps and an encrypted payload. Only the id and type are in plaintext; payloads are padded to 64-byte buckets and secret fields are zeroizing `SecretString`s.
  - `Vault`: create a vault, open it with the master password or the recovery code, list, read, insert, update and remove items, and save with atomic writes (temporary file, flush, rename; owner-only permissions on Unix).
  - A whole-file integrity seal that detects removed, duplicated, reordered or replayed records.
  - Typed `VaultError` for I/O, foreign files, unsupported versions and algorithms, corruption and wrong credentials, with no secret data in any variant.
- `Vault::change_password` and `Vault::regenerate_recovery_code`: rewrite only the matching key slot and save the vault; if the save fails the previous slot is kept, so memory always matches the file.
- `Vault::verify_password` and `Vault::verify_recovery_code`: check a secret against the in-memory key slots without changing anything.
- Encrypted file attachments: each file is encrypted as a stream into its own blob next to the vault file (`<vault>.attachments/`), in 64 KiB XChaCha20-Poly1305 chunks whose associated data binds the vault, the attachment, the chunk index and a final-chunk flag. Only one chunk is held in memory. Exports are written to a temporary file and moved into place only after the whole file has authenticated. Blobs of removed attachments are deleted after the vault is saved, and unreferenced blobs are cleaned up on open.
- Vault format version 1: item payloads list their attachments (identifier, size, time added, name). Version 0 vaults are migrated when unlocked, with a backup kept until the new file is verified.
- Tests: RFC 9106, Argon2 reference and XChaCha20-Poly1305 draft test vectors; known-encoding vectors for the header, payload and recovery code; round-trip, version-mismatch, truncation, per-field corruption, record-replay and tamper-detection tests; attachment tests for large files, every chunk boundary, truncation and tampering.

#### Tauri shell (`src-tauri`)

- Minimal Tauri 2 desktop shell with one window.
- A vault session that is the single owner of the unlocked vault. One vault per installation, at a fixed path in the app data directory; every edit is saved immediately.
- Commands, run off the main thread and documented in `docs/api-reference.md`:
  - `vault_status`, `create_vault` (returns the recovery code once), `unlock` (master password or recovery code), `lock`, `change_master_password` and `regenerate_recovery_code`.
  - `get_settings`, `set_auto_lock_timeout` (1 to 60 minutes, default 5) and `set_clipboard_clear_delay` (5 to 300 seconds, default 30); both setters require an unlocked vault. Settings are stored in a non-secret `settings.json` next to the vault that falls back to the defaults if missing or invalid.
  - `list_items` (no secret fields), `get_item` (plain fields, attachment metadata and, for each secret field, only whether it is set), `reveal_item_field` (one secret field on request), `copy_item_field`, `create_item`, `update_item` and `delete_item`.
  - `add_attachment`, `export_attachment` and `delete_attachment`, with native open and save dialogs shown by the shell (`tauri-plugin-dialog`) and progress reported over an IPC channel.
- Typed, serializable command errors (`{ code, message }`), including `clipboard_unavailable`, `attachment_not_found`, `file_access` and `cancelled`.
- Auto-lock after a configurable time without a command, before system suspend and on screen lock, with one `LockMonitor` per OS (Win32 power and session notifications, AppKit notifications, logind and screen saver D-Bus signals). Auto-lock reloads the webview to discard the UI's in-memory view.
- Copy to clipboard with automatic clearing, handled in Rust by a clipboard worker. The clipboard is cleared after the configured delay, when the vault locks and when the app exits, but only if it still holds the copied value.
- Unit tests for the session, errors, DTOs, settings, the capability allow-list and the Windows message mapping.

#### User interface (`ui`)

- React + TypeScript UI built with Vite: strict TypeScript, ESLint (type-checked strict rules, React hooks, jsx-a11y), Prettier and Vitest with Testing Library.
- App shell: header with skip link, main navigation and lock button, typed screen routing (a reducer, no router library) that picks the create, unlock or vault screen from `vault_status` at startup, and a startup error screen with retry.
- Typed API layer (`ui/src/api`) over the Tauri commands, with a `CommandError` type mirroring the shell's error codes and user-facing messages.
- Light and dark themes from CSS tokens, following the system by default; accessible base components (button, text and password fields, alerts, focus-managed screen headings) and a responsive layout.
- Create-vault flow: master password with live zxcvbn strength feedback (at least “Strong” required) and confirmation, then a one-time recovery code screen with a print-only sheet, an “I have saved it” confirmation and a retype step that must match the code before the vault opens.
- Unlock screen with keyboard-accessible tabs for the master password and the recovery code (forgotten-password flow); the secret field is cleared after a failed attempt, with specific messages for wrong, malformed, unsupported and corrupted cases. The header's Lock button wipes the vault key and returns to this screen.
- Settings screen: change the master password (with strength feedback) and regenerate the recovery code (shown once, printable, retyped to confirm), both after re-entering the current password or recovery code; auto-lock timeout (1, 5, 15, 30 or 60 minutes); clipboard clear delay (10 seconds, 30 seconds, 1 minute or 2 minutes); theme (match system, light or dark).
- Item list: every password, note and secret sorted by title with its type, a search by title (ignoring case and accents), a type filter, a live result count, empty and no-match states, and a retry if loading fails.
- Create, view, edit and delete passwords, notes and secrets. Item details keep every secret field hidden until the user reveals it (fetched on demand and dropped when hidden). The edit form leaves secret fields empty and keeps their stored values unless the user types a new one or loads the current one with “Reveal current …”. Deleting asks for confirmation (focus starts on Cancel; Escape cancels). Titles are required.
- Copy buttons for every field on the item details, with a polite status announcing when the clipboard clears.
- Attachments on the item details screen: list with size and date, attach a file, export it, or remove it after confirmation, with a progress bar during transfers.

#### Project, tooling and documentation

- README, security policy (`SECURITY.md`), Code of Conduct (Contributor Covenant 2.1), contributing guidelines and this changelog.
- Documentation in `docs/`, indexed in `docs/README.md` and linked from the README: architecture overview, threat model, security design, cryptography dependency inventory (`docs/dependencies.md`), vault format specification (format version 1, with test vectors), Tauri command API reference, development guide and clipboard behaviour (`docs/clipboard.md`).
- Rustdoc and TSDoc for the core library, the Tauri shell and the UI, explaining the security reasoning behind each module.
- GitHub issue forms for bug reports and feature requests (blank issues disabled, vulnerabilities directed to private advisories) and a pull request template with a security impact section.
- CI workflow running UI format, lint, typecheck, tests and build, then Rust format, clippy and tests on Windows, macOS and Linux, with caching and actions pinned to commit SHAs; CI status badge in the README.
- Dependabot configuration for Cargo, npm (root and `ui/`) and GitHub Actions updates.
- Cargo workspace with shared lints (unsafe code denied; `unwrap()`/`expect()` denied outside tests).
- Single full local check, `npm run check` (`scripts/check.mjs`), covering formatting, linting, typechecking, tests and builds for the UI and the Rust workspace.
- App icons generated from a square emblem (`assets/icon.svg`).

### Changed

- Vault format version 0 was replaced by version 1 for attachments; version 0 vaults are migrated on unlock.
- Key slots authenticate the vault header with its version field zeroed, so format migrations never need to rewrap a slot; the version stays authenticated by the records and the integrity seal.
- `change_master_password` and `regenerate_recovery_code` require a `currentCredential` (the current master password, or the recovery code for the forgotten-password flow) and return `invalid_credentials` if it is wrong.
- `update_item` keeps a secret field's stored value when it is sent as `null` or left out, so editing an item never requires loading its secrets into the UI; changing an item's type is rejected with `invalid_input`.
- Locking the vault manually or on suspend or screen lock stops a running attachment transfer at its next chunk instead of waiting for it.
- The `corrupted` error message no longer refers only to the vault file, since it also covers damaged attachments.
- CI builds the UI before the Rust checks (the Tauri shell embeds the built UI) and checks UI formatting.
- README rewritten to match the implemented code: available features, security model, per-OS build instructions, data locations, project structure and links to every document in `docs/`.
- Threat model limited to implemented mitigations, with a section of known limitations and gaps; the architecture document links to the dependency inventory instead of duplicating it.
- Contributing guidelines describe the implemented setup and `npm run check` instead of announcing them as coming soon.

### Fixed

- The OS lock monitors did not build on macOS and Linux (duplicate screen saver proxy types, a target-specific error variant); the Linux monitor now refuses a second start like the others.
- Development guide: running the UI tests in watch mode from the repository root skipped the UI's Vite config and most tests failed; the guide now runs Vitest from `ui/`, and Rust commands use `--locked` like CI.
- Threat model: corrected the recovery code entropy (150 bits), removed an unimplemented fuzz-testing claim and clarified that new key slots always use the default Argon2id parameters.
- Broken link from the clipboard document to a renamed threat model section.
- Inaccurate code documentation: `SecretKey::from_bytes` copy semantics, the recovery code specification reference, a stale password strength comment, the delete prompt wording and why the recovery code has no copy button.

### Security

- All cryptography and data access live in Rust; the UI never receives the vault key, a wrapping key or any key derivation output. Secret field values cross IPC only when the user reveals one field, and copying happens entirely in Rust.
- Strict production Content Security Policy, frozen prototypes and no global Tauri API. The window's capability grants only NovaVault's own commands, which are declared in `build.rs` so any other call is rejected; file dialogs are opened from Rust, so the UI never sends or receives a file path.
- Keys and secret values are held in `secrecy` types and wiped with `zeroize`; locking (manual, idle, suspend or screen lock) wipes the vault key. Command errors use fixed messages that never contain secrets, paths or command arguments.
- Tamper detection: records, key slots and attachment chunks are authenticated with associated data binding them to the vault and their position, and a whole-file integrity seal detects removed, duplicated, reordered or replayed records. Attachment chunk reordering, dropping, duplication, truncation and blob swaps are detected.
- Vault saves are atomic with owner-only permissions on Unix; payloads are padded to hide exact sizes.
- The clipboard is cleared after a delay, on lock and on exit, and clipboard history and cloud sync are asked to skip copied values (Windows history and cloud formats, macOS `org.nspasteboard.ConcealedType`, Linux `x-kde-passwordManagerHint`).

[Unreleased]: https://github.com/acevflow/NovaVault/commits/main
