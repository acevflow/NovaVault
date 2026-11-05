# [`ui/src/api/commands.ts`](../../../../../ui/src/api/commands.ts)

## Purpose

Typed wrappers over the Tauri commands; together they are the UI's whole IPC surface. Each function calls exactly one command of [`src-tauri/src/commands.rs`](../../../src-tauri/src/commands.rs.md) through one private helper, which turns every rejection into a [`CommandError`](errors.ts.md). It is the only app file that imports `@tauri-apps/api`; outside it, only test code does (the mocks in [`test/ipc.ts`](../test/ipc.ts.md) and [`test/setup.ts`](../test/setup.ts.md), and a type import in [`AttachmentList.test.tsx`](../screens/items/AttachmentList.test.tsx.md)).

## Public API

| Name                     | Kind     | Signature                                                                                                                        | Description                                                 |
| ------------------------ | -------- | -------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `CommandName`            | type     | `export type CommandName = 'vault_status' \| 'create_vault' \| … \| 'delete_attachment'`                                         | Union of the 19 command names the UI calls.                 |
| `vaultStatus`            | function | `export function vaultStatus(): Promise<VaultStatus>`                                                                            | Whether a vault exists and is unlocked.                     |
| `createVault`            | function | `export async function createVault(password: string): Promise<string>`                                                           | Creates and unlocks the vault; returns the recovery code.   |
| `unlock`                 | function | `export function unlock(credential: Credential): Promise<void>`                                                                  | Unlocks with the master password or the recovery code.      |
| `lock`                   | function | `export function lock(): Promise<void>`                                                                                          | Locks the vault.                                            |
| `changeMasterPassword`   | function | `export function changeMasterPassword(currentCredential: Credential, newPassword: string): Promise<void>`                        | Changes the master password.                                |
| `regenerateRecoveryCode` | function | `export async function regenerateRecoveryCode(currentCredential: Credential): Promise<string>`                                   | Replaces the recovery code; returns the new one.            |
| `getSettings`            | function | `export function getSettings(): Promise<Settings>`                                                                               | Reads the app settings.                                     |
| `setAutoLockTimeout`     | function | `export function setAutoLockTimeout(minutes: number): Promise<void>`                                                             | Sets the inactivity timeout.                                |
| `setClipboardClearDelay` | function | `export function setClipboardClearDelay(seconds: number): Promise<void>`                                                         | Sets the clipboard clear delay.                             |
| `copyItemField`          | function | `export async function copyItemField(id: string, field: ItemField): Promise<number>`                                             | Copies a field in the shell; returns seconds until it is cleared. |
| `listItems`              | function | `export function listItems(): Promise<ItemSummary[]>`                                                                            | Lists items without secret fields.                          |
| `getItem`                | function | `export function getItem(id: string): Promise<ItemDetails>`                                                                      | Reads one item's plain fields.                              |
| `revealItemField`        | function | `export async function revealItemField(id: string, field: SecretItemField): Promise<string>`                                     | Reads one secret field.                                     |
| `createItem`             | function | `export function createItem(item: ItemInput): Promise<string>`                                                                   | Adds an item; returns its id.                               |
| `updateItem`             | function | `export function updateItem(id: string, item: ItemUpdate): Promise<void>`                                                        | Replaces an item's content.                                 |
| `deleteItem`             | function | `export function deleteItem(id: string): Promise<void>`                                                                          | Deletes an item and its attachments.                        |
| `addAttachment`          | function | `export function addAttachment(itemId: string, onProgress: (progress: TransferProgress) => void): Promise<Attachment \| null>`  | Picks a file in a native dialog and attaches it.            |
| `exportAttachment`       | function | `export function exportAttachment(itemId: string, attachmentId: string, onProgress: (progress: TransferProgress) => void): Promise<boolean>` | Picks a destination and decrypts the attachment there. |
| `deleteAttachment`       | function | `export function deleteAttachment(itemId: string, attachmentId: string): Promise<void>`                                          | Removes an attachment.                                      |

## Details

### `CommandName`

- Lists the 19 names in the same order as `COMMANDS` in [`src-tauri/build.rs`](../../../src-tauri/build.rs.md); the two lists currently match. The shell's capability allowlist is generated from `COMMANDS`, so a name missing there would be rejected at runtime.
- [`test/ipc.ts`](../test/ipc.ts.md) uses it to type fake command handlers.

### `call` (private)

`async function call<T>(command: CommandName, args?: InvokeArgs): Promise<T>`

1. Awaits `invoke<T>(command, args)`.
2. On rejection, throws `toCommandError(reason)`, so callers only ever see a `CommandError`.

- **Security**: nothing is logged, because `args` may hold a master password or a recovery code. The comment in the source says so.

### Argument and response shapes

Tauri maps the camelCase argument names to the snake_case Rust parameters. Each wrapper's command, arguments and unwrapping:

