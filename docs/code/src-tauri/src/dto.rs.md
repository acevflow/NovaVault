# [`src-tauri/src/dto.rs`](../../../../src-tauri/src/dto.rs)

## Purpose

Defines every data type exchanged with the UI over Tauri IPC: the command inputs (deserialized from the webview) and the outputs (serialized to it). It decides what crosses the Rust/UI boundary. Secret inputs land directly in `SecretString`. Listings and item details carry no secret value. The only outputs that expose a secret are `RevealedValueDto` (one field, on request) and `RecoveryCodeDto` (shown once). Field names are camelCase on the wire, matching `ui/src/api/types.ts`.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `ItemInput` | enum | `pub enum ItemInput` | New item content, tagged by `kind`. `Deserialize` only. |
| `impl From<ItemInput> for ItemContent` | trait impl | `fn from(input: ItemInput) -> Self` | Moves the fields into the core item types. |
| `ItemUpdate` | enum | `pub enum ItemUpdate` | New content for an existing item; secret fields optional. `Deserialize` only. |
| `ItemUpdate::apply_to` | method | `pub fn apply_to(self, mut content: ItemContent) -> CommandResult<ItemContent>` | Merges the update into the stored content. |
| `ItemField` | enum | `pub enum ItemField` | Names one field, for reveal and copy. `Debug, Clone, Copy, PartialEq, Eq, Deserialize`. |
| `ItemField::is_secret` | method | `pub const fn is_secret(self) -> bool` | `false` only for `Username` and `Url`. |
| `ItemField::value_in` | method | `pub fn value_in(self, content: &ItemContent) -> CommandResult<&str>` | The field's text in `content`. |
| `ItemDetailsDto` | struct | `pub struct ItemDetailsDto` | One item for its details screen, without secret values. `Serialize`; custom `Drop`. |
| `ItemDetailFields` | enum | `pub enum ItemDetailFields` | Kind-specific part of `ItemDetailsDto`, tagged by `kind`. `Serialize`. |
| `impl From<&Item> for ItemDetailsDto` | trait impl | `fn from(item: &Item) -> Self` | Builds the DTO from a decrypted item. |
| `AttachmentDto` | struct | `pub struct AttachmentDto` | Attachment identifier, name, size and time added. `Serialize, Zeroize, ZeroizeOnDrop`. |
| `impl From<&Attachment> for AttachmentDto` | trait impl | `fn from(attachment: &Attachment) -> Self` | Copies the metadata. |
| `TransferProgressDto` | struct | `pub struct TransferProgressDto` | Bytes done and total. `Clone, Copy, Serialize`. |
| `RevealedValueDto` | struct | `pub struct RevealedValueDto(SecretString);` | One revealed secret value. Custom `Serialize`, no `Debug`. |
| `RevealedValueDto::new` | function | `pub const fn new(value: SecretString) -> Self` | Wraps a value. |
| `CopyResultDto` | struct | `pub struct CopyResultDto` | Seconds until the clipboard is cleared. `Serialize`. |
| `Credential` | enum | `pub enum Credential` | Master password or recovery code. `Deserialize` only. |
| `ItemSummaryDto` | struct | `pub struct ItemSummaryDto` | One list entry, without secret fields. `Serialize, Zeroize, ZeroizeOnDrop`. |
| `impl From<ItemSummary> for ItemSummaryDto` | trait impl | `fn from(mut summary: ItemSummary) -> Self` | Moves the title out of the core summary. |
| `VaultStatusDto` | struct | `pub struct VaultStatusDto` | Whether a vault exists and is unlocked. `Serialize`. |
| `RecoveryCodeDto` | struct | `pub struct RecoveryCodeDto(SecretString);` | A recovery code to show once. Custom `Serialize`, no `Debug`. |
| `impl From<&RecoveryCode> for RecoveryCodeDto` | trait impl | `fn from(code: &RecoveryCode) -> Self` | Stores the display form. |
| `parse_item_id` | function | `pub fn parse_item_id(text: &str) -> CommandResult<ItemId>` | Parses 32 hexadecimal digits. |
| `parse_attachment_id` | function | `pub fn parse_attachment_id(text: &str) -> CommandResult<AttachmentId>` | Parses 32 hexadecimal digits. |

## Details

### `ItemInput`

`#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]`. Wire shapes:

