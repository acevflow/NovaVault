# [`ui/src/screens/items/ItemDetails.tsx`](../../../../../../ui/src/screens/items/ItemDetails.tsx)

## Purpose

Shows one item: its kind, title, plain fields (always visible), secret fields (hidden until revealed through [`SecretField`](../../components/SecretField.tsx.md)), last update time and attachments, with Edit and Delete actions. It loads the item's plain fields with `get_item`; secret values never arrive with it.

## Public API

| Name               | Kind      | Signature | Description |
| ------------------ | --------- | --------- | ----------- |
| `ItemDetailsProps` | interface | `export interface ItemDetailsProps { readonly itemId: string; readonly onEdit: (details: Details) => void; readonly onDeleted: () => void; readonly onLocked: () => void; }` | Props of `ItemDetails`. `Details` is `ItemDetails` from `api/types.ts`, renamed on import. |
| `ItemDetails`      | component | `function ItemDetails({ itemId, onEdit, onDeleted, onLocked }: ItemDetailsProps)` | One item's fields and actions. |

The `Fields` and `PlainField` components are internal and described below.

## Details

### `ItemDetailsProps`

- `itemId`: the item to show. The parent remounts the component with a `key` to show another one.
- `onEdit`: opens the edit form with the loaded details (plain fields and `has…` flags only).
- `onDeleted`: called after the item was deleted.
- `onLocked`: called if a command reports the vault is locked.

### `ItemDetails`

- **State**: `details` (`Details | null`), `error` (`string | null`), `confirming` (delete question open), `deleting` (delete in flight).
- **Load effect** (on `itemId` and `onLocked`): calls `getItem(itemId)`. On success stores `details`. On `locked` calls `onLocked()`. Otherwise sets `error` (for example `item_not_found`). Results from a superseded run are dropped.
- **Rendered states**:
  - **Loading**: `<section className="card" aria-label="Item details">` with `<p role="status">Loading item…</p>`.
  - **Load failed**: the same section with an error [`Alert`](../../components/Alert.tsx.md).
  - **Loaded**: the same section with:
    1. a badge with the kind label;
    2. a level-2 [`ScreenHeading`](../../components/ScreenHeading.tsx.md) with the title, or "Untitled" (takes focus);
    3. a `<dl>` with the kind's rows from `Fields`, then "Last updated" from [`formatDateTime`](../../../../../../ui/src/format/dateTime.ts);
    4. [`AttachmentList`](AttachmentList.tsx.md); its `onChange` replaces `details.attachments`;
    5. an error `Alert` when `error` is set (for example a failed delete);
    6. either Edit (primary) and Delete (danger) buttons, or [`ConfirmDelete`](ConfirmDelete.tsx.md) while `confirming`.
- **Delete** (`remove`): sets `deleting`, clears `error`, calls `deleteItem(itemId)`, then `onDeleted()`. On `locked` calls `onLocked()`. On other errors shows the error and closes the question.

### `Fields` (internal)

The kind-specific rows:

| Kind       | Rows |
| ---------- | ---- |
| `password` | Username (plain), Password (secret), Website (plain, field `url`), Notes (secret, multiline) |
| `note`     | Text (secret `body`, multiline) |
| `secret`   | Value (secret), Notes (secret, multiline) |

Each secret row is a `SecretField` with `hasValue` from the matching `has…` flag.

### `PlainField` (internal)

```ts
interface PlainFieldProps {
  readonly itemId: string;
  readonly field: 'username' | 'url';
  readonly label: string;
  readonly value: string;
  readonly onLocked: () => void;
}
```

A non-secret row: "Empty" (muted) when the value is `''`, otherwise the value and a [`CopyButton`](../../components/CopyButton.tsx.md). It uses the same CSS classes as secret rows, with a `--plain` modifier.

### Flow summary

- **User flows**: view → Reveal or Copy a field; Attach / Export / Remove files; Edit → the parent opens the form; Delete → confirm → the parent shows "Item deleted.".
- **Accessibility**: the item section is labelled "Item details", and its heading takes focus when it loads. Loading text is a status message and errors are alerts. Fields form a description list (`dt`/`dd`).
- **Tauri calls**:
  - `getItem` → `get_item` with `{ id }` on mount;
  - `deleteItem` → `delete_item` with `{ id }` after confirmation;
  - through children: `reveal_item_field`, `copy_item_field`, `add_attachment`, `export_attachment`, `delete_attachment`.
  See [`commands.rs`](../../../../src-tauri/src/commands.rs.md).
- **Security**: `get_item` returns plain fields and `has…` flags only. Username and URL are treated as not secret and shown in full. Secret values are fetched one at a time on demand by `SecretField`. The `Details` passed to `onEdit` contain no secret values.

## Dependencies

- **Internal**: [`api/commands.ts`](../../../../../../ui/src/api/commands.ts) (`getItem`, `deleteItem`), [`api/errors.ts`](../../../../../../ui/src/api/errors.ts), [`api/types.ts`](../../../../../../ui/src/api/types.ts), [`Alert`](../../components/Alert.tsx.md), [`Button`](../../components/Button.tsx.md), [`CopyButton`](../../components/CopyButton.tsx.md), [`ScreenHeading`](../../components/ScreenHeading.tsx.md), [`SecretField`](../../components/SecretField.tsx.md), [`format/dateTime.ts`](../../../../../../ui/src/format/dateTime.ts), [`AttachmentList`](AttachmentList.tsx.md), [`ConfirmDelete`](ConfirmDelete.tsx.md), [`kinds.ts`](../../../../../../ui/src/screens/items/kinds.ts).
- **External**: `react`: `useEffect`, `useState`.

## Used by

- [`ItemsScreen`](ItemsScreen.tsx.md): in `view` mode.

## Tests

- [`ItemsScreen.crud.test.tsx`](../../../../../../ui/src/screens/items/ItemsScreen.crud.test.tsx): `shows plain fields and keeps secret fields hidden`, `copies plain and secret fields through the shell`, `asks for confirmation and can be cancelled`, `deletes after confirmation`.
- [`SecretField.test.tsx`](../../../../../../ui/src/components/SecretField.test.tsx) and [`AttachmentList.test.tsx`](../../../../../../ui/src/screens/items/AttachmentList.test.tsx) cover the child components.

## Notes

- "Last updated" is not refreshed after attaching or removing a file, because only `attachments` is patched locally. Whether the shell bumps `updatedAtMs` on attachment changes is not visible from this file.
- After the delete question is cancelled, or closes because the delete failed, focus is not returned to the Delete button; the focused element unmounts.
- The item's delete question and an attachment's remove question can be open at once. Both listen for Escape on the document, so one press closes both.
