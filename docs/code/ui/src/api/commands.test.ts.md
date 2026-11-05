# [`ui/src/api/commands.test.ts`](../../../../../ui/src/api/commands.test.ts)

## Purpose

Unit tests for the IPC wrappers in [`commands.ts`](commands.ts.md) and the error normalization in [`errors.ts`](errors.ts.md). A fake shell records every call, so the tests check the exact command names and camelCase arguments sent, and how responses and rejections are unwrapped.

## Public API

| Test case                                                        | What it checks                                                                                                                                                                                                                       |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `commands` › `passes camelCase arguments and unwraps responses`  | `vaultStatus`, `createVault`, `unlock`, `changeMasterPassword`, `regenerateRecoveryCode`, `getSettings`, `setAutoLockTimeout` and `setClipboardClearDelay` send the expected command and arguments (for example `{ currentCredential, newPassword }`), and the two recovery-code calls return the bare code string. |
| `commands` › `wraps the item commands`                           | `listItems`, `getItem`, `revealItemField`, `createItem`, `updateItem` (with `body: null`), `deleteItem` and `copyItemField` send the expected arguments; `revealItemField` returns the bare value and `copyItemField` returns `30`. |
| `commands` › `rejects with a typed CommandError`                 | A shell rejection with `invalid_credentials` reaches the caller as a `CommandError` with that code, and `errorMessage` gives "That password or recovery code is not correct."                                                        |
| `toCommandError` › `maps malformed and unknown rejections to internal` | `'boom'`, `null`, `new Error('x')`, `{ code: 'from_the_future' }` and `{ code: 42 }` all become `internal`.                                                                                                                   |
| `toCommandError` › `keeps known codes and existing errors`       | An existing `CommandError` is returned as the same object; `{ code: 'storage', message: 'm' }` keeps `storage`.                                                                                                                     |

## Details

### Setup

- [`mockCommands`](../test/ipc.ts.md) installs one handler per command and returns the recorded calls. The tests compare `calls.map(({ command, args }) => [command, args])` with the expected list, so order matters.
- Commands without arguments are recorded with `{}`, because the fake shell defaults a missing payload to `{}`.
- **Security**: only fake values are used (`'pw'`, `'old'`, `'new'`, `'code'` and [`TEST_RECOVERY_CODE`](../test/ipc.ts.md)).

## Dependencies

- **Internal**: [`commands.ts`](commands.ts.md), [`errors.ts`](errors.ts.md), [`test/ipc.ts`](../test/ipc.ts.md) (`mockCommands`, `shellError`, `TEST_RECOVERY_CODE`).
- **External**: `vitest`.

## Used by

_Not applicable: test file, run by `npm test`._

## Tests

- Files under test: [`api/commands.ts`](../../../../../ui/src/api/commands.ts), [`api/errors.ts`](../../../../../ui/src/api/errors.ts).

## Notes

- `lock`, `addAttachment`, `exportAttachment` and `deleteAttachment` are not covered here. They are exercised by [`App.test.tsx`](../App.test.tsx.md) and [`AttachmentList.test.tsx`](../screens/items/AttachmentList.test.tsx.md).
- The `get_item` fake response has no `attachments` field, although `ItemDetails` requires one. The test only checks that the value is passed through unchanged, so the shape does not matter there.
