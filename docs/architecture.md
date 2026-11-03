# Architecture

This document describes how NovaVault is put together: its components, the layers they form, how data moves between them, and why every cryptographic operation lives in Rust.

> [!NOTE]
> NovaVault is pre-alpha and the application code has not landed yet. This document describes the intended design. Details will be refined as the core implementation is written, and this page will be kept in sync with the code.

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

## Session state

- While a vault is unlocked, the core holds the 256-bit vault key in a `secrecy` wrapper inside the Rust process. The key is zeroized when the vault is locked, when the session is dropped and when the app exits.
- Decrypted records are produced on demand for a single command and dropped (zeroized where they contain secrets) once the response has been built. The core does not keep a long-lived plaintext cache of the vault.
- Locking discards the session. Any subsequent command that needs vault data fails with a "locked" error until the user unlocks again.

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

- Commands are coarse-grained and intent-based (`unlock`, `lock`, `list_records`, `get_record`, `copy_secret`, …), never generic ("read bytes", "decrypt blob").
- No command ever returns the vault key, wrapping keys, KDF output, or raw ciphertext.
- Inputs are validated in Rust; errors returned to the UI are typed and contain no secret material.
- The exact command list will be documented here once implemented.

## Shell hardening

- **Content Security Policy:** strict, with no remote origins, no inline scripts and no `eval`.
- **Capabilities:** the minimal Tauri capability set the UI needs. No shell, no arbitrary filesystem access and no HTTP from the webview.
- **No network:** core features do not open network connections.

## Why all cryptography lives in Rust

- **The webview is the largest attack surface.** It parses and renders content and runs a large JavaScript engine. Keeping keys out of it means a script-injection bug in the UI cannot directly exfiltrate the vault key.
- **Memory control.** JavaScript strings are immutable and garbage-collected, so secrets cannot be reliably wiped. Rust with `zeroize` and `secrecy` gives deterministic clearing and makes accidental copies or logging harder.
- **Audited primitives.** The RustCrypto crates (or `age`) provide reviewed implementations of Argon2id and AEAD ciphers. Using them from one place avoids mixing implementations across languages.
- **One testable core.** Crypto, format and key-slot logic can be unit-tested with known vectors without a UI or a running app, and reused by future tools (for example a format validator).
- **Smaller review scope.** Auditors and contributors can focus on one crate to verify the security properties of the whole application.

## Related documents

- [Threat model](threat-model.md)
- [Vault format specification](vault-format.md)
- [Development guide](development.md)
