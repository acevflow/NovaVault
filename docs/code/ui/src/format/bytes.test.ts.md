# [`ui/src/format/bytes.test.ts`](../../../../../ui/src/format/bytes.test.ts)

## Purpose

Unit tests for [`bytes.ts`](bytes.ts.md): size formatting at each unit boundary and progress rounding.

## Public API

| Test case                                           | What it checks                                                                                        |
| --------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `formatBytes` › `uses bytes below one kilobyte`     | `0` → `'0 bytes'`, `1` → `'1 byte'`, `1023` → `'1023 bytes'`.                                         |
| `formatBytes` › `uses one decimal below 10 and none above` | `1024` → `'1.0 KB'`, `1536` → `'1.5 KB'`, 12 MiB → `'12 MB'`, 3.25 GiB → `'3.3 GB'`.           |
| `formatBytes` › `stops at terabytes`                | 2048 TiB → `'2048 TB'`.                                                                               |
| `percentDone` › `rounds down and caps at 100`       | `(0, 100)` → 0, `(999, 1000)` → 99, `(1000, 1000)` → 100, `(1200, 1000)` → 100.                      |
| `percentDone` › `treats an empty transfer as done`  | `(0, 0)` → 100.                                                                                       |

## Details

Not applicable beyond the table: each test calls the function directly and compares the result.

## Dependencies

- **Internal**: [`bytes.ts`](bytes.ts.md).
- **External**: `vitest`.

## Used by

_Not applicable: test file, run by `npm test`._

## Tests

- File under test: [`format/bytes.ts`](../../../../../ui/src/format/bytes.ts).

## Notes

- None.
