# [`ui/src/password/newPassword.ts`](../../../../../ui/src/password/newPassword.ts)

## Purpose

Validation of a new master password, shared by vault creation and the password change. It decides whether the password can be submitted yet and, if not, gives the reason and the message to show.

## Public API

| Name                        | Kind     | Signature                                                                                                                         | Description                                           |
| --------------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `NewPasswordProblem`        | type     | `export type NewPasswordProblem = 'empty' \| 'checking' \| 'weak' \| 'mismatch'`                                                  | Why a new password cannot be used yet.                |
| `newPasswordProblem`        | function | `export function newPasswordProblem(password: string, confirmation: string, estimate: StrengthEstimate \| null): NewPasswordProblem \| null` | The first problem, or `null` if the password can be used. |
| `newPasswordProblemMessage` | function | `export function newPasswordProblemMessage(problem: NewPasswordProblem \| null): string \| null`                                 | Message for a problem not shown next to a field.      |

## Details

### `newPasswordProblem`

Checks in this order and returns the first problem:

1. `'empty'`: `password` is `''`.
2. `'checking'`: `estimate` is `null` (still loading, stale, or failed to load).
3. `'weak'`: `estimate.score` is below [`MIN_ACCEPTED_SCORE`](strength.ts.md) (3).
4. `'mismatch'`: `password` differs from `confirmation`.
5. Otherwise `null`.

- **Parameters**: the new password, its confirmation, and the estimate from [`usePasswordStrength`](usePasswordStrength.ts.md).
- **Security**: strength is checked before the match, so a weak password is reported as weak even when both fields agree. A missing estimate blocks submission.

### `newPasswordProblemMessage`

| Problem                | Message                                                                                                         |
| ---------------------- | --------------------------------------------------------------------------------------------------------------- |
| `'empty'`              | `Enter a master password.`                                                                                      |
| `'checking'`           | `Password strength is still being checked. Try again in a moment.`                                              |
| `'weak'`               | `The password must rate at least “Strong”. Make it longer or add unrelated words.` (the label comes from `STRENGTH_LABELS[MIN_ACCEPTED_SCORE]`) |
| `'mismatch'`, `null`   | `null`: a mismatch is shown on the confirmation field instead.                                                  |

## Dependencies

- **Internal**: [`strength.ts`](strength.ts.md) (`MIN_ACCEPTED_SCORE`, `STRENGTH_LABELS`, `StrengthEstimate`).
- **External**: none.

## Used by

- [`CreateVaultScreen.tsx`](../screens/create/CreateVaultScreen.tsx.md) and [`ChangePasswordSection.tsx`](../screens/settings/ChangePasswordSection.tsx.md): to block submission and show the message.

## Tests

- No unit test. [`CreateVaultScreen.test.tsx`](../screens/create/CreateVaultScreen.test.tsx.md) checks that a weak password is refused, through the screen.

## Notes

- None.
