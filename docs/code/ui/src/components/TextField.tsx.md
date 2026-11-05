# [`ui/src/components/TextField.tsx`](../../../../../ui/src/components/TextField.tsx)

## Purpose

A labelled single-line input with optional help text, error text and trailing content on the same row. It is the base of every text and password input in the UI, and sets `id`, `aria-describedby` and `aria-invalid` itself so callers cannot get them wrong.

## Public API

| Name             | Kind      | Signature | Description |
| ---------------- | --------- | --------- | ----------- |
| `TextFieldProps` | interface | `export interface TextFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'id' \| 'aria-describedby' \| 'aria-invalid'> { readonly label: string; readonly description?: ReactNode; readonly error?: string \| undefined; readonly trailing?: ReactNode; readonly ref?: Ref<HTMLInputElement> \| undefined; }` | Native input attributes, minus the three it manages, plus label, help, error, trailing content and ref. |
| `TextField`      | component | `function TextField({ label, description, error, trailing, className, ref, ...inputProps }: TextFieldProps)` | A labelled input with help and error text. |

## Details

### `TextFieldProps`

- `label`: visible label text, linked to the input with `htmlFor`.
- `description`: help text under the input, linked with `aria-describedby`.
- `error`: error text. When set, the input is marked invalid and the error is linked with `aria-describedby`.
- `trailing`: content after the input on the same row, for example a Show/Hide or Copy button.
- `ref`: ref to the `<input>`, for focus management (for example focusing the first invalid field).
- `id`, `aria-describedby` and `aria-invalid` are left out of the accepted attributes because the component generates them.

### `TextField`

- **State**: `useId()` for a unique base id.
- **Rendered output**:
  1. A wrapper `<div className="field [className]">`.
  2. `<label htmlFor={id}>{label}</label>`.
  3. `<div className="field__control">` with the `<input>` (remaining props forwarded first, then `ref`, `id`, `className="field__input"`, `aria-describedby`, `aria-invalid`) followed by `trailing`.
  4. If `description` is set: `<div id="<id>-description" className="field__description">`.
  5. If `error` is set: `<p id="<id>-error" className="field__error">`.
- **Behavior**: `aria-describedby` lists the description and error ids that exist, in that order, and is omitted when neither exists. `aria-invalid` is `true` only when `error` is defined.
- **Accessibility**: a real `<label>` gives the input its accessible name; help and error text are read with it. The error is not a live region: it is announced when the input gets focus, which is why forms move focus to the first invalid field.
- **Tauri calls**: none.
- **Security**: stores no value; the caller controls the input. The caller's `className` goes on the wrapper; the input's class is fixed.

## Dependencies

- **Internal**: none.
- **External**: `react`: `useId`, the `InputHTMLAttributes`, `ReactNode` and `Ref` types.

## Used by

[`PasswordField`](PasswordField.tsx.md), [`RecoveryCodeFlow`](../recovery/RecoveryCodeFlow.tsx.md), [`UnlockScreen`](../screens/unlock/UnlockScreen.tsx.md), [`ItemForm`](../screens/items/ItemForm.tsx.md), [`ItemList`](../screens/items/ItemList.tsx.md), [`CurrentCredentialFields`](../screens/settings/CurrentCredentialFields.tsx.md).

## Tests

- No dedicated test file. Form tests find inputs by label and check validation through it, for example [`CreateVaultScreen.test.tsx`](../../../../../ui/src/screens/create/CreateVaultScreen.test.tsx).

## Notes

- Spell checking is not turned off here. [`PasswordField`](PasswordField.tsx.md) does that for passwords; plain fields such as titles and usernames keep the browser default unless the caller sets it.
