# Architecture

This document describes how NovaVault is put together: its components, the layers they form, how data moves between them, and why every cryptographic operation lives in Rust.

> [!NOTE]
> NovaVault is pre-alpha. The project scaffold and the core's cryptographic primitives (see [Cryptography and dependencies](#cryptography-and-dependencies)) are in place; key slots, the vault format and the app flows are not implemented yet. This document describes the intended design. Details will be refined as the core implementation is written, and this page will be kept in sync with the code.

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

## Cryptography and dependencies

The primitives live in `crates/novavault-core` (`kdf`, `aead`, `keys` and `error` modules). Everything else in the core is built on these few functions: the `recovery` module (recovery codes) and the `slots` module (key slots and their operations) use only `derive_key`, `encrypt` and `decrypt`, and add no dependencies.

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

### Test vectors

- Argon2id: RFC 9106 §5.3 (with secret and associated data) and the reference implementation's `password`/`somesalt` vector, both run through the same parameter wiring as production derivation.
- XChaCha20-Poly1305: draft-irtf-cfrg-xchacha-03 Appendix A.3.1.
- Tamper detection: every single-bit flip of a sealed message, truncation, appended bytes, a wrong key and wrong associated data are all rejected.

## Related documents

- [Threat model](threat-model.md)
- [Vault format specification](vault-format.md)
- [Development guide](development.md)
