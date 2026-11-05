# [`ui/src/test/ipc.ts`](../../../../../ui/src/test/ipc.ts)

## Purpose

Test helpers that fake the Tauri shell. They install fake command handlers through Tauri's `mockIPC`, record every call, and build rejections shaped like the shell's errors. Every UI test that touches IPC uses them, so no test needs a running shell.

## Public API

| Name                 | Kind      | Signature                                                                                   | Description                                          |
| -------------------- | --------- | ------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| `CommandArgs`        | type      | `export type CommandArgs = Record<string, unknown>`                                         | Arguments a command was called with.                 |
| `CommandHandlers`    | type      | `export type CommandHandlers = Partial<Record<CommandName, (args: CommandArgs) => unknown>>` | Fake command implementations.                        |
| `RecordedCall`       | interface | `export interface RecordedCall { readonly command: string; readonly args: CommandArgs; }`   | One recorded call.                                   |
| `mockCommands`       | function  | `export function mockCommands(handlers: CommandHandlers): RecordedCall[]`                   | Installs fakes and returns the list of calls.        |
| `ShellError`         | class     | `export class ShellError extends Error { readonly code: ErrorCode; constructor(code: ErrorCode) }` | A rejection shaped like `{ code, message }`.   |
| `shellError`         | function  | `export function shellError(code: ErrorCode): ShellError`                                   | The rejection the shell sends for `code`.            |
| `TEST_RECOVERY_CODE` | constant  | `export const TEST_RECOVERY_CODE = 'ABCD-EFGH-JKLM-NPQR-STUV-WXYZ-2345-6789'`                | A fake recovery code in display form.                |

## Details

### `CommandHandlers`

- Keyed by [`CommandName`](../api/commands.ts.md), so a misspelled command name in a test fails to compile. A handler may return a value or a promise, or throw.

### `mockCommands`

1. Creates an empty `calls` array.
2. Calls `mockIPC` with a handler that, for each `invoke`:
   1. records `{ command, args }`, with `args` defaulting to `{}` when there is no payload;
   2. looks up the handler for `command`;
   3. throws `shellError('internal')` if there is none, so unexpected calls fail visibly;
   4. otherwise returns the handler's result.
3. Returns `calls`. The array keeps filling as the component under test calls commands.

- **Returns**: the live list of recorded calls, in call order.
- **Edge cases and invariants**: each call replaces any previous fake. [`setup.ts`](setup.ts.md) calls `clearMocks()` after each test.

### `ShellError` and `shellError`

- `ShellError` extends `Error` only so tests can `throw` it. The UI reads just `code` and `message`; [`toCommandError`](../api/errors.ts.md) converts it like a real shell rejection. Its message is `shell error: <code>`.

### `TEST_RECOVERY_CODE`

- 8 groups of 4 characters, all from the recovery code alphabet. It is an obviously fake value, used wherever a test needs a code.

- **Security**: test-only code; never imported by app code.

## Dependencies

- **Internal**: [`api/commands.ts`](../api/commands.ts.md) (the `CommandName` type), [`api/errors.ts`](../api/errors.ts.md) (the `ErrorCode` type).
- **External**: `@tauri-apps/api/mocks` (`mockIPC`).

## Used by

- [`App.test.tsx`](../App.test.tsx.md), [`commands.test.ts`](../api/commands.test.ts.md), [`SecretField.test.tsx`](../components/SecretField.test.tsx.md), [`CreateVaultScreen.test.tsx`](../screens/create/CreateVaultScreen.test.tsx.md), [`AttachmentList.test.tsx`](../screens/items/AttachmentList.test.tsx.md), [`ItemsScreen.test.tsx`](../screens/items/ItemsScreen.test.tsx.md), [`ItemsScreen.crud.test.tsx`](../screens/items/ItemsScreen.crud.test.tsx.md), [`SettingsScreen.test.tsx`](../screens/settings/SettingsScreen.test.tsx.md), [`UnlockScreen.test.tsx`](../screens/unlock/UnlockScreen.test.tsx.md).

## Tests

_Not applicable: this file is test infrastructure and has no tests of its own._

## Notes

- `RecordedCall.command` is typed `string`, not `CommandName`, because `mockIPC` reports any name it receives.
- The `args` cast to `CommandArgs` is not checked. Handlers cast individual arguments themselves, as in `ItemsScreen.crud.test.tsx`.
