# Cryptography-related dependencies

This document lists every dependency that implements cryptography, handles secrets or reacts to OS security events, with the exact version resolved in the lockfiles and the reason it was chosen. CLAUDE.md requires every cryptography-related dependency to be justified here.

Versions are taken from `Cargo.lock` (Rust workspace) and `ui/package-lock.json` (UI) at the time of writing. Dependabot proposes updates weekly; when a version changes, update this table in the same pull request.

## Algorithm choices

NovaVault uses exactly one key derivation function and one authenticated cipher. Both are fixed by the vault format (`aead_id` `0x01`); changing either would need a new format version.

| Purpose | Choice | Why |
| ------- | ------ | --- |
| Key derivation | Argon2id, version 0x13 (RFC 9106) | Memory-hard, the RFC 9106 and OWASP recommendation for password hashing; resists GPU and ASIC guessing better than PBKDF2, bcrypt or scrypt. |
| Authenticated encryption | XChaCha20-Poly1305 | The 192-bit nonce makes random nonces safe, so no counters or persisted nonce state are needed. Constant-time in plain software on every target, with no dependence on AES hardware instructions. |
| Randomness | OS CSPRNG through `getrandom` | The only source of keys, salts, nonces, recovery codes and identifiers. |

**Rejected alternatives.**

