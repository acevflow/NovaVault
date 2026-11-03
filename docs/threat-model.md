# Threat model

This document states what NovaVault protects, who it protects it from, which threats it is designed to resist, and which it explicitly does not. It complements the [architecture](architecture.md) and the [vault format specification](vault-format.md), and is consistent with the scope in [SECURITY.md](../SECURITY.md).

> [!NOTE]
> This threat model reflects the intended MVP design. It will be revised as the implementation lands and whenever a change affects what an attacker can or cannot do.

## Assets

| Asset | Description | Where it exists |
| ----- | ----------- | --------------- |
| Vault key | Random 256-bit key that encrypts all records and attachments. The single most valuable secret. | Wrapped in the vault file; in plaintext only in Rust memory while unlocked. |
| Master password | User-chosen secret that unlocks the password key slot. | User's memory; briefly in UI and Rust memory during unlock or change. |
| Recovery code | Random code that unlocks the recovery key slot. | User's offline copy; shown once at creation or regeneration. |
| Record contents | Passwords, notes, secrets and their fields. | Encrypted in the vault file; decrypted on demand in Rust; shown in the UI when the user views or reveals them. |
| Attachments | Files stored inside the vault. | Encrypted in the vault file; decrypted only when the user opens or exports them. |
| Metadata | Vault size, number and size of records, modification times. | Partly observable from the vault file (see [Residual risks](#residual-risks)). |

## Adversaries

1. **Offline attacker.** Obtains a copy of the vault file (stolen laptop with the vault locked, backup, cloud folder, discarded disk) and can attack it with unlimited time and hardware.
2. **File tamperer.** Can modify the vault file while it is at rest, for example via a shared or synced folder, and wants to corrupt, roll back or forge contents.
3. **Hostile content in the UI.** Content displayed by the app (record fields, file names, imported data) that tries to execute script in the webview, or a bug in a UI dependency.
4. **Unprivileged local process.** Another program running as a different user, or a sandboxed app, that can read shared resources such as the clipboard but not the NovaVault process memory.
5. **Opportunistic physical attacker.** Someone who gets brief access to the machine after the user walks away, or observes the screen.
6. **Supply-chain attacker.** Tries to introduce malicious code through a dependency or the build pipeline.

## Security goals

- Without the master password or the recovery code, the vault file reveals no record contents or attachments.
- Any modification of the vault file is detected before decrypted data is used.
- The vault key and raw key material never leave the Rust process and never reach the UI.
- Secrets spend as little time in memory and on the clipboard as practical.
- An unattended, unlocked vault locks itself.

## In-scope threats and mitigations

| Threat | Adversary | Mitigation |
| ------ | --------- | ---------- |
| Brute-forcing the master password from a copied vault | 1 | Argon2id with memory-hard parameters stored per key slot, random per-slot salt. Parameters can be raised over time by re-wrapping the slot. |
| Brute-forcing the recovery code | 1 | Recovery code is generated from the OS CSPRNG with enough entropy (target ≥ 128 bits) to be infeasible to guess, and also stretched with Argon2id. |
| Reading records or attachments from the file | 1 | All data encrypted with an AEAD cipher (XChaCha20-Poly1305 or AES-GCM) under the vault key. |
| Modifying, swapping or truncating ciphertext | 2 | AEAD authentication on every record, attachment chunk and key slot. Associated data binds each ciphertext to its record ID, type and the vault header so pieces cannot be moved between positions or vaults. A whole-file integrity seal under the vault key detects deleted, duplicated or reordered records and an individual record replaced by an older version. |
| Downgrading format or KDF parameters in the header | 2 | Header is authenticated (bound as associated data). The core enforces minimum KDF parameters and refuses unknown or older-than-supported format versions. |
| Malformed file triggering parser bugs | 2 | Strict, bounds-checked parsing in Rust with typed errors; no `unwrap()`; fuzz and negative tests. |
| Script injection in the webview exfiltrating keys | 3 | Keys never cross the IPC boundary. Strict CSP (no remote origins, no inline script, no `eval`), React escaping by default, minimal Tauri capabilities and no network access from the webview. |
| Script injection abusing Tauri commands | 3 | Coarse, intent-based commands that return only what a screen needs; no generic decrypt or file-access commands. |
| Secrets lingering in memory | 4, 5 | `secrecy` wrappers and `zeroize` for keys and secret fields; no long-lived plaintext cache; session dropped on lock. |
| Secrets left on the clipboard | 4, 5 | Copy is performed from Rust and the clipboard is cleared after a short timeout if it still holds the copied value. |
| Unattended unlocked vault | 5 | Auto-lock on inactivity, system suspend and screen lock. |
| Shoulder surfing | 5 | Secret fields are masked by default and revealed only on explicit user action. |
| Secrets in logs or crash output | all | Secrets are never logged; secret types do not implement printable formatting; error messages contain no key material. |
| Lost master password | user | Emergency recovery code as a second key slot, shown once and confirmed by the user. |
| Leaked or exposed recovery code | 1, 5 | The user can regenerate it, which replaces the recovery key slot and invalidates the old code. |
| Malicious or compromised dependency | 6 | Minimal dependencies, audited crypto crates only (justified in `docs/`), lockfiles committed, CI on all platforms. |

## Out of scope

NovaVault does not attempt to defend against the following. These match the out-of-scope list in [SECURITY.md](../SECURITY.md).

- **Fully compromised host:** malware running as the user, keyloggers, administrator or root access, a malicious OS, kernel or hypervisor. Such an attacker can read process memory or capture the master password as it is typed.
- **Physical access while the vault is unlocked**, including cold-boot and DMA attacks on a running machine.
- **Weak or reused master passwords.** Argon2id raises the cost of guessing but cannot make a weak password strong.
- **Insecure storage of the recovery code by the user.**
- **Side channels** such as power analysis, electromagnetic emissions or microarchitectural attacks beyond what the underlying audited crates already mitigate.
- **Denial of service** by someone who already controls the machine, including deleting the vault file. Backups are the user's responsibility.
- **Network attackers**, because core features do not use the network. Sync, browser autofill and sharing are out of scope for the product.

## Residual risks

- **Metadata leakage.** The vault file size, the number of records, each record's type and identifier, and each record's size rounded up to 64 bytes are observable without the master password. Titles, timestamps and all field contents are encrypted; payload padding hides the exact length of short secrets but not the rough size of long notes.
- **Rollback.** An attacker who can replace the file may restore an older, validly encrypted version of the whole file. The integrity seal prevents mixing records from different versions, but detecting a full rollback would require state outside the file and is not planned for the MVP.
- **OS-level copies.** Swap, hibernation files, crash dumps and the OS clipboard history may retain secrets outside NovaVault's control. Memory locking and platform-specific mitigations are to be evaluated.
- **Webview display.** Values revealed in the UI exist as JavaScript strings until garbage-collected. Only explicitly revealed values reach the UI.

## Assumptions

- The OS CSPRNG is sound.
- The audited cryptographic crates are correctly implemented.
- The user's machine is not compromised at the time they unlock the vault.
- The user keeps the recovery code somewhere separate from the vault file.

## Maintaining this document

Any pull request that changes cryptography, key handling, the vault format, the Tauri command surface or memory handling must describe its impact on this threat model, as required by [CONTRIBUTING.md](../CONTRIBUTING.md#security-sensitive-changes).