| `kind` | Fields (plain `String`) | Fields (`SecretString`) |
| ------ | ----------------------- | ----------------------- |
| `"password"` | `title`, `username`, `url` | `password`, `notes` |
| `"note"` | `title` | `body` |
| `"secret"` | `title` | `value`, `notes` |

Every field is required. A missing field or an unknown `kind` fails deserialization, so Tauri rejects the call before the handler runs.

- **Security**: secret fields are never held in a plain `String`. The type has no `Serialize` and no `Debug`.

### `impl From<ItemInput> for ItemContent`

Moves each field into `PasswordItem`, `NoteItem` or `SecretItem` without copying, so only one copy of each value exists. The core types wipe themselves on drop.

### `ItemUpdate`

Same tagging and the same plain fields as `ItemInput`. Every secret field is `Option<SecretString>` with `#[serde(default)]`, so `null` and an absent field both mean "keep the stored value". The edit form therefore never needs a secret the user did not reveal.

### `ItemUpdate::apply_to`

1. Matches the update's kind against `content`'s kind.
2. For each plain field, calls `replace_plain`: the old `String` is zeroized, then replaced.
3. For each secret field, calls `replace_secret`: if a value was given, it replaces the old one, which wipes itself on drop.
4. Returns the updated content.

- **Errors**: `CommandError::InvalidInput` if the kinds differ. Changing the kind would silently drop fields.
- **Security**: old plain values are wiped explicitly; old secret values are wiped by `SecretString`'s drop.

### `ItemField`

`#[serde(rename_all = "camelCase")]`: `"username"`, `"password"`, `"url"`, `"notes"`, `"body"`, `"value"`. Anything else (for example `"title"`) fails deserialization.

### `ItemField::is_secret`

