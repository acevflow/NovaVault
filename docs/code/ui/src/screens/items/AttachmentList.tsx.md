# [`ui/src/screens/items/AttachmentList.tsx`](../../../../../../ui/src/screens/items/AttachmentList.tsx)

## Purpose

An item's attached files, with actions to attach, export and remove them. Files are chosen and saved in native dialogs that the shell opens, and encrypted or decrypted in Rust: this component only learns names, sizes, dates and transfer progress. Only one transfer runs at a time; the other actions are disabled meanwhile.

## Public API

| Name                  | Kind      | Signature | Description |
| --------------------- | --------- | --------- | ----------- |
| `AttachmentListProps` | interface | `export interface AttachmentListProps { readonly itemId: string; readonly attachments: readonly Attachment[]; readonly onChange: (attachments: readonly Attachment[]) => void; readonly onLocked: () => void; }` | Props of `AttachmentList`. |
| `AttachmentList`      | component | `function AttachmentList({ itemId, attachments, onChange, onLocked }: AttachmentListProps)` | Attachments with attach, export and remove. |

The `Transfer` interface and the `TransferStatus` component are internal and described below.

## Details

### `AttachmentListProps`

- `itemId`: the item the attachments belong to.
- `attachments`: the item's attachments, in the order they were added.
- `onChange`: called with the new list after a file was attached or removed. [`ItemDetails`](ItemDetails.tsx.md) patches its local details with it.
- `onLocked`: called if a command reports the vault is locked.

### `Transfer` (internal)

```ts
interface Transfer {
  readonly kind: 'attach' | 'export';
  readonly label: string;
  readonly progress: TransferProgress | null;
}
```

`progress` is `null` while the native dialog is open, before any data moves.

### `AttachmentList`

- **State**: `transfer` (`Transfer | null`), `confirming` (the attachment being removed, or `null`), `removing`, `notice`, `error`. `busy` is derived: a transfer or a removal is running.
- **Attach** (`attach`):
  1. `start('attach', 'Attaching file')` clears the notice and error and sets an indeterminate transfer.
  2. Calls `addAttachment(itemId, onProgress)`. The shell shows the file picker, then reports progress through a Tauri `Channel`; `onProgress` updates `transfer.progress`.
  3. If an attachment comes back, calls `onChange([...attachments, added])` and sets "Attached “<name>”.". `null` (dialog closed) shows nothing.
  4. Errors: `locked` → `onLocked()`; others → `errorMessage` (for example `file_access`, or `cancelled` when a lock stopped the transfer).
  5. Always clears `transfer`.
- **Export** (`exportFile`): the same pattern with `exportAttachment(itemId, attachment.id, onProgress)` and the label Exporting “<name>”. `true` sets "Exported “<name>”."; `false` (dialog closed) shows nothing.
- **Remove**: "Remove" opens [`ConfirmDelete`](ConfirmDelete.tsx.md) with `subject="attachment"`. Confirming calls `deleteAttachment(itemId, attachment.id)`, then `onChange` with the attachment filtered out and "Removed “<name>”.". On error, shows it. Either way the question closes and `removing` is cleared.
- **Rendered output**: `<section className="attachments" aria-labelledby=…>` with:
  1. `<h3>Attachments</h3>`.
  2. "No files attached." or a `<ul>` with, per file: the name, "<size> · added <date>" ([`formatBytes`](../../../../../../ui/src/format/bytes.ts), [`formatDateTime`](../../../../../../ui/src/format/dateTime.ts)), and small Export and Remove buttons, disabled while `busy`.
  3. `TransferStatus` while a transfer runs.
  4. A success [`Alert`](../../components/Alert.tsx.md) with the notice, and an error `Alert` with the error.
  5. Either the "Attach file" button (busy during an attach, disabled while `busy`) or the `ConfirmDelete` question.

### `TransferStatus` (internal)

- Renders a label, then:
  - with no progress yet: an indeterminate `<progress>` and "Choose a file in the dialog…" or "Choose where to save it in the dialog…";
  - with progress: `<progress max={100} value={percent}>` and "<percent>% · <done> of <total>", using [`percentDone`](../../../../../../ui/src/format/bytes.ts) (whole percent, capped at 100; a total of 0 counts as 100%).
- The progress bar is named by the label through `aria-labelledby`.

### Flow summary

- **User flows**: Attach file → pick in the native dialog → progress → "Attached …"; Export → choose a destination → progress → "Exported …"; Remove → confirm → "Removed …". Closing either dialog quietly cancels.
- **Accessibility**:
  - The section is labelled by its heading.
  - Export and Remove have per-file accessible names ("Export <name>", "Remove <name>").
  - Notices are polite status messages and errors are alerts.
  - Progress bars are labelled.
- **Tauri calls**:
  - `addAttachment` → `add_attachment` with `{ itemId, onProgress: Channel }`, returning `Attachment | null`;
  - `exportAttachment` → `export_attachment` with `{ itemId, attachmentId, onProgress: Channel }`, returning `boolean`;
  - `deleteAttachment` → `delete_attachment` with `{ itemId, attachmentId }`.
  See [`commands.rs`](../../../../src-tauri/src/commands.rs.md).
- **Security**: file paths and contents never reach the webview. The shell opens the dialogs and does all file I/O and cryptography. File names are shown as plain text by React, so they cannot inject markup.

## Dependencies

- **Internal**: [`api/commands.ts`](../../../../../../ui/src/api/commands.ts) (`addAttachment`, `exportAttachment`, `deleteAttachment`), [`api/errors.ts`](../../../../../../ui/src/api/errors.ts), [`api/types.ts`](../../../../../../ui/src/api/types.ts) (`Attachment`, `TransferProgress`), [`Alert`](../../components/Alert.tsx.md), [`Button`](../../components/Button.tsx.md), [`ConfirmDelete`](ConfirmDelete.tsx.md), [`format/bytes.ts`](../../../../../../ui/src/format/bytes.ts), [`format/dateTime.ts`](../../../../../../ui/src/format/dateTime.ts).
- **External**: `react`: `useId`, `useState`.

## Used by

- [`ItemDetails`](ItemDetails.tsx.md).

## Tests

- [`AttachmentList.test.tsx`](../../../../../../ui/src/screens/items/AttachmentList.test.tsx): `lists files with their size and actions`, `says when there are no files`, `attaches a file and shows its progress`, `does nothing when the dialog is closed`, `exports a file`, `removes a file after confirmation`, `keeps the file when removal is cancelled`, `reports a locked vault to its parent`, `shows errors`.

## Notes

- Export shows no busy state on its own button; only the Attach button gets `busy`, and only during an attach.
- The progress text is not a live region, so screen readers hear progress only when they read the bar or text.
- While the remove question is open, the Attach button is replaced by the question, but Export and Remove on other rows stay enabled (they are disabled only while `busy`).
