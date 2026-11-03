# Vault format specification

| | |
| --- | --- |
| **Status** | Specified for format version 0, implemented in `crates/novavault-core`. Pre-release: not yet declared stable. |
| **Format version** | 0 (pre-release) |
| **Applies to** | NovaVault pre-1.0 development builds |

> [!WARNING]
> Format version 0 is complete for passwords, notes and secrets but does not include attachments (§9). Until NovaVault 1.0 declares a stable version, the format may still change; any change will follow the rules in §11. Do not rely on it for real data yet.

## 1. Scope and goals

This document specifies the on-disk format of a NovaVault vault so that it can be reviewed independently and read by an independent implementation.

Design goals:

- **Confidentiality and integrity** of all user data using audited primitives only.
- **Two key slots** (master password and recovery code) that each unlock the same vault key.
- **Cheap secret rotation:** changing the master password or regenerating the recovery code rewrites only one key slot.
- **Versioned and self-describing:** a reader can tell which version and which algorithms a file uses before attempting to decrypt it.
- **Strict parsing:** any unknown, malformed or unauthenticated input is rejected.
- **Minimal metadata:** only what is needed to unlock and locate records is stored in plaintext.

## 2. Conventions

- The key words MUST, MUST NOT, SHOULD and MAY are used as described in RFC 2119.
- Multi-byte integers are unsigned and little-endian.
- `||` denotes byte concatenation. `len(x)` is the length of `x` in bytes.
- `str` denotes a `u32` byte length followed by that many bytes of UTF-8 text.
- All random values are generated with the operating system CSPRNG.
- Offsets and sizes are in bytes.

## 3. Cryptographic primitives

