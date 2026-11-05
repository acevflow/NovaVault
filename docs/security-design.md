# Security design

This document explains how NovaVault protects the vault: the key hierarchy, the recovery code, how secrets are handled in memory, and how auto-lock and the clipboard behave on each operating system. It describes the implemented code. Byte layouts are specified in the [vault format specification](vault-format.md); threats and open gaps are in the [threat model](threat-model.md); the crates used are justified in [dependencies.md](dependencies.md).

## Principles

- **All cryptography lives in Rust**, in `crates/novavault-core`. The shell (`src-tauri`) holds the unlocked vault and talks to the OS; the webview UI never receives the vault key, a wrapping key or any KDF output.
- **Two primitives only.** Argon2id (version 0x13) for key derivation and XChaCha20-Poly1305 for every encryption, both from audited RustCrypto crates. Randomness comes only from the OS CSPRNG through `getrandom`.
- **Fail closed.** File structure is checked before key derivation, authentication before any plaintext is used, and every decryption failure returns the same error.

## Key hierarchy

```text
master password ──┐                               ┌── recovery code (150 bits)
                  │ Argon2id                      │ Argon2id
                  │ (salt_p, slot params)         │ (salt_r, slot params)
                  ▼                               ▼
        password wrapping key           recovery wrapping key
                  │ XChaCha20-Poly1305            │ XChaCha20-Poly1305
                  │ ad = slot_ad ‖ metadata       │ ad = slot_ad ‖ metadata
                  ▼                               ▼
          password key slot ──── both wrap ──── recovery key slot
                                    │
                                    ▼
                     vault key (random 256 bits, never stored in clear)
                                    │ XChaCha20-Poly1305, fresh 192-bit nonce per message
          ┌─────────────────────────┼──────────────────────────────┐
          ▼                         ▼                              ▼
  records                   integrity seal                 attachment chunks
  ad = header ‖ 0x01 ‖      ad = header ‖ 0x02 ‖           ad = blob header ‖ 0x03 ‖
       record_id ‖ type          slots ‖ count ‖ records        index ‖ is_final
```

| Key | Origin | Lifetime |
| --- | ------ | -------- |
| Vault key | 32 bytes from the OS CSPRNG when the vault is created (`SecretKey::generate`). | Encrypts all data for the life of the vault; it is never rotated. In memory only while unlocked. |
| Wrapping keys | Argon2id over the master password bytes (used exactly as typed, no normalization) or the canonical 32-symbol recovery code, with the slot's salt and parameters; 32-byte output. | Derived for one wrap, unwrap or verification, then dropped and wiped. |
| Salts | 16 random bytes per slot. | Replaced every time a slot is rewritten, so a new slot shares nothing with the old one. |

**Argon2id parameters.** New slots use 64 MiB of memory, 3 iterations and 4 lanes (the second recommended option of RFC 9106). Because the parameters are read back from the file, they are bounded on read: memory 19 MiB to 4 GiB, iterations 2 to 64, lanes 1 to 16. The lower bound stops a tampered file from downgrading protection; the upper bound stops a crafted file from forcing a huge allocation or a very long unlock.

**Associated data.** The 32-byte header (magic, version, algorithm, flags, slot count, random vault identifier) is part of the associated data of everything encrypted under the vault key, so ciphertext cannot be moved between vaults or positions. Key slots use `slot_ad`, the header with its version field zeroed, followed by the slot's type, salt and parameters: editing any of them makes the unwrap fail, and a format migration does not need to rewrap slots for which it has only one of the two secrets.

**Records and seal.** Each item is one record whose payload (timestamps, title, fields, attachment list) is padded with zeros to a multiple of 64 bytes and encrypted. The file ends with a seal: an AEAD encryption of an empty message under the vault key whose associated data is the header, a context byte and the rest of the file before the seal (both key slots, the record count and every record), so deleting, duplicating, reordering or replaying a record is detected. The seal is verified right after the vault key is unwrapped and before any record is decrypted.

**Attachments.** Each attached file becomes a blob: a 48-byte header (magic, version, algorithm, vault identifier, attachment identifier) and the file encrypted in 64 KiB chunks. Each chunk's associated data carries the blob header, its index and whether it is the last chunk, so chunks cannot be reordered, dropped, duplicated or truncated and blobs cannot be swapped between attachments or vaults. Exports are decrypted to a temporary file and renamed into place only after every chunk authenticated and the size matches the one stored in the item.

