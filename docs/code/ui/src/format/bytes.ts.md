# [`ui/src/format/bytes.ts`](../../../../../ui/src/format/bytes.ts)

## Purpose

Formats file sizes and transfer progress for UI text. The attachment list uses it to show each file's size and the progress of an encryption or decryption.

## Public API

| Name          | Kind     | Signature                                                    | Description                                        |
| ------------- | -------- | ------------------------------------------------------------ | -------------------------------------------------- |
| `formatBytes` | function | `export function formatBytes(bytes: number): string`         | A size in bytes as text, in 1024-based units.      |
| `percentDone` | function | `export function percentDone(done: number, total: number): number` | Whole percentage from 0 to 100.              |

## Details

### `formatBytes`

1. Below 1024: `'1 byte'` for 1, otherwise `'<n> bytes'` (for example `'0 bytes'`, `'512 bytes'`).
2. Otherwise divides by 1024 and keeps dividing while the value is at least 1024 and a larger unit exists. Units: `KB`, `MB`, `GB`, `TB`.
3. Values below 10 get one decimal (`toFixed(1)`); larger values are rounded to a whole number.

- **Parameters**: `bytes`, a size in bytes.
- **Returns**: for example `'1.5 KB'`, `'12 MB'`, `'3.3 GB'`, `'2048 TB'`.
- **Edge cases and invariants**: `TB` is the largest unit, so larger sizes show more than 1024 TB. The `?? 'TB'` fallback only satisfies `noUncheckedIndexedAccess`; the loop never runs past the last unit.

### `percentDone`

- **Returns**: `Math.min(100, Math.floor((done / total) * 100))`, so progress rounds down and never shows 100 % before the transfer is complete, even if `done` exceeds `total`.
- **Edge cases and invariants**: a `total` of 0 or less returns 100, so an empty file counts as done and there is no division by zero.

## Dependencies

- **Internal**: none.
- **External**: none.

## Used by

- [`AttachmentList.tsx`](../screens/items/AttachmentList.tsx.md): attachment sizes and transfer progress.

## Tests

- [`bytes.test.ts`](bytes.test.ts.md): all branches of both functions.

## Notes

- Rounding happens after the unit is chosen, so a value just below the next unit is shown in the smaller one: 1,048,575 bytes is `'1024 KB'`, not `'1.0 MB'`. Likewise a value from 9.95 up to 10 shows `'10.0'` with a decimal.
- The units are 1024-based but use the `KB`/`MB` labels, not `KiB`/`MiB`.
