# [`ui/src/screens/items/ConfirmDelete.tsx`](../../../../../../ui/src/screens/items/ConfirmDelete.tsx)

## Purpose

An inline confirmation shown below whatever it deletes: an item or an attachment. It states what will be lost, starts with focus on Cancel so a reflexive Enter deletes nothing, and closes on Escape.

## Public API

| Name                 | Kind       | Signature | Description |
| -------------------- | ---------- | --------- | ----------- |
| `DeleteSubject`      | type alias | `export type DeleteSubject = 'item' \| 'attachment';` | What `ConfirmDelete` deletes. |
| `ConfirmDeleteProps` | interface  | `export interface ConfirmDeleteProps { readonly title: string; readonly subject?: DeleteSubject; readonly busy: boolean; readonly onConfirm: () => void; readonly onCancel: () => void; }` | Props of `ConfirmDelete`. |
| `ConfirmDelete`      | component  | `function ConfirmDelete({ title, subject = 'item', busy, onConfirm, onCancel, }: ConfirmDeleteProps)` | Inline delete confirmation. |

## Details

### `DeleteSubject` and wording

The module constant `WORDING` holds the text for each subject:

| Subject      | Heading                    | Consequence                                                            | Action | Busy        |
| ------------ | -------------------------- | ---------------------------------------------------------------------- | ------ | ----------- |
| `item`       | Delete this item?          | will be deleted from the vault                                         | Delete | Deleting…   |
| `attachment` | Remove this attachment?    | will be removed from this item and its encrypted copy deleted          | Remove | Removing…   |

### `ConfirmDeleteProps`

- `title`: the item title or file name, quoted in the question; "Untitled" when empty.
- `subject`: optional, default `'item'`.
- `busy`: whether the deletion is running.
- `onConfirm`: performs the deletion.
- `onCancel`: closes the question without deleting.

### `ConfirmDelete`

- **State**: two `useId` ids and a ref to the Cancel button.
- **Effects**:
  - On mount: focuses Cancel.
  - On `busy` and `onCancel`: adds a `keydown` listener to `document` that calls `onCancel()` on Escape unless `busy`, and removes it on cleanup. It is on the document so Escape works wherever focus is.
- **Rendered output**: `<div role="alertdialog" aria-labelledby=… aria-describedby=… className="confirm">` with:
  - an `<h3>` heading;
  - the message “<title>” <consequence>. This cannot be undone.;
  - a danger [`Button`](../../components/Button.tsx.md) with the action text, or the busy text and `busy` while running;
  - a Cancel `Button`, disabled while busy.
- **User flows**: confirm → `onConfirm`; Cancel or Escape → `onCancel`. Neither works while busy, apart from the danger button, which is disabled by its own busy state.
- **Accessibility**:
  - `role="alertdialog"`, named by the heading and described by the message, so it is announced when it appears.
  - Initial focus on Cancel is a safe default.
  - Escape closes it.
- **Tauri calls**: none; the parent performs the deletion.

## Dependencies

- **Internal**: [`Button`](../../components/Button.tsx.md).
- **External**: `react`: `useEffect`, `useId`, `useRef`.

## Used by

- [`ItemDetails`](ItemDetails.tsx.md): deleting an item (`subject` `item`).
- [`AttachmentList`](AttachmentList.tsx.md): removing an attachment (`subject` `attachment`).

## Tests

- [`ItemsScreen.crud.test.tsx`](../../../../../../ui/src/screens/items/ItemsScreen.crud.test.tsx): `asks for confirmation and can be cancelled`, `deletes after confirmation`.
- [`AttachmentList.test.tsx`](../../../../../../ui/src/screens/items/AttachmentList.test.tsx): `removes a file after confirmation`, `keeps the file when removal is cancelled`.

## Notes

- `role="alertdialog"` is used for an inline, non-modal panel: focus is not trapped and the rest of the page stays interactive. ARIA describes `alertdialog` as a modal-style dialog, so this is a deliberate loose use.
- The Escape listener is global. With two instances open (item and attachment), one Escape cancels both.
- Parents pass inline `onCancel` functions, so the listener is removed and re-added on every parent render. This is harmless.
- On close, focus is not restored to the button that opened the question.
