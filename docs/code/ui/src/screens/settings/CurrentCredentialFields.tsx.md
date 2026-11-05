# [`ui/src/screens/settings/CurrentCredentialFields.tsx`](../../../../../../ui/src/screens/settings/CurrentCredentialFields.tsx)

## Purpose

The re-authentication fields shown before a key slot change (changing the master password or regenerating the recovery code). The user confirms with either the current master password or the recovery code, for users who unlocked with the code because they forgot the password. It is controlled; the state lives in [`useCredential`](../../../../../../ui/src/screens/settings/useCredential.ts) in the parent.

## Public API

| Name                           | Kind      | Signature | Description |
| ------------------------------ | --------- | --------- | ----------- |
| `CurrentCredentialFieldsProps` | interface | `export interface CurrentCredentialFieldsProps { readonly method: CredentialMethod; readonly secret: string; readonly onMethodChange: (method: CredentialMethod) => void; readonly onSecretChange: (secret: string) => void; readonly disabled?: boolean; readonly inputRef?: Ref<HTMLInputElement>; }` | Props of `CurrentCredentialFields`. |
| `CurrentCredentialFields`      | component | `function CurrentCredentialFields({ method, secret, onMethodChange, onSecretChange, disabled = false, inputRef, }: CurrentCredentialFieldsProps)` | Method choice and the matching secret input. |

## Details

### `CurrentCredentialFieldsProps`

- `method`: `'password'` or `'recoveryCode'`.
- `secret`: the typed secret.
- `onMethodChange`: called when the user picks the other method. Per the prop's comment the parent should clear the secret; `useCredential().setMethod` does.
- `onSecretChange`: called on every keystroke.
- `disabled`: optional, default `false`; disables the radios and the input.
- `inputRef`: ref to the secret input, so the parent can focus it after an error.

### `CurrentCredentialFields`

- **State**: a `useId` for the radio group name.
- **Rendered output**:
  1. A `<fieldset>` with legend "Confirm it is you with" and two radios, "Master password" and "Recovery code".
  2. For `password`: a [`PasswordField`](../../components/PasswordField.tsx.md) "Current master password" with `autoComplete="current-password"`.
  3. For `recoveryCode`: a [`TextField`](../../components/TextField.tsx.md) "Current recovery code" with class `field--code`, `autoComplete="off"`, `spellCheck={false}`, `autoCorrect="off"`, `autoCapitalize="characters"`.
  The two inputs have different `key`s, so switching method remounts the input; the password's Show toggle resets as well.
- **User flows**: pick a method → type the secret → the parent submits.
- **Accessibility**: a native radio group with a legend; the input is labelled and receives focus from the parent after an error.
- **Tauri calls**: none; the parent sends `{ method, secret }` as the `currentCredential`.
- **Security**:
  - The component stores no value.
  - Switching method clears the secret in `useCredential`, so a password never travels as a recovery code or the other way round.
  - Spell checking is off for both inputs (forced by `PasswordField` for the password, set here for the code).

## Dependencies

- **Internal**: [`PasswordField`](../../components/PasswordField.tsx.md), [`TextField`](../../components/TextField.tsx.md), [`api/types.ts`](../../../../../../ui/src/api/types.ts) (`CredentialMethod`).
- **External**: `react`: `useId`, the `Ref` type.

## Used by

- [`ChangePasswordSection`](ChangePasswordSection.tsx.md).
- [`RecoveryCodeSection`](RecoveryCodeSection.tsx.md).

## Tests

- [`SettingsScreen.test.tsx`](../../../../../../ui/src/screens/settings/SettingsScreen.test.tsx): `accepts the recovery code as the current credential`, `reports a wrong current password`, `requires a current credential before calling the shell`.

## Notes

- The recovery code input has no description or placeholder about the accepted format, unlike the confirmation step in [`RecoveryCodeFlow`](../../recovery/RecoveryCodeFlow.tsx.md) ("Dashes, spaces and letter case do not matter.").
- [`UnlockScreen`](../unlock/UnlockScreen.tsx.md) implements its own method switch instead of reusing this component.
