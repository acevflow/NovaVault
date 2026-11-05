# [`crates/novavault-core/src/header.rs`](../../../../../crates/novavault-core/src/header.rs)

## Purpose

Encodes and parses the 32-byte plaintext header at the start of every vault file. The header carries the format version, the algorithm identifier and the random vault identifier. It is the root of format versioning, and it is bound into the associated data of every record, of the integrity seal and (with the version zeroed) of every key slot. Parsing is strict and ordered, so a file from a newer NovaVault is reported as an unsupported version rather than as corruption.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `MAGIC` | constant | `pub const MAGIC: [u8; 8] = *b"NOVAVLT\0";` | File signature. |
| `FORMAT_VERSION` | constant | `pub const FORMAT_VERSION: u16 = 1;` | Version written by this build. |
| `MIN_READ_VERSION` | constant | `pub const MIN_READ_VERSION: u16 = 0;` | Oldest version read (and migrated). |
| `HEADER_LEN` | constant | `pub const HEADER_LEN: usize = 32;` | Encoded header length. |
| `SLOT_COUNT` | constant | `pub const SLOT_COUNT: u8 = 2;` | Number of key slots. |
| `VAULT_ID_LEN` | constant | `pub const VAULT_ID_LEN: usize = 16;` | Length of a `VaultId`. |
| `AeadId` | enum | `pub enum AeadId { XChaCha20Poly1305 }` | Algorithm used for every ciphertext. `Debug, Clone, Copy, PartialEq, Eq`. |
| `AeadId::id` | function | `pub const fn id(self) -> u8` | On-disk identifier (`1`). |
| `AeadId::from_id` | function | `pub const fn from_id(id: u8) -> VaultResult<Self>` | Parses an identifier. |
| `VaultId` | struct | `pub struct VaultId([u8; VAULT_ID_LEN]);` | Random, non-secret vault identifier. `Clone, Copy, PartialEq, Eq`. |
| `VaultId::generate` | function | `pub fn generate() -> Result<Self>` | New random identifier. |
| `VaultId::from_bytes` | function | `pub const fn from_bytes(bytes: [u8; VAULT_ID_LEN]) -> Self` | From stored bytes. |
| `VaultId::as_bytes` | function | `pub const fn as_bytes(&self) -> &[u8; VAULT_ID_LEN]` | The bytes. |
| `Header` | struct | `pub struct Header { version: u16, vault_id: VaultId }` | Decoded header. `Debug, Clone, Copy, PartialEq, Eq`. |
| `Header::generate` | function | `pub fn generate() -> Result<Self>` | Header of a new vault. |
| `Header::new` | function | `pub const fn new(vault_id: VaultId) -> Self` | Header at `FORMAT_VERSION`. |
| `Header::with_version` | function (crate) | `pub(crate) const fn with_version(self, version: u16) -> Self` | Same header, another version. |
| `Header::version` | function | `pub const fn version(&self) -> u16` | Format version. |
| `Header::vault_id` | function | `pub const fn vault_id(&self) -> VaultId` | Vault identifier. |
| `Header::aead` | function | `pub const fn aead(&self) -> AeadId` | Algorithm (always XChaCha20-Poly1305). |
| `Header::to_bytes` | function | `pub fn to_bytes(&self) -> [u8; HEADER_LEN]` | 32-byte encoding. |
| `Header::slot_ad` | function | `pub fn slot_ad(&self) -> [u8; HEADER_LEN]` | Encoding with the version zeroed, for key slots. |
| `Header::parse` | function | `pub fn parse(file: &[u8]) -> VaultResult<Self>` | Strict, ordered parse. |

The crate root re-exports `AeadId`, `FORMAT_VERSION`, `HEADER_LEN`, `Header`, `MAGIC`, `MIN_READ_VERSION` and `VaultId`. It does not re-export `SLOT_COUNT` or `VAULT_ID_LEN`.

## Details

### Layout

```text
offset  size  field
     0     8  magic           "NOVAVLT\0"
     8     2  format_version  u16 LE, 1 (0 is read and migrated)
    10     1  aead_id         1 = XChaCha20-Poly1305
    11     1  flags           must be 0
    12     1  slot_count      must be 2
    13     3  reserved        must be 0
    16    16  vault_id        random
```

### `MAGIC`, `FORMAT_VERSION`, `MIN_READ_VERSION`

- **Format versioning**: this build writes version 1 and reads versions 0 and 1. Opening a version 0 file migrates it to version 1 (see [`vault`](vault.rs.md)). Any other version is `UnsupportedVersion`. The version is checked immediately after the magic, before any other field, because a newer format may lay out the rest differently.

### `HEADER_LEN`, `SLOT_COUNT`, `VAULT_ID_LEN`

The fixed sizes above. `SLOT_COUNT` is written into byte 12 and must be read back unchanged.

### `AeadId`

A single variant, `XChaCha20Poly1305`, with on-disk identifier `1`. It is the only algorithm in format versions 0 and 1.

### `AeadId::id` and `AeadId::from_id`

`id` returns `1`. `from_id(1)` returns `XChaCha20Poly1305`.

- **Errors**:
  - `VaultError::UnsupportedAlgorithm { id }`: any other identifier.

### `VaultId`

Sixteen random bytes that identify one vault. It is not secret.

- **Security**: it is bound into the associated data of every ciphertext (through the header and the attachment blob header), so ciphertexts cannot be moved from one vault to another. `Debug` prints it in hexadecimal.

### `VaultId::generate`, `from_bytes`, `as_bytes`

`generate` fills 16 bytes from `getrandom`. `from_bytes` and `as_bytes` are `const` conversions.

- **Errors** (`generate`):
  - `CryptoError::Random`: the OS random number generator failed.

