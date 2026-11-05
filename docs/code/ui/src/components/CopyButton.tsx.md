# [`ui/src/components/CopyButton.tsx`](../../../../../ui/src/components/CopyButton.tsx)

## Purpose

A button that copies one item field to the system clipboard. The Rust shell reads the value and copies it itself, so the value never reaches the UI. The UI only learns how long until the clipboard is cleared, and announces that politely.

## Public API

| Name              | Kind      | Signature | Description |
| ----------------- | --------- | --------- | ----------- |
| `CopyButtonProps` | interface | `export interface CopyButtonProps { readonly itemId: string; readonly field: ItemField; readonly label: string; readonly onLocked: () => void; }` | Props of `CopyButton`. |
| `CopyButton`      | component | `function CopyButton({ itemId, field, label, onLocked }: CopyButtonProps)` | Copies a field through the shell and reports the outcome. |

## Details

### `CopyButtonProps`

- `itemId`: the item the field belongs to.
- `field`: which field to copy, plain or secret ([`ItemField`](../../../../../ui/src/api/types.ts)).
- `label`: the field's visible label, used in the accessible name.
- `onLocked`: called if the shell reports the vault is locked.

### `CopyButton`

- **State**: `busy` (request in flight), `message` (text of the last outcome), `failed` (whether that outcome was an error).
- **Behavior** (`copy`):
  1. Sets `busy`.
  2. Calls `copyItemField(itemId, field)`.
  3. On success: clears `failed` and sets "Copied. The clipboard clears in <duration>." using [`formatSeconds`](../../../../../ui/src/format/duration.ts) (for example "30 seconds" or "1 minute").
  4. On a `locked` error: calls `onLocked()` and sets no message.
  5. On any other error: sets `failed` and the text from `errorMessage` (for example the `clipboard_unavailable` message).
  6. Always clears `busy`.
- **Rendered output**: a `<button type="button" className="field__toggle">` with the text "Copy", followed by a `<span role="status">` holding `message`, styled as an error when `failed`.
- **Accessibility**:
  - Accessible name `Copy <label in lower case>`, for example "Copy password".
  - `aria-busy` and `disabled` while the request runs, which also blocks double copies.
  - The status span is always rendered, even when empty, so screen readers announce changes to its text.
- **Tauri calls**: `copyItemField` → `copy_item_field` with `{ id, field }`, returning `clearAfterSeconds`. See [`commands.rs`](../../../src-tauri/src/commands.rs.md) and [`clipboard.rs`](../../../src-tauri/src/clipboard.rs.md).
- **Security**: the copied value never enters the webview. The shell, not this component, clears the clipboard after the delay.

## Dependencies

- **Internal**: [`api/commands.ts`](../../../../../ui/src/api/commands.ts) (`copyItemField`), [`api/errors.ts`](../../../../../ui/src/api/errors.ts) (`errorMessage`, `isCommandError`), [`api/types.ts`](../../../../../ui/src/api/types.ts) (`ItemField`), [`format/duration.ts`](../../../../../ui/src/format/duration.ts) (`formatSeconds`).
- **External**: `react`: `useState`.

## Used by

- [`SecretField`](SecretField.tsx.md): next to each secret value.
- [`ItemDetails`](../screens/items/ItemDetails.tsx.md): next to plain fields.

## Tests

- [`SecretField.test.tsx`](../../../../../ui/src/components/SecretField.test.tsx):
  - `copies through the shell without revealing the value`.
  - `reports a clipboard that is not available`.

## Notes

- The "Copied…" message stays after the shell has cleared the clipboard; the UI is not told when the clear happens.
- If the component unmounts during the request (for example after `onLocked`), the `finally` block still calls `setBusy`; React ignores updates to unmounted components.
