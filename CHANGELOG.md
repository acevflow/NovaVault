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
- Configurable auto-lock timeout: `get_settings` and `set_auto_lock_timeout` commands (1 to 60 minutes, default 5; changing it requires an unlocked vault), stored in a non-secret `settings.json` next to the vault that falls back to the default if missing or invalid.
- `Vault::verify_password` and `Vault::verify_recovery_code` in `novavault-core`: check a secret against the in-memory key slots without changing anything.
- UI app shell: header with skip link, main navigation and lock button, typed screen routing (a reducer, no router library) that picks the create, unlock or vault screen from `vault_status` at startup, and a startup error screen with retry.
- Typed UI API layer (`ui/src/api`) over the Tauri commands, with a `CommandError` type mirroring the shell's error codes and user-facing messages.
- Light and dark themes from CSS tokens, following the system by default; accessible base components (button, text and password fields, alerts, focus-managed screen headings) and a responsive layout.
- Create-vault flow: master password with live zxcvbn strength feedback (at least “Strong” required) and confirmation, then a one-time recovery code screen with a print-only sheet, an “I have saved it” confirmation and a retype step that must match the code before the vault opens.
- Unlock screen with keyboard-accessible tabs for the master password and the recovery code (forgotten-password flow); the secret field is cleared after a failed attempt, with specific messages for wrong, malformed, unsupported and corrupted cases. The header's Lock button wipes the vault key and returns to this screen.
- Settings screen: change the master password (with strength feedback) and regenerate the recovery code (shown once, printable, retyped to confirm), both after re-entering the current password or recovery code; choose the auto-lock timeout (1, 5, 15, 30 or 60 minutes); choose the theme (match system, light or dark).
- Clipboard behaviour document (`docs/clipboard.md`): how copy and auto-clear work, and per-OS behaviour on Windows, macOS, X11 and Wayland (data-control and the XWayland fallback), linked from the README and the docs index.
- Cryptography and dependencies section in `docs/architecture.md`: algorithms, parameters, the XChaCha20-Poly1305 choice and the justification for every cryptography-related crate.

- `get_item` command: an item's plain fields (title, user name, URL, timestamps) and, for each secret field, only whether it is set. Secret values never come with it.
- `reveal_item_field` command: returns one secret field (password, notes, note body or secret value) when the user asks to see it.
- Item list on the vault screen: every password, note and secret sorted by title with its type, a search by title (ignoring case and accents), a type filter, a live result count, empty and no-match states, and a retry if loading fails. Typed UI wrappers for the item commands.
- Create, view, edit and delete passwords, notes and secrets. Item details show plain fields and keep every secret field hidden until the user reveals it (fetched from the shell on demand and dropped when hidden). The edit form leaves secret fields empty and keeps their stored values unless the user types a new one or loads the current one with “Reveal current …”. Deleting asks for confirmation (focus starts on Cancel; Escape cancels). Titles are required.
- Copy to clipboard with automatic clearing, handled in Rust: `copy_item_field` reads the field and copies it from the shell, so the value never reaches the UI. The clipboard is cleared after a configurable delay, when the vault locks (manually or automatically) and when the app exits, but only if it still holds the copied value. Clipboard history and cloud sync are asked to skip copied values (Windows history and cloud formats, macOS `org.nspasteboard.ConcealedType`, Linux `x-kde-passwordManagerHint`). Copy buttons for every field on the item details, with a polite status announcing when the clipboard clears.
- Clipboard clear delay setting: `set_clipboard_clear_delay` command (5 to 300 seconds, default 30; requires an unlocked vault), stored in `settings.json` (older files without it still load), and a Clipboard section in Settings (10 seconds, 30 seconds, 1 minute or 2 minutes).
- `clipboard_unavailable` error code.
- Encrypted file attachments in `novavault-core`: each file is encrypted as a stream into its own blob next to the vault file (`<vault>.attachments/`), in 64 KiB XChaCha20-Poly1305 chunks whose associated data binds the vault, the attachment, the chunk index and a final-chunk flag, so reordering, dropping, duplicating or truncating chunks and swapping blobs are detected. Only one chunk is held in memory. Exports are written to a temporary file and moved into place only after the whole file has authenticated. Blobs of removed attachments are deleted after the vault is saved, and unreferenced blobs are cleaned up on open. Tests cover large files, every chunk boundary, truncation and tampering.
- Vault format version 1: item payloads list their attachments (identifier, size, time added, name). Version 0 vaults are migrated when unlocked, with a backup kept until the new file is verified.
- `add_attachment`, `export_attachment` and `delete_attachment` commands. Native open and save dialogs are shown by the shell (`tauri-plugin-dialog`), so the UI never sends or receives a file path; transfers report progress over an IPC channel. `get_item` returns attachment metadata. New error codes `attachment_not_found`, `file_access` and `cancelled`.
- Attachments on the item details screen: list with size and date, attach a file, export it, or remove it after confirmation, with a progress bar during transfers.

### Changed

- Key slots authenticate the vault header with its version field zeroed, so format migrations never need to rewrap a slot; the version stays authenticated by the records and the integrity seal.
- Locking the vault manually or on suspend or screen lock stops a running attachment transfer at its next chunk instead of waiting for it.
- The `corrupted` error message no longer refers only to the vault file, since it also covers damaged attachments.
- Vault format specification updated to format version 1 with the attachment storage, blob layout, chunk encryption, reader rules, lifecycle and migration from version 0, plus new test vectors. The architecture document, threat model, documentation index and README status describe attachments.

- Architecture document describes the item commands, the read, reveal and copy data flow, the clipboard worker, the items UI and the `arboard` dependency; the threat model records clipboard exposure and per-field reveal; the development guide lists the new UI folders; README status ticks item management and clipboard auto-clear.

- Architecture document describes the UI structure, routing and how the UI handles secrets; the threat model records typed secrets in the webview and printed recovery codes as residual risks; the development guide lists the UI folders; README status ticks the vault lifecycle and recovery code UI.
- `change_master_password` and `regenerate_recovery_code` now require a `currentCredential` (the current master password, or the recovery code for the forgotten-password flow) and return `invalid_credentials` if it is wrong.
- Vault format specification: AEAD fixed to XChaCha20-Poly1305; Argon2id bounds and salt, nonce and wrapped-key sizes filled in.
- Vault format specification: key slot byte layout, wrapping associated data (`header ‖ slot_metadata`), slot operations and the recovery code alphabet, length, checksum, display form and parsing rules are now specified, with a recovery code test vector.
- Vault format specification finalized for format version 0: file layout, header, record types and encoding, payload encoding and padding, integrity seal, reader validation order and test vectors. Attachments are no longer part of version 0 and will be specified with a later format version.
- Architecture document describes the core library modules and the vault file; the threat model records the payload padding and the integrity seal.
- `update_item` now keeps a secret field's stored value when it is sent as `null` or left out, so editing an item never requires loading its secrets into the UI; changing an item's type is rejected with `invalid_input`.
- CI builds the UI before the Rust checks (the Tauri shell embeds the built UI) and checks UI formatting; Dependabot also tracks the root npm package (Tauri CLI).

[Unreleased]: https://github.com/acevflow/NovaVault/commits/main
