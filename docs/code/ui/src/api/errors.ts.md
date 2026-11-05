# [`ui/src/api/errors.ts`](../../../../../ui/src/api/errors.ts)

## Purpose

The UI side of the shell's error contract. The shell rejects a failed command with `{ code, message }` ([`src-tauri/src/error.rs`](../../../src-tauri/src/error.rs.md)). This file turns any rejection into a typed `CommandError` and maps each code to the user-facing text shown in alerts.

## Public API

| Name             | Kind     | Signature                                                                          | Description                                           |
| ---------------- | -------- | ---------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `ERROR_CODES`    | constant | `export const ERROR_CODES = ['locked', …, 'internal'] as const`                    | Every code the shell can return.                      |
| `ErrorCode`      | type     | `export type ErrorCode = (typeof ERROR_CODES)[number]`                             | One of those codes.                                   |
| `CommandError`   | class    | `export class CommandError extends Error { readonly code: ErrorCode; constructor(code: ErrorCode, message: string) }` | A failed Tauri command.            |
| `isCommandError` | function | `export function isCommandError(value: unknown, code?: ErrorCode): value is CommandError` | Type guard, optionally for one code.           |
| `toCommandError` | function | `export function toCommandError(reason: unknown): CommandError`                    | Normalizes any rejection.                             |
| `errorMessage`   | function | `export function errorMessage(error: unknown): string`                             | User-facing text for an error.                        |

## Details

### `ERROR_CODES` and `ErrorCode`

- The 16 codes, in order: `locked`, `vault_not_found`, `vault_exists`, `invalid_credentials`, `invalid_recovery_code`, `invalid_input`, `item_not_found`, `attachment_not_found`, `file_access`, `cancelled`, `invalid_item`, `unsupported_vault`, `corrupted`, `storage`, `clipboard_unavailable`, `internal`.
- They mirror `CommandError::code()` in `src-tauri/src/error.rs`; the two lists currently match, in the same order.

### `CommandError`

- **Parameters**: `code`, the shell's code; `message`, the shell's fixed English message, kept for diagnostics only.
- Sets `name` to `'CommandError'`.
- **Security**: the shell's messages are fixed strings without secrets, so keeping one in `message` is safe.

### `isCommandError`

- Returns `true` when `value` is a `CommandError` and, if `code` is given, has that code. It uses `instanceof`, so it only recognizes errors created by this module.

### `toCommandError`

1. A `CommandError` is returned unchanged.
2. An object with a `code` property that is a known `ErrorCode` becomes `new CommandError(code, message)`. `message` is the object's `message` if it is a string, else `''`.
3. Anything else (a string, `null`, a plain `Error`, an unknown or non-string code) becomes `new CommandError('internal', 'unexpected command failure')`.

- **Edge cases**: a code added by a newer shell, or an IPC failure, maps to `internal`, so callers handle a single type.

### `errorMessage`

- Normalizes `error` with `toCommandError` and returns the UI's own text for its code from the private `MESSAGES` record (`Record<ErrorCode, string>`). The compiler requires an entry for every code.
- The shell's `message` is never shown. For example, `invalid_credentials` shows "That password or recovery code is not correct." and `storage` shows "The vault file could not be read or written. Check the disk and try again."
- **Security**: `invalid_credentials` does not say whether the password or the recovery code was tried, and no message echoes user input.

## Dependencies

- **Internal**: none.
- **External**: none.

## Used by

- [`commands.ts`](commands.ts.md): `toCommandError` in the single `invoke` helper.
- Screens and components that show or branch on errors: [`App.tsx`](../App.tsx.md), [`CreateVaultScreen`](../screens/create/CreateVaultScreen.tsx.md), [`UnlockScreen`](../screens/unlock/UnlockScreen.tsx.md), [`ItemsScreen`](../screens/items/ItemsScreen.tsx.md), [`ItemDetails`](../screens/items/ItemDetails.tsx.md), [`ItemForm`](../screens/items/ItemForm.tsx.md), [`AttachmentList`](../screens/items/AttachmentList.tsx.md), [`CopyButton`](../components/CopyButton.tsx.md), [`SecretField`](../components/SecretField.tsx.md) and the settings sections.
- [`test/ipc.ts`](../test/ipc.ts.md): the `ErrorCode` type.

## Tests

- [`commands.test.ts`](commands.test.ts.md): `rejects with a typed CommandError`, `maps malformed and unknown rejections to internal`, `keeps known codes and existing errors`.

## Notes

- The `invalid_recovery_code` text says the code "has 8 groups of 4 characters", which matches the display format in [`types.ts`](types.ts.md) and the test code in [`test/ipc.ts`](../test/ipc.ts.md).
- `ERROR_CODES` is kept in sync with the Rust list by hand; no test compares them.
- The module comment in `src-tauri/src/error.rs` says the UI "may show `message`"; this UI never does.
