# Architecture

This document describes how NovaVault is put together: its components, the layers they form, how data moves between them, and why every cryptographic operation lives in Rust.

> [!NOTE]
> NovaVault is pre-alpha. The core library (cryptographic primitives, key slots, recovery codes and the vault file format with passwords, notes and secrets) and the Tauri command layer with auto-lock are in place (see [Core library](#core-library) and [Tauri command surface](#tauri-command-surface)). The UI flows, revealing and copying secrets, and attachments are not implemented yet; those parts of this document describe the intended design and will be kept in sync with the code as they land.

## Goals

- **Local only.** The vault is a single file on the user's disk. Core features never use the network.
- **Small trusted core.** Everything that touches keys or plaintext secrets is concentrated in one Rust library that can be reviewed and tested on its own.
- **Untrusted presentation layer.** The UI is treated as the most exposed part of the app and is given as little power as possible.
- **Simplicity.** A solo maintainer must be able to understand and audit the whole system.

## Components

| Component | Path | Language | Responsibility |
| --------- | ---- | -------- | -------------- |
| Core | `crates/novavault-core/` | Rust | Cryptography, key derivation, key slots, vault format (read/write/validate), record and attachment storage, session state. No dependency on Tauri or any UI. |
| Shell | `src-tauri/` | Rust | Thin Tauri 2 application: window lifecycle, the Tauri command handlers, OS integration (auto-lock triggers on suspend and screen lock, clipboard, file dialogs), Content Security Policy and capability configuration. Delegates all vault logic to the core. |
| UI | `ui/` | React + TypeScript | Presentation and user input only. Renders forms and lists, sends user intents to the shell through Tauri commands and displays the results. Holds no keys. |

## Layers and trust boundary

```text
┌──────────────────────────────────────────────────────────┐
│ UI (webview)  React + TypeScript                         │  untrusted
│   forms, lists, navigation, inactivity timer events      │
└───────────────────────────┬──────────────────────────────┘
                            │  Tauri IPC: minimal commands,
                            │  validated inputs, minimal outputs
════════════════════════════╪═══════════════ trust boundary ═
┌───────────────────────────┴──────────────────────────────┐
│ Shell (src-tauri)  Rust                                  │  trusted
│   command handlers, OS hooks, clipboard, CSP/capabilities │
├──────────────────────────────────────────────────────────┤
│ Core (novavault-core)  Rust                              │  trusted
│   session: vault key (secrecy + zeroize)                 │
│   key slots · Argon2id · AEAD · vault format · records   │
└───────────────────────────┬──────────────────────────────┘
                            │  file I/O
                   ┌────────┴────────┐
                   │  vault file     │  encrypted at rest
                   └─────────────────┘
```

The trust boundary is the Tauri IPC bridge. Everything above it is assumed to be potentially hostile (for example, if malicious content ever managed to execute script in the webview). Everything below it is the trusted computing base and is kept as small as possible.

## Core library

`crates/novavault-core` is organized in layers, each using only the ones below it:

| Module | Responsibility |
| ------ | -------------- |
| `vault` | `Vault`, the unlocked vault: `create`, `open_with_password`, `open_with_recovery_code`, `list`, `get`, `insert`, `update`, `remove`, `save`, `change_password`, `regenerate_recovery_code` (each rewrites one key slot and saves, restoring the previous slot if the save fails). |
| `item` | The item model (`PasswordItem`, `NoteItem`, `SecretItem`, `ItemId`) and the padded payload encoding. |
| `format` (private) | Byte framing of the file, record associated data and the whole-file integrity seal. Pure functions, no I/O. |
| `atomic` (private) | Atomic file replacement (temporary file, flush, rename). |
| `header` | The plaintext versioned header. |
| `slots`, `recovery` | Key slots and recovery codes. |
| `kdf`, `aead`, `keys`, `error` | Primitives, key types and typed errors (`CryptoError`, `VaultError`). |

### Vault file

The byte-level format is specified in [vault-format.md](vault-format.md). In short:

```text
header (32) ‖ password slot (101) ‖ recovery slot (101) ‖ record_count ‖ records… ‖ seal (40)
```

- **Records** hold one item each. Only the random record identifier and the item type are in plaintext; the title, timestamps and fields are encrypted together under the vault key, padded to 64-byte buckets. Each record's associated data binds it to the vault header, its identifier and its type.
- **Integrity seal.** The file ends with an AEAD tag under the vault key over everything before it, so removing, duplicating, reordering or replaying an individual record is detected, not only modifying one.
- **Opening** checks the whole structure (magic, version, header fields, slots, record framing, sizes) before running Argon2id, so a foreign, newer or truncated file is rejected quickly with a precise `VaultError`. A wrong secret or damaged slot is reported as the single `InvalidCredentials` error. The seal is verified before any record can be read.
- **Saving** encodes the file in memory and replaces the old one atomically: a temporary file in the same directory is written, flushed and renamed over the vault, so a crash leaves either the old or the new file, never a mix. On Unix the file is created owner-only.
- **No new dependencies** were needed: the encoding is a small hand-written binary format and atomic writes use the standard library.

## Session state

- While a vault is unlocked, the core holds the 256-bit vault key in a `secrecy` wrapper inside the Rust process, owned by a `Vault` value. The key is zeroized when the `Vault` is dropped, which is how a vault is locked, and when the app exits.
- Records stay encrypted in memory. A record is decrypted on demand for a single request and dropped (zeroized) once the response has been built; secret fields are `SecretString`s and other text fields are wiped on drop. The core does not keep a long-lived plaintext cache of the vault.
- Locking discards the session. Any subsequent command that needs vault data fails with a `locked` error until the user unlocks again.
- In the shell, `Session` (`src-tauri/src/session.rs`) is the single owner of the unlocked `Vault`, behind a mutex in Tauri's managed state. It holds the vault path, the Argon2id parameters for new key slots and the time of the last command. It has no Tauri types, so the logic behind every command is unit-tested without a running app.
- There is one vault per installation, at `vault.nvault` in the app data directory (for example `%APPDATA%\io.github.acevflow.novavault` on Windows, `~/Library/Application Support/io.github.acevflow.novavault` on macOS and `~/.local/share/io.github.acevflow.novavault` on Linux). The path is chosen by the shell and never sent by the UI.

## Auto-lock

Three triggers lock the vault automatically. All of them wipe the vault key (by dropping the `Vault`) and then reload the main webview. Reloading throws away everything the UI holds in memory and returns it to its start screen without the UI having to subscribe to events, so the webview needs no event permission.

| Trigger | Implementation |
| ------- | -------------- |
| Inactivity | A background thread checks every 5 seconds and locks once 5 minutes have passed since the last command. Every command counts as activity. The timeout is fixed until settings exist. |
| System suspend | Windows: `WM_POWERBROADCAST` / `PBT_APMSUSPEND`. macOS: `NSWorkspaceWillSleepNotification`. Linux: logind `PrepareForSleep(true)`. |
| Screen lock | Windows: `WM_WTSSESSION_CHANGE` / `WTS_SESSION_LOCK`. macOS: `com.apple.screenIsLocked` and `NSWorkspaceSessionDidResignActiveNotification` (fast user switching). Linux: logind `Session.Lock`, and `ActiveChanged(true)` from `org.freedesktop.ScreenSaver` or `org.gnome.ScreenSaver`. |

- The OS triggers sit behind one `LockMonitor` trait (`src-tauri/src/os_lock/`) with one implementation per OS, selected at compile time. Windows uses a dedicated thread with a hidden, never-shown top-level window: power broadcasts reach only top-level windows. macOS registers notification observers on the main thread. Linux listens to D-Bus signals on Tauri's async runtime.
- Locking on suspend runs synchronously inside the notification handler on Windows and macOS, so the key is wiped before the system sleeps.
- If a monitor cannot start (for example, no logind on Linux), the app prints a fixed message and keeps running with the inactivity timer and manual locking.
- **Scoped `unsafe`.** Windows and macOS offer these notifications only through FFI. The workspace denies `unsafe_code`, and only `os_lock/windows.rs` and `os_lock/macos.rs` opt out. Each `unsafe` block is a single FFI call with a `SAFETY` comment. The core crate keeps `#![forbid(unsafe_code)]`.

## Data flows

### Create a vault

1. The UI collects the master password and sends it with a `create_vault` command.
2. The core generates a random 256-bit vault key and a recovery code from the OS CSPRNG.
3. It derives a wrapping key from the master password and another from the recovery code with Argon2id, and wraps the vault key under each one, producing two key slots.
4. It writes the header, key slots and an empty record set to disk (see [vault-format.md](vault-format.md)).
5. The recovery code is returned to the UI exactly once for display. The user must confirm they have saved it before the flow completes. The core does not store the code itself.

### Unlock

1. The UI sends the master password (or the recovery code) with an `unlock` command.
2. The core reads the header and the matching key slot, derives the wrapping key with the stored Argon2id parameters and attempts to unwrap the vault key.
3. Authentication failure yields a generic "wrong password or corrupted vault" error. On success the vault key is stored in the session.
4. The UI receives only a success indicator and non-secret metadata (for example the list of record titles needed for navigation).

### Read and edit a record

1. The UI requests a record by identifier.
2. The core decrypts it with the vault key, verifying its authentication tag, and returns only the fields the screen needs.
3. Secret field values (such as a password) are returned only when the user explicitly reveals them. Copying a secret is done by a dedicated command that places it on the clipboard from Rust and schedules auto-clear, so the value does not need to pass through the UI at all.
4. Edits are sent as field values; the core re-encrypts the record with a fresh nonce and writes it atomically.

### Lock

Triggered by the user, by the inactivity timer, or by OS events (suspend, screen lock) observed in the shell. The core zeroizes the vault key and drops the session; the UI clears its in-memory view state and returns to the unlock screen.

### Change master password or regenerate the recovery code

With the vault unlocked, the core derives a new wrapping key from the new secret and re-wraps the existing vault key, replacing only the corresponding key slot. Records and attachments are not re-encrypted. Regenerating the recovery code replaces the recovery slot, which invalidates the previous code.

## Tauri command surface

- Commands are coarse-grained and intent-based, never generic ("read bytes", "decrypt blob").
- No command ever returns the vault key, wrapping keys, KDF output, or raw ciphertext.
- Inputs are validated in Rust; errors returned to the UI are typed and contain no secret material.
- Every handler (`src-tauri/src/commands.rs`) moves its arguments to a blocking worker thread, because Argon2id takes a noticeable time, then locks the session and delegates to it.
- **Allow-list.** `src-tauri/build.rs` declares the commands, so Tauri generates one `allow-<command>` permission per command and rejects any call the window's capability does not grant. `capabilities/default.json` grants exactly these nine and no core plugin permission. A test checks that the two lists match.

### Commands

Arguments are camelCase in JavaScript (`invoke('change_master_password', { newPassword })`).

| Command | Arguments | Returns | Notes |
| ------- | --------- | ------- | ----- |
| `create_vault` | `password` | `{ recoveryCode }` | Creates the vault and leaves it unlocked. The code (`XXXX-XXXX-…`) is returned only here; the UI must show it once and have the user confirm it was saved. |
| `unlock` | `credential`: `{ method: "password" \| "recoveryCode", secret }` | nothing | Locks any open vault first, so a failed attempt leaves the app locked. Recovery codes are accepted in any case, with or without dashes. |
| `lock` | none | nothing | Wipes the vault key. |
| `change_master_password` | `newPassword` | nothing | Re-wraps the vault key in the password slot. Works after unlocking with the recovery code (forgotten-password flow). |
| `regenerate_recovery_code` | none | `{ recoveryCode }` | Replaces the recovery slot; the previous code stops working. |
| `list_items` | none | `[{ id, kind, title, createdAtMs, updatedAtMs }]` | No secret fields. `kind` is `"password"`, `"note"` or `"secret"`; `id` is 32 hex digits. |
| `create_item` | `item` | `id` | |
| `update_item` | `id`, `item` | nothing | The new content may have a different kind. |
| `delete_item` | `id` | nothing | |

`item` is tagged by `kind`: `{ kind: "password", title, username, password, url, notes }`, `{ kind: "note", title, body }` or `{ kind: "secret", title, value, notes }`. Every field is required (use empty strings). Secret fields are deserialized straight into `SecretString`.

Every edit is saved to disk at once (atomically). If a save fails, the command returns `storage`; the edit stays in memory and is written by the next successful save.

### Errors

A failed command rejects with `{ code, message }`. The message is a fixed string. Details from the core (such as I/O error text, which can contain paths) are dropped, and no error ever contains a command argument.

| Code | Meaning |
| ---- | ------- |
| `locked` | The vault is locked. |
| `vault_not_found` | No vault exists yet (offer to create one). |
| `vault_exists` | `create_vault` was called but a vault already exists. |
| `invalid_credentials` | Wrong password or recovery code, or damaged key slot (intentionally indistinguishable). |
| `invalid_recovery_code` | The recovery code is malformed (length, character or checksum): a typo, caught before key derivation. |
| `invalid_input` | Malformed argument: an empty password or a bad item id. |
| `item_not_found` | No item has that id. |
| `invalid_item` | The item is too large or the vault is full. |
| `unsupported_vault` | Not a vault file, or a newer format version or unknown algorithm. |
| `corrupted` | The file failed a structural or integrity check. |
| `storage` | The vault file could not be read or written. |
| `internal` | Unexpected failure, for example of the system random number generator. |

## Shell hardening

- **Content Security Policy:** strict, with no remote origins, no inline scripts and no `eval`.
- **Capabilities:** only the nine vault commands are granted (see [Tauri command surface](#tauri-command-surface)). No core plugin permissions: no events, shell, filesystem or HTTP from the webview.
- **No network:** core features do not open network connections.

## Why all cryptography lives in Rust

- **The webview is the largest attack surface.** It parses and renders content and runs a large JavaScript engine. Keeping keys out of it means a script-injection bug in the UI cannot directly exfiltrate the vault key.
- **Memory control.** JavaScript strings are immutable and garbage-collected, so secrets cannot be reliably wiped. Rust with `zeroize` and `secrecy` gives deterministic clearing and makes accidental copies or logging harder.
- **Audited primitives.** The RustCrypto crates (or `age`) provide reviewed implementations of Argon2id and AEAD ciphers. Using them from one place avoids mixing implementations across languages.
- **One testable core.** Crypto, format and key-slot logic can be unit-tested with known vectors without a UI or a running app, and reused by future tools (for example a format validator).
- **Smaller review scope.** Auditors and contributors can focus on one crate to verify the security properties of the whole application.

## Cryptography and dependencies

The primitives live in `crates/novavault-core` (`kdf`, `aead`, `keys` and `error` modules). Everything else in the core is built on these few functions: recovery codes, key slots, records and the integrity seal use only `derive_key`, `encrypt` and `decrypt`, and add no dependencies.

### Algorithms and parameters

| Purpose | Choice | Parameters |
| ------- | ------ | ---------- |
| Key derivation | Argon2id, version 0x13 (RFC 9106) | Defaults: 64 MiB memory, 3 iterations, 4 lanes, 16-byte random salt, 32-byte output (RFC 9106 second recommended option). Accepted range: memory 19 MiB to 4 GiB, iterations 2 to 64, lanes 1 to 16. |
| Authenticated encryption | XChaCha20-Poly1305 | 256-bit key, 192-bit random nonce per message, 128-bit tag. Sealed message layout: `nonce (24) ‖ ciphertext ‖ tag (16)`. |
| Randomness | OS CSPRNG via `getrandom` | Keys, salts and nonces. |

- **KDF bounds.** Argon2id parameters are stored in each key slot, so on unlock they are untrusted input. The lower bound (the OWASP minimum of 19 MiB and 2 iterations) stops a tampered file from silently downgrading protection; the upper bound stops a crafted file from forcing a huge allocation or an extremely long unlock. Bounds are checked again inside `derive_key`, so unchecked values cannot reach Argon2.
- **Nonces.** Every encryption draws a fresh random nonce. Callers cannot supply one; the internal function that accepts a fixed nonce exists only for known-answer tests.
- **Errors.** Every decryption failure (wrong key, modified nonce, ciphertext or tag, different associated data) returns the same `CryptoError::Decryption`, so the error reveals nothing about the cause. Error variants never contain secret data.
- **Key material in memory.** `SecretKey` holds its 32 bytes in a `secrecy::SecretBox`, which zeroizes them on drop and redacts them in `Debug`. It does not implement `Clone`, and its bytes are only readable inside the crate. Derived keys and decrypted plaintext are written straight into zeroizing storage, and the cipher's internal key copy is wiped too (the `zeroize` feature of `chacha20poly1305`).

### Why XChaCha20-Poly1305 rather than AES-GCM

- **Random nonces are safe.** With a 192-bit nonce the chance of a repeat is negligible even after far more messages than a vault will ever encrypt under one key. AES-GCM has a 96-bit nonce, for which NIST SP 800-38D caps random nonces at 2^32 messages per key. A nonce repeat in either cipher is catastrophic, so the extended nonce removes a whole class of bugs (counters, persisted nonce state) from the design.
- **Constant time everywhere.** ChaCha20 and Poly1305 are fast and constant-time in plain software. AES-GCM is only fast and side-channel resistant with hardware AES and carry-less multiply instructions, which would be an assumption about every user's machine on Windows, macOS and Linux.
- **Simplicity.** No nonce-misuse workaround or per-message key derivation layer is needed, and the same construction is used by other well-reviewed tools (for example libsodium's `crypto_aead_xchacha20poly1305_ietf`).
- **Trade-off accepted.** XChaCha20-Poly1305 is specified by an IETF draft (draft-irtf-cfrg-xchacha) rather than a finished RFC. It is a straightforward composition of RFC 8439 ChaCha20-Poly1305 with HChaCha20, and the implementation is tested against the draft's vector.

### Dependencies

Every cryptography-related crate and why it is used:

| Crate | Purpose | Justification |
| ----- | ------- | ------------- |
| [`argon2`](https://crates.io/crates/argon2) | Argon2id | RustCrypto, pure Rust, widely used. Default features are disabled (no `password-hash` string format, which the vault does not need); `zeroize` is enabled to wipe its internal memory. |
| [`chacha20poly1305`](https://crates.io/crates/chacha20poly1305) | XChaCha20-Poly1305 | RustCrypto; its ChaCha20Poly1305 implementation has had an independent security audit (NCC Group). Default features are disabled; only `zeroize` is enabled. |
| [`getrandom`](https://crates.io/crates/getrandom) | OS CSPRNG | The standard minimal interface to the operating system's random number generator, already a dependency of the RustCrypto crates. Used directly so random bytes can be written straight into zeroizing buffers. |
| [`secrecy`](https://crates.io/crates/secrecy) | Secret wrappers | Prevents accidental exposure (no `Debug` leak, explicit `expose_secret`) and zeroizes on drop. |
| [`zeroize`](https://crates.io/crates/zeroize) | Memory wiping | Clears buffers in a way the compiler cannot optimize away. Also used by `secrecy` and the RustCrypto crates. |
| [`thiserror`](https://crates.io/crates/thiserror) | Error types | Derive macro only, no runtime code. Not cryptographic; listed for completeness. |

Shell (`src-tauri`) dependencies that handle secrets or OS security events:

| Crate | Purpose | Justification |
| ----- | ------- | ------------- |
| [`secrecy`](https://crates.io/crates/secrecy) (`serde` feature) | Secret command arguments | Deserializes passwords, recovery codes and secret fields straight into `SecretString`, so Rust never holds them in a plain `String`. Serialization of secrets stays disabled. |
| [`zeroize`](https://crates.io/crates/zeroize) (`derive`) | Wiping responses | Item titles in list responses are wiped once serialized. |
| [`serde`](https://crates.io/crates/serde) | IPC (de)serialization | Already used by Tauri for every command; no new crate. |
| [`windows`](https://crates.io/crates/windows) (Windows only) | Suspend and screen-lock events | Microsoft's official Win32 bindings, same version Tauri already uses. Only the window, library loader and remote desktop session APIs are enabled. |
| [`objc2-app-kit`](https://crates.io/crates/objc2-app-kit), [`objc2-foundation`](https://crates.io/crates/objc2-foundation), [`block2`](https://crates.io/crates/block2) (macOS only) | Sleep and screen-lock notifications | The Objective-C bindings Tauri itself uses, same versions; only `NSWorkspace` and the notification center APIs are enabled. |
| [`zbus`](https://crates.io/crates/zbus), [`futures-util`](https://crates.io/crates/futures-util) (Linux only) | logind and screen saver D-Bus signals | Pure-Rust D-Bus with no C library, run on Tauri's Tokio runtime (no extra executor, default features off). `futures-util` is already a Tauri dependency. Talks only to local system and session buses, never the network. |

### Test vectors

- Argon2id: RFC 9106 §5.3 (with secret and associated data) and the reference implementation's `password`/`somesalt` vector, both run through the same parameter wiring as production derivation.
- XChaCha20-Poly1305: draft-irtf-cfrg-xchacha-03 Appendix A.3.1.
- Tamper detection: every single-bit flip of a sealed message, truncation, appended bytes, a wrong key and wrong associated data are all rejected.
- Vault format: known encodings of the header and of an item payload (published in [vault-format.md](vault-format.md#13-test-vectors)); every truncation of a vault file; a flipped byte in every field of every section; a removed or replayed record; unsupported versions, algorithms and foreign files.

## Related documents

- [Threat model](threat-model.md)
- [Vault format specification](vault-format.md)
- [Development guide](development.md)