`true` for `Password`, `Notes`, `Body` and `Value`. [`Session::reveal_field`](session.rs.md#reveal_field) uses it to refuse plain fields, which already come with `get_item`.

### `ItemField::value_in`

| Field | Valid for | Source |
| ----- | --------- | ------ |
| `Username` | password | `username` |
| `Password` | password | `password` (exposed) |
| `Url` | password | `url` |
| `Notes` | password, secret | `notes` (exposed) |
| `Body` | note | `body` (exposed) |
| `Value` | secret | `value` (exposed) |

- **Returns**: a borrow of the stored text. The caller copies it into zeroizing storage.
- **Errors**: `CommandError::InvalidInput` for any other combination.

### `ItemDetailsDto` and `ItemDetailFields`

`ItemDetailsDto` has `id`, `title`, `createdAtMs`, `updatedAtMs`, `attachments` and, through `#[serde(flatten)]`, the `ItemDetailFields` enum. That enum is tagged by `kind` and carries:

| `kind` | Fields |
| ------ | ------ |
| `"password"` | `username`, `url`, `hasPassword`, `hasNotes` |
| `"note"` | `hasBody` |
| `"secret"` | `hasValue`, `hasNotes` |

- **Security**: each secret field becomes only a "non-empty" flag. Its value never appears. On drop, `ItemDetailsDto` zeroizes `title` and, for a password item, `username` and `url`, because those can be revealing too. Attachments wipe their own names.

### `impl From<&Item> for ItemDetailsDto`

Builds the flags with `!expose_secret().is_empty()`. It clones `username`, `url` and `title`, formats `id` as hexadecimal, and maps each attachment through `AttachmentDto::from`, in the order stored.

### `AttachmentDto`

Fields: `id` (hexadecimal, `#[zeroize(skip)]`), `name`, `size` and `addedAtMs`. `name` is wiped on drop.

- **Security**: no path of any kind: not the source file, not the blob, not the destination.

### `TransferProgressDto`

`{ "done": u64, "total": u64 }` (no rename needed). Sent over a Tauri `Channel` by the attachment commands.

### `RevealedValueDto`

Serialized by hand as `{ "value": "<secret>" }`. It is the only place an item secret is exposed for serialization.

- **Security**: no `Deserialize`, so the UI cannot send it back. No `Debug`, so it cannot be printed by accident. The inner `SecretString` wipes itself when the DTO is dropped after serialization. The copy inside the webview's JavaScript heap is outside Rust's control.

### `CopyResultDto`

`{ "clearAfterSeconds": u32 }`. The copied value never comes back.

### `Credential`

`#[serde(tag = "method", content = "secret", rename_all = "camelCase")]`. Wire shapes:

- `{ "method": "password", "secret": "…" }`
- `{ "method": "recoveryCode", "secret": "…" }`

Both payloads are `SecretString`. A recovery code is accepted in any case and with or without separators; parsing happens later, in the core's `RecoveryCode::parse`.

### `ItemSummaryDto` and `impl From<ItemSummary> for ItemSummaryDto`

Fields: `id`, `kind` (`"password"`, `"note"` or `"secret"`, from the private `kind_name`), `title`, `createdAtMs` and `updatedAtMs`. The title is moved out of the core summary with `mem::take`, so only one copy exists. It is wiped when the DTO drops.

- **Security**: no secret field and no user name or URL.

### `VaultStatusDto`

`{ "exists": bool, "unlocked": bool }`.

### `RecoveryCodeDto` and `impl From<&RecoveryCode> for RecoveryCodeDto`

Holds `RecoveryCode::to_display()` (8 groups of 4 symbols) and serializes by hand as `{ "recoveryCode": "<code>" }`.

- **Security**: the one deliberate exposure of a vault credential to the UI, so the user can write it down. It is returned only by `create_vault` and `regenerate_recovery_code`. No `Deserialize`, no `Debug`.

### `parse_item_id` and `parse_attachment_id`

Delegate to the core's `ItemId::from_hex` or `AttachmentId::from_hex`.

- **Errors**: `CommandError::InvalidInput` unless the text is exactly 32 hexadecimal digits. Upper case is accepted; multi-byte UTF-8 of the right byte length is rejected (per the tests).

### Private helpers

- `replace_plain(field, value)`: zeroizes `field`, then assigns `value`.
- `replace_secret(field, value)`: assigns `value` if it is `Some`.
- `kind_name(kind)`: the wire name of an `ItemKind`.

## Dependencies

- **Internal**: [`error`](error.rs.md): `CommandError::InvalidInput` and `CommandResult`.
- **External**:
  - `novavault_core`: the item, attachment, identifier and recovery code types.
  - `secrecy`: `SecretString` and `ExposeSecret`, so secrets are redacted and wiped.
  - `serde`: the wire formats.
  - `zeroize`: wiping plain fields and DTOs on drop.

## Used by

- [`commands.rs`](commands.rs.md): argument and return types of the commands.
- [`session.rs`](session.rs.md): builds the output DTOs, parses identifiers and applies updates.

## Tests

- [`item_input_deserializes_every_kind`](../../../../src-tauri/src/dto.rs#L548): each of the three kinds deserializes and converts to the matching `ItemContent`.
- [`item_input_rejects_missing_fields_and_unknown_kinds`](../../../../src-tauri/src/dto.rs#L571): a note without `body`, and `kind: "file"`, are rejected.
- [`item_update_treats_null_or_missing_secrets_as_unchanged`](../../../../src-tauri/src/dto.rs#L577): `null` and absent secret fields become `None`; a given value becomes `Some`.
- [`item_fields_deserialize_from_camel_case`](../../../../src-tauri/src/dto.rs#L599): `"username"` is plain, `"value"` is secret, and `"title"` is not a field.
- [`credential_deserializes_both_methods`](../../../../src-tauri/src/dto.rs#L609): both `method` values deserialize with their secret.
- [`recovery_code_is_serialized_in_display_form`](../../../../src-tauri/src/dto.rs#L619): `recoveryCode` equals the core's display form.
- [`item_ids_round_trip_through_hex`](../../../../src-tauri/src/dto.rs#L629): identifiers round-trip in lower and upper case. Empty, short, non-hex, 33-digit, sign-prefixed and multi-byte inputs are rejected. Attachment identifiers behave the same way.
- [`transfer_progress_serializes_both_counts`](../../../../src-tauri/src/dto.rs#L656): the exact JSON of a progress message.

Session-level tests that check what these DTOs leak are listed in [`session.rs.md`](session.rs.md#tests).

## Notes

- The module doc says "No type here derives `Debug`", but `ItemField` derives `Debug`. It holds only a field name, never a value, so nothing secret can be printed. The statement is still inaccurate as written.
- `ItemDetailsDto` clones `username`, `url` and `title` from the decrypted `Item`, so two copies exist until both drop. Both are wiped on drop: the DTO by its `Drop` impl, the item by the core.
- `TransferProgressDto` has no `rename_all`. That makes no difference today because both field names are single words.