**Nonces.** `aead::encrypt` draws a fresh 192-bit nonce from the OS CSPRNG for every message; callers cannot supply one. With a 192-bit nonce, random nonces are safe for far more messages than a vault will ever encrypt under one key.

## Unlocking

1. The shell locks any open vault first, so a failed attempt always leaves the app locked.
2. For a recovery code, the shell parses it (alphabet, length, checksum) before reading the file, so a typo returns `invalid_recovery_code` without running Argon2id.
3. The core reads at most the maximum file size, then checks the whole structure (magic, version, header fields, slots, KDF bounds, record framing) before any key derivation.
4. It derives the wrapping key with the slot's stored parameters and unwraps the vault key. A wrong secret and a damaged slot both return `invalid_credentials`.
5. It verifies the seal, migrates an older format version (keeping a backup until the new file reads back), decrypts each record once to check attachment identifiers, and deletes blobs no item refers to.

## Recovery code

**Format.** 30 random symbols from the alphabet `23456789ABCDEFGHJKLMNPQRSTUVWXYZ` (no `0`, `1`, `O` or `I`), 5 bits each, for 150 bits of entropy, followed by 2 checksum symbols. Each symbol is the low 5 bits of a random byte, which is uniform because 256 is a multiple of 32. The checksum is `Σ (i + 1) · symbol[i] mod 1021`, which detects every single-symbol substitution and every swap of two adjacent different symbols. It catches typos; it is not a security mechanism. The code is shown as 8 groups of 4 (`XXXX-XXXX-…`); parsing ignores case, dashes and whitespace, and Argon2id runs over the 32-symbol canonical form.

**Lifecycle.**

| Step | What happens |
| ---- | ------------ |
| Creation | `create_vault` generates the vault key and the code, wraps the key under both secrets and saves. The code is returned once, in display form, in the command's response. NovaVault never stores it. |
| Confirmation | The UI shows the code and offers to print it, then requires the user to retype it before continuing. There is no copy button, because the shell's timed clipboard clear applies only to item fields. The UI drops the code from its state when the flow ends. |
| Use | The code unlocks the vault through the recovery slot, and is accepted as the current credential for changing the master password (the forgotten-password flow) or regenerating the code. |
| Regeneration | `regenerate_recovery_code` first verifies the current password or recovery code against the in-memory slot, then generates a new code and rewrites the recovery slot with a fresh salt. If saving fails, the previous slot is restored in memory so it still matches the file. The previous code stops opening the current file. |

## Changing the master password

`change_master_password` checks the current credential (password or recovery code) by unwrapping the matching in-memory slot, rejects an empty new password, then wraps the existing vault key under the new password with a fresh salt and the default parameters. Only the password slot changes: records and attachments are not re-encrypted, and the recovery code keeps working. On a failed save the previous slot is restored in memory.