- **AES-GCM.** Its 96-bit nonce limits random nonces to 2^32 messages per key (NIST SP 800-38D), and it is fast and side-channel resistant only with hardware AES and carry-less multiply, which cannot be assumed on every user's machine. See [Architecture](architecture.md#why-xchacha20-poly1305-rather-than-aes-gcm).
- **`age`.** A file encryption tool and format with its own key and recipient model. NovaVault needs key slots, per-record associated data and a whole-file seal, which would have to be built around `age` anyway; using the primitives directly keeps one format and fewer dependencies.
- **`aead-stream`.** Implements the STREAM construction used for attachments, but with counter-derived nonces. Building it from the existing `aead::encrypt`/`decrypt` with random nonces adds about a hundred lines, no dependency and no nonce bookkeeping. See [vault-format.md](vault-format.md#9-attachments).
- **`rand`.** Not needed: `getrandom` writes random bytes straight into zeroizing buffers, and with default features off neither `argon2` nor `chacha20poly1305` pulls in a random number generator.

## Core (`crates/novavault-core`)

Direct dependencies. The core has no other dependencies.

| Crate | Version | Features | Purpose and justification |
| ----- | ------- | -------- | ------------------------- |
| [`argon2`](https://crates.io/crates/argon2) | 0.6.0 | defaults off; `alloc`, `zeroize` | Argon2id. RustCrypto, pure Rust, widely used. `zeroize` wipes its working memory. The core calls `hash_password_into` for raw 32-byte output and does not use the PHC password-hash string format. Tested against the RFC 9106 §5.3 vector and the reference implementation's `password`/`somesalt` vector. The crate documents no independent audit. |
| [`chacha20poly1305`](https://crates.io/crates/chacha20poly1305) | 0.11.0 | defaults off; `zeroize` | XChaCha20-Poly1305. RustCrypto; the crate, `chacha20` and `poly1305` received a security audit by NCC Group in 2020 with no significant findings (it predates the current versions and the Poly1305 AVX2 backend). `zeroize` wipes the cipher's internal key copy. Tested against draft-irtf-cfrg-xchacha-03 A.3.1. |
| [`getrandom`](https://crates.io/crates/getrandom) | 0.4.3 | defaults | The standard minimal interface to the OS random number generator, and the core's only randomness source. |
| [`secrecy`](https://crates.io/crates/secrecy) | 0.10.3 | defaults | `SecretBox`, `SecretSlice`, `SecretString`: explicit `expose_secret`, redacted `Debug`, zeroized on drop. |
| [`zeroize`](https://crates.io/crates/zeroize) | 1.9.1 | `derive` | Wipes buffers in a way the compiler cannot optimize away; `Zeroizing` and `ZeroizeOnDrop` for item types and working buffers. Also used by `secrecy` and the RustCrypto crates. |
| [`zeroize_derive`](https://crates.io/crates/zeroize_derive) | 1.5.0 | | Proc macro behind the `derive` feature of `zeroize`. |
| [`thiserror`](https://crates.io/crates/thiserror) | 2.0.21 | | Derive macro for the typed errors (`CryptoError`, `VaultError`); no runtime code. Not cryptographic, listed because error types must never carry secrets. |

Transitive cryptographic crates pulled in by `argon2` and `chacha20poly1305`. They are all RustCrypto crates and are not used directly:

| Crate | Version | Pulled in by | Role |
| ----- | ------- | ------------ | ---- |
| `chacha20` | 0.10.2 | `chacha20poly1305` | ChaCha20 and HChaCha20 (covered by the NCC Group audit). |
| `poly1305` | 0.9.1 | `chacha20poly1305` | Poly1305 authenticator (covered by the audit, except the AVX2 backend). |
| `aead` | 0.6.1 | `chacha20poly1305` | AEAD traits (`AeadInOut`, `KeyInit`). |
| `cipher` | 0.5.2 | `chacha20poly1305`, `chacha20` | Stream cipher traits. |
| `universal-hash` | 0.6.1 | `poly1305` | Universal hash traits. |
| `blake2` | 0.11.0 | `argon2` | BLAKE2b, the hash function inside Argon2. |
| `digest` | 0.11.3 | `blake2` | Hash function traits. |
| `password-hash` | 0.6.1 | `argon2` | PHC string API; compiled in but not used by NovaVault. |
| `phc` | 0.6.1 | `password-hash` | PHC string format; not used by NovaVault. |
| `base64ct` | 1.8.3 | `argon2`, `phc` | Constant-time Base64 for PHC strings; not used by NovaVault. |
| `ctutils` | 0.4.2 | `digest`, `universal-hash`, `phc` | Constant-time comparison and selection helpers. |
| `cmov` | 0.5.4 | `ctutils` | Constant-time conditional move. |
| `cpufeatures` | 0.3.1 | `argon2`, `chacha20`, `poly1305` | Runtime CPU feature detection for SIMD backends. |
| `crypto-common` | 0.2.2 | `aead`, `cipher`, `digest`, `universal-hash` | Shared key and block size types. |
| `inout` | 0.2.5 | `aead`, `cipher` | In-place and out-of-place buffer types. |

## Shell (`src-tauri`)

The shell implements no cryptography. These dependencies handle secrets, decide which files are read or written, or deliver the OS events that trigger auto-lock.

| Crate | Version | Features | Purpose and justification |
| ----- | ------- | -------- | ------------------------- |
| [`secrecy`](https://crates.io/crates/secrecy) | 0.10.3 | `serde` | Deserializes the master password, recovery codes and secret fields straight into `SecretString`, so the shell never holds them in a plain `String`. Serialization of secrets stays disabled; the two output DTOs that carry a secret serialize it explicitly. |
| [`zeroize`](https://crates.io/crates/zeroize) | 1.9.1 | `derive` | Wipes response DTOs (titles, user names, URLs) and the copied clipboard value once used. |
| [`tauri`](https://crates.io/crates/tauri) | 2.11.6 | defaults | The app shell: IPC, capabilities, CSP. Its command allow-list is what limits the webview to the nineteen vault commands. |
| [`serde`](https://crates.io/crates/serde) | 1.0.229 | `derive` | IPC (de)serialization; already required by Tauri. |
| [`serde_json`](https://crates.io/crates/serde_json) | 1.0.154 | defaults | Reads and writes the non-secret `settings.json`; already a Tauri dependency. |
| [`tauri-plugin-dialog`](https://crates.io/crates/tauri-plugin-dialog) | 2.7.3 | defaults | The official Tauri plugin for native open and save dialogs, called from Rust only; its webview commands are not granted, so the UI never opens dialogs or sees paths. Not cryptographic; listed because it chooses the files the shell reads and writes. |
| [`arboard`](https://crates.io/crates/arboard) | 3.7.0 | defaults off; `wayland-data-control` | Clipboard copy and clear on Windows, macOS, X11 and Wayland, maintained by 1Password, including the clipboard-history opt-outs (Windows history and cloud formats, macOS `ConcealedType`, KDE password hint). Doing the copy in Rust keeps copied values out of the webview. No image support. |
| `clipboard-win` | 5.4.3 | (via `arboard`, Windows) | Win32 clipboard access. |
| `wl-clipboard-rs` | 0.9.5 | (via `arboard`, Linux) | Pure-Rust Wayland data-control client, so clipboard copy and clear work natively on Wayland instead of only through XWayland. |
| [`windows`](https://crates.io/crates/windows) | 0.61.3 | `Win32_Foundation`, `Win32_Graphics_Gdi`, `Win32_System_LibraryLoader`, `Win32_System_RemoteDesktop`, `Win32_UI_WindowsAndMessaging` | Microsoft's official Win32 bindings, same version Tauri already uses; suspend and session-lock notifications through a hidden window. Windows only. |
| [`objc2-app-kit`](https://crates.io/crates/objc2-app-kit) | 0.3.2 | `NSWorkspace` | Sleep and session notifications. macOS only; same version Tauri uses. |
| [`objc2-foundation`](https://crates.io/crates/objc2-foundation) | 0.3.2 | `NSDistributedNotificationCenter`, `NSNotification`, `NSOperation`, `NSString`, `block2` | Notification centers, including the screen-lock notification. macOS only. |
| [`block2`](https://crates.io/crates/block2) | 0.6.2 | defaults | Objective-C blocks for the notification observers. macOS only. |
| [`zbus`](https://crates.io/crates/zbus) | 5.13.2 | defaults off; `tokio` | Pure-Rust D-Bus client for logind and screen saver signals, on Tauri's Tokio runtime, with no C library and no extra executor. Talks only to the local system and session buses. Linux only. |
| [`futures-util`](https://crates.io/crates/futures-util) | 0.3.34 | defaults off | Merges the D-Bus signal streams; already a Tauri dependency. Linux only. |

## UI (`ui/`)

No UI dependency implements cryptography. These are listed because they handle secrets the user types.

| Package | Version | Purpose and justification |
| ------- | ------- | ------------------------- |
| [`@tauri-apps/api`](https://www.npmjs.com/package/@tauri-apps/api) | 2.12.4 | The official Tauri client; `invoke` is the only way the UI reaches the shell (the global Tauri object is disabled). |
| [`@zxcvbn-ts/core`](https://www.npmjs.com/package/@zxcvbn-ts/core) | 4.2.0 | Master password strength estimate from dictionaries, common passwords, keyboard patterns and dates, which a length-and-character-classes rule cannot provide. Runs in the webview with no network access, on the password the UI already holds while the user types it; loaded lazily. The UI requires a score of at least 3 of 4. |
| [`@zxcvbn-ts/language-common`](https://www.npmjs.com/package/@zxcvbn-ts/language-common) | 4.1.3 | Common-password and keyboard-pattern dictionaries for zxcvbn. |
| [`@zxcvbn-ts/language-en`](https://www.npmjs.com/package/@zxcvbn-ts/language-en) | 4.1.1 | English word and name dictionaries for zxcvbn. |

## Keeping this up to date

- Lockfiles are committed and CI builds with `--locked` (Rust) and `npm ci` (UI) on Windows, macOS and Linux.
- Dependabot opens weekly update pull requests for Cargo, both npm packages and GitHub Actions.
- To check a version: `grep -A1 '^name = "argon2"$' Cargo.lock`, or look up `"node_modules/<package>"` in `ui/package-lock.json`.
- A new cryptography-related dependency needs a row here with its justification, and a note in the [threat model](threat-model.md) if it changes what an attacker can do.
- There is no automated vulnerability scanning (`cargo audit`, `cargo deny`, `npm audit`) in CI yet; see the [threat model](threat-model.md#process-and-assurance).
