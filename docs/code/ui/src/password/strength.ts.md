# [`ui/src/password/strength.ts`](../../../../../ui/src/password/strength.ts)

## Purpose

Estimates master password strength with zxcvbn-ts. The estimator and its dictionaries load lazily on first use, so they never slow down startup or the unlock screen. Only the score and feedback leave the module. A new master password must reach `MIN_ACCEPTED_SCORE`.

## Public API

| Name                 | Kind      | Signature                                                                                                              | Description                                        |
| -------------------- | --------- | ---------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| `StrengthScore`      | type      | `export type StrengthScore = Score`                                                                                    | zxcvbn score, 0 to 4.                              |
| `MIN_ACCEPTED_SCORE` | constant  | `export const MIN_ACCEPTED_SCORE: StrengthScore = 3`                                                                   | Lowest score accepted for a master password.       |
| `StrengthEstimate`   | interface | `export interface StrengthEstimate { readonly score: StrengthScore; readonly warning: string \| null; readonly suggestions: readonly string[]; }` | Result of `estimateStrength`. |
| `STRENGTH_LABELS`    | constant  | `export const STRENGTH_LABELS: Readonly<Record<StrengthScore, string>>`                                               | Display label for each score.                      |
| `estimateStrength`   | function  | `export async function estimateStrength(password: string, userInputs: readonly string[] = ['novavault', 'nova', 'vault']): Promise<StrengthEstimate>` | Estimates how hard `password` is to guess. |

## Details

### `StrengthScore`

- Re-exports zxcvbn's `Score` (`0 | 1 | 2 | 3 | 4`): 0 is too guessable, 4 is very unguessable.

### `MIN_ACCEPTED_SCORE`

- `3`, "safely unguessable": about 10^10 guesses according to the source comment. Combined with the Argon2id cost of every guess, the comment says this resists offline attacks.

### `STRENGTH_LABELS`

- `0: 'Very weak'`, `1: 'Weak'`, `2: 'Fair'`, `3: 'Strong'`, `4: 'Very strong'`.

### `estimateStrength`

1. If no estimator is cached, starts `loadEstimator()` and stores the promise in a module-level variable (`??=`), so concurrent calls share one load.
2. `loadEstimator` (private) imports `@zxcvbn-ts/core`, `@zxcvbn-ts/language-common` and `@zxcvbn-ts/language-en` in parallel with dynamic `import()`. It builds a `ZxcvbnFactory` with the merged common and English dictionaries, the common keyboard adjacency graphs and the English translations.
3. If loading fails, the cache is reset to `null` before the error is rethrown, so the next call retries instead of reusing the failure.
4. Calls `check(password, [...userInputs])` and returns only `score`, `feedback.warning` and `feedback.suggestions`.

- **Parameters**: `password`, the candidate master password; `userInputs`, words the password should not be based on, by default the app name and its parts.
- **Returns**: a `StrengthEstimate`.
- **Errors**: rejects with the import error if the dictionaries cannot load.
- **Security**: the password is secret. It is analysed only in the webview, by code bundled with the app, with no network access. The zxcvbn result also contains the password and its matched sequences; the function drops them and returns only the score and feedback. The UI already holds the password, because the user is typing it.
- **Edge cases and invariants**: an empty password is scored like any other string; callers skip it ([`usePasswordStrength`](usePasswordStrength.ts.md) does not estimate `''`).

## Dependencies

- **Internal**: none.
- **External**: `@zxcvbn-ts/core` (`ZxcvbnFactory`, the `Score` type), `@zxcvbn-ts/language-common` (dictionaries and keyboard graphs), `@zxcvbn-ts/language-en` (English dictionaries and feedback text). Justified in [`docs/dependencies.md`](../../../../dependencies.md).

## Used by

- [`usePasswordStrength.ts`](usePasswordStrength.ts.md): `estimateStrength`.
- [`newPassword.ts`](newPassword.ts.md): `MIN_ACCEPTED_SCORE`, `STRENGTH_LABELS`.
- [`StrengthMeter.tsx`](StrengthMeter.tsx.md): `MIN_ACCEPTED_SCORE`, `STRENGTH_LABELS`.
- [`NewPasswordFields.tsx`](NewPasswordFields.tsx.md): the `StrengthEstimate` type.

## Tests

- No unit test. [`CreateVaultScreen.test.tsx`](../screens/create/CreateVaultScreen.test.tsx.md) runs the real estimator and waits for the `Very weak` and `Very strong` ratings.

## Notes

- The module comment gives the size of the estimator and dictionaries as "about 1.5 MB", while the comment in [`vite.config.ts`](../../vite.config.ts.md) gives the chunk as "about 1.2 MB". The two may measure different things (sources vs. bundled chunk); the source does not say.