| Wrapper                  | Command                     | Arguments sent                           | Shell returns                    | Wrapper returns          |
| ------------------------ | --------------------------- | ---------------------------------------- | -------------------------------- | ------------------------ |
| `vaultStatus`            | `vault_status`              | none                                     | `VaultStatusDto`                 | as is                    |
| `createVault`            | `create_vault`              | `{ password }`                           | `{ recoveryCode }`               | `recoveryCode`           |
| `unlock`                 | `unlock`                    | `{ credential }`                         | `null`                           | `void`                   |
| `lock`                   | `lock`                      | none                                     | `null`                           | `void`                   |
| `changeMasterPassword`   | `change_master_password`    | `{ currentCredential, newPassword }`     | `null`                           | `void`                   |
| `regenerateRecoveryCode` | `regenerate_recovery_code`  | `{ currentCredential }`                  | `{ recoveryCode }`               | `recoveryCode`           |
| `getSettings`            | `get_settings`              | none                                     | `Settings`                       | as is                    |
| `setAutoLockTimeout`     | `set_auto_lock_timeout`     | `{ minutes }`                            | `null`                           | `void`                   |
| `setClipboardClearDelay` | `set_clipboard_clear_delay` | `{ seconds }`                            | `null`                           | `void`                   |
| `copyItemField`          | `copy_item_field`           | `{ id, field }`                          | `{ clearAfterSeconds }`          | `clearAfterSeconds`      |
| `listItems`              | `list_items`                | none                                     | `ItemSummaryDto[]`               | as is                    |
| `getItem`                | `get_item`                  | `{ id }`                                 | `ItemDetailsDto`                 | as is                    |
| `revealItemField`        | `reveal_item_field`         | `{ id, field }`                          | `{ value }`                      | `value`                  |
| `createItem`             | `create_item`               | `{ item }`                               | item id string                   | as is                    |
| `updateItem`             | `update_item`               | `{ id, item }`                           | `null`                           | `void`                   |
| `deleteItem`             | `delete_item`               | `{ id }`                                 | `null`                           | `void`                   |
| `addAttachment`          | `add_attachment`            | `{ itemId, onProgress: Channel }`        | `AttachmentDto` or `null`        | as is                    |
| `exportAttachment`       | `export_attachment`         | `{ itemId, attachmentId, onProgress: Channel }` | `bool`                    | as is                    |
| `deleteAttachment`       | `delete_attachment`         | `{ itemId, attachmentId }`               | `null`                           | `void`                   |

- **Errors**: every wrapper rejects with a `CommandError`. The codes each command can return are documented on [`commands.rs`](../../../src-tauri/src/commands.rs.md).

### Secrets crossing IPC

- **Sent to the shell**: the master password (`createVault`, `changeMasterPassword`), a `Credential` (`unlock`, `changeMasterPassword`, `regenerateRecoveryCode`) and the secret fields of `ItemInput` and `ItemUpdate`. The module keeps no copy.
- **Received from the shell**: the recovery code, only from `createVault` and `regenerateRecoveryCode`, so the user can write it down. A secret field value, only from `revealItemField`, when the user asks to see it; its doc comment tells callers to drop the value when it is hidden again.
- **Never received**: the vault key or any key material. `copyItemField` copies inside the shell and returns only the clear delay. `addAttachment` and `exportAttachment` let the shell show the native dialog, so the UI never sees file paths or contents.

### `addAttachment` and `exportAttachment`

- `onProgress` is wrapped in a Tauri `Channel`; the shell sends `{ done, total }` updates through it while it encrypts or decrypts.
- `addAttachment` resolves to `null`, and `exportAttachment` to `false`, when the user closes the dialog.
- **Edge cases**: a transfer interrupted by a lock rejects with `cancelled` (see the message in [`errors.ts`](errors.ts.md)).

## Dependencies

- **Internal**: [`errors.ts`](errors.ts.md) (`toCommandError`, `CommandError`), [`types.ts`](types.ts.md) (all request and response types).
- **External**: `@tauri-apps/api/core` (`invoke`, `Channel`, `InvokeArgs`).

## Used by

- [`App.tsx`](../App.tsx.md): `vaultStatus`, `lock`.
- [`CreateVaultScreen`](../screens/create/CreateVaultScreen.tsx.md), [`UnlockScreen`](../screens/unlock/UnlockScreen.tsx.md).
- [`ItemsScreen`](../screens/items/ItemsScreen.tsx.md), [`ItemDetails`](../screens/items/ItemDetails.tsx.md), [`ItemForm`](../screens/items/ItemForm.tsx.md), [`AttachmentList`](../screens/items/AttachmentList.tsx.md).
- [`CopyButton`](../components/CopyButton.tsx.md), [`SecretField`](../components/SecretField.tsx.md).
- [`AutoLockSection`](../screens/settings/AutoLockSection.tsx.md), [`ClipboardSection`](../screens/settings/ClipboardSection.tsx.md), [`ChangePasswordSection`](../screens/settings/ChangePasswordSection.tsx.md), [`RecoveryCodeSection`](../screens/settings/RecoveryCodeSection.tsx.md).
- [`test/ipc.ts`](../test/ipc.ts.md): the `CommandName` type.

## Tests

- [`commands.test.ts`](commands.test.ts.md): arguments and unwrapping of every command except `lock` and the three attachment commands, and the typed rejection.
- The attachment wrappers are covered by [`AttachmentList.test.tsx`](../screens/items/AttachmentList.test.tsx.md); `lock` by [`App.test.tsx`](../App.test.tsx.md).

## Notes

- `CommandName` and `COMMANDS` in `build.rs` must be kept in sync by hand; no test compares them.