| Purpose | Algorithm | Notes |
| ------- | --------- | ----- |
| Key derivation | Argon2id v0x13 (RFC 9106) | Parameters stored per key slot. Readers MUST reject memory below 19 MiB or above 4 GiB, iterations below 2 or above 64, and parallelism below 1 or above 16. Writers use 64 MiB, 3 iterations and 4 lanes by default. Output is 32 bytes. |
| Authenticated encryption | XChaCha20-Poly1305 (draft-irtf-cfrg-xchacha) | 256-bit key, 24-byte random nonce, 16-byte tag. Rationale in [architecture.md](architecture.md#why-xchacha20-poly1305-rather-than-aes-gcm). The header records the algorithm identifier so the choice is explicit in every file. |
| Vault key | 256-bit random key | Generated once at vault creation. |
| Randomness | OS CSPRNG | Used for the vault key, vault and record identifiers, salts, nonces and the recovery code. |

No custom primitives are used. Crate choices and their justification are documented in [architecture.md](architecture.md#dependencies).

A **sealed message** is the output of one XChaCha20-Poly1305 encryption with a fresh random nonce, stored as `nonce (24) || ciphertext || tag (16)`. It is `len(plaintext) + 40` bytes long. `Seal(key, plaintext, ad)` and `Open(key, sealed, ad)` denote encryption and verified decryption.

## 4. File layout

A vault is a single file with the following sections, in order and with no gaps:

```text
offset              size              section
0                   32                header             plaintext (§5)
32                  202               key slot table     2 × 101-byte key slot (§6)
234                 4                 record_count       u32
238                 variable          records            record_count × record (§7)
len(file) − 40      40                integrity seal     (§8)
```

A file MUST NOT contain any byte after the integrity seal. Writers MUST NOT produce files larger than 256 MiB, and readers MUST reject them (after checking the magic and version, §12).

Writes MUST be atomic: a new version of the file is written to a temporary file in the same directory, flushed to disk, and then renamed over the old one. On systems with POSIX permissions the file SHOULD be readable and writable by its owner only.

The recommended file extension is `.nvault`.

## 5. Header

| Offset | Field | Size | Description |
| ------ | ----- | ---- | ----------- |
| 0 | `magic` | 8 | `4E 4F 56 41 56 4C 54 00` (`"NOVAVLT\0"`). |
| 8 | `format_version` | 2 (u16) | `0` for this version. |
| 10 | `aead_id` | 1 | `0x01` = XChaCha20-Poly1305. Readers MUST reject any other value. |
| 11 | `flags` | 1 | Reserved. MUST be zero in version 0. |
| 12 | `slot_count` | 1 | Number of key slots. MUST be `2` in version 0. |
| 13 | `reserved` | 3 | MUST be zero. |
| 16 | `vault_id` | 16 | Random identifier generated at creation. |

The header is stored in plaintext so a reader can select the right algorithms. It is authenticated by being part of the associated data of every key slot (§6.3), record (§7.3) and the integrity seal (§8). In version 0 no header field changes after creation, so records never need to be re-encrypted because of the header.

`header` below always denotes these 32 bytes.

## 6. Key slots

### 6.1 Purpose

The vault key is never stored in the clear. It is stored wrapped (encrypted) once per key slot. Version 0 defines exactly two slot types:

| Slot type | Secret | Required |
| --------- | ------ | -------- |
| `password` | Master password chosen by the user | Exactly one |
| `recovery` | Emergency recovery code generated by NovaVault | Exactly one |

Either slot alone is sufficient to recover the vault key.

### 6.2 Slot structure

A key slot is exactly 101 bytes.

| Offset | Field | Size | Description |
| ------ | ----- | ---- | ----------- |
| 0 | `slot_type` | 1 | `0x01` = `password`, `0x02` = `recovery`. Readers MUST reject any other value. |
| 1 | `kdf_salt` | 16 | Random per-slot salt. A new salt MUST be generated whenever the slot is rewritten. |
| 17 | `kdf_memory_kib` | 4 (u32) | Argon2id memory cost in KiB. |
| 21 | `kdf_iterations` | 4 (u32) | Argon2id time cost. |
| 25 | `kdf_parallelism` | 4 (u32) | Argon2id lanes. |
| 29 | `nonce` | 24 | Random XChaCha20-Poly1305 nonce for wrapping. |
| 53 | `wrapped_key` | 48 (32 + 16-byte tag) | Vault key encrypted under the slot's wrapping key, followed by the Poly1305 tag. |

`slot_metadata` is bytes 0–28 (`slot_type` to `kdf_parallelism`, 29 bytes). Readers MUST reject a slot whose KDF parameters are outside the bounds in §3 before running Argon2id.

The key slot table is the `password` slot followed by the `recovery` slot. Readers MUST reject a table in any other order.

### 6.3 Wrapping and unwrapping

```text
wrapping_key = Argon2id(secret, kdf_salt, memory, iterations, parallelism, out_len = 32)
nonce        = random(24)
wrapped_key  = XChaCha20-Poly1305_Encrypt(wrapping_key, nonce, vault_key, ad = header || slot_metadata)
```

Because `header` and `slot_metadata` have fixed lengths, the concatenation is unambiguous. Binding both into the associated data means changing a slot's type, salt or KDF parameters, or moving a slot to another vault, makes unwrapping fail.

Unwrapping performs the same derivation and `XChaCha20-Poly1305_Decrypt`. An authentication failure MUST be reported as a single generic error that does not distinguish a wrong secret, a wrong header or a corrupted slot.

The `secret` input is:

- **`password` slot:** the UTF-8 bytes of the master password, exactly as entered. Unicode normalization (for example NFC) is *to be decided before format version 1*; until then, a password typed with a different Unicode composition will not match.
- **`recovery` slot:** the 32 ASCII bytes of the recovery code in canonical form (§10): upper case, no separators, checksum symbols included.

### 6.4 Operations

- **Create:** generate a random 256-bit vault key, a `vault_id` and a new recovery code, then write a `password` slot and a `recovery` slot that both wrap the vault key.
- **Unlock:** unwrap the `password` slot with the master password, or the `recovery` slot with the recovery code. Both yield the same vault key.
- **Change the master password:** with the vault key from any unlock (including one with the recovery code, for a forgotten password), write a new `password` slot with a fresh salt and nonce. The vault key, the `recovery` slot and the records are unchanged; only the integrity seal is recomputed.
- **Regenerate the recovery code:** generate a new code and write a new `recovery` slot with a fresh salt and nonce. The previous code no longer opens the vault once the new file is written. The `password` slot is unchanged.
- KDF parameters MAY be increased during either rotation.
- A new slot is fully computed before it replaces the old one, and the file is written atomically (§4), so a failed rotation leaves the vault openable with the previous secrets.

## 7. Records

### 7.1 Record types

Each record holds one item. Version 0 defines three record types:

| `record_type` | Item | Fields after the title, in order |
| ------------- | ---- | -------------------------------- |
| `0x01` | password | `username`, `password`, `url`, `notes` |
| `0x02` | note | `body` |
| `0x03` | secret | `value`, `notes` |

Readers MUST reject any other value. Every field is a `str` and MAY be empty.

### 7.2 Record encoding

| Offset | Field | Size | Description |
| ------ | ----- | ---- | ----------- |
| 0 | `record_id` | 16 | Random identifier, unique within the vault. |
| 16 | `record_type` | 1 | §7.1. |
| 17 | `sealed_len` | 4 (u32) | Length of `sealed`. MUST be between 40 and 1 048 616 (1 MiB + 40). |
| 21 | `sealed` | `sealed_len` | Sealed message of the payload (§7.4). |

Records MUST be sorted by strictly ascending `record_id` (compared as unsigned byte strings). This makes the encoding canonical and rules out duplicate identifiers. `record_count` MUST NOT exceed 100 000.

Only `record_id` and `record_type` are visible in plaintext. The title, timestamps and all fields are encrypted.

### 7.3 Record encryption

```text
ad     = header || 0x01 || record_id || record_type        (50 bytes)
sealed = Seal(vault_key, payload, ad)
```

- Records are encrypted directly under the vault key. This is safe because the vault key is never used for anything else that could collide: key slots use Argon2id-derived wrapping keys, and every use of the vault key has its own context byte in the associated data (`0x01` for records, `0x02` for the seal).
- A fresh random nonce is used for every write of a record, including when it is rewritten unchanged.
- The associated data binds a ciphertext to its vault (through `vault_id` in the header), its identifier and its type, so it cannot be moved to another record or vault, or reinterpreted as another type.

### 7.4 Payload

```text
payload = created_at_ms (u64) || updated_at_ms (u64) || title (str) || fields (str…) || padding
```

- `created_at_ms` and `updated_at_ms` are milliseconds since the Unix epoch, UTC.
- `fields` are the type's fields in the order of §7.1.
- `padding` is zero bytes such that `len(payload)` is a multiple of 64. It MUST be shorter than 64 bytes. The padding hides the exact length of short secrets: the ciphertext reveals only a 64-byte size bucket.
- `len(payload)` MUST NOT exceed 1 MiB (1 048 576 bytes).

Readers MUST reject a payload in which a length runs past the end, text is not valid UTF-8, the padding is not all zero or is 64 bytes or longer, or the total length is not a multiple of 64.

## 8. Integrity seal

Per-record authentication does not detect a record that was deleted, duplicated, or replaced by an older valid version of the same record. The file therefore ends with a seal over everything before it:

```text
body = header || key slot table || record_count || records
ad   = header || 0x02 || (body without its first 32 bytes)
seal = Seal(vault_key, empty plaintext, ad)                    (40 bytes)
```

The seal is recomputed on every save. Readers MUST verify it right after unlocking the vault key and MUST NOT return any record from a file whose seal does not verify. Replacing the *whole* file with an older valid version (rollback) is not detected; see the [threat model](threat-model.md#residual-risks).

## 9. Attachments

Attachments are **not part of format version 0**. A version 0 file contains no attachment data, and the record types of §7.1 have no attachment fields. They will be specified, with a format version change following §11, before encrypted attachments ship. The intended design:

- An attachment belongs to exactly one record. The owning record's encrypted payload stores the attachment's identifier, file name, media type and size, so none of these are visible in plaintext.
- Attachment data is split into fixed-size chunks and each chunk is encrypted separately so large files can be streamed without loading them fully into memory.
- Each chunk's associated data binds `vault_id`, the attachment identifier, the chunk index and a final-chunk marker, so chunks cannot be reordered, dropped, duplicated or truncated undetected.

## 10. Recovery code

- **Alphabet:** the 32 symbols `23456789ABCDEFGHJKLMNPQRSTUVWXYZ`, where symbol `i` has value `i`. The look-alikes `0`, `1`, `O` and `I` are excluded.
- **Random part:** 30 symbols. Each symbol is the low 5 bits of a byte from the OS CSPRNG; since 256 is a multiple of 32 this is uniform, giving 150 bits of entropy.
- **Checksum:** with `v[0..30]` the values of the random symbols, `c = (Σ (i + 1) · v[i]) mod 1021`, appended as two symbols with values `c / 32` and `c mod 32`. Because 1021 is a prime larger than every weight and every symbol difference, the checksum detects every single-symbol substitution and every swap of two adjacent symbols in the random part, and every single-symbol substitution in the checksum. It is a typo check, not a security mechanism.
- **Canonical form:** the 32 symbols (random part then checksum), upper case, no separators. This is the Argon2id input (§6.3).
- **Display form:** the canonical form in 8 groups of 4 symbols separated by `-`, for example `ABCD-EFGH-JKLM-NPQR-STUV-WXYZ-2345-6789`.
- **Parsing:** input is upper-cased and `-` and whitespace are removed. Readers MUST then reject input that does not have exactly 32 symbols, contains a character outside the alphabet, or fails the checksum, before running Argon2id.
- Shown once at creation and at each regeneration. The user MUST confirm they saved it before the operation completes (an application-level step). The vault never stores the code itself.

## 11. Versioning and compatibility

- Every file carries `format_version`. Readers MUST reject versions they do not support and MUST NOT attempt best-effort parsing. Because the version is checked right after the magic, a file written by a newer NovaVault is reported as an unsupported version, not as a corrupted file.
- Any change to the on-disk format requires a version bump, an update to this specification and a migration path that upgrades older vaults on unlock.
- Migrations rewrite the file atomically and SHOULD keep a backup of the previous file until the new one has been verified.
- Format version 1 will be the first stable version and will be declared at the NovaVault 1.0 release.

## 12. Validation rules for readers

A conforming reader MUST perform these steps in order and stop at the first failure:

1. Check `magic` (otherwise: not a vault file), then `format_version` (otherwise: unsupported version).
2. Check the rest of the header: known `aead_id`, zero `flags` and `reserved`, `slot_count` of 2.
3. Reject files larger than 256 MiB.
4. Parse the key slot table: known slot types in the order `password`, `recovery`, and KDF parameters within the bounds of §3.
5. Parse the record framing: `record_count` and each `sealed_len` within bounds, known record types, strictly ascending `record_id`, and exactly 40 bytes left for the seal.
6. Derive the wrapping key and unwrap the vault key from the slot matching the user's secret. Report any failure as a single generic error (§6.3).
7. Verify the integrity seal (§8).
8. Decrypt and decode individual records on demand (§7.3, §7.4).

Steps 1–5 need no key and run before the expensive key derivation. Throughout, a reader MUST check every length against the remaining input before allocating memory, and MUST treat any authentication failure as fatal and never return unauthenticated plaintext.

## 13. Test vectors

The following vectors are deterministic and checked by the core crate's unit tests (`header::tests::known_encoding`, `item::tests::known_note_encoding`). Values are hex.

**Header** with `vault_id = 000102030405060708090a0b0c0d0e0f`:

```text
4e4f5641564c5400 0000 01 00 02 000000 000102030405060708090a0b0c0d0e0f
magic            ver  aead flags slots reserved vault_id
```

**Note payload** with title `"t"`, body `"b"`, `created_at_ms = 1`, `updated_at_ms = 2` (64 bytes):

```text
0100000000000000 0200000000000000 01000000 74 01000000 62 00 × 38
created_at_ms    updated_at_ms    len      t  len      b  padding
```

**Recovery code:** a random part of 30 × `3` (value 1) has checksum `465 = 14 · 32 + 17`, so the code is `3333-3333-3333-3333-3333-3333-3333-33GK`.

Key derivation and AEAD vectors (RFC 9106 Argon2id, draft-irtf-cfrg-xchacha XChaCha20-Poly1305) are listed in [architecture.md](architecture.md#test-vectors). Sealed values use random nonces, so complete-file vectors are not deterministic; the core's tests instead check round trips, every truncation, and tampering with every field of every section.

## Changelog of this specification

| Version | Date | Change |
| ------- | ---- | ------ |
| 0 (draft) | 2026-11-03 | Initial skeleton. |
| 0 (draft) | 2026-11-03 | Key slot encoding, wrapping associated data, slot operations and recovery code format specified. |
| 0 | 2026-11-03 | Header, file layout, record types, record encoding and encryption, payload encoding and padding, integrity seal, reader validation order and test vectors specified. Attachments moved out of version 0. |
