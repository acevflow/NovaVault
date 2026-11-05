# Threat model

This document states what NovaVault protects, who it protects it from, which mitigations are implemented today, and which threats remain open. It complements the [security design](security-design.md), the [architecture](architecture.md) and the [vault format specification](vault-format.md), and is consistent with the scope in [SECURITY.md](../SECURITY.md).

> [!NOTE]
> This document describes the code as it is implemented, not planned work. Every mitigation listed below exists in the source; anything not yet done is listed under [Known limitations and gaps](#known-limitations-and-gaps). NovaVault has not had an external security review.

## Assets

| Asset | Description | Where it exists |
| ----- | ----------- | --------------- |
| Vault key | Random 256-bit key that encrypts all records and attachments. The single most valuable secret. | Wrapped in the vault file's two key slots; in plaintext only in Rust memory while unlocked. |
| Master password | User-chosen secret that unlocks the password key slot. | User's memory; briefly in UI and Rust memory during create, unlock, re-authentication or change. |
| Recovery code | Random code that unlocks the recovery key slot. | User's offline copy; in UI and Rust memory only while shown, retyped or used. Never stored by NovaVault. |
| Record contents | Passwords, notes, secrets and their fields. | Encrypted in the vault file and in memory; decrypted in Rust for a single request; shown in the UI only when the user reveals a field. |
| Attachments | Files attached to items. | Encrypted blob files in `<vault file>.attachments/`, with their names and sizes inside the encrypted records; decrypted only in Rust, when the user exports them. |
| Metadata | Vault size, number and size of records, file times. | Partly observable from the files (see [Known limitations and gaps](#known-limitations-and-gaps)). |

## Adversaries

1. **Offline attacker.** Obtains a copy of the vault file and its attachment blobs (stolen laptop with the vault locked, backup, cloud folder, discarded disk) and can attack it with unlimited time and hardware.
2. **File tamperer.** Can modify the vault files while they are at rest, for example through a shared or synced folder, and wants to corrupt, roll back or forge contents.
3. **Hostile content in the UI.** Content displayed by the app (record fields, file names) that tries to execute script in the webview, or a bug in a UI dependency.
4. **Unprivileged local process.** Another program that can read shared resources such as the clipboard but not the NovaVault process memory.
5. **Opportunistic physical attacker.** Someone who gets brief access to the machine after the user walks away, or observes the screen.
6. **Supply-chain attacker.** Tries to introduce malicious code through a dependency or the build pipeline.

## Security goals

- Without the master password or the recovery code, the vault files reveal no record contents or attachment contents.
- Any modification of the vault file is detected before decrypted data is used; any modification of an attachment blob is detected before an export is put in place.
- The vault key and raw key material never leave the Rust process and never reach the UI.
- Secrets spend as little time in memory and on the clipboard as practical.
- An unattended, unlocked vault locks itself.

## Implemented mitigations

| Threat | Adversary | Mitigation (as implemented) |
| ------ | --------- | --------------------------- |
| Brute-forcing the master password from a copied vault | 1 | Each key slot wraps the vault key under an Argon2id (v0x13) key derived with a random 16-byte salt, fresh on every wrap. New slots use 64 MiB, 3 iterations, 4 lanes (`KdfParams::default`). The parameters are stored in the slot and authenticated with it. |
| Weakening key derivation by editing the file | 2 | Slot parameters are untrusted input: `KdfParams::new` rejects memory below 19 MiB or above 4 GiB, iterations outside 2 to 64 and lanes outside 1 to 16, and `derive_key` checks them again. Editing a slot's type, salt or parameters makes its unwrap fail. |
| Brute-forcing the recovery code | 1 | 30 symbols from a 32-symbol alphabet drawn from the OS CSPRNG (150 bits), plus 2 checksum symbols, stretched with the same Argon2id parameters as the password slot. |
| Reading records or attachments from the files | 1 | Records, the integrity seal and attachment chunks are encrypted with XChaCha20-Poly1305 under the vault key, each with a fresh random 192-bit nonce. Titles, timestamps and fields are inside the encrypted payload. |
| Modifying, swapping or truncating ciphertext | 2 | Every record's associated data binds it to the header, its identifier and its type. A whole-file seal (an AEAD tag over the header, slots and every record) is verified before any record is read, so deleted, duplicated, reordered or replayed records are detected. Each attachment chunk's associated data binds its vault, attachment, index and final flag, so chunks cannot be reordered, dropped, duplicated or truncated and blobs cannot be swapped; the size stored in the item must match. Exports go to a temporary file renamed into place only after the whole blob authenticated. |
| Downgrading the format version | 2 | Readers refuse unknown versions and algorithms. The version is authenticated by every record and by the seal; key slots authenticate the header with the version zeroed, so a migration need not rewrap them, and a rewritten version still fails the seal. |
| Malformed file triggering parser bugs | 2 | Strict, bounds-checked parsing in Rust with typed errors, run before any key derivation. The core is `#![forbid(unsafe_code)]`, and `unwrap()`/`expect()` are denied by Clippy outside tests. Unit tests cover every truncation, a flipped byte in every field and unsupported versions; there is no fuzzing yet (see gaps). |
| Crash or power loss while saving | 2 | The vault file is written to a temporary file (`create_new`), flushed and renamed over the old one; on Unix the directory is synced. A migration keeps a backup until the new file reads back correctly. |
| Script injection in the webview exfiltrating keys | 3 | Keys never cross the IPC boundary. The production CSP allows only `'self'` scripts and styles, no inline code, no `eval`, and only IPC connections. `withGlobalTauri` is off and `freezePrototype` is on. React escapes rendered content. |
| Script injection abusing Tauri commands | 3 | `build.rs` declares nineteen coarse, intent-based commands and `capabilities/default.json` grants exactly those, with no core or plugin permission; a test checks that the two lists match. Item details never contain secret values; `reveal_item_field` returns one secret field per call and `copy_item_field` copies from Rust without returning the value. No command takes a path: the vault location is fixed by the shell and attachment files are picked in native dialogs opened from Rust. Exports over the vault file or into the attachments directory are refused. |
| Taking over an unlocked, unattended vault | 5 | Changing the master password or regenerating the recovery code requires the current password or recovery code first (`verify_password`, `verify_recovery_code`). Changing the auto-lock timeout or clipboard delay requires an unlocked vault. |
| Secrets lingering in memory | 4, 5 | The vault key is a `SecretKey` (`secrecy::SecretBox`, no `Clone`, bytes readable only inside the core). Records stay encrypted in memory and are decrypted for one request. Decrypted payloads, secret fields, derived keys and recovery codes live in zeroizing types; failed decryptions wipe their buffer. Locking drops the `Vault`, wiping the key. |
| Secrets left on the clipboard | 4, 5 | Copy is done in Rust. The clipboard is cleared after a configurable delay (5 to 300 seconds, default 30), on lock and on normal exit, only if it still holds the copied value. Copies carry the platform's history opt-out markers. See [clipboard.md](clipboard.md). |
| Unattended unlocked vault | 5 | Auto-lock after a configurable time since the last command (1 to 60 minutes, default 5, checked every 5 seconds), before system suspend and on screen lock, with one native monitor per OS. A manual or OS-triggered lock stops a running attachment transfer within one chunk. Every lock clears a copied value and reloads the webview, discarding the UI's state. |
| Shoulder surfing | 5 | Secret fields are masked by default and revealed one at a time on explicit request. The edit form starts secret fields empty and keeps stored values unless the user changes them. |
| Secrets in logs or error output | all | Errors returned to the UI carry fixed messages and never a command argument or I/O detail. Secret types redact or omit `Debug`; output-only DTOs that hold secrets have no `Debug`. The shell's stderr messages are fixed strings, apart from a Tauri startup error printed before any vault can be opened. |
| Lost master password | user | The recovery code opens the vault through its own slot and is accepted as the current credential for setting a new password. |
| Leaked recovery code | 1, 5 | Regenerating replaces the recovery slot with a new code and fresh salt; the previous code stops opening the current vault file (but see gaps for older copies). |
| Malicious or compromised dependency | 6 | Few dependencies, audited RustCrypto crates for all cryptography (see [dependencies.md](dependencies.md)), committed lockfiles, `--locked` CI builds on Windows, macOS and Linux, and weekly Dependabot updates. |

## Out of scope

NovaVault does not attempt to defend against the following. These match the out-of-scope list in [SECURITY.md](../SECURITY.md).

- **Fully compromised host:** malware running as the user, keyloggers, administrator or root access, a malicious OS, kernel or hypervisor. Such an attacker can read process memory or capture the master password as it is typed.
- **Physical access while the vault is unlocked**, including cold-boot and DMA attacks on a running machine.
- **Weak or reused master passwords.** Argon2id raises the cost of guessing but cannot make a weak password strong.
- **Insecure storage of the recovery code by the user.**
- **Side channels** such as power analysis, electromagnetic emissions or microarchitectural attacks beyond what the underlying audited crates already mitigate.
- **Denial of service** by someone who already controls the machine, including deleting the vault file. Backups are the user's responsibility.
- **Network attackers**, because core features do not use the network. Sync, browser autofill and sharing are out of scope for the product.

## Known limitations and gaps

These are properties of the current implementation that users and reviewers should know about. Some are inherent to the design; others are open work.

### Cryptography and key management

- **The vault key never changes.** Changing the master password or regenerating the recovery code rewrites one key slot only. Any older copy of the vault file (a backup, a synced or cloud copy, a leftover `.v<N>.bak` from a migration) still opens with the old password or the old recovery code, and once opened reveals the same vault key that protects the current file and its blobs. There is no key rotation or full re-encryption.
- **KDF parameters cannot be raised by the user.** New and rewritten slots always use the compiled defaults (64 MiB, 3 iterations, 4 lanes). Slots are upgraded only when the user changes the password or regenerates the recovery code after the defaults change in a release.
- **Password strength is enforced only by the UI.** The UI requires a zxcvbn score of at least 3 of 4; the shell itself rejects only an empty password, so a compromised webview could create a vault with a weak password.
- **No attempt limiting.** Unlock attempts are slowed only by the cost of Argon2id; there is no lockout or delay between attempts (an offline attacker could not be limited anyway).
- **Rollback.** An attacker who can replace the files may restore an older, validly encrypted version of the whole vault. The seal prevents mixing records from different versions, but detecting a full rollback would need state outside the file.

### Memory and the operating system

- **No memory locking.** Key material and decrypted data are wiped when dropped, but their pages are not locked (`mlock`/`VirtualLock` are not used), so they can reach swap, hibernation files or crash dumps.
- **Quitting does not lock first.** On exit the shell clears the clipboard but does not drop the `Vault`; its memory is released with the process without being wiped.
- **Copies outside NovaVault's control.** Tauri and the webview hold command arguments and responses (master password, recovery code, item fields, revealed values) in JSON buffers that are freed without wiping. The webview keeps typed or revealed secrets as JavaScript strings until garbage-collected; the UI drops them from its state as soon as they are used. The clipboard library and the OS keep their own copies of a copied value.
- **No secure deletion.** Replaced vault files, deleted blobs, failed temporary files and migration backups are removed with ordinary file deletion; their contents (encrypted) may remain on disk or in snapshots.

### Files and storage

- **Metadata leakage.** The vault file size, the number of records, each record's type and identifier, and each record's size rounded up to 64 bytes are observable without a secret. For attachments, the number of blob files, each blob's exact size and the blobs' file times are observable; names, contents and which item a blob belongs to are not. Blobs are not padded.
- **Attachment storage.** Blobs live in a directory next to the vault file, so a backup or move that copies only the vault file loses the attachments. A deleted or damaged blob is detected only when that attachment is exported.
- **Exported files.** Exporting writes the decrypted file wherever the user chooses; from then on it is outside NovaVault's protection. While an export runs, the plaintext written so far sits in a temporary file next to the destination (deleted on failure or cancellation, but not if the app is killed).
- **File permissions.** On Unix the vault file, blobs, exports and their temporary files are created `0600` and the attachments directory `0700`, but the app data directory and `settings.json` use the default permissions. On Windows and macOS NovaVault sets no permissions and relies on the user profile's default access control.
- **Unauthenticated settings.** `settings.json` (auto-lock timeout, clipboard delay) is not encrypted or authenticated. Anyone who can write it can lengthen the timeout to at most 60 minutes and the clipboard delay to at most 300 seconds; invalid values fall back to the defaults.

### Auto-lock and clipboard

- **Inactivity means no command.** The timer restarts only on commands, so the vault can lock while the user is reading a screen, and mouse or keyboard activity without a command does not keep it unlocked.
- **OS monitor coverage.** The macOS screen-lock signal (`com.apple.screenIsLocked`) is long-standing but undocumented. On Linux, suspend and session-lock events need systemd-logind and screen-saver events need `org.freedesktop.ScreenSaver` or `org.gnome.ScreenSaver`. If a monitor is unavailable, only the inactivity timer and manual locking remain.
- **Silent monitor failures on Windows.** A monitor failure is reported only on stderr, which Windows release builds (built for the GUI subsystem) do not show; the user is not told in the app.
- **Clipboard exposure.** While a copied value is on the clipboard, any app running as the user can read it, and clipboard tools that ignore the opt-out markers may keep copies. A crash or forced quit skips the clear on exit; on Windows and macOS the value then stays on the clipboard. On Wayland compositors without the data-control protocol, the XWayland fallback may not propagate a timed clear to native Wayland apps until NovaVault's window is focused again. Details per OS are in [clipboard.md](clipboard.md).
- **Printed recovery codes.** Printing sends the code to the OS print system, which may spool it to disk or to a network printer. The UI recommends writing the code down or printing to a trusted local printer.

### Process and assurance

- **No external audit** of NovaVault itself has been done; only some of the underlying crates have been audited (see [dependencies.md](dependencies.md)).
- **No fuzzing.** Parsers are covered by unit and negative tests only.
- **No dependency vulnerability scanning in CI** (such as `cargo audit` or `npm audit`); updates arrive through Dependabot.
- **No signed or reproducible release builds** yet; there is no release.

## Assumptions

- The OS CSPRNG is sound.
- The audited cryptographic crates are correctly implemented.
- The user's machine is not compromised at the time they unlock the vault.
- The user keeps the recovery code somewhere separate from the vault file.

## Maintaining this document

Any pull request that changes cryptography, key handling, the vault format, the Tauri command surface or memory handling must describe its impact on this threat model, as required by [CONTRIBUTING.md](../CONTRIBUTING.md#security-sensitive-changes).
