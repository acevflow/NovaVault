# [`ui/src/screens/items/AttachmentList.test.tsx`](../../../../../../ui/src/screens/items/AttachmentList.test.tsx)

## Purpose

Component tests for [`AttachmentList`](AttachmentList.tsx.md). They cover listing, attaching with progress over a Tauri channel, a closed file dialog, exporting, removing with confirmation, a locked vault and error display. They also exercise the three attachment wrappers in [`api/commands.ts`](../../api/commands.ts.md).

## Public API

| Test case                                | What it checks                                                                                                                                                                                                                                                                                                                |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `lists files with their size and actions` | The `Attachments` heading, file names, `1.5 KB · added` and `3.0 MB · added`, and enabled `Export photo.jpg` and `Remove photo.jpg` buttons.                                                                                                                                                                                  |
| `says when there are no files`           | An empty list shows `No files attached.`                                                                                                                                                                                                                                                                                      |
| `attaches a file and shows its progress` | While `add_attachment` is pending, `Choose a file in the dialog…` is shown and the other actions are disabled; `itemId` is sent. A progress message of 512 KB of 1 MB shows a `progressbar` named `Attaching file` with `value="50"` and `50% · 512 KB of 1.0 MB`. On completion: `Attached “photo.jpg”.`, `onChange` receives both files, the bar disappears and `Attach file` is enabled again. |
| `does nothing when the dialog is closed` | A `null` result re-enables `Attach file` without calling `onChange` and without an alert or status message.                                                                                                                                                                                                                    |
| `exports a file`                         | `export_attachment` is called with `itemId` and `attachmentId`, and `Exported “passport scan.pdf”.` is shown.                                                                                                                                                                                                                 |
| `removes a file after confirmation`      | `Remove` opens an `alertdialog` named `Remove this attachment?` without calling any command. Confirming calls `delete_attachment` with `{ itemId, attachmentId }`, shows `Removed “passport scan.pdf”.` and passes the remaining file to `onChange`.                                                                         |
| `keeps the file when removal is cancelled` | `Cancel` closes the dialog and no command is called.                                                                                                                                                                                                                                                                          |
| `reports a locked vault to its parent`   | A `locked` rejection from `add_attachment` calls `onLocked` once.                                                                                                                                                                                                                                                             |
| `shows errors`                           | A `corrupted` rejection from `export_attachment` shows `The vault data is damaged and cannot be read.` in an alert.                                                                                                                                                                                                           |

## Details

### Fixtures and helpers

- `SCAN` (`passport scan.pdf`, 1536 bytes) and `PHOTO` (`photo.jpg`, 3 MiB): fake attachments of item `'a'.repeat(32)`.
- `deferred<T>()`: a promise plus its `resolve` function, so a fake command can stay pending while the test inspects the in-progress state.
- `sendProgress(args, progress)`: calls `onmessage` on the `Channel` that the wrapper passed as `onProgress`, the way the shell sends progress.
- `renderList(attachments = [SCAN])`: renders the component with `vi.fn` callbacks for `onChange` and `onLocked`.

- **Security**: no real files are involved; the native dialog is faked away with the command.

## Dependencies

- **Internal**: [`AttachmentList.tsx`](AttachmentList.tsx.md), [`api/types.ts`](../../api/types.ts.md) (`Attachment`, `TransferProgress`), [`test/ipc.ts`](../../test/ipc.ts.md) (`mockCommands`, `shellError`, `CommandArgs`).
- **External**: `@tauri-apps/api/core` (the `Channel` type), `@testing-library/react`, `@testing-library/user-event`, `vitest`.

## Used by

_Not applicable: test file, run by `npm test`._

## Tests

- Files under test: [`screens/items/AttachmentList.tsx`](../../../../../../ui/src/screens/items/AttachmentList.tsx), [`ConfirmDelete.tsx`](../../../../../../ui/src/screens/items/ConfirmDelete.tsx), [`format/bytes.ts`](../../../../../../ui/src/format/bytes.ts), and the attachment wrappers in [`api/commands.ts`](../../../../../../ui/src/api/commands.ts).

## Notes

- `sendProgress` relies on the `Channel` object that the wrapper creates being passed through `mockIPC` unchanged, which lets the test call its `onmessage` directly.
