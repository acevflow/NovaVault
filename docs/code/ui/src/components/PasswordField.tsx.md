# [`ui/src/components/PasswordField.tsx`](../../../../../ui/src/components/PasswordField.tsx)

## Purpose

A password input with a Show/Hide toggle, built on [`TextField`](TextField.tsx.md). It forces spell checking, autocorrection and auto-capitalization off so the browser never sends the value to a dictionary service or learns it.

## Public API

| Name                 | Kind       | Signature | Description |
| -------------------- | ---------- | --------- | ----------- |
| `PasswordFieldProps` | type alias | `export type PasswordFieldProps = Omit<TextFieldProps, 'type' \| 'trailing'>;` | `TextField` props without `type` and `trailing`, which this component sets. |
| `PasswordField`      | component  | `function PasswordField({ label, ...rest }: PasswordFieldProps)` | A password input with a visibility toggle. |

## Details

### `PasswordField`

- **Props**: everything `TextField` accepts except `type` and `trailing`. `label` also builds the toggle's accessible name.
- **State**: `visible` (`useState(false)`): whether the value is shown as plain text.
- **Rendered output**: a `TextField` with:
  - `type="text"` when visible, `type="password"` otherwise;
  - `spellCheck={false}`, `autoCorrect="off"`, `autoCapitalize="off"`, placed after `rest` so a caller cannot turn them back on;
  - a trailing `<button type="button" className="field__toggle">` showing "Show" or "Hide".
- **User flows**: pressing the toggle flips `visible`. The value is untouched; only the input's `type` changes.
- **Accessibility**: the toggle has `aria-pressed={visible}` and `aria-label="Show <label in lower case>"` (for example "Show master password"), so it is announced as a toggle button whose pressed state means "shown".
- **Tauri calls**: none.
- **Security**: the component keeps no copy of the value. Visibility is local state and goes back to hidden whenever the field remounts.

## Dependencies

- **Internal**: [`TextField`](TextField.tsx.md).
- **External**: `react`: `useState`.

## Used by

[`NewPasswordFields`](../password/NewPasswordFields.tsx.md), [`UnlockScreen`](../screens/unlock/UnlockScreen.tsx.md), [`ItemForm`](../screens/items/ItemForm.tsx.md), [`CurrentCredentialFields`](../screens/settings/CurrentCredentialFields.tsx.md).

## Tests

- No dedicated test file. Screen tests fill password inputs by label, for example [`UnlockScreen.test.tsx`](../../../../../ui/src/screens/unlock/UnlockScreen.test.tsx).

## Notes

- The toggle's accessible name stays "Show …" while its visible text changes to "Hide". With `aria-pressed` this is the standard toggle-button pattern (fixed name, changing state), but the accessible name and the visible text differ in the pressed state.
