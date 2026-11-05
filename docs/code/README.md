# Code reference

Per-file reference documentation for NovaVault. `docs/code/` mirrors the source tree: each hand-written source file and each significant configuration file has one page at the same relative path with `.md` appended (for example `crates/novavault-core/src/aead.rs` → [`crates/novavault-core/src/aead.rs.md`](crates/novavault-core/src/aead.rs.md)). Every page follows [TEMPLATE.md](TEMPLATE.md), which also lists what is covered and what is excluded.

The inline `#[cfg(test)] mod tests` of a Rust file is documented on a companion page named `<file>.rs.tests.md`, listed next to its source page below. TypeScript test files have their own source files and therefore their own pages.

For the design behind the code, start with the [architecture](../architecture.md), the [security design](../security-design.md) and the [vault format specification](../vault-format.md). The IPC surface is summarized in the [API reference](../api-reference.md).

- [Core](#core)
- [Tauri shell](#tauri-shell)
- [UI](#ui)
- [Configuration](#configuration)

## Core

`crates/novavault-core`: cryptography, the vault format and the key slots. It is a UI-independent library.

| File | Description |
| ---- | ----------- |
| [`src/lib.rs`](crates/novavault-core/src/lib.rs.md) | Crate root: declares the modules, forbids `unsafe` and re-exports the public API. |
| [`src/error.rs`](crates/novavault-core/src/error.rs.md) | Typed errors: `CryptoError` for primitives and key slots, `VaultError` for vault files. |
| [`src/error.rs` tests](crates/novavault-core/src/error.rs.tests.md) | Pins the generic error messages and the `CryptoError` → `VaultError` conversion. |
| [`src/keys.rs`](crates/novavault-core/src/keys.rs.md) | `SecretKey`, the zeroizing 256-bit key container, and `Salt`. |
| [`src/keys.rs` tests](crates/novavault-core/src/keys.rs.tests.md) | Random generation, construction and redacted `Debug` of keys and salts. |
| [`src/kdf.rs`](crates/novavault-core/src/kdf.rs.md) | Argon2id (RFC 9106) key derivation with validated parameters. |
| [`src/kdf.rs` tests](crates/novavault-core/src/kdf.rs.tests.md) | Known-answer vectors, determinism, defaults and parameter bounds. |
| [`src/aead.rs`](crates/novavault-core/src/aead.rs.md) | XChaCha20-Poly1305 encryption with a fresh random nonce per message. |
| [`src/aead.rs` tests](crates/novavault-core/src/aead.rs.tests.md) | Draft test vector, round trips, nonce freshness and tamper rejection. |
| [`src/recovery.rs`](crates/novavault-core/src/recovery.rs.md) | Emergency recovery codes: generation, checksum, display form and parsing. |
| [`src/recovery.rs` tests](crates/novavault-core/src/recovery.rs.tests.md) | Alphabet, checksum vector, lenient parsing and exhaustive typo detection. |
| [`src/header.rs`](crates/novavault-core/src/header.rs.md) | The 32-byte plaintext vault header: encoding, parsing and associated data. |
| [`src/header.rs` tests](crates/novavault-core/src/header.rs.tests.md) | Published encoding, version 0 handling and strict rejection of bad headers. |
| [`src/slots.rs`](crates/novavault-core/src/slots.rs.md) | Key slots: the vault key wrapped under the master password and the recovery code. |
| [`src/slots.rs` tests](crates/novavault-core/src/slots.rs.tests.md) | Both slots, generic failures, encoding, tampering and password change flows. |
| [`src/item.rs`](crates/novavault-core/src/item.rs.md) | Item model (passwords, notes, secrets, attachments) and the padded payload codec. |
| [`src/item.rs` tests](crates/novavault-core/src/item.rs.tests.md) | Published payload vectors, round trips, limits, strict decoding and redaction. |
| [`src/format.rs`](crates/novavault-core/src/format.rs.md) | Byte framing of a vault file and its whole-file integrity seal (crate-internal). |
| [`src/format.rs` tests](crates/novavault-core/src/format.rs.tests.md) | Framing round trips, truncation, ordering, size limits and seal sensitivity. |
| [`src/atomic.rs`](crates/novavault-core/src/atomic.rs.md) | Atomic file replacement: temporary file, flush, rename (crate-internal). |
| [`src/atomic.rs` tests](crates/novavault-core/src/atomic.rs.tests.md) | Creation, replacement, failure safety and owner-only permissions. |
| [`src/attachment.rs`](crates/novavault-core/src/attachment.rs.md) | Streaming, chunked encryption and decryption of attachment blobs. |
| [`src/attachment.rs` tests](crates/novavault-core/src/attachment.rs.tests.md) | Blob header vector, chunk boundaries, large files and tamper detection. |
| [`src/vault.rs`](crates/novavault-core/src/vault.rs.md) | `Vault`, the unlocked vault and the crate's main entry point. |
| [`src/vault.rs` tests](crates/novavault-core/src/vault.rs.tests.md) | End-to-end suite: open, edit, save, key slot changes, migration and attachments. |
| [`src/test_util.rs`](crates/novavault-core/src/test_util.rs.md) | Helpers shared by the crate's unit tests (test builds only). |

## Tauri shell

`src-tauri`: the thin Tauri 2 shell that holds the session and exposes the IPC commands.

| File | Description |
| ---- | ----------- |
| [`build.rs`](src-tauri/build.rs.md) | Build script: declares the app's commands for Tauri's permission system. |
| [`src/main.rs`](src-tauri/src/main.rs.md) | App entry point: builds the Tauri app, its state and its background tasks. |
| [`src/main.rs` tests](src-tauri/src/main.rs.tests.md) | The capability grants exactly the commands that `build.rs` declares. |
| [`src/commands.rs`](src-tauri/src/commands.rs.md) | The Tauri command handlers: the whole IPC surface. |
| [`src/dto.rs`](src-tauri/src/dto.rs.md) | Data types exchanged with the UI: command inputs and outputs. |
| [`src/dto.rs` tests](src-tauri/src/dto.rs.tests.md) | IPC wire format: deserialization, rejection of bad input, camelCase output. |
| [`src/error.rs`](src-tauri/src/error.rs.md) | `CommandError`, the only error type that reaches the UI, with stable codes. |
| [`src/error.rs` tests](src-tauri/src/error.rs.tests.md) | Unique codes, constant messages, dropped I/O details and core error mapping. |
| [`src/session.rs`](src-tauri/src/session.rs.md) | `Session`, the one place the shell holds an unlocked vault. |
| [`src/session.rs` tests](src-tauri/src/session.rs.tests.md) | The shell's integration suite: lifecycle, items, settings and attachments. |
| [`src/settings.rs`](src-tauri/src/settings.rs.md) | App settings outside the vault: auto-lock timeout and clipboard clear delay. |
| [`src/settings.rs` tests](src-tauri/src/settings.rs.tests.md) | Bounds, file format, round trip and safe fallback to defaults. |
| [`src/autolock.rs`](src-tauri/src/autolock.rs.md) | Automatic locking on inactivity and on OS suspend and screen lock. |
| [`src/clipboard.rs`](src-tauri/src/clipboard.rs.md) | Copies fields to the clipboard and clears them after a delay, on lock and on exit. |
| [`src/clipboard.rs` tests](src-tauri/src/clipboard.rs.tests.md) | The clipboard worker driven against an in-memory fake clipboard. |
| [`src/os_lock/mod.rs`](src-tauri/src/os_lock/mod.rs.md) | Common interface for OS lock triggers, one implementation per OS. |
| [`src/os_lock/windows.rs`](src-tauri/src/os_lock/windows.rs.md) | Windows lock monitor: suspend and session lock messages. |
| [`src/os_lock/windows.rs` tests](src-tauri/src/os_lock/windows.rs.tests.md) | Mapping of Win32 messages to lock reasons. |
| [`src/os_lock/macos.rs`](src-tauri/src/os_lock/macos.rs.md) | macOS lock monitor. |
| [`src/os_lock/linux.rs`](src-tauri/src/os_lock/linux.rs.md) | Linux lock monitor. |
| [`src/os_lock/unsupported.rs`](src-tauri/src/os_lock/unsupported.rs.md) | Fallback lock monitor for other targets. |

## UI

`ui`: the React + TypeScript front end. It never receives key material.

### Entry points and app frame

| File | Description |
| ---- | ----------- |
| [`index.html`](ui/index.html.md) | The single HTML page and Vite entry point. |
| [`src/main.tsx`](ui/src/main.tsx.md) | Mounts the React app. |
| [`src/styles.css`](ui/src/styles.css.md) | The global stylesheet, with light and dark colour tokens. |
| [`src/App.tsx`](ui/src/App.tsx.md) | Root component: startup status check, routing and manual lock. |
| [`src/App.test.tsx`](ui/src/App.test.tsx.md) | End-to-end tests of `App` against a fake shell. |
| [`src/app/AppShell.tsx`](ui/src/app/AppShell.tsx.md) | Frame around every screen: skip link, header and `main` landmark. |
| [`src/app/router.ts`](ui/src/app/router.ts.md) | Screen routing as a typed state machine. |
| [`src/app/router.test.ts`](ui/src/app/router.test.ts.md) | Unit tests for the route reducer. |

### IPC

| File | Description |
| ---- | ----------- |
| [`src/api/commands.ts`](ui/src/api/commands.ts.md) | Typed wrappers over the Tauri commands. |
| [`src/api/commands.test.ts`](ui/src/api/commands.test.ts.md) | Tests for the IPC wrappers and error normalization. |
| [`src/api/errors.ts`](ui/src/api/errors.ts.md) | The UI side of the shell's error contract. |
| [`src/api/types.ts`](ui/src/api/types.ts.md) | Types of the data exchanged with the shell. |

### Components

| File | Description |
| ---- | ----------- |
| [`src/components/Alert.tsx`](ui/src/components/Alert.tsx.md) | Message box for errors, confirmations, warnings and information. |
| [`src/components/Button.tsx`](ui/src/components/Button.tsx.md) | Standard button with variants and a busy state. |
| [`src/components/CopyButton.tsx`](ui/src/components/CopyButton.tsx.md) | Copies one item field to the clipboard through the shell. |
| [`src/components/PasswordField.tsx`](ui/src/components/PasswordField.tsx.md) | Password input with a Show/Hide toggle. |
| [`src/components/ScreenHeading.tsx`](ui/src/components/ScreenHeading.tsx.md) | Heading of a screen or of a step in a flow. |
| [`src/components/SecretField.tsx`](ui/src/components/SecretField.tsx.md) | One secret item field, hidden until revealed. |
| [`src/components/SecretField.test.tsx`](ui/src/components/SecretField.test.tsx.md) | Component tests for `SecretField`. |
| [`src/components/TextAreaField.tsx`](ui/src/components/TextAreaField.tsx.md) | Labelled multi-line input. |
| [`src/components/TextField.tsx`](ui/src/components/TextField.tsx.md) | Labelled single-line input with help and error text. |

### Formatting

| File | Description |
| ---- | ----------- |
| [`src/format/bytes.ts`](ui/src/format/bytes.ts.md) | File sizes and transfer progress as text. |
| [`src/format/bytes.test.ts`](ui/src/format/bytes.test.ts.md) | Unit tests for size and progress formatting. |
| [`src/format/dateTime.ts`](ui/src/format/dateTime.ts.md) | Timestamps as text. |
| [`src/format/duration.ts`](ui/src/format/duration.ts.md) | Seconds as readable English text. |

### Passwords and recovery codes

| File | Description |
| ---- | ----------- |
| [`src/password/newPassword.ts`](ui/src/password/newPassword.ts.md) | Validation of a new master password. |
| [`src/password/NewPasswordFields.tsx`](ui/src/password/NewPasswordFields.tsx.md) | New password and confirmation inputs with a strength meter. |
| [`src/password/strength.ts`](ui/src/password/strength.ts.md) | Password strength estimate with zxcvbn-ts. |
| [`src/password/StrengthMeter.tsx`](ui/src/password/StrengthMeter.tsx.md) | Strength meter, label, warning and suggestions. |
| [`src/password/usePasswordStrength.ts`](ui/src/password/usePasswordStrength.ts.md) | Hook that re-estimates strength as the user types. |
| [`src/recovery/recoveryCode.ts`](ui/src/recovery/recoveryCode.ts.md) | Helpers for showing and retyping a recovery code. |
| [`src/recovery/recoveryCode.test.ts`](ui/src/recovery/recoveryCode.test.ts.md) | Unit tests for the recovery code helpers. |
| [`src/recovery/RecoveryCodeFlow.tsx`](ui/src/recovery/RecoveryCodeFlow.tsx.md) | Shows a new recovery code once and confirms the user saved it. |

### Screens

| File | Description |
| ---- | ----------- |
| [`src/screens/StatusScreens.tsx`](ui/src/screens/StatusScreens.tsx.md) | Loading and startup error screens. |
| [`src/screens/create/CreateVaultScreen.tsx`](ui/src/screens/create/CreateVaultScreen.tsx.md) | First-run flow: master password, then recovery code. |
| [`src/screens/create/CreateVaultScreen.test.tsx`](ui/src/screens/create/CreateVaultScreen.test.tsx.md) | Component tests for vault creation. |
| [`src/screens/unlock/UnlockScreen.tsx`](ui/src/screens/unlock/UnlockScreen.tsx.md) | Unlock with the master password or the recovery code. |
| [`src/screens/unlock/UnlockScreen.test.tsx`](ui/src/screens/unlock/UnlockScreen.test.tsx.md) | Component tests for unlocking. |
| [`src/screens/items/ItemsScreen.tsx`](ui/src/screens/items/ItemsScreen.tsx.md) | Main screen of an unlocked vault. |
| [`src/screens/items/ItemsScreen.test.tsx`](ui/src/screens/items/ItemsScreen.test.tsx.md) | Tests for the item list: loading, sorting, search and filters. |
| [`src/screens/items/ItemsScreen.crud.test.tsx`](ui/src/screens/items/ItemsScreen.crud.test.tsx.md) | Tests for viewing, copying, creating, editing and deleting items. |
| [`src/screens/items/ItemList.tsx`](ui/src/screens/items/ItemList.tsx.md) | Item list with title search and type filter. |
| [`src/screens/items/filterItems.ts`](ui/src/screens/items/filterItems.ts.md) | Search, filter and sort of the item list. |
| [`src/screens/items/filterItems.test.ts`](ui/src/screens/items/filterItems.test.ts.md) | Unit tests for `filterItems`. |
| [`src/screens/items/ItemDetails.tsx`](ui/src/screens/items/ItemDetails.tsx.md) | One item: plain fields, hidden secret fields and attachments. |
| [`src/screens/items/ItemForm.tsx`](ui/src/screens/items/ItemForm.tsx.md) | Create or edit an item of a given kind. |
| [`src/screens/items/ConfirmDelete.tsx`](ui/src/screens/items/ConfirmDelete.tsx.md) | Inline delete confirmation for items and attachments. |
| [`src/screens/items/AttachmentList.tsx`](ui/src/screens/items/AttachmentList.tsx.md) | An item's files: attach, export and remove. |
| [`src/screens/items/AttachmentList.test.tsx`](ui/src/screens/items/AttachmentList.test.tsx.md) | Component tests for `AttachmentList`. |
| [`src/screens/items/kinds.ts`](ui/src/screens/items/kinds.ts.md) | Display names of the three item kinds. |
| [`src/screens/settings/SettingsScreen.tsx`](ui/src/screens/settings/SettingsScreen.tsx.md) | Settings screen of an unlocked vault. |
| [`src/screens/settings/SettingsScreen.test.tsx`](ui/src/screens/settings/SettingsScreen.test.tsx.md) | Component tests for the settings screen and its sections. |
| [`src/screens/settings/ChangePasswordSection.tsx`](ui/src/screens/settings/ChangePasswordSection.tsx.md) | Change the master password. |
| [`src/screens/settings/RecoveryCodeSection.tsx`](ui/src/screens/settings/RecoveryCodeSection.tsx.md) | Regenerate the recovery code. |
| [`src/screens/settings/CurrentCredentialFields.tsx`](ui/src/screens/settings/CurrentCredentialFields.tsx.md) | Re-authentication fields before a key slot change. |
| [`src/screens/settings/useCredential.ts`](ui/src/screens/settings/useCredential.ts.md) | Hook holding the re-authentication credential. |
| [`src/screens/settings/AutoLockSection.tsx`](ui/src/screens/settings/AutoLockSection.tsx.md) | Inactivity auto-lock timeout. |
| [`src/screens/settings/ClipboardSection.tsx`](ui/src/screens/settings/ClipboardSection.tsx.md) | Clipboard clear delay. |
| [`src/screens/settings/AppearanceSection.tsx`](ui/src/screens/settings/AppearanceSection.tsx.md) | Colour theme choice. |

### Theme

| File | Description |
| ---- | ----------- |
| [`src/theme/theme.ts`](ui/src/theme/theme.ts.md) | The theme preference: system, light or dark. |
| [`src/theme/ThemeContext.ts`](ui/src/theme/ThemeContext.ts.md) | Theme context and the `useTheme` hook. |
| [`src/theme/ThemeProvider.tsx`](ui/src/theme/ThemeProvider.tsx.md) | Loads, applies and shares the theme preference. |
| [`src/theme/ThemeProvider.test.tsx`](ui/src/theme/ThemeProvider.test.tsx.md) | Component tests for `ThemeProvider`. |

### Test helpers

| File | Description |
| ---- | ----------- |
| [`src/test/ipc.ts`](ui/src/test/ipc.ts.md) | Fakes the Tauri shell for tests. |
| [`src/test/setup.ts`](ui/src/test/setup.ts.md) | Vitest setup run before every test file. |

## Configuration

### Repository

| File | Description |
| ---- | ----------- |
| [`Cargo.toml`](Cargo.toml.md) | Cargo workspace: members, shared package fields and workspace lints. |
| [`clippy.toml`](clippy.toml.md) | Allows `unwrap()` and `expect()` in tests only. |
| [`package.json`](package.json.md) | Root npm package: the Tauri CLI and the `check`, `dev` and `build` scripts. |
| [`scripts/check.mjs`](scripts/check.mjs.md) | The full local check: format, lint, typecheck, tests and builds. |
| [`.editorconfig`](.editorconfig.md) | Shared editor settings: UTF-8, LF and indentation. |
| [`.gitattributes`](.gitattributes.md) | Line endings, binary files and generated lockfiles. |
| [`.gitignore`](.gitignore.md) | Build output, dependencies, secrets and vault files Git must never track. |

### CI

| File | Description |
| ---- | ----------- |
| [`.github/workflows/ci.yml`](.github/workflows/ci.yml.md) | CI workflow: the full check on Windows, macOS and Linux. |
| [`.github/dependabot.yml`](.github/dependabot.yml.md) | Dependabot version updates for Cargo, npm and GitHub Actions. |

### Core

| File | Description |
| ---- | ----------- |
| [`crates/novavault-core/Cargo.toml`](crates/novavault-core/Cargo.toml.md) | Core crate manifest and its cryptography dependencies. |

### Tauri shell

| File | Description |
| ---- | ----------- |
| [`src-tauri/Cargo.toml`](src-tauri/Cargo.toml.md) | Shell crate manifest and its dependencies. |
| [`src-tauri/tauri.conf.json`](src-tauri/tauri.conf.json.md) | Tauri app configuration, including the CSP. |
| [`src-tauri/capabilities/default.json`](src-tauri/capabilities/default.json.md) | The only capability: the allow-list of what the webview may call. |

### UI

| File | Description |
| ---- | ----------- |
| [`ui/package.json`](ui/package.json.md) | UI npm manifest: scripts and dependencies. |
| [`ui/vite.config.ts`](ui/vite.config.ts.md) | Vite and Vitest configuration. |
| [`ui/tsconfig.json`](ui/tsconfig.json.md) | Root TypeScript configuration with project references. |
| [`ui/tsconfig.app.json`](ui/tsconfig.app.json.md) | TypeScript settings for the browser code under `ui/src/`. |
| [`ui/tsconfig.node.json`](ui/tsconfig.node.json.md) | TypeScript settings for the Node-side tool configs. |
| [`ui/eslint.config.js`](ui/eslint.config.js.md) | ESLint flat configuration. |
| [`ui/.prettierrc.json`](ui/.prettierrc.json.md) | Prettier configuration. |
| [`ui/.prettierignore`](ui/.prettierignore.md) | Paths Prettier skips. |
