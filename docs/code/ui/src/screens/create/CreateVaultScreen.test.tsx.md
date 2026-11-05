# [`ui/src/screens/create/CreateVaultScreen.test.tsx`](../../../../../../ui/src/screens/create/CreateVaultScreen.test.tsx)

## Purpose

Component tests for [`CreateVaultScreen`](CreateVaultScreen.tsx.md). They cover the password step (strength and confirmation checks with the real zxcvbn estimator), shell errors, and the recovery code steps: showing, printing, confirming the code was saved, and retyping it.

## Public API

| Test case                                                         | What it checks                                                                                                                                                                                                                                                                         |
| ----------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rejects weak passwords without creating a vault`                 | `password1` rates `Very weak`; submitting shows an alert containing `must rate at least “Strong”`, and no command is called.                                                                                                                                                            |
| `reports mismatched confirmations on the field`                   | A strong password with a different confirmation sets the confirmation field's accessible description to `The passwords do not match.`, and no command is called.                                                                                                                       |
| `shows shell errors`                                              | A `vault_exists` rejection shows an alert containing `A vault already exists`.                                                                                                                                                                                                        |
| `creates the vault, shows the code once and requires retyping it` | `create_vault` is called once with `{ password }`. The `Save your recovery code` heading gets focus, the list items joined with `-` equal the code, and `Print` calls `window.print` once. `Continue` stays disabled until the "I have saved my recovery code" checkbox is ticked. A code that differs in its last character sets `aria-invalid="true"` and does not finish. The right code in lower case with spaces calls `onCreated` once, after which the code is no longer in the page. |
| `can go back to the code from the confirmation step`              | `Show the code again` on the retype step brings back the `Recovery code` list.                                                                                                                                                                                                         |

## Details

### Helpers

- `STRONG`: the fake passphrase `violet anchor gravel summit 42 lantern`, which rates `Very strong`.
- `waitForStrength(label: RegExp)`: waits up to 10 seconds for the strength text, because zxcvbn and its dictionaries load lazily on first use.
- `choosePassword(password: string, confirmation = password)`: types into `Master password` and `Confirm master password` and returns the `userEvent` instance.

### Setup

- [`mockCommands`](../../test/ipc.ts.md) fakes the shell. `mockCommands({})` makes any call reject, so the "no command is called" checks are strict.
- `window.print` is replaced with `vi.spyOn(...).mockImplementation`, and `restoreMocks` in [`vite.config.ts`](../../../vite.config.ts.md) restores it afterwards.
- **Security**: only fake values ([`TEST_RECOVERY_CODE`](../../test/ipc.ts.md) and the passphrase above). The last check, `document.body` no longer containing `ABCD`, verifies that the recovery code leaves the DOM when the flow finishes.

## Dependencies

- **Internal**: [`CreateVaultScreen.tsx`](CreateVaultScreen.tsx.md), [`test/ipc.ts`](../../test/ipc.ts.md) (`mockCommands`, `shellError`, `TEST_RECOVERY_CODE`).
- **External**: `@testing-library/react`, `@testing-library/user-event`, `vitest`.

## Used by

_Not applicable: test file, run by `npm test`._

## Tests

- File under test: [`screens/create/CreateVaultScreen.tsx`](../../../../../../ui/src/screens/create/CreateVaultScreen.tsx), and through it [`NewPasswordFields.tsx`](../../../../../../ui/src/password/NewPasswordFields.tsx), [`StrengthMeter.tsx`](../../../../../../ui/src/password/StrengthMeter.tsx), [`newPassword.ts`](../../../../../../ui/src/password/newPassword.ts), [`usePasswordStrength.ts`](../../../../../../ui/src/password/usePasswordStrength.ts), [`strength.ts`](../../../../../../ui/src/password/strength.ts), [`RecoveryCodeFlow.tsx`](../../../../../../ui/src/recovery/RecoveryCodeFlow.tsx) and [`recoveryCode.ts`](../../../../../../ui/src/recovery/recoveryCode.ts).

## Notes

- These tests load the real zxcvbn dictionaries instead of mocking the estimator, hence the 10-second wait.
