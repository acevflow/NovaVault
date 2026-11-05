# [`crates/novavault-core/src/item.rs`](../../../../../crates/novavault-core/src/item.rs)

## Purpose

Defines the vault's item model (passwords, notes and secrets, with their identifiers and attachment metadata) and the strict, padded binary encoding of an item's payload. Only an item's `ItemId` and `ItemKind` are stored in plaintext. The title, timestamps, fields and attachment list form the payload, which [`vault`](vault.rs.md) encrypts as a whole. The module also defines how item data is wiped from memory and how `Debug` redacts it.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `ITEM_ID_LEN` | constant | `pub const ITEM_ID_LEN: usize = 16;` | Length of an `ItemId`. |
| `ATTACHMENT_ID_LEN` | constant | `pub const ATTACHMENT_ID_LEN: usize = 16;` | Length of an `AttachmentId`. |
| `MAX_ATTACHMENTS_PER_ITEM` | constant | `pub const MAX_ATTACHMENTS_PER_ITEM: usize = 1000;` | Attachment limit per item. |
| `MAX_ATTACHMENT_NAME_LEN` | constant | `pub const MAX_ATTACHMENT_NAME_LEN: usize = 255;` | Longest attachment name in UTF-8 bytes. |
| `PAD_BLOCK` | constant | `pub const PAD_BLOCK: usize = 64;` | Payload padding block. |
| `MAX_PAYLOAD_LEN` | constant | `pub const MAX_PAYLOAD_LEN: usize = 1024 * 1024;` | Largest padded payload (1 MiB). |
| `ItemId` | struct | `pub struct ItemId([u8; ITEM_ID_LEN]);` | Random item identifier. `Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash`, `Display`, `Debug`. |
| `AttachmentId` | struct | `pub struct AttachmentId([u8; ATTACHMENT_ID_LEN]);` | Random attachment identifier. Same derives. |
| `ItemId::generate` / `AttachmentId::generate` | function | `pub fn generate() -> Result<Self>` | New random identifier. |
| `ItemId::from_bytes` / `AttachmentId::from_bytes` | function | `pub const fn from_bytes(bytes: [u8; $len]) -> Self` | From bytes. |
| `ItemId::as_bytes` / `AttachmentId::as_bytes` | function | `pub const fn as_bytes(&self) -> &[u8; $len]` | The bytes. |
| `ItemId::from_hex` / `AttachmentId::from_hex` | function | `pub fn from_hex(text: &str) -> Option<Self>` | Strict 32-digit hexadecimal parse. |
| `ItemKind` | enum | `pub enum ItemKind { Password, Note, Secret }` | Item type, stored as the record type. `Debug, Clone, Copy, PartialEq, Eq`. |
| `ItemKind::id` | function | `pub const fn id(self) -> u8` | On-disk record type (`1`, `2`, `3`). |
| `ItemKind::from_id` | function | `pub const fn from_id(id: u8) -> VaultResult<Self>` | Parses a record type. |
| `PasswordItem` | struct | `pub struct PasswordItem { pub title: String, pub username: String, pub password: SecretString, pub url: String, pub notes: SecretString }` | Login fields. `Zeroize, ZeroizeOnDrop`. |
| `NoteItem` | struct | `pub struct NoteItem { pub title: String, pub body: SecretString }` | Note fields. `Zeroize, ZeroizeOnDrop`. |
| `SecretItem` | struct | `pub struct SecretItem { pub title: String, pub value: SecretString, pub notes: SecretString }` | Secret-value fields. `Zeroize, ZeroizeOnDrop`. |
| `ItemContent` | enum | `pub enum ItemContent { Password(PasswordItem), Note(NoteItem), Secret(SecretItem) }` | User-editable content of an item. |
| `ItemContent::kind` | function | `pub const fn kind(&self) -> ItemKind` | The item's type. |
| `ItemContent::title` | function | `pub fn title(&self) -> &str` | The item's title. |
| `Attachment` | struct | `pub struct Attachment { id: AttachmentId, name: String, size: u64, added_at_ms: u64 }` | Metadata of one attached file. `Clone, Zeroize, ZeroizeOnDrop`. |
| `Attachment::new` | function (crate) | `pub(crate) const fn new(id: AttachmentId, name: String, size: u64, added_at_ms: u64) -> Self` | Assembles metadata. |
| `Attachment::id` | function | `pub const fn id(&self) -> AttachmentId` | Identifier. |
| `Attachment::name` | function | `pub fn name(&self) -> &str` | Original file name. |
| `Attachment::size` | function | `pub const fn size(&self) -> u64` | Plaintext size in bytes. |
| `Attachment::added_at_ms` | function | `pub const fn added_at_ms(&self) -> u64` | When it was attached (ms since the Unix epoch, UTC). |
| `validate_attachment_name` | function | `pub fn validate_attachment_name(name: &str) -> VaultResult<()>` | Checks an attachment file name. |
| `Item` | struct | `pub struct Item { id: ItemId, created_at_ms: u64, updated_at_ms: u64, content: ItemContent, attachments: Vec<Attachment> }` | A decrypted item. |
| `Item::new` | function (crate) | `pub(crate) const fn new(id: ItemId, created_at_ms: u64, updated_at_ms: u64, content: ItemContent, attachments: Vec<Attachment>) -> Self` | Assembles an item. |
| `Item::attachments` | function | `pub fn attachments(&self) -> &[Attachment]` | Attachments in the order they were added. |
| `Item::into_parts` | function (crate) | `pub(crate) fn into_parts(self) -> (ItemContent, Vec<Attachment>)` | Splits into content and attachments. |
| `Item::id` | function | `pub const fn id(&self) -> ItemId` | Identifier. |
| `Item::created_at_ms` | function | `pub const fn created_at_ms(&self) -> u64` | Creation time. |
| `Item::updated_at_ms` | function | `pub const fn updated_at_ms(&self) -> u64` | Last update time. |
| `Item::content` | function | `pub const fn content(&self) -> &ItemContent` | Borrowed content. |
| `Item::into_content` | function | `pub fn into_content(self) -> ItemContent` | Takes the content. |
| `ItemSummary` | struct | `pub struct ItemSummary { pub id: ItemId, pub kind: ItemKind, pub title: String, pub created_at_ms: u64, pub updated_at_ms: u64 }` | The non-secret-field view of an item, for listing. `Zeroize, ZeroizeOnDrop`. |
| `impl From<Item> for ItemSummary` | trait impl | `fn from(item: Item) -> Self` | Keeps the identifier, kind, title and times. |
| `Payload` | struct (crate) | `pub(crate) struct Payload { pub(crate) created_at_ms: u64, pub(crate) updated_at_ms: u64, pub(crate) content: ItemContent, pub(crate) attachments: Vec<Attachment> }` | A decoded payload. |
| `encode_payload` | function (crate) | `pub(crate) fn encode_payload(version: u16, created_at_ms: u64, updated_at_ms: u64, content: &ItemContent, attachments: &[Attachment]) -> VaultResult<Zeroizing<Vec<u8>>>` | Encodes and pads a payload. |
| `decode_payload` | function (crate) | `pub(crate) fn decode_payload(version: u16, kind: ItemKind, payload: &[u8]) -> VaultResult<Payload>` | Strictly decodes a payload. |

