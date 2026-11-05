# [`ui/src/screens/settings/useCredential.ts`](../../../../../../ui/src/screens/settings/useCredential.ts)

## Purpose

A React hook that holds the re-authentication credential typed in a settings form: the current master password or the recovery code. The master password change and the recovery code regeneration both use it with [`CurrentCredentialFields`](CurrentCredentialFields.tsx.md).

## Public API

| Name              | Kind      | Signature                                                    | Description                                    |
| ----------------- | --------- | ------------------------------------------------------------ | ---------------------------------------------- |
| `CredentialState` | interface | `export interface CredentialState { readonly method: CredentialMethod; readonly secret: string; readonly credential: Credential; readonly setMethod: (method: CredentialMethod) => void; readonly setSecret: (secret: string) => void; readonly clear: () => void; }` | State and setters of a re-authentication form. |
| `useCredential`   | hook      | `export function useCredential(): CredentialState`           | Creates that state.                            |

## Details

### `CredentialState`

- `method`: which secret the user confirms with, `'password'` or `'recoveryCode'`.
- `secret`: the typed secret.
- `credential`: `{ method, secret }`, ready to send to the shell.
- `setMethod`: switches the method and clears the secret.
- `setSecret`: replaces the typed secret.
- `clear`: empties the secret.

### `useCredential`

1. `method` starts as `'password'` and `secret` as `''`.
2. `setMethod` and `clear` are stable `useCallback` functions; `setSecret` is React's own state setter.
3. Returns a new object on every render, with a new `credential` object built from the current state.

- **Security**: `secret` is a master password or a recovery code. Switching the method clears it, so a password typed in the password field is never sent as a recovery code, or the other way round. The hook comment requires callers to call `clear` after every attempt; [`ChangePasswordSection`](ChangePasswordSection.tsx.md) and [`RecoveryCodeSection`](RecoveryCodeSection.tsx.md) do so on success and on failure, except on a `locked` error, where they call `onLocked` and the screen is replaced by the unlock screen. Clearing only drops the reference from React state; JavaScript strings cannot be zeroized.
- **Edge cases and invariants**: the hook does not validate the secret. An empty secret is checked by the sections before they call the shell.

## Dependencies

- **Internal**: [`api/types.ts`](../../api/types.ts.md) (`Credential`, `CredentialMethod`).
- **External**: `react` (`useCallback`, `useState`).

## Used by

- [`ChangePasswordSection.tsx`](ChangePasswordSection.tsx.md) and [`RecoveryCodeSection.tsx`](RecoveryCodeSection.tsx.md): the current credential for `changeMasterPassword` and `regenerateRecoveryCode`.

## Tests

- No unit test. [`SettingsScreen.test.tsx`](SettingsScreen.test.tsx.md) covers it through both sections: the current password and the recovery code as credentials, and the field being cleared after a failed attempt.

## Notes

- `CredentialState` is exported, but no other file imports the type by name.
