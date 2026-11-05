# [`ui/src/screens/unlock/UnlockScreen.test.tsx`](../../../../../../ui/src/screens/unlock/UnlockScreen.test.tsx)

## Purpose

Component tests for [`UnlockScreen`](UnlockScreen.tsx.md): unlocking with the master password or the recovery code, keyboard navigation between the two tabs, error messages, and the empty-field check. A last test runs the full [`App`](../../App.tsx.md) to go from the unlock screen to the vault and back by locking.

## Public API

| Test case                                                          | What it checks                                                                                                                                                                       |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `UnlockScreen` › `unlocks with the master password`                | The `Unlock your vault` heading has focus; typing a password and pressing Enter sends `{ credential: { method: 'password', secret } }` and calls `onUnlocked` once.                  |
| `UnlockScreen` › `unlocks with the recovery code from its tab`     | `Master password` is the selected tab; ArrowRight moves focus to and selects `Recovery code`; the tab panel is named `Recovery code`; the code is sent with `method: 'recoveryCode'`. |
| `UnlockScreen` › `reports a wrong password and clears the field`   | An `invalid_credentials` rejection shows `That master password is not correct.`, empties and refocuses the field, and does not call `onUnlocked`.                                    |
| `UnlockScreen` › `explains %s errors` (3 cases)                    | Through the recovery code tab: `invalid_recovery_code` mentions `Check it for typos`, `unsupported_vault` mentions `not supported by this version`, `corrupted` mentions `damaged`.  |
| `UnlockScreen` › `asks for a secret before calling the shell`      | Clicking `Unlock` with an empty field shows `Enter your master password.` and calls nothing.                                                                                         |
| `lock and unlock in the app` › `goes from unlock to the vault and back on lock` | With `vault_status` reporting a locked vault, unlocking shows `Your vault`; `Lock` returns to `Unlock your vault` with an empty password field.                            |

## Details

### `INPUT`

- `{ selector: 'input' }`, passed to `getByLabelText`. The tab panel is labelled by its tab, so a label query without it would also match the panel; this restricts it to the input.

- **Security**: only fake values (`my secret`, `wrong`, `pw`, `abcd` and [`TEST_RECOVERY_CODE`](../../test/ipc.ts.md)). The tests check that a wrong secret is cleared from the field, and that the field is empty after locking.

## Dependencies

- **Internal**: [`UnlockScreen.tsx`](UnlockScreen.tsx.md), [`App.tsx`](../../App.tsx.md), [`test/ipc.ts`](../../test/ipc.ts.md) (`mockCommands`, `shellError`, `TEST_RECOVERY_CODE`).
- **External**: `@testing-library/react`, `@testing-library/user-event`, `vitest`.

## Used by

_Not applicable: test file, run by `npm test`._

## Tests

- Files under test: [`screens/unlock/UnlockScreen.tsx`](../../../../../../ui/src/screens/unlock/UnlockScreen.tsx), [`api/errors.ts`](../../../../../../ui/src/api/errors.ts) (the three explained messages), and, in the last test, [`App.tsx`](../../../../../../ui/src/App.tsx), [`AppShell.tsx`](../../../../../../ui/src/app/AppShell.tsx) and [`router.ts`](../../../../../../ui/src/app/router.ts).

## Notes

- The wrong-password message, `That master password is not correct.`, is the screen's own text. It differs from the generic `invalid_credentials` message in [`errors.ts`](../../api/errors.ts.md) because the screen knows which tab was used.
- The last test reaches `home` through the `unlocked` router action, which [`router.test.ts`](../../app/router.test.ts.md) does not test directly.