### `Header`

Holds only the version and the vault identifier. Every other field is fixed. The version changes only when a file is migrated. Because the header is associated data for every record and for the seal, a migration must re-encrypt every record.

### `Header::generate` and `Header::new`

`generate` builds `new(VaultId::generate()?)`. `new` sets the version to `FORMAT_VERSION`.

- **Errors** (`generate`):
  - `CryptoError::Random`: the OS random number generator failed.

### `Header::with_version`

Crate-internal. Returns a copy with another version. [`vault`](vault.rs.md) uses it to migrate (old header to new header), and the tests use it to build version 0 files.

### `Header::version`, `vault_id`, `aead`

Getters. `aead` always returns `AeadId::XChaCha20Poly1305`, since `parse` rejects any other identifier and the supported versions define only that one.

### `Header::to_bytes`

Writes the magic, the little-endian version, `aead().id()`, zero flags, `SLOT_COUNT`, three zero reserved bytes and the vault identifier.

- **Returns**: the 32 bytes that records and the seal authenticate.

### `Header::slot_ad`

`to_bytes()` with bytes 8–9 (`format_version`) set to zero.

- **Security**:
  - **Format versioning and key slots**: a slot can only be rewritten by someone holding its secret, and a migration has at most one of the two secrets. Leaving the version out of the slots' associated data lets a migration change the version without rewrapping either slot.
  - The version stays authenticated, because it is part of every record's associated data and of the integrity seal, both of which are checked right after unlocking.
  - For a version 0 header, `slot_ad()` equals `to_bytes()`, so slots written by version 0 builds remain valid unchanged.

### `Header::parse`

Checks run in this order and stop at the first failure:

1. `file` must start with `MAGIC`, otherwise `NotAVault`. A file shorter than 8 bytes also fails here.
2. Bytes 8–9 must be present (otherwise `Corrupted { reason: "truncated header" }`) and the version must be within `MIN_READ_VERSION..=FORMAT_VERSION` (otherwise `UnsupportedVersion { found }`).
3. The file must hold all 32 header bytes, otherwise `Corrupted { reason: "truncated header" }`.
4. `aead_id` must be known, otherwise `UnsupportedAlgorithm { id }`.
5. `flags` must be 0, otherwise `Corrupted { reason: "unknown header flags" }`.
6. `slot_count` must be 2, otherwise `Corrupted { reason: "unexpected key slot count" }`.
7. The reserved bytes must be 0, otherwise `Corrupted { reason: "non-zero reserved header bytes" }`.

- **Parameters**: `file`, the whole file or any prefix of it. Bytes after the header are ignored.
- **Returns**: the header with the file's own version, which may be older than `FORMAT_VERSION`.
- **Security**: nothing here is authenticated yet. The header becomes trusted only once the integrity seal has been verified with the vault key (see [`format`](format.rs.md)).

## Dependencies

- **Internal**: [`error`](error.rs.md): `CryptoError`, `Result`, `VaultError`, `VaultResult`.
- **External**: `getrandom`: vault identifiers.

## Used by

- [`format`](format.rs.md): parses the header first, includes it in record and seal associated data, and uses `HEADER_LEN`.
- [`attachment`](attachment.rs.md): `AeadId` and `VaultId` in the blob header.
- [`vault`](vault.rs.md): `Header::generate`, `slot_ad`, `with_version`, `FORMAT_VERSION` (migration decision).
- [`item`](item.rs.md) (tests only): `FORMAT_VERSION`.
- [`lib.rs`](lib.rs.md): re-exports.

## Tests

- [`known_encoding`](../../../../../crates/novavault-core/src/header.rs#L292): the encoding of a sample header matches the hex published in [vault-format.md §13](../../../../vault-format.md#13-test-vectors).
- [`round_trips_with_trailing_data`](../../../../../crates/novavault-core/src/header.rs#L297): parsing ignores bytes after the header.
- [`reads_version_zero_for_migration`](../../../../../crates/novavault-core/src/header.rs#L304): version 0 parses and keeps version 0.
- [`slot_ad_ignores_only_the_version`](../../../../../crates/novavault-core/src/header.rs#L312): versions 0 and 1 share a `slot_ad`; a version 0 header is its own `slot_ad`; another vault identifier changes it.
- [`generated_ids_are_distinct`](../../../../../crates/novavault-core/src/header.rs#L324): two `VaultId`s differ.
- [`rejects_wrong_magic`](../../../../../crates/novavault-core/src/header.rs#L329): empty input, short input, a wrong signature and a ZIP header give `NotAVault`.
- [`rejects_newer_versions_before_anything_else`](../../../../../crates/novavault-core/src/header.rs#L339): versions 2, 256 and 65535 give `UnsupportedVersion`, even with an invalid algorithm byte or nothing after the version.
- [`rejects_truncation`](../../../../../crates/novavault-core/src/header.rs#L359): every length from 8 to 31 bytes gives `Corrupted`.
- [`rejects_unknown_algorithm`](../../../../../crates/novavault-core/src/header.rs#L373): identifiers 0, 2 and 255 give `UnsupportedAlgorithm`.
- [`rejects_flags_slot_count_and_reserved`](../../../../../crates/novavault-core/src/header.rs#L385): a flipped bit in flags, slot count or any reserved byte gives `Corrupted`.

## Notes

- The module documentation says the header "is authenticated by being part of the associated data of every record and of the file's integrity seal". That is accurate for the seal and records. Attachment chunks bind only `vault_id` (through their own blob header), not the whole vault header, as [vault-format.md §5](../../../../vault-format.md#5-header) also states.
- `SLOT_COUNT` and `VAULT_ID_LEN` are public but not re-exported at the crate root, unlike the other header constants.
