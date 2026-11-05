# [`ui/src/components/Button.tsx`](../../../../../ui/src/components/Button.tsx)

## Purpose

The app's standard button. It adds a visual variant and a busy state on top of the native `<button>`, and defaults to `type="button"` so it never submits a form by accident.

## Public API

| Name            | Kind       | Signature | Description |
| --------------- | ---------- | --------- | ----------- |
| `ButtonVariant` | type alias | `export type ButtonVariant = 'primary' \| 'secondary' \| 'danger';` | Visual weight of a `Button`. |
| `ButtonProps`   | interface  | `export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> { readonly variant?: ButtonVariant; readonly busy?: boolean; readonly ref?: Ref<HTMLButtonElement> \| undefined; }` | Native button attributes plus variant, busy and ref. |
| `Button`        | component  | `function Button({ variant = 'secondary', busy = false, disabled, className, type = 'button', ...rest }: ButtonProps)` | A styled button with a busy state. |

## Details

### `ButtonProps`

- All native `<button>` attributes are accepted and forwarded.
- `variant`: `'secondary'` by default. Maps to the class `button--<variant>`.
- `busy`: shows the button as working and disables it. The source comment asks callers to change the label to say what is happening (for example "Unlocking…").
- `ref`: a ref to the `<button>` element, for focus management. It travels in `rest` as an ordinary prop (React 19 accepts `ref` as a prop on function components).

### `Button`

- **State**: none.
- **Rendered output**: `<button>` with every forwarded prop from `rest`, then `type` (default `'button'`), `className` (`button button--<variant>` plus the caller's `className`, if any), `disabled` and `aria-busy`. These four are set after `rest`, so they cannot be overridden through it.
- **Behavior**:
  - `disabled` is `true` when the caller passes `disabled={true}` or `busy` is true.
  - `aria-busy` is `true` while busy and omitted otherwise.
- **Accessibility**: a native button, so keyboard activation and focus work without extra code. `aria-busy` tells assistive technology the control is working; the caller's changed label carries the meaning.
- **Tauri calls**: none.

## Dependencies

- **Internal**: none.
- **External**: `react`: the `ButtonHTMLAttributes` and `Ref` types.

## Used by

[`AppShell`](../app/AppShell.tsx.md), [`RecoveryCodeFlow`](../recovery/RecoveryCodeFlow.tsx.md), [`StatusScreens`](../screens/StatusScreens.tsx.md), [`CreateVaultScreen`](../screens/create/CreateVaultScreen.tsx.md), [`UnlockScreen`](../screens/unlock/UnlockScreen.tsx.md), [`ItemsScreen`](../screens/items/ItemsScreen.tsx.md), [`ItemDetails`](../screens/items/ItemDetails.tsx.md), [`ItemForm`](../screens/items/ItemForm.tsx.md), [`ConfirmDelete`](../screens/items/ConfirmDelete.tsx.md), [`AttachmentList`](../screens/items/AttachmentList.tsx.md), [`AutoLockSection`](../screens/settings/AutoLockSection.tsx.md), [`ClipboardSection`](../screens/settings/ClipboardSection.tsx.md), [`ChangePasswordSection`](../screens/settings/ChangePasswordSection.tsx.md), [`RecoveryCodeSection`](../screens/settings/RecoveryCodeSection.tsx.md).

## Tests

- No dedicated test file. Screen tests exercise it through button roles and names.

## Notes

- Not every button uses this component. The inline toggles in [`PasswordField`](PasswordField.tsx.md), [`SecretField`](SecretField.tsx.md) and [`CopyButton`](CopyButton.tsx.md), and the navigation entries in [`AppShell`](../app/AppShell.tsx.md), render a plain `<button>` and repeat the `disabled`/`aria-busy` pattern by hand.
