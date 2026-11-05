# [`ui/src/password/NewPasswordFields.tsx`](../../../../../ui/src/password/NewPasswordFields.tsx)

## Purpose

The pair of inputs used to choose a new master password: the password, with a [`StrengthMeter`](StrengthMeter.tsx.md) as its description, and a confirmation field that shows a mismatch error once the user has tried to submit. It is controlled: the parent owns both values and the strength estimate, and decides when to show errors.

## Public API

| Name                     | Kind      | Signature | Description |
| ------------------------ | --------- | --------- | ----------- |
| `NewPasswordFieldsProps` | interface | `export interface NewPasswordFieldsProps { readonly password: string; readonly confirmation: string; readonly estimate: StrengthEstimate \| null; readonly onPasswordChange: (value: string) => void; readonly onConfirmationChange: (value: string) => void; readonly label?: string; readonly showErrors: boolean; readonly disabled?: boolean; }` | Props of `NewPasswordFields`. |
| `NewPasswordFields`      | component | `function NewPasswordFields({ password, confirmation, estimate, onPasswordChange, onConfirmationChange, label = 'Master password', showErrors, disabled = false, }: NewPasswordFieldsProps)` | New password with strength feedback and confirmation. |

## Details

### `NewPasswordFieldsProps`

- `password`, `confirmation`: the current values (controlled).
- `estimate`: strength of `password`, from [`usePasswordStrength`](../../../../../ui/src/password/usePasswordStrength.ts); `null` while empty or still being estimated.
- `onPasswordChange`, `onConfirmationChange`: called with the new value on every keystroke.
- `label`: label of the first field; default "Master password". The second is labelled `Confirm <label in lower case>`.
- `showErrors`: whether to show the mismatch error. Parents set it after a submit attempt.
- `disabled`: optional, default `false`; disables both fields, for example while a request runs.

### `NewPasswordFields`

- **State**: none; fully controlled.
- **Rendered output**: two [`PasswordField`](../components/PasswordField.tsx.md)s, both `required`, `autoComplete="new-password"` and `disabled` as given.
  1. The password field, with `<StrengthMeter estimate={estimate} hasPassword={password !== ''} />` as its `description`.
  2. The confirmation field, with the error "The passwords do not match." when `showErrors` is true and the two values differ.
- **User flows**: typing updates the parent through the callbacks. The mismatch error appears only after the parent sets `showErrors`, and disappears as soon as the values match.
- **Accessibility**: the strength meter is linked to the password input with `aria-describedby` through `TextField`'s `description`, and announces changes itself (see [`StrengthMeter`](StrengthMeter.tsx.md)). The mismatch error is linked to the confirmation input and marks it `aria-invalid`.
- **Tauri calls**: none. Submitting is up to the parent.
- **Security**: `autoComplete="new-password"` tells the platform this is a new credential. Both inputs inherit `PasswordField`'s disabled spell checking and autocorrection. Values live only in the parent's state.

## Dependencies

- **Internal**: [`PasswordField`](../components/PasswordField.tsx.md), [`StrengthMeter`](StrengthMeter.tsx.md), [`password/strength.ts`](../../../../../ui/src/password/strength.ts) (`StrengthEstimate` type).
- **External**: none.

## Used by

- [`CreateVaultScreen`](../screens/create/CreateVaultScreen.tsx.md): the master password at vault creation.
- [`ChangePasswordSection`](../screens/settings/ChangePasswordSection.tsx.md): the new master password.

## Tests

- No dedicated test file. Covered through its parents:
  - [`CreateVaultScreen.test.tsx`](../../../../../ui/src/screens/create/CreateVaultScreen.test.tsx): `rejects weak passwords without creating a vault`, `reports mismatched confirmations on the field`.
  - [`SettingsScreen.test.tsx`](../../../../../ui/src/screens/settings/SettingsScreen.test.tsx): `changes the password after re-authentication`.

## Notes

- The component only shows the mismatch. Which problem blocks submission (empty, still checking, too weak, mismatch) is decided by [`newPasswordProblem`](../../../../../ui/src/password/newPassword.ts) in the parent.