Because the vault key does not change, an older copy of the vault file still opens with the old password, and with the old recovery code after a regeneration (see the [threat model](threat-model.md#cryptography-and-key-management)).

## Memory handling

| Data | Handling |
| ---- | -------- |
| Vault key and wrapping keys | `SecretKey` wraps a `secrecy::SecretBox<[u8; 32]>`: wiped on drop, redacted in `Debug`, not `Clone`, and its bytes are readable only inside the core crate. Argon2 writes its output straight into that storage. The cipher's internal key copy is wiped too (the `zeroize` feature of `chacha20poly1305`), as is Argon2's working memory (the `zeroize` feature of `argon2`). |
| Decrypted plaintext | `aead::decrypt` returns a `SecretSlice`; on authentication failure the working buffer is wiped before the error is returned. Payloads being encoded are `Zeroizing<Vec<u8>>`. Allocations are sized exactly so conversions do not reallocate and leave copies behind. |
| Items | Records stay encrypted in memory and are decrypted for one request. Secret fields are `SecretString`; other item fields and response DTOs (titles, user names, URLs) derive `ZeroizeOnDrop`. |
| Attachment data | One 64 KiB plaintext chunk at a time, in buffers wiped on drop. |
| Secrets from the UI | The master password, recovery codes and secret fields are deserialized straight into `SecretString`. Output-only DTOs carrying a revealed value or a recovery code have no `Debug` and no `Deserialize`. |
| Clipboard | The copied value is held in `Zeroizing<String>` by the clipboard worker until it is cleared or replaced. |
| UI | Typed secrets, revealed fields and a new recovery code live only in React component state and are dropped after use, on hide, on navigation and on lock. They are never logged or written to storage. |

**Locking** drops the `Vault`, which wipes the vault key; the session then answers `locked` to any command that needs vault data.

**Limits.** Memory pages are not locked, so secrets can be swapped or written to hibernation files or crash dumps. JavaScript strings and the IPC buffers owned by Tauri and the webview cannot be wiped. Quitting the app does not drop the `Vault` first; its memory is released with the process. See the [threat model](threat-model.md#memory-and-the-operating-system).

## Auto-lock

| Trigger | Behaviour |
| ------- | --------- |
| Inactivity | A background thread checks every 5 seconds and locks once the timeout has passed since the last command. Every command counts as activity, including an attachment transfer for its whole duration. The timeout is 1 to 60 minutes, default 5. |
| System suspend | Locks unconditionally before the system sleeps. |
| Screen lock or session switch | Locks unconditionally. |
| Manual | The Lock button calls `lock`. |

- A manual or OS-triggered lock sets an interrupt flag before waiting for the session, so a running attachment transfer stops at its next chunk; the inactivity timer never interrupts a transfer.
- After any lock the shell clears a copied value from the clipboard. For automatic locks it also reloads the main webview, which discards everything the UI held in memory and returns it to the unlock screen without granting the webview any event permission.
- Settings live in `settings.json` next to the vault file. They are not secret and not authenticated; a missing, malformed or out-of-range file falls back to the defaults. Changing a setting requires an unlocked vault.
- If an OS monitor cannot start, the app prints a fixed message to stderr and keeps the inactivity timer and manual locking.

## Clipboard

Copying is done in Rust: `copy_item_field` decrypts the field and places it on the clipboard itself, returning only the clear delay. One worker thread owns the clipboard through `arboard` for the life of the app. The clipboard is cleared after the configured delay (5 to 300 seconds, default 30), when the vault locks and when the app exits normally, but only if it still holds the copied value (if it cannot be read, it is cleared anyway). A new copy replaces the previous one. Each copy carries the platform's "do not record" marker. Full per-OS behaviour is in [clipboard.md](clipboard.md).

## Per-OS notes

| | Windows | macOS | Linux |
| - | ------- | ----- | ----- |
| Suspend event | `WM_POWERBROADCAST` / `PBT_APMSUSPEND` | `NSWorkspaceWillSleepNotification` | logind `PrepareForSleep(true)` (system bus) |
| Screen lock event | `WM_WTSSESSION_CHANGE` / `WTS_SESSION_LOCK` | `com.apple.screenIsLocked` (undocumented), `NSWorkspaceSessionDidResignActiveNotification` (fast user switching) | logind `Session.Lock` for the app's session; `ActiveChanged(true)` from `org.freedesktop.ScreenSaver` or `org.gnome.ScreenSaver` (session bus) |
| Monitor implementation | Hidden, never-shown top-level window on its own thread; lock runs synchronously in the window procedure, so it completes before suspend. | Observers on the main thread; lock runs synchronously in the notification handler. | `zbus` tasks on Tauri's async runtime; each source is optional. |
| `unsafe` code | Yes, scoped to `os_lock/windows.rs`, one FFI call per block with a `SAFETY` comment. | Yes, scoped to `os_lock/macos.rs`, same rules. | None. |
| Clipboard opt-out | `CanIncludeInClipboardHistory = 0`, `CanUploadToCloudClipboard = 0` | `org.nspasteboard.ConcealedType` | `x-kde-passwordManagerHint: secret`; only `CLIPBOARD`, never `PRIMARY` |
| Clipboard after a crash | Value stays until replaced. | Value stays until replaced; Universal Clipboard may offer it to nearby devices. | Usually disappears with the process; XWayland may delay a clear for native Wayland apps. |
| Data directory | `%APPDATA%\io.github.acevflow.novavault` | `~/Library/Application Support/io.github.acevflow.novavault` | `~/.local/share/io.github.acevflow.novavault` |
| File permissions | Inherited from the user profile; none set by NovaVault. | Vault file, blobs and exports `0600`; attachments directory `0700`. | Same as macOS. |
| Visible monitor errors | No: release builds use the GUI subsystem, so stderr is not shown. | Only when started from a terminal. | Only when started from a terminal. |

On every OS, saving writes a temporary file with `create_new`, flushes it and renames it over the vault file; on Unix the directory is also synced after the rename.

## Related documents

- [Threat model](threat-model.md)
- [Vault format specification](vault-format.md)
- [Architecture](architecture.md)
- [Clipboard behaviour](clipboard.md)
- [Cryptography dependencies](dependencies.md)
