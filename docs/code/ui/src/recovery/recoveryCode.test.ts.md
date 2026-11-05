# [`ui/src/recovery/recoveryCode.test.ts`](../../../../../ui/src/recovery/recoveryCode.test.ts)

## Purpose

Unit tests for the recovery code helpers in [`recoveryCode.ts`](recoveryCode.ts.md), using the fake code `ABCD-EFGH-JKLM-NPQR-STUV-WXYZ-2345-6789`.

## Public API

| Test case                                    | What it checks                                                                                                                                   |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `ignores case, spaces and dashes when comparing` | `' ab-cd ef '` normalizes to `'ABCDEF'`; the code in lower case with spaces matches; the code missing its last character does not; two empty codes do not match. |
| `splits the display form into groups`        | The code splits into 8 groups, the first being `'ABCD'`.                                                                                         |

## Details

Not applicable beyond the table: each test calls the helpers directly.

## Dependencies

- **Internal**: [`recoveryCode.ts`](recoveryCode.ts.md).
- **External**: `vitest`.

## Used by

_Not applicable: test file, run by `npm test`._

## Tests

- File under test: [`recovery/recoveryCode.ts`](../../../../../ui/src/recovery/recoveryCode.ts).

## Notes

- The test defines its own copy of the fake code instead of importing `TEST_RECOVERY_CODE` from [`test/ipc.ts`](../test/ipc.ts.md); the two values are the same.
