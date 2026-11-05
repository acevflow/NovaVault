# Vault format specification

| | |
| --- | --- |
| **Status** | Specified for format version 1, implemented in `crates/novavault-core`. Pre-release: not yet declared stable. |
| **Format version** | 1 (pre-release). Version 0 files are read and migrated to version 1 (§11.1). |
| **Applies to** | NovaVault pre-1.0 development builds |

> [!WARNING]
> Format version 1 covers passwords, notes, secrets and encrypted file attachments (§9). Until NovaVault 1.0 declares a stable version, the format may still change; any change will follow the rules in §11. Do not rely on it for real data yet.

## 1. Scope and goals

This document specifies the on-disk format of a NovaVault vault so that it can be reviewed independently and read by an independent implementation.

Design goals:

- **Confidentiality and integrity** of all user data using audited primitives only.
- **Two key slots** (master password and recovery code) that each unlock the same vault key.
- **Cheap secret rotation:** changing the master password or regenerating the recovery code rewrites only one key slot.
- **Versioned and self-describing:** a reader can tell which version and which algorithms a file uses before attempting to decrypt it.
- **Strict parsing:** any unknown, malformed or unauthenticated input is rejected.
- **Minimal metadata:** only what is needed to unlock and locate records is stored in plaintext.
- **Streaming attachments:** files of any size are encrypted and decrypted chunk by chunk, never loaded into memory whole.

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
| Key derivation | Argon2id v0x13 (RFC 9106) | Parameters stored per key slot. Readers MUST reject memory below 19 MiB (19 456 KiB) or above 4 GiB (4 194 304 KiB), iterations below 2 or above 64, and parallelism below 1 or above 16. Writers use 64 MiB (65 536 KiB), 3 iterations and 4 lanes by default. Output is 32 bytes. |
| Authenticated encryption | XChaCha20-Poly1305 (draft-irtf-cfrg-xchacha) | 256-bit key, 24-byte random nonce, 16-byte tag. Rationale in [architecture.md](architecture.md#why-xchacha20-poly1305-rather-than-aes-gcm). The header records the algorithm identifier so the choice is explicit in every file. |
| Vault key | 256-bit random key | Generated once at vault creation. |
| Randomness | OS CSPRNG | Used for the vault key, vault, record and attachment identifiers, salts, nonces and the recovery code. |

No custom primitives are used. Crate choices and their justification are documented in [architecture.md](architecture.md#dependencies).

A **sealed message** is the output of one XChaCha20-Poly1305 encryption with a fresh random nonce, stored as `nonce (24) || ciphertext || tag (16)`. It is `len(plaintext) + 40` bytes long. `Seal(key, plaintext, ad)` and `Open(key, sealed, ad)` denote encryption and verified decryption.

## 4. File layout

A vault is one file, the *vault file*, plus a directory of attachment blobs next to it (§9.1). The vault file has the following sections, in order and with no gaps:

```text
offset              size              section
0                   32                header             plaintext (§5)
32                  202               key slot table     2 × 101-byte key slot (§6)
234                 4                 record_count       u32
238                 variable          records            record_count × record (§7)
len(file) − 40      40                integrity seal     (§8)
```

A file MUST NOT contain any byte after the integrity seal. Writers MUST NOT produce files larger than 256 MiB, and readers MUST reject them (after checking the header, §12).

Writes MUST be atomic: a new version of the file is written to a temporary file in the same directory, flushed to disk, and then renamed over the old one. NovaVault also syncs the directory after the rename on Unix, so the rename itself survives a crash. On systems with POSIX permissions the file SHOULD be readable and writable by its owner only (NovaVault creates it with mode `0600`).

The recommended file extension is `.nvault`.

## 5. Header

| Offset | Field | Size | Description |
| ------ | ----- | ---- | ----------- |
| 0 | `magic` | 8 | `4E 4F 56 41 56 4C 54 00` (`"NOVAVLT\0"`). |
| 8 | `format_version` | 2 (u16) | `1` for this version. Readers also accept `0` and migrate the file (§11.1). |
| 10 | `aead_id` | 1 | `0x01` = XChaCha20-Poly1305. Readers MUST reject any other value. |
| 11 | `flags` | 1 | Reserved. MUST be zero. |
| 12 | `slot_count` | 1 | Number of key slots. MUST be `2`. |
| 13 | `reserved` | 3 | MUST be zero. |
| 16 | `vault_id` | 16 | Random identifier generated at creation. |

The header is stored in plaintext so a reader can select the right algorithms. It is authenticated by being part of the associated data of every record (§7.3), every attachment chunk (through `vault_id`, §9.3) and the integrity seal (§8), and, with `format_version` zeroed, of every key slot (§6.3). The header changes only when a file is migrated to a newer version (§11.1), which re-encrypts every record.

`header` below always denotes these 32 bytes.

## 6. Key slots

### 6.1 Purpose

The vault key is never stored in the clear. It is stored wrapped (encrypted) once per key slot. There are exactly two slot types:

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
slot_header  = header with format_version (bytes 8–9) set to zero
wrapping_key = Argon2id(secret, kdf_salt, memory, iterations, parallelism, out_len = 32)
nonce        = random(24)
wrapped_key  = XChaCha20-Poly1305_Encrypt(wrapping_key, nonce, vault_key, ad = slot_header || slot_metadata)
```

Because `slot_header` and `slot_metadata` have fixed lengths, the concatenation is unambiguous. Binding both into the associated data means changing a slot's type, salt or KDF parameters, or moving a slot to another vault, makes unwrapping fail.

The version is left out of the slots' associated data on purpose. A slot can only be rewritten with its secret, and a migration (§11.1) has at most one of the two secrets, so a version change must not invalidate either slot. For a version 0 file, `slot_header` equals `header`, so slots written by version 0 remain valid unchanged. The version is still authenticated: it is part of every record's associated data and of the integrity seal, which readers verify right after unwrapping (§12), so a changed version field is rejected.

Unwrapping performs the same derivation and `XChaCha20-Poly1305_Decrypt`. An authentication failure MUST be reported as a single generic error that does not distinguish a wrong secret, a wrong header or a corrupted slot.

The `secret` input is:

- **`password` slot:** the UTF-8 bytes of the master password, exactly as entered. No Unicode normalization and no trimming is applied, so a password typed with a different Unicode composition (for example a precomposed `é` instead of `e` plus a combining accent) does not match. Introducing normalization would change the key derivation input, so it would require a new format version (§11). NovaVault refuses an empty master password; that is an application check, not a format rule.
- **`recovery` slot:** the 32 ASCII bytes of the recovery code in canonical form (§10): upper case, no separators, checksum symbols included.

### 6.4 Operations

- **Create:** generate a random 256-bit vault key, a `vault_id` and a new recovery code, then write a `password` slot and a `recovery` slot that both wrap the vault key.
- **Unlock:** unwrap the `password` slot with the master password, or the `recovery` slot with the recovery code. Both yield the same vault key.
- **Change the master password:** with the vault key from any unlock (including one with the recovery code, for a forgotten password), write a new `password` slot with a fresh salt and nonce. The vault key, the `recovery` slot, the records and the attachments are unchanged; only the integrity seal is recomputed.
- **Regenerate the recovery code:** generate a new code and write a new `recovery` slot with a fresh salt and nonce. The previous code no longer opens the vault once the new file is written. The `password` slot is unchanged.
- KDF parameters MAY be increased during either rotation.
- A new slot is fully computed before it replaces the old one, and the file is written atomically (§4), so a failed rotation leaves the vault openable with the previous secrets.

## 7. Records

### 7.1 Record types

Each record holds one item. There are three record types:

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

- Records are encrypted directly under the vault key. This is safe because the vault key is never used for anything else that could collide: key slots use Argon2id-derived wrapping keys, and every use of the vault key has its own context byte in the associated data (`0x01` for records, `0x02` for the seal, `0x03` for attachment chunks).
- A fresh random nonce is used for every write of a record, including when it is rewritten unchanged.
- The associated data binds a ciphertext to its vault (through `vault_id` in the header), its identifier and its type, so it cannot be moved to another record or vault, or reinterpreted as another type.

### 7.4 Payload

```text
payload    = created_at_ms (u64) || updated_at_ms (u64) || title (str) || fields (str…)
             || attachment_count (u32) || attachment… || padding
attachment = attachment_id (16) || size (u64) || added_at_ms (u64) || name (str)
```

- `created_at_ms` and `updated_at_ms` are milliseconds since the Unix epoch, UTC.
- `fields` are the type's fields in the order of §7.1.
- `attachment_count` is the number of files attached to the item, at most 1000, followed by one `attachment` entry per file in the order they were added. `attachment_id` names the file's blob (§9.1), `size` is its plaintext length in bytes and `added_at_ms` when it was attached. `name` is the original file name: 1 to 255 bytes, no `/` or `\`, no control characters (Unicode category Cc), and not `.` or `..`, so it can be offered as a file name on export without pointing outside the chosen directory. Attachment identifiers MUST be unique within the vault.
- In format version 0 the payload has no `attachment_count` and no attachments.
- `padding` is zero bytes such that `len(payload)` is a multiple of 64. It MUST be shorter than 64 bytes. The padding hides the exact length of short secrets: the ciphertext reveals only a 64-byte size bucket.
- `len(payload)` MUST NOT exceed 1 MiB (1 048 576 bytes).

Readers MUST reject a payload in which a length runs past the end, text is not valid UTF-8, `attachment_count` exceeds 1000, an attachment name breaks the rules above, an attachment identifier repeats, the padding is not all zero or is 64 bytes or longer, or the total length is not a multiple of 64.

## 8. Integrity seal

Per-record authentication does not detect a record that was deleted, duplicated, or replaced by an older valid version of the same record. The file therefore ends with a seal over everything before it:

```text
body = header || key slot table || record_count || records
ad   = header || 0x02 || (body without its first 32 bytes)
seal = Seal(vault_key, empty plaintext, ad)                    (40 bytes)
```

The seal is recomputed on every save. Readers MUST verify it right after unlocking the vault key and MUST NOT return any record from a file whose seal does not verify. Replacing the *whole* file with an older valid version (rollback) is not detected; see the [threat model](threat-model.md#residual-risks). The seal covers attachment metadata (it is inside the records) but not the blob files, which are authenticated chunk by chunk (§9.6).

## 9. Attachments

An attachment is a file of any size belonging to exactly one item. Its metadata (identifier, name, size, time added) is stored in the owning record's encrypted payload (§7.4), so none of it is visible in plaintext. Its contents are stored in a separate *blob* file, encrypted in chunks so that it can be written and read as a stream.

Blobs are kept out of the vault file because the vault file is read and rewritten whole on every save (§4) and is limited to 256 MiB.

### 9.1 Storage

- Blobs live in a directory named after the vault file with `.attachments` appended, in the same directory as the vault file. For `vault.nvault` this is `vault.nvault.attachments/`. Writers create it when the first file is attached; on systems with POSIX permissions it SHOULD be accessible to its owner only.
- Each blob is named `<attachment_id>.nvatt`, where `attachment_id` is written as 32 lower-case hexadecimal digits.
- Readers MUST ignore files in the directory that do not match this pattern.
- A backup of a vault MUST include the directory as well as the vault file. Moving or renaming the vault file without its directory loses the attachments' contents (their metadata remains).

### 9.2 Blob layout

```text
offset  size  field
     0     8  magic          4E 56 41 54 54 43 48 00 ("NVATTCH\0")
     8     2  blob_version   u16, 1
    10     1  aead_id        0x01 = XChaCha20-Poly1305
    11     5  reserved       MUST be zero
    16    16  vault_id       the vault's identifier (§5)
    32    16  attachment_id  the attachment's identifier (§7.4)
    48     …  chunks
```

`blob_header` below denotes these 48 bytes. `blob_version` is versioned independently of `format_version`.

### 9.3 Chunks

The file's plaintext is split into chunks of `CHUNK_LEN = 65536` bytes. Every chunk except the last holds exactly `CHUNK_LEN` bytes; the last holds between 0 and `CHUNK_LEN` bytes and is always present, so an empty file has exactly one, empty, chunk, and a file whose length is a multiple of `CHUNK_LEN` (other than 0) has no empty chunk at the end. Chunk `i` (counting from 0) is stored as a sealed message (§3):

```text
ad_i    = blob_header || 0x03 || i (u64) || is_final (u8: 0x01 for the last chunk, 0x00 otherwise)
chunk_i = Seal(vault_key, plaintext_i, ad_i)
```

The chunks follow the header with no gaps or framing: every chunk but the last is exactly `CHUNK_LEN + 40` bytes, and the last chunk ends the file. A blob of a `size`-byte file is therefore `48 + size + 40 × max(1, ⌈size / 65536⌉)` bytes long.

This is the STREAM construction (Hoang, Reyhanitabar, Rogaway and Vizár, 2015) applied with the vault's AEAD, using a fresh random 192-bit nonce per chunk instead of a counter-derived one, so no nonce state is ever kept:

- The chunk index in the associated data stops chunks from being reordered, dropped or duplicated.
- The final flag stops truncation: a reader treats a chunk as final exactly when the file ends after it, so a file cut after a full chunk makes that chunk fail authentication.
- `blob_header`, with `vault_id` and `attachment_id`, stops a blob from being moved to another attachment or vault, even with its header rewritten.

### 9.4 Reading a blob

To read attachment `a` of a record, a reader:

1. Opens `<a.attachment_id>.nvatt`. A missing blob is a corrupted attachment.
2. Reads 48 bytes and checks them in order: `magic` (otherwise: not a blob, corrupted), `blob_version` (otherwise: unsupported version), `aead_id` (otherwise: unsupported algorithm), zero `reserved`, then that `vault_id` and `attachment_id` equal the vault's and the record's. Any mismatch is a corrupted attachment.
3. Reads chunks of up to `CHUNK_LEN + 40` bytes. A chunk is final if it is shorter than that or if the file ends right after it. A chunk shorter than 40 bytes is corrupted. Each chunk is opened with `ad_i` built from its position and finality before any of its plaintext is released.
4. Stops after the final chunk and checks that the total plaintext length equals `a.size`. NovaVault also stops as soon as the running total exceeds `a.size`.

Any failure is fatal for that attachment and leaves the rest of the vault readable. Because a damaged chunk is only found when it is reached, a reader that writes the plaintext to a file MUST write it to a temporary file and delete that file on failure, so no partial, partly unauthenticated output remains; NovaVault renames the temporary file to the user's chosen name only after step 4 succeeds.

### 9.5 Lifecycle

- **Add.** A new random `attachment_id` is generated. The blob is written in full to a temporary file in the attachments directory, flushed and renamed to its final name; only then is the record updated to list it. The record change is saved like any other edit.
- **Immutability.** A blob is never rewritten. Each `attachment_id` is used for exactly one blob, so there is never an older valid version of a blob to roll back to.
- **Remove.** The attachment is removed from its record (or the whole record is removed), and the vault file is saved. Only after that save succeeds is the blob deleted, so the saved vault file never refers to a missing blob because of a removal.
- **Clean-up.** An interrupted add or a save that never happened can leave a blob that no record lists. When a vault is opened, writers MAY delete blob files that no record refers to, and temporary files named `.<blob name>.<16 hex digits>.tmp`. NovaVault does so after any migration (§11.1) and after checking that attachment identifiers are unique (§12, step 9). It deletes only regular files, skips names that are not valid UTF-8, and matches blob names case-insensitively in their hexadecimal part. Failures are ignored and retried on the next open.

### 9.6 Integrity

The vault file's integrity seal (§8) authenticates every attachment's metadata, but not the blob files. A blob is authenticated chunk by chunk and bound to its attachment by `blob_header`, and blobs are immutable (§9.5), so the remaining ways to tamper with blobs are deleting one or replacing it with garbage. Both are detected when the attachment is read (§9.4). The number of blobs, their sizes (from which each file's exact size follows, §9.3) and their file times are visible without the master password; see the [threat model](threat-model.md#residual-risks).

## 10. Recovery code

- **Alphabet:** the 32 symbols `23456789ABCDEFGHJKLMNPQRSTUVWXYZ`, where symbol `i` has value `i`. The look-alikes `0`, `1`, `O` and `I` are excluded.
- **Random part:** 30 symbols. Each symbol is the low 5 bits of a byte from the OS CSPRNG; since 256 is a multiple of 32 this is uniform, giving 150 bits of entropy.
- **Checksum:** with `v[0..30]` the values of the random symbols, `c = (Σ (i + 1) · v[i]) mod 1021`, appended as two symbols with values `c / 32` and `c mod 32`. Because 1021 is a prime larger than every weight and every symbol difference, the checksum detects every single-symbol substitution and every swap of two adjacent symbols in the random part, and every single-symbol substitution in the checksum. It is a typo check, not a security mechanism.
- **Canonical form:** the 32 symbols (random part then checksum), upper case, no separators. This is the Argon2id input (§6.3).
- **Display form:** the canonical form in 8 groups of 4 symbols separated by `-`, for example `ABCD-EFGH-JKLM-NPQR-STUV-WXYZ-2345-6789`.
- **Parsing:** ASCII letters are upper-cased and `-` and whitespace are removed. Readers MUST then reject input that contains a character outside the alphabet, does not have exactly 32 symbols, or fails the checksum, before running Argon2id. NovaVault checks in that order and reports each case with its own error.
- Shown once at creation and at each regeneration. The user MUST confirm they saved it before the operation completes (an application-level step). The vault never stores the code itself.

## 11. Versioning and compatibility

- Every file carries `format_version`. Readers MUST reject versions they do not support and MUST NOT attempt best-effort parsing. Because the version is checked right after the magic, a file written by a newer NovaVault is reported as an unsupported version, not as a corrupted file.
- Any change to the on-disk format requires a version bump, an update to this specification and a migration path that upgrades older vaults on unlock.
- Migrations rewrite the file atomically and SHOULD keep a backup of the previous file until the new one has been verified.
- The format version current at the NovaVault 1.0 release will be declared the first stable version. Until then, versions may change following these rules.

| Version | Change |
| ------- | ------ |
| 0 | Initial version: header, key slots, password, note and secret records, integrity seal. |
| 1 | Attachments: the record payload gains an attachment list (§7.4) and blobs are stored next to the vault file (§9). Key slots authenticate the header with `format_version` zeroed (§6.3). |

### 11.1 Migrating from version 0

A version 0 file is upgraded when it is unlocked, with either secret:

1. Parse, unlock and verify the seal as for any file (§12, steps 1–7). The key slots unwrap unchanged, because for version 0 `slot_header` equals `header`.
2. Decrypt every record with the version 0 header in its associated data and decode its version 0 payload.
3. Re-encrypt every record with the version 1 header (`format_version = 1`) in its associated data and a version 1 payload with `attachment_count = 0`, using fresh nonces.
4. Write a copy of the original file as `<file name>.v0.bak` next to it.
5. Write the version 1 file (same key slots, new records, new seal) atomically (§4).
6. Read the new file back and verify it: structure, seal, and a header and records identical to those written. Then delete the backup.

If step 5 fails, the original file is untouched and the backup is deleted. If step 6 fails, the backup is kept. The key slots are not changed, so the master password and the recovery code both keep working.

## 12. Validation rules for readers

A conforming reader MUST perform these steps in order and stop at the first failure:

1. Check `magic` (otherwise: not a vault file), then `format_version`, which must be 0 or 1 (otherwise: unsupported version).
2. Check the rest of the header, in this order: a header of 32 bytes, known `aead_id` (otherwise: unsupported algorithm), zero `flags`, `slot_count` of 2, zero `reserved`.
3. Reject files larger than 256 MiB.
4. Parse the key slot table: known slot types in the order `password`, `recovery`, and KDF parameters within the bounds of §3.
5. Parse the record framing: `record_count` and each `sealed_len` within bounds, known record types, strictly ascending `record_id`, and exactly 40 bytes left for the seal.
6. Derive the wrapping key and unwrap the vault key from the slot matching the user's secret, with `slot_header` in the associated data. Report any failure as a single generic error (§6.3).
7. Verify the integrity seal (§8).
8. For a version 0 file, migrate it (§11.1).
9. Decrypt and decode records (§7.3, §7.4). NovaVault decrypts every record once on open, to check that attachment identifiers are unique and to clean up unreferenced blobs (§9.5), and then again on demand.
10. Read attachment blobs on demand (§9.4).

Steps 1–5 need no key and run before the expensive key derivation. Throughout, a reader MUST check every length against the remaining input before allocating memory, and MUST treat any authentication failure as fatal and never return unauthenticated plaintext.

## 13. Test vectors

The following vectors are deterministic and checked by the core crate's unit tests (`header::tests::known_encoding`, `item::tests::known_note_encoding_v0`, `item::tests::known_note_encoding_v1`, `attachment::tests::known_header_encoding`). Values are hex.

**Header** with `vault_id = 000102030405060708090a0b0c0d0e0f`:

```text
4e4f5641564c5400 0100 01 00 02 000000 000102030405060708090a0b0c0d0e0f
magic            ver  aead flags slots reserved vault_id
```

Its `slot_header` (§6.3) is the same bytes with `0100` replaced by `0000`.

**Note payload, version 1**, with title `"t"`, body `"b"`, `created_at_ms = 1`, `updated_at_ms = 2` and one attachment (`attachment_id = aa × 16`, `size = 3`, `added_at_ms = 4`, `name = "a.txt"`), 128 bytes:

```text
0100000000000000 0200000000000000 01000000 74 01000000 62 01000000
created_at_ms    updated_at_ms    len      t  len      b  attachment_count
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa 0300000000000000 0400000000000000 05000000 612e747874 00 × 57
attachment_id                    size             added_at_ms      len      "a.txt"    padding
```

**Note payload, version 0**, with the same title, body and times (64 bytes):

```text
0100000000000000 0200000000000000 01000000 74 01000000 62 00 × 38
created_at_ms    updated_at_ms    len      t  len      b  padding
```

**Blob header** with `vault_id = 01 × 16` and `attachment_id = 02 × 16`:

```text
4e56415454434800 0100 01 0000000000 01010101010101010101010101010101 02020202020202020202020202020202
magic            ver  aead reserved vault_id                         attachment_id
```

**Recovery code:** a random part of 30 × `3` (value 1) has checksum `465 = 14 · 32 + 17`, so the code is `3333-3333-3333-3333-3333-3333-3333-33GK`.

Key derivation and AEAD vectors (RFC 9106 Argon2id, draft-irtf-cfrg-xchacha XChaCha20-Poly1305) are listed in [architecture.md](architecture.md#test-vectors). Sealed values use random nonces, so complete-file and complete-blob vectors are not deterministic; the core's tests instead check round trips, every truncation, and tampering with every field of every section. For blobs they cover files of 0, 1, `CHUNK_LEN − 1`, `CHUNK_LEN`, `CHUNK_LEN + 1` and several chunks, a 32 MiB file streamed through bounded buffers, truncation at every chunk boundary and inside chunks, flipped bytes in every region of every chunk, reordered, dropped, duplicated and appended chunks, blobs moved between attachments and vaults, and size mismatches.

## Changelog of this specification

| Version | Date | Change |
| ------- | ---- | ------ |
| 0 (draft) | 2026-11-03 | Initial skeleton. |
| 0 (draft) | 2026-11-03 | Key slot encoding, wrapping associated data, slot operations and recovery code format specified. |
| 0 | 2026-11-03 | Header, file layout, record types, record encoding and encryption, payload encoding and padding, integrity seal, reader validation order and test vectors specified. Attachments moved out of version 0. |
| 1 | 2026-11-05 | Encrypted file attachments: attachment list in the record payload, blob files with chunked authenticated encryption, storage, lifecycle and reader rules (§9). Key slots authenticate the header with the version zeroed. Migration from version 0 (§11.1). New test vectors. |
| 1 | 2026-11-05 | Aligned with the implementation, no format change: master passwords are not Unicode-normalized (§6.3), KDF bounds in KiB, header check order (§12), size check after the header (§4), blob size and clean-up details (§9.4, §9.5), recovery code check order (§10), migration read-back (§11.1). |
