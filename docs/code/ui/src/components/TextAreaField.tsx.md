# [`ui/src/components/TextAreaField.tsx`](../../../../../ui/src/components/TextAreaField.tsx)

## Purpose

A labelled multi-line input with optional help text and trailing content. It is used for notes and long secret values, so spell checking and autocorrection are off by default: notes may hold secrets, which must not reach a dictionary service.

## Public API

| Name                 | Kind      | Signature | Description |
| -------------------- | --------- | --------- | ----------- |
| `TextAreaFieldProps` | interface | `export interface TextAreaFieldProps extends Omit<TextareaHTMLAttributes<HTMLTextAreaElement>, 'id' \| 'aria-describedby'> { readonly label: string; readonly description?: ReactNode; readonly trailing?: ReactNode; }` | Native textarea attributes, minus `id` and `aria-describedby`, plus label, help and trailing content. |
| `TextAreaField`      | component | `function TextAreaField({ label, description, trailing, className, ...textareaProps }: TextAreaFieldProps)` | A labelled textarea. |

## Details

### `TextAreaFieldProps`

- `label`: visible label, linked with `htmlFor`.
- `description`: help text under the input, linked with `aria-describedby`.
- `trailing`: content rendered under the textarea and its description, for example a button that fills it.

### `TextAreaField`

- **State**: `useId()` for the element ids.
- **Rendered output**: a wrapper `<div className="field [className]">` with a `<label>`, a `<textarea>`, the optional description `<div id="<id>-description">`, then `trailing`.
- **Textarea attributes**: `rows={4}`, `spellCheck={false}`, `autoCorrect="off"` and `autoCapitalize="off"` come first, then the caller's props, then `id`, `className="field__input field__textarea"` and `aria-describedby`, which cannot be overridden.
- **Accessibility**: a real label; the description is linked with `aria-describedby`.
- **Tauri calls**: none.
- **Security**: stores no value. The spell-check settings are only defaults (see Notes).

## Dependencies

- **Internal**: none.
- **External**: `react`: `useId`, the `ReactNode` and `TextareaHTMLAttributes` types.

## Used by

- [`ItemForm`](../screens/items/ItemForm.tsx.md): multi-line fields of the item form.

## Tests

- No dedicated test file. [`ItemsScreen.crud.test.tsx`](../../../../../ui/src/screens/items/ItemsScreen.crud.test.tsx) goes through the item form.

## Notes

- [`PasswordField`](PasswordField.tsx.md) places its spell-check overrides after the caller's props "so a caller cannot turn these back on". This component places them before `...textareaProps`, so a caller can turn spell checking back on. No current caller does.
- There is no `error` prop, unlike [`TextField`](TextField.tsx.md), and `aria-invalid` is not left out of the props type.
