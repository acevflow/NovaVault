# [`ui/src/screens/settings/ChangePasswordSection.tsx`](../../../../../../ui/src/screens/settings/ChangePasswordSection.tsx)

## Purpose

The "Master password" card of the settings screen. It changes the master password after re-authentication with the current password or the recovery code. The shell only re-wraps the vault key, so the change is quick and the recovery code keeps working.

## Public API

| Name                         | Kind      | Signature | Description |
| ---------------------------- | --------- | --------- | ----------- |
| `ChangePasswordSectionProps` | interface | `export interface ChangePasswordSectionProps { readonly onLocked: () => void; }` | Props of `ChangePasswordSection`. |
| `ChangePasswordSection`      | component | `function ChangePasswordSection({ onLocked }: ChangePasswordSectionProps)` | Re-authenticate and set a new master password. |

## Details

### `ChangePasswordSectionProps`

- `onLocked`: called if the shell reports the vault is locked.

### `ChangePasswordSection`

- **State**:
  - `current` from [`useCredential()`](../../../../../../ui/src/screens/settings/useCredential.ts): method, secret and helpers;
  - `password`, `confirmation`: the new password;
  - `submitted`, `busy`, `error`, `done`;
  - a ref to the current-credential input.
- **Derived**: `estimate` from `usePasswordStrength(password)`, then `problem` and `problemMessage` from [`newPassword.ts`](../../../../../../ui/src/password/newPassword.ts), as in [`CreateVaultScreen`](../create/CreateVaultScreen.tsx.md).
- **Rendered output**: `<section className="card">` with `<h2>Master password</h2>`, the note "Changing it re-encrypts only the vault key, so it is quick. Your recovery code keeps working.", and a `<form noValidate>` with:
  1. [`CurrentCredentialFields`](CurrentCredentialFields.tsx.md), disabled while busy.
  2. [`NewPasswordFields`](../../password/NewPasswordFields.tsx.md) labelled "New master password", with `showErrors={submitted}`, disabled while busy.
  3. After a submit attempt, an error [`Alert`](../../components/Alert.tsx.md) with `problemMessage` if any.
  4. An error `Alert` with `error`.
  5. On success, "Master password changed. Use the new password next time you unlock."
  6. A primary submit [`Button`](../../components/Button.tsx.md) "Change master password", shown as "Changing password…" while busy.
- **Submit**:
  1. Prevents the default; sets `submitted`; clears `done` and `error`.
  2. With no current secret, shows "Enter your current master password or recovery code.", focuses that input and stops.
  3. If `problem` is not `null` (empty, checking, weak, mismatch), stops; the messages are already shown.
  4. Otherwise calls `change()`.
- **`change()`**: sets `busy` and calls `changeMasterPassword(current.credential, password)`.
  - Success: clears the current secret, the new password, the confirmation and `submitted`, and sets `done`.
  - `locked`: calls `onLocked()`.
  - `invalid_credentials`: clears the current secret, shows "The current password or recovery code is not correct. Nothing was changed." and focuses the current-credential input.
  - Other errors: clears the current secret, shows `errorMessage`, focuses the input.
  - Always clears `busy`.
- Typing a new password hides the success message.
- **Accessibility**:
  - The card is labelled by its heading.
  - Validation and shell errors are alerts; success is a polite status.
  - Focus moves to the current-credential input after a missing or wrong credential.
  - The new-password fields bring their strength meter and mismatch error.
- **Tauri calls**: `changeMasterPassword` → `change_master_password` with `{ currentCredential: { method, secret }, newPassword }`. See [`commands.rs`](../../../../src-tauri/src/commands.rs.md).
- **Security**:
  - The current secret is cleared after every shell attempt, successful or not, as `useCredential` asks.
  - The new password is cleared on success, and kept after a failure so the user does not retype it.
  - The new password must reach "Strong" before it is sent.

## Dependencies

- **Internal**: [`api/commands.ts`](../../../../../../ui/src/api/commands.ts) (`changeMasterPassword`), [`api/errors.ts`](../../../../../../ui/src/api/errors.ts), [`Alert`](../../components/Alert.tsx.md), [`Button`](../../components/Button.tsx.md), [`NewPasswordFields`](../../password/NewPasswordFields.tsx.md), [`password/newPassword.ts`](../../../../../../ui/src/password/newPassword.ts), [`password/usePasswordStrength.ts`](../../../../../../ui/src/password/usePasswordStrength.ts), [`CurrentCredentialFields`](CurrentCredentialFields.tsx.md), [`useCredential.ts`](../../../../../../ui/src/screens/settings/useCredential.ts).
- **External**: `react`: `useId`, `useRef`, `useState`, the `SubmitEvent` type.

## Used by

- [`SettingsScreen`](SettingsScreen.tsx.md).

## Tests

- [`SettingsScreen.test.tsx`](../../../../../../ui/src/screens/settings/SettingsScreen.test.tsx): `changes the password after re-authentication`, `reports a wrong current password`, `accepts the recovery code as the current credential`.

## Notes

- If the new password fails validation, no shell call is made and the typed current secret stays in state until the next attempt or until the screen unmounts.
- The "re-encrypts only the vault key" wording matches the key-slot design in CLAUDE.md (changing the password only re-wraps the vault key).
