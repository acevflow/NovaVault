# [`ui/src/api/types.ts`](../../../../../ui/src/api/types.ts)

## Purpose

TypeScript types for the data exchanged with the Rust shell over Tauri IPC. They mirror [`src-tauri/src/dto.rs`](../../../src-tauri/src/dto.rs.md) and [`src-tauri/src/settings.rs`](../../../src-tauri/src/settings.rs.md), with camelCase field names as serde serializes them. The file contains only types; nothing exists at runtime.

## Public API

| Name                   | Kind      | Signature                                                                    | Description                                        |
| ---------------------- | --------- | ---------------------------------------------------------------------------- | -------------------------------------------------- |
| `VaultStatus`          | interface | `{ readonly exists: boolean; readonly unlocked: boolean }`                   | Whether a vault exists and is unlocked.            |
| `CredentialMethod`     | type      | `'password' \| 'recoveryCode'`                                               | Which secret a credential holds.                   |
| `Credential`           | interface | `{ readonly method: CredentialMethod; readonly secret: string }`             | Master password or recovery code.                  |
| `RecoveryCodeResponse` | interface | `{ readonly recoveryCode: string }`                                          | A recovery code shown once.                        |
| `Settings`             | interface | `{ readonly autoLockMinutes: number; readonly clipboardClearSeconds: number }` | App settings, not secret.                        |
| `ItemKind`             | type      | `'password' \| 'note' \| 'secret'`                                           | Item type.                                         |
| `ItemSummary`          | interface | `{ id; kind: ItemKind; title; createdAtMs; updatedAtMs }`                    | Item list entry, no secret fields.                 |
| `Attachment`           | interface | `{ id; name; size; addedAtMs }`                                              | Attachment metadata.                               |
| `TransferProgress`     | interface | `{ readonly done: number; readonly total: number }`                          | Attachment transfer progress.                      |
| `ItemDetails`          | type      | union over `kind` of `ItemDetailsBase & { … }`                               | Plain fields plus `has*` flags for secret fields.  |
| `ItemInput`            | type      | union over `kind`                                                            | Content of a new item.                             |
| `ItemUpdate`           | type      | union over `kind`, secret fields `string \| null`                            | New content for an existing item.                  |
| `ItemField`            | type      | `'username' \| 'password' \| 'url' \| 'notes' \| 'body' \| 'value'`          | Field that can be revealed or copied.              |
| `SecretItemField`      | type      | `Exclude<ItemField, 'username' \| 'url'>`                                    | Secret fields only.                                |
| `CopyResult`           | interface | `{ readonly clearAfterSeconds: number }`                                     | Result of a copy.                                  |
| `RevealedValue`        | interface | `{ readonly value: string }`                                                 | A revealed secret field.                           |

All properties are `readonly`.

## Details

### `VaultStatus`

- Mirrors `VaultStatusDto`. `exists`: a vault file exists at the app's vault location. `unlocked`: the vault is unlocked now.

### `Credential` and `CredentialMethod`

- Matches the Rust `Credential` enum, serialized with `tag = "method"`, `content = "secret"` and camelCase variant names, so the JSON is `{ "method": "password" | "recoveryCode", "secret": "…" }`.
- `secret` is the master password, or the recovery code in any letter case, with or without dashes.
- **Security**: the value is a secret. The UI sends it once and keeps it only in form state.

### `RecoveryCodeResponse`

- JSON of `RecoveryCodeDto`: `{ "recoveryCode": "XXXX-XXXX-…" }`, 8 groups of 4 characters.
- **Security**: one of only two secrets the shell sends to the UI, so the user can write it down.

### `Settings`

- Mirrors `Settings` in `settings.rs`. `autoLockMinutes`: minutes without activity before the vault locks. `clipboardClearSeconds`: seconds before a copied field is cleared.

### `ItemSummary` and `Attachment`

- Identifiers are 32 lower-case hexadecimal digits. Times are milliseconds since the Unix epoch (UTC). `Attachment.size` is in bytes; the file contents never reach the UI.

### `TransferProgress`

- `done` and `total` bytes, sent through the progress channel of `addAttachment` and `exportAttachment`.

### `ItemDetails`

- `ItemDetailsBase` (not exported) holds `id`, `title`, `createdAtMs`, `updatedAtMs` and `attachments` (in the order they were added).
- Per `kind`:
  - `password`: `username`, `url` (plain), `hasPassword`, `hasNotes`.
  - `note`: `hasBody`.
  - `secret`: `hasValue`, `hasNotes`.
- Mirrors `ItemDetailsDto`, whose kind-specific `fields` are flattened next to the common fields with a `kind` tag.
- **Security**: secret values are absent. Only whether each one is set is sent; values come one at a time from `revealItemField`.

### `ItemInput`

- `password`: `title`, `username`, `password`, `url`, `notes`. `note`: `title`, `body`. `secret`: `title`, `value`, `notes`.
- **Security**: `password`, `notes`, `body` and `value` are secret; they go to the shell once and are not kept.

### `ItemUpdate`

- The same shapes as `ItemInput`, except that each secret field is `string | null`.
- `null` keeps the stored value, so the edit form can save without ever reading it. An empty string clears the field. The kind must match the stored item's kind.

### `ItemField` and `SecretItemField`

- `ItemField` mirrors the Rust `ItemField` enum. `SecretItemField` removes the two plain fields, so `revealItemField` cannot be called for `username` or `url`.

### `CopyResult` and `RevealedValue`

- `CopyResult` carries only the clear delay; the copied value never comes back.
- `RevealedValue` carries one secret field value, sent only when the user asks to reveal it.

## Dependencies

- **Internal**: none.
- **External**: none.

## Used by

- [`commands.ts`](commands.ts.md): argument and return types of every wrapper.
- [`app/router.ts`](../app/router.ts.md), [`screens/items/kinds.ts`](../screens/items/kinds.ts.md), [`screens/items/filterItems.ts`](../screens/items/filterItems.ts.md), [`screens/settings/useCredential.ts`](../screens/settings/useCredential.ts.md).
- Components: [`CopyButton`](../components/CopyButton.tsx.md), [`SecretField`](../components/SecretField.tsx.md), [`ItemList`](../screens/items/ItemList.tsx.md), [`ItemDetails`](../screens/items/ItemDetails.tsx.md), [`ItemForm`](../screens/items/ItemForm.tsx.md), [`AttachmentList`](../screens/items/AttachmentList.tsx.md), [`ItemsScreen`](../screens/items/ItemsScreen.tsx.md), [`CurrentCredentialFields`](../screens/settings/CurrentCredentialFields.tsx.md), [`UnlockScreen`](../screens/unlock/UnlockScreen.tsx.md).
- Tests: [`filterItems.test.ts`](../screens/items/filterItems.test.ts.md), [`AttachmentList.test.tsx`](../screens/items/AttachmentList.test.tsx.md), [`ItemsScreen.test.tsx`](../screens/items/ItemsScreen.test.tsx.md), [`ItemsScreen.crud.test.tsx`](../screens/items/ItemsScreen.crud.test.tsx.md).

## Tests

_Not applicable: types only. [`commands.test.ts`](commands.test.ts.md) checks that wrappers send and unwrap these shapes._

## Notes

- The types are kept in sync with the Rust DTOs by hand; nothing checks them at build time or at runtime. Responses are cast, not validated.
- Rust sends sizes and times as `u64`. JavaScript numbers are exact only up to 2^53, which is far beyond any realistic size or timestamp.
