# [`ui/src/screens/settings/SettingsScreen.test.tsx`](../../../../../../ui/src/screens/settings/SettingsScreen.test.tsx)

## Purpose

Component tests for [`SettingsScreen`](SettingsScreen.tsx.md) and its sections: master password change, recovery code regeneration, auto-lock timeout, clipboard clear delay and theme. The screen is rendered inside [`ThemeProvider`](../../theme/ThemeProvider.tsx.md) with a fake shell that answers `get_settings` with a 5-minute timeout and a 30-second clear delay.

## Public API

| Test case                                                                | What it checks                                                                                                                                                                                          |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `SettingsScreen: master password` › `changes the password after re-authentication` | `change_master_password` receives `{ currentCredential: { method: 'password', secret: 'old secret' }, newPassword }`, `Master password changed` is shown and the new-password field is emptied. |
| `SettingsScreen: master password` › `reports a wrong current password`    | An `invalid_credentials` rejection shows an alert containing `Nothing was changed`, and the current-password field is emptied.                                                                         |
| `SettingsScreen: master password` › `accepts the recovery code as the current credential` | After choosing the `Recovery code` radio, the credential is sent as `{ method: 'recoveryCode', secret }`.                                                                   |
| `SettingsScreen: recovery code` › `regenerates and confirms a new code`  | The section warns that the old code `stops working`; `regenerate_recovery_code` receives the current password; the `Save your recovery code` heading gets focus; after the code is confirmed the section says `saved and confirmed` and the code is no longer in the page. |
| `SettingsScreen: recovery code` › `keeps the old code when re-authentication fails` | An `invalid_credentials` rejection shows an alert containing `was not changed`.                                                                                                     |
| `SettingsScreen: recovery code` › `requires a current credential before calling the shell` | With an empty credential, an alert starting `Enter your current` appears and `regenerate_recovery_code` is not called.                                                      |
| `SettingsScreen: auto-lock and appearance` › `loads and saves the auto-lock timeout` | The select starts at `5`; saving `15` sends `{ minutes: 15 }` and the status mentions `15 minutes`.                                                                                    |
| `SettingsScreen: auto-lock and appearance` › `returns to the unlock screen if the vault is locked` | A `locked` rejection from `set_auto_lock_timeout` calls `onLocked` once.                                                                                                 |
| `SettingsScreen: auto-lock and appearance` › `loads and saves the clipboard clear delay` | The select starts at `30`; saving `1 minute` sends `{ seconds: 60 }` and the status says `after 1 minute`.                                                                         |
| `SettingsScreen: auto-lock and appearance` › `switches the theme`        | `Dark` sets `data-theme="dark"` on `<html>`; `Match system` removes it.                                                                                                                                 |

## Details

### Helpers

- `STRONG`: the fake passphrase `violet anchor gravel summit 42 lantern`.
- `renderSettings(handlers, onLocked = vi.fn())`: fakes `get_settings` plus the given handlers and renders the screen inside `ThemeProvider`.
- `section(name)`: queries scoped to the `region` with that accessible name.
- `fillChange(user, current)`: fills the current and new passwords, waits up to 10 seconds for the real zxcvbn estimator to rate the new one `Very strong`, and submits.

- **Security**: only fake values (`old secret`, `wrong`, `pw`, `x`, the passphrase and [`TEST_RECOVERY_CODE`](../../test/ipc.ts.md)). The tests check that secret fields are emptied after success and after failure, and that the new recovery code leaves the DOM once confirmed.

## Dependencies

- **Internal**: [`SettingsScreen.tsx`](SettingsScreen.tsx.md), [`ThemeProvider.tsx`](../../theme/ThemeProvider.tsx.md), [`test/ipc.ts`](../../test/ipc.ts.md) (`mockCommands`, `shellError`, `TEST_RECOVERY_CODE`, `CommandHandlers`).
- **External**: `@testing-library/react` (`render`, `screen`, `within`), `@testing-library/user-event`, `vitest`.

## Used by

_Not applicable: test file, run by `npm test`._

## Tests

- Files under test: [`screens/settings/SettingsScreen.tsx`](../../../../../../ui/src/screens/settings/SettingsScreen.tsx) and its sections [`ChangePasswordSection.tsx`](../../../../../../ui/src/screens/settings/ChangePasswordSection.tsx), [`RecoveryCodeSection.tsx`](../../../../../../ui/src/screens/settings/RecoveryCodeSection.tsx), [`AutoLockSection.tsx`](../../../../../../ui/src/screens/settings/AutoLockSection.tsx), [`ClipboardSection.tsx`](../../../../../../ui/src/screens/settings/ClipboardSection.tsx), [`AppearanceSection.tsx`](../../../../../../ui/src/screens/settings/AppearanceSection.tsx), plus [`CurrentCredentialFields.tsx`](../../../../../../ui/src/screens/settings/CurrentCredentialFields.tsx), [`useCredential.ts`](../../../../../../ui/src/screens/settings/useCredential.ts), [`RecoveryCodeFlow.tsx`](../../../../../../ui/src/recovery/RecoveryCodeFlow.tsx) and [`ThemeProvider.tsx`](../../../../../../ui/src/theme/ThemeProvider.tsx).

## Notes

- The master password tests use the real zxcvbn estimator, like [`CreateVaultScreen.test.tsx`](../create/CreateVaultScreen.test.tsx.md).
