# [`ui/src/recovery/recoveryCode.ts`](../../../../../ui/src/recovery/recoveryCode.ts)

## Purpose

Helpers for showing and retyping a recovery code in the UI. The shell parses and validates codes; these helpers only split the code for display and compare the user's retyped code with the one just shown, before the recovery code flow can finish.

## Public API

| Name                    | Kind     | Signature                                                              | Description                                                  |
| ----------------------- | -------- | ---------------------------------------------------------------------- | ------------------------------------------------------------ |
| `normalizeRecoveryCode` | function | `export function normalizeRecoveryCode(text: string): string`          | Uppercases and keeps only letters and digits.                |
| `recoveryCodesMatch`    | function | `export function recoveryCodesMatch(shown: string, typed: string): boolean` | Whether two codes are the same, ignoring case, spaces and dashes. |
| `recoveryCodeGroups`    | function | `export function recoveryCodeGroups(code: string): string[]`           | Splits a display-form code into its groups.                  |

## Details

### `normalizeRecoveryCode`

- Returns `text.toUpperCase().replace(/[^0-9A-Z]/g, '')`, for example `' ab-cd ef '` becomes `'ABCDEF'`.

### `recoveryCodesMatch`

- Normalizes both codes and returns `true` only if they are equal and the shown code is not empty after normalization.
- **Security**: both arguments are the recovery code, a secret. The comparison is a plain string equality, not constant-time. Both values are already in the UI, and the check only confirms the user wrote the code down; it grants no access.
- **Edge cases and invariants**: `recoveryCodesMatch('', '')` is `false`, so an empty code can never be confirmed.

### `recoveryCodeGroups`

- Splits on `-` and drops empty groups. For the display form `XXXX-XXXX-…` it returns 8 groups of 4 characters.

## Dependencies

- **Internal**: none.
- **External**: none.

## Used by

- [`RecoveryCodeFlow.tsx`](RecoveryCodeFlow.tsx.md): shows the code as a list of groups and checks the retyped code. The flow is shared by vault creation and recovery code regeneration.

## Tests

- [`recoveryCode.test.ts`](recoveryCode.test.ts.md): normalization, matching and splitting.
- [`CreateVaultScreen.test.tsx`](../screens/create/CreateVaultScreen.test.tsx.md): a near-miss code is rejected, and the right code in lower case with spaces is accepted.

## Notes

- The comment on `normalizeRecoveryCode` says it works "like the shell's parser". The shell's `RecoveryCode::parse` in [`recovery.rs`](../../../crates/novavault-core/src/recovery.rs.md) ignores only dashes and whitespace and rejects any other character outside its alphabet. This helper drops every non-alphanumeric character, so `ABCD.EFGH…` matches here but would be rejected by the shell. This only affects the confirmation step, which never sends the code to the shell.
