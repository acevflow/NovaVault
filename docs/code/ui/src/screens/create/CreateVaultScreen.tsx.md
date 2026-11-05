# [`ui/src/screens/create/CreateVaultScreen.tsx`](../../../../../../ui/src/screens/create/CreateVaultScreen.tsx)

## Purpose

The first-run flow, shown when no vault exists. The user chooses a master password (with strength feedback and confirmation); the shell then creates and unlocks the vault and returns the emergency recovery code, which is shown once through [`RecoveryCodeFlow`](../../recovery/RecoveryCodeFlow.tsx.md). The vault is created as soon as the password is accepted, because that is when the shell generates the code.

## Public API

| Name                     | Kind      | Signature | Description |
| ------------------------ | --------- | --------- | ----------- |
| `CreateVaultScreenProps` | interface | `export interface CreateVaultScreenProps { readonly onCreated: () => void; }` | Props of `CreateVaultScreen`. |
| `CreateVaultScreen`      | component | `function CreateVaultScreen({ onCreated }: CreateVaultScreenProps)` | Password step, then the recovery code flow. |

The internal `PasswordStep` component is described below.

## Details

### `CreateVaultScreenProps`

- `onCreated`: called when the vault exists and the user has confirmed the recovery code. [`App`](../../App.tsx.md) passes its `unlocked` handler, which opens the items screen.

### `CreateVaultScreen`

- **State**: `recoveryCode` (`string | null`).
- **Rendered states**:
  - `recoveryCode === null`: `PasswordStep`.
  - Otherwise: `<section className="screen">` with `RecoveryCodeFlow` at heading level 1 and the default intro.
- **Flow**: `PasswordStep` reports the code through its `onCreated`, which stores it. When the recovery flow finishes, `onDone` first sets `recoveryCode` back to `null` ("drop the code from memory before leaving the flow"), then calls the screen's `onCreated()`.

### `PasswordStep` (internal)

```ts
interface PasswordStepProps {
  readonly onCreated: (recoveryCode: string) => void;
}
```

- **State**: `password`, `confirmation`, `submitted` (a submit was attempted), `busy`, `error` (shell error text or `null`).
- **Derived values**:
  - `estimate` from [`usePasswordStrength(password)`](../../../../../../ui/src/password/usePasswordStrength.ts): `null` while empty, still estimating, or for a stale value.
  - `problem` from [`newPasswordProblem`](../../../../../../ui/src/password/newPassword.ts): `'empty'`, `'checking'`, `'weak'` (score below 3, "Strong"), `'mismatch'` or `null`.
  - `problemMessage` from `newPasswordProblemMessage`; `null` for a mismatch, which is shown on the confirmation field instead.
- **Rendered output**: a narrow section with the [`ScreenHeading`](../../components/ScreenHeading.tsx.md) "Create your vault", an explanation, and a `<form noValidate>` containing:
  1. [`NewPasswordFields`](../../password/NewPasswordFields.tsx.md) with `showErrors={submitted}` and `disabled={busy}`.
  2. After a submit attempt, an error [`Alert`](../../components/Alert.tsx.md) with `problemMessage` when there is one.
  3. An error `Alert` with the shell error, if any.
  4. The primary submit [`Button`](../../components/Button.tsx.md) "Create vault", shown as "Creating vault…" with `busy`.
  5. While busy, `<p role="status">Deriving encryption keys. This takes a few seconds.</p>`.
- **Submit**: prevents the default, sets `submitted`, and only if `problem` is `null` calls `create()`:
  1. Sets `busy` and clears `error`.
  2. Calls `createVault(password)`.
  3. On success clears `password` and `confirmation`, then calls `onCreated(code)`. `busy` is not cleared, because this step unmounts.
  4. On failure sets `error` to `errorMessage(reason)` (for example `vault_exists` or `storage`) and clears `busy`.

### Flow summary

- **User flows**: type a password → strength feedback updates → type the confirmation → Create vault → wait for key derivation → save the code (print / checkbox) → retype it → items screen. A weak, empty, still-checking or mismatched password blocks submission with a message and makes no shell call.
- **Accessibility**:
  - Each step's heading takes focus when it appears.
  - Problems and shell errors are announced at once (`role="alert"`); the mismatch is linked to the confirmation field.
  - The key-derivation notice is a polite status message, so the wait is explained.
- **Tauri calls**: `createVault` → `create_vault` with `{ password }`, returning `{ recoveryCode }`. The shell creates the vault file, derives the key slots with Argon2id and leaves the vault unlocked. See [`commands.rs`](../../../../src-tauri/src/commands.rs.md).
- **Security**:
  - The password is sent to the shell once and cleared from state on success. It stays in state if creation fails, so the user can retry.
  - The recovery code is held only in `recoveryCode` and dropped before `onCreated`.
  - Submission is blocked until a fresh estimate for the current value scores at least "Strong"; a stale estimate never counts (see `usePasswordStrength`).

## Dependencies

- **Internal**: [`api/commands.ts`](../../../../../../ui/src/api/commands.ts) (`createVault`), [`api/errors.ts`](../../../../../../ui/src/api/errors.ts) (`errorMessage`), [`Alert`](../../components/Alert.tsx.md), [`Button`](../../components/Button.tsx.md), [`ScreenHeading`](../../components/ScreenHeading.tsx.md), [`NewPasswordFields`](../../password/NewPasswordFields.tsx.md), [`password/newPassword.ts`](../../../../../../ui/src/password/newPassword.ts), [`password/usePasswordStrength.ts`](../../../../../../ui/src/password/usePasswordStrength.ts), [`RecoveryCodeFlow`](../../recovery/RecoveryCodeFlow.tsx.md).
- **External**: `react`: `useState`, the `SubmitEvent` type.

## Used by

- [`App.tsx`](../../App.tsx.md): for the `create` route.

## Tests

- [`CreateVaultScreen.test.tsx`](../../../../../../ui/src/screens/create/CreateVaultScreen.test.tsx):
  - `rejects weak passwords without creating a vault`: a weak password shows the message and `create_vault` is not called.
  - `reports mismatched confirmations on the field`: the mismatch error appears on the confirmation field.
  - `shows shell errors`: a shell failure is shown as an alert.
  - `creates the vault, shows the code once and requires retyping it`: the full flow up to `onCreated`.
  - `can go back to the code from the confirmation step`.

## Notes

- The vault is created and unlocked before the user confirms the code. If the app is closed, or auto-lock reloads the webview, during the recovery code flow, the code is lost from the UI without having been confirmed. The vault still opens with the master password, and the code can be regenerated in settings.
- During the recovery code flow the route is still `create`, so [`AppShell`](../../app/AppShell.tsx.md) shows no navigation and no Lock button although the vault is unlocked.
- The intro text says the master password "cannot be reset by anyone". The recovery code can replace a forgotten password ([`ChangePasswordSection`](../settings/ChangePasswordSection.tsx.md)), which the next step explains.