The crate root re-exports `Attachment`, `AttachmentId`, `Item`, `ItemContent`, `ItemId`, `ItemKind`, `ItemSummary`, `NoteItem`, `PasswordItem` and `SecretItem`. The constants and `validate_attachment_name` are reachable through `novavault_core::item`.

## Details

### Payload encoding

```text
created_at_ms u64 ‖ updated_at_ms u64 ‖ title str ‖ fields…
    ‖ attachment_count u32 ‖ attachment… ‖ zero padding

password:   username str ‖ password str ‖ url str ‖ notes str
note:       body str
secret:     value str ‖ notes str
attachment: attachment_id (16) ‖ size u64 ‖ added_at_ms u64 ‖ name str
```

Integers are little-endian. A `str` is a `u32` byte length followed by that many UTF-8 bytes.

- **Format versioning**: format version 0 payloads have no `attachment_count` and no attachments. `version >= 1` selects the attachment section in both directions.

### Constants

The limits above. `PAD_BLOCK` and `MAX_PAYLOAD_LEN` are also used by [`format`](format.rs.md) to bound `sealed_len`.

### `ItemId` and `AttachmentId`

Both are generated by the private `random_id!` macro and are identical except for their names. They are random 16-byte values, unique within a vault, and not secret. An `AttachmentId` also names the attachment's blob file on disk. `Display` prints 32 lower-case hexadecimal digits, and `Debug` prints `ItemId(<hex>)` or `AttachmentId(<hex>)`. `Ord` compares as unsigned byte strings, which is the record order the format requires.

