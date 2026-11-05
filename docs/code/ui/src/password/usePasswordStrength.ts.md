# [`ui/src/password/usePasswordStrength.ts`](../../../../../ui/src/password/usePasswordStrength.ts)

## Purpose

A React hook that re-estimates a new master password's strength as the user types. It returns an estimate only when that estimate matches the current value, so a stale result can never enable submitting a weaker password.

## Public API

| Name                  | Kind | Signature                                                                       | Description                                  |
| --------------------- | ---- | ------------------------------------------------------------------------------- | -------------------------------------------- |
| `usePasswordStrength` | hook | `export function usePasswordStrength(password: string): StrengthEstimate \| null` | Strength of `password`, or `null` if not ready. |

## Details

### `usePasswordStrength`

1. Keeps `{ password, estimate }` in state, `null` at first.
2. An effect on `password` does nothing for `''`. For any other value it calls [`estimateStrength`](strength.ts.md) and stores the result together with the password it was computed for.
3. The effect's cleanup sets a `current` flag to `false`, so an answer for a value the user has already changed is ignored.
4. Returns the stored estimate only if its password equals the current `password`; otherwise `null`.

- **Parameters**: `password`, the current value of the new-password field.
- **Returns**: the estimate, or `null` while the password is empty or its estimate is not ready.
- **Errors**: none. If estimation fails (the dictionaries could not load), the failure is swallowed and the hook keeps returning `null`.
- **Security**: the result state holds a copy of the password so the hook can compare it, alongside the copy the form already holds. Because `null` means "checking" to [`newPasswordProblem`](newPassword.ts.md), a password can never be submitted unless its own estimate is ready and strong enough; a failed load blocks submission instead of letting it through.
- **Edge cases and invariants**: clearing the field returns `null` at once without starting an estimate. The previous result stays in state but no longer matches.

## Dependencies

- **Internal**: [`strength.ts`](strength.ts.md) (`estimateStrength`, `StrengthEstimate`).
- **External**: `react` (`useEffect`, `useState`).

## Used by

- [`CreateVaultScreen.tsx`](../screens/create/CreateVaultScreen.tsx.md) and [`ChangePasswordSection.tsx`](../screens/settings/ChangePasswordSection.tsx.md): the estimate for their new-password fields.

## Tests

- No unit test. [`CreateVaultScreen.test.tsx`](../screens/create/CreateVaultScreen.test.tsx.md) covers it through the screen.

## Notes

- After a failed load, the hook does not retry for the same value. It retries only when the password changes, because only then does the effect run again.
