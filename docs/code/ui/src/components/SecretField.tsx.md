# [`ui/src/components/SecretField.tsx`](../../../../../ui/src/components/SecretField.tsx)

## Purpose

One secret field of an item (password, notes, note body or secret value), rendered as a row of a description list. The value is hidden by default, fetched from the shell only when the user asks to reveal it, kept only in this component's state, and dropped as soon as it is hidden again or the component unmounts. Each row also has a [`CopyButton`](CopyButton.tsx.md) that copies without revealing.

## Public API

| Name               | Kind      | Signature | Description |
| ------------------ | --------- | --------- | ----------- |
| `SecretFieldProps` | interface | `export interface SecretFieldProps { readonly label: string; readonly itemId: string; readonly field: SecretItemField; readonly hasValue: boolean; readonly multiline?: boolean; readonly onLocked: () => void; }` | Props of `SecretField`. |
| `SecretField`      | component | `function SecretField({ label, itemId, field, hasValue, multiline = false, onLocked, }: SecretFieldProps)` | A hidden-by-default secret field with Reveal/Hide and Copy. |

## Details

### `SecretFieldProps`

- `label`: visible label, for example "Password"; also used in the buttons' accessible names.
- `itemId`: the item the field belongs to.
- `field`: which secret field to reveal ([`SecretItemField`](../../../../../ui/src/api/types.ts): `password`, `notes`, `body` or `value`).
- `hasValue`: whether the field has a value, from the item details' `has…` flags. An empty field offers no Reveal or Copy.
- `multiline`: optional, default `false`; keeps line breaks when revealed.
- `onLocked`: called if the shell reports the vault is locked.

### `SecretField`

- **State**: `value` (`string | null`; `null` means hidden), `busy` (reveal in flight), `error` (last reveal error text, or `null`).
- **Rendered states**, inside `<div className="details__row"><dt>{label}</dt><dd>…</dd></div>`:
  - **Empty** (`hasValue` false): the muted text "Empty"; no buttons.
  - **Hidden**: a mask `••••••••` marked `aria-hidden`, plus visually hidden text "Hidden", then the Reveal and Copy buttons.
  - **Revealed**: the value in a `<span>`, with the `--multiline` modifier when `multiline` is set; the toggle reads "Hide".
  - **Error**: below the content, a `<span role="alert">` with the error text.
- **User flows**:
  - Reveal: clears `error`, sets `busy`, calls `revealItemField`, stores the result in `value`. A `locked` error calls `onLocked()` instead of showing an error.
  - Hide: sets `value` to `null`. Revealing again asks the shell anew; nothing is cached.
  - Copy: delegated to `CopyButton` with the same `itemId`, `field`, `label` and `onLocked`.
- **Accessibility**:
  - The toggle's accessible name is `Reveal <label in lower case>`, with `aria-pressed` set while revealed and `aria-busy` plus `disabled` while fetching.
  - The mask is hidden from assistive technology and replaced with the word "Hidden".
  - Reveal errors use `role="alert"`.
- **Tauri calls**:
  - `revealItemField` → `reveal_item_field` with `{ id, field }`, only after an explicit Reveal press.
  - Through `CopyButton`: `copy_item_field`.
  See [`commands.rs`](../../../src-tauri/src/commands.rs.md).
- **Security**:
  - Nothing is fetched on mount; a secret enters the webview only after a user action.
  - The revealed value lives only in this component's React state. Hiding or unmounting drops the reference. JavaScript strings cannot be zeroized, so the memory is reclaimed by the garbage collector, not wiped.
  - An auto-lock reloads the webview, which discards revealed values.

## Dependencies

- **Internal**: [`CopyButton`](CopyButton.tsx.md), [`api/commands.ts`](../../../../../ui/src/api/commands.ts) (`revealItemField`), [`api/errors.ts`](../../../../../ui/src/api/errors.ts), [`api/types.ts`](../../../../../ui/src/api/types.ts) (`SecretItemField`).
- **External**: `react`: `useState`.

## Used by

- [`ItemDetails`](../screens/items/ItemDetails.tsx.md): one per secret field of the selected item.

## Tests

- [`SecretField.test.tsx`](../../../../../ui/src/components/SecretField.test.tsx):
  - `is hidden by default and fetches nothing`.
  - `reveals the value on demand and drops it when hidden`.
  - `copies through the shell without revealing the value`.
  - `reports a clipboard that is not available`.
  - `says when the field is empty and offers no reveal`.
  - `reports a failure to reveal`.
  - `returns to the unlock screen if the vault is locked`.

## Notes

- The source doc comment says the value is "dropped as soon as it is hidden again or the component unmounts". That holds for the component's reference to it; see the garbage-collection note under **Security**.