- **`generate`**:
  - **Errors**: `CryptoError::Random` if the OS random number generator fails.
- **`from_hex`**: accepts exactly 32 ASCII hexadecimal digits in either case. It returns `None` for any other length, any sign or whitespace, or multi-byte UTF-8 (the length is checked in bytes). The shell uses it to parse identifiers sent by the UI.

### `ItemKind`

`Password` is `1`, `Note` is `2` and `Secret` is `3`.

- **`from_id`**:
  - **Errors**: `VaultError::Corrupted { reason: "unknown record type" }` for any other value.

### `PasswordItem`, `NoteItem`, `SecretItem`

Public-field structs.

- **Security**:
  - **Key material and secrets**: the fields that usually hold secrets (`password`, `notes`, `body`, `value`) are `SecretString`, which wipes itself on drop and is skipped by the derived `Zeroize`. The other strings (`title`, `username`, `url`) are wiped by the derived `ZeroizeOnDrop`.
  - **Debug**: `Debug` is hand-written with `finish_non_exhaustive()` and prints no field at all, because titles, user names and URLs can be as revealing as the secrets themselves.

### `ItemContent`, `kind`, `title`

The three item kinds. `kind` maps each variant to its `ItemKind`, and `title` borrows the title. `Debug` delegates to the redacted inner `Debug`.

### `Attachment`

Fields are private. Instances are built only by the crate: by [`vault`](vault.rs.md) while encrypting a new attachment, or by `decode_payload`.

- **Security**: the name and size live inside the item's encrypted payload, so they are never stored in plaintext. The name is wiped on drop (`ZeroizeOnDrop`); the identifier is skipped because it is not secret. `Debug` shows only the identifier.

### `Attachment::new`, `id`, `name`, `size`, `added_at_ms`

The crate-internal constructor and public getters.

### `validate_attachment_name`

A name is valid when it:

- is 1 to `MAX_ATTACHMENT_NAME_LEN` (255) bytes long;
- is not `.` or `..`;
- contains no `/`, no `\` and no control character (`char::is_control`, Unicode category Cc).

- **Errors**:
  - `VaultError::InvalidItem { reason: "invalid attachment name" }`: any rule is broken.
- **Security**: the name is offered again as a file name when the attachment is exported, so it must not be able to point outside the directory the user picks.

### `Item`

A decrypted item. It has no public constructor: identifiers and timestamps are assigned by [`vault`](vault.rs.md).

- **Security**: `Item` itself has no `Zeroize` impl, but each of its parts wipes itself on drop: the content structs, the `Attachment`s, and the `ItemId`, which is not secret. `Debug` prints the identifier, the kind and the number of attachments only.

### `Item::attachments`, `id`, `created_at_ms`, `updated_at_ms`, `content`, `into_content`

Getters. The timestamps are milliseconds since the Unix epoch, UTC. `into_content` hands the content back to the caller, for example to edit it and store it again.

### `Item::new` and `Item::into_parts`

Crate-internal. `into_parts` lets [`vault`](vault.rs.md) re-encrypt an item with changed content or attachments while keeping the rest.

### `ItemSummary` and `From<Item>`

The identifier, kind, title and timestamps, without any type-specific field, for listing the vault. `From<Item>` copies the title and drops the item, which wipes its secret fields at once.

- **Security**: `title` is wiped on drop. `Debug` prints only the identifier and the kind.

### `Payload`

Crate-internal result of `decode_payload`. In format version 0, `attachments` is always empty.

### `encode_payload`

1. Rejects more than 1000 attachments, and any attachment when `version` is 0.
2. Computes the exact unpadded length with checked arithmetic. Each string length must fit in a `u32`, and each attachment name must pass `validate_attachment_name`.
3. Rounds the length up to a multiple of `PAD_BLOCK` and rejects anything above `MAX_PAYLOAD_LEN`.
4. Allocates a `Zeroizing<Vec<u8>>` with exactly that capacity and writes the timestamps, the length-prefixed fields and (for `version >= 1`) the attachment section.
5. Pads with zeros to the final length.

- **Parameters**:
  - `version`: the vault's format version, which decides whether the attachment section is written.
  - Timestamps, content and attachments to encode.
- **Returns**: the padded plaintext payload, in a buffer the caller encrypts and then drops.
- **Errors**:
  - `VaultError::InvalidItem { reason: "item is too large" }`: the padded payload would exceed 1 MiB, or a length overflows.
  - `VaultError::InvalidItem { reason: "too many attachments" }`: more than 1000 attachments, or any attachment with `version` 0.
  - `VaultError::InvalidItem { reason: "invalid attachment name" }`: an attachment name is invalid.
- **Security**:
  - **Zeroization**: the buffer is zeroizing and sized exactly up front, so it never reallocates and never leaves unwiped plaintext copies in freed memory.
  - **Padding**: the payload length is padded to a 64-byte bucket, so the ciphertext length reveals only a coarse size, not the exact length of a password or note.

### `decode_payload`

Reads the timestamps, the title and the type-specific fields for `kind`, then (for `version >= 1`) the attachment section, and finally checks the padding.

- **Parameters**:
  - `version`: the format version the record was written with.
  - `kind`: the record's plaintext type. It is safe to trust, because it is part of the record's authenticated data.
  - `payload`: the decrypted bytes.
- **Returns**: the decoded `Payload`.
- **Errors**:
  - `VaultError::Corrupted { reason: "malformed record payload" }`: a truncated field, a length past the end, invalid UTF-8, more than 1000 attachments, an invalid attachment name, an attachment identifier repeated within the item, or invalid padding (total length not a multiple of 64, 64 or more padding bytes, or non-zero padding).
- **Security**:
  - The payload has already been authenticated, so a decoding failure points to a writer bug, not an attack, and a single reason is enough.
  - Each string is validated in place and copied exactly once into an exactly sized `String` or `SecretString`, so no stray plaintext copy is left behind, even on error.
  - The attachment vector is pre-allocated only after its count has been bounded.

### `fields` and `Reader` (private)

`fields` lists the string fields in payload order and is used by `encode_payload`. `Reader` is a bounds-checked cursor (`take`, `array`, `u64`, `u32`, `attachments`, `str`, `string`, `secret`, `padding`) that maps every problem to the single `MALFORMED` error.

## Dependencies

- **Internal**: [`error`](error.rs.md): `CryptoError`, `Result`, `VaultError`, `VaultResult`.
- **External**:
  - `secrecy`: `SecretString` for secret fields, `ExposeSecret` to encode them.
  - `zeroize`: `Zeroize`, `ZeroizeOnDrop` derives and the `Zeroizing` payload buffer.
  - `getrandom`: identifiers.

## Used by

- [`format`](format.rs.md): `ItemId`, `ItemKind`, `ITEM_ID_LEN` and `MAX_PAYLOAD_LEN` for record framing.
- [`attachment`](attachment.rs.md): `AttachmentId` and `ATTACHMENT_ID_LEN` in the blob header.
- [`vault`](vault.rs.md): the whole model, plus `encode_payload`, `decode_payload`, `validate_attachment_name` and `MAX_ATTACHMENTS_PER_ITEM`.
- [`lib.rs`](lib.rs.md): re-exports.
- [`src-tauri/src/dto.rs`](../../../../../src-tauri/src/dto.rs): converts items to and from the UI's types, and parses identifiers with `from_hex`.

## Tests

- [`known_note_encoding_v0`](../../../../../crates/novavault-core/src/item.rs#L806): version 0 payload of a note matches the vector in [vault-format.md §13](../../../../vault-format.md#13-test-vectors).
- [`known_note_encoding_v1`](../../../../../crates/novavault-core/src/item.rs#L820): version 1 payload with one attachment matches the published vector (128 bytes).
- [`round_trips_every_kind`](../../../../../crates/novavault-core/src/item.rs#L849): all three kinds round-trip in versions 0 and 1, including `u64::MAX` timestamps.
- [`attachments_round_trip_in_order`](../../../../../crates/novavault-core/src/item.rs#L865): attachments keep their order and metadata.
- [`version_zero_cannot_hold_attachments`](../../../../../crates/novavault-core/src/item.rs#L883): encoding attachments for version 0 fails.
- [`attachment_limits_are_enforced`](../../../../../crates/novavault-core/src/item.rs#L891): 1001 attachments fail, 1000 succeed, a name with `/` fails.
- [`attachment_names_are_validated`](../../../../../crates/novavault-core/src/item.rs#L920): accepted and rejected name examples, including 255 and 256 bytes.
- [`rejects_malformed_attachment_sections`](../../../../../crates/novavault-core/src/item.rs#L945): too many, a duplicate identifier, a bad name and an empty name give `Corrupted`.
- [`padding_hides_exact_length`](../../../../../crates/novavault-core/src/item.rs#L979): two notes of different lengths in one bucket encode to the same size.
- [`exact_block_needs_no_padding`](../../../../../crates/novavault-core/src/item.rs#L992): a payload of exactly 64 bytes gets no padding and decodes.
- [`rejects_oversized_items`](../../../../../crates/novavault-core/src/item.rs#L1004): a 1 MiB body fails with `InvalidItem`.
- [`rejects_malformed_payloads`](../../../../../crates/novavault-core/src/item.rs#L1023): every truncation, an extra padding block, non-zero padding, an overlong length and invalid UTF-8 give `Corrupted`.
- [`kind_ids_are_strict`](../../../../../crates/novavault-core/src/item.rs#L1052): identifiers round-trip; 0, 4 and 255 are rejected.
- [`ids_are_random_and_hex_formatted`](../../../../../crates/novavault-core/src/item.rs#L1065): identifiers differ and format as hexadecimal in `Display` and `Debug`.
- [`ids_parse_from_strict_hex`](../../../../../crates/novavault-core/src/item.rs#L1082): `from_hex` accepts either case and rejects bad lengths, non-hex characters, signs, spaces and multi-byte UTF-8.
- [`debug_shows_no_field_content`](../../../../../crates/novavault-core/src/item.rs#L1107): `Debug` of an item, its content, its attachments and its summary contains no field content.

## Notes

- **Misleading error reason.** For a version 0 payload with attachments, `encode_payload` returns `InvalidItem { reason: "too many attachments" }`. The doc comment describes this case separately ("attachments are given for format version 0"), but the reason string is the one for the 1000-attachment limit. [`vault`](vault.rs.md) never triggers it, because it migrates version 0 vaults on open.
- **Windows-unsafe names are accepted.** `validate_attachment_name` rejects `/`, `\` and control characters, but accepts names that are unsafe on Windows: `:` (for example `C:report.pdf`, a drive-relative path, or `a:b`, an alternate data stream), reserved device names such as `CON` or `NUL`, and names ending in a dot or a space. The doc comment says the name "must never be able to point outside the directory the user picks". On Windows, `Path::join` with a name such as `C:x` replaces the directory with a drive-relative path. The core never joins an attachment name onto a path, and [`vault`](vault.rs.md) exports to a destination path chosen by the caller, so the risk depends on how the shell builds the suggested file name. That remains an open question. [vault-format.md §7.4](../../../../vault-format.md#74-payload) states the same rules as the code.
- **Item kind can change on update.** `ItemContent` may switch kind through `Vault::update`, and the record type then changes with it.
- **User edits can leave copies.** The public `String` fields can be edited in place by callers. A reallocation caused by the caller (for example `push_str` beyond capacity) can leave an unwiped copy in freed memory, which `ZeroizeOnDrop` cannot reach.
