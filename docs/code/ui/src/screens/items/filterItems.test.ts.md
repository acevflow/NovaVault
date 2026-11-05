# [`ui/src/screens/items/filterItems.test.ts`](../../../../../../ui/src/screens/items/filterItems.test.ts)

## Purpose

Unit tests for [`filterItems.ts`](filterItems.ts.md) over four sample items: `GitHub` (password), `Café receipts` (note), `github token` (secret) and `Bank` (password).

## Public API

| Test case                                                        | What it checks                                                                                                 |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `returns every item sorted by title when nothing is filtered`    | Order `Bank`, `Café receipts`, `GitHub`, `github token`.                                                       |
| `searches titles ignoring case, accents and surrounding spaces`  | `'  GITHUB '` finds both GitHub items; `'cafe'` finds `Café receipts`; `'nothing'` finds nothing.             |
| `filters by type and combines with the search`                   | `password` gives `Bank`, `GitHub`; `'git'` with `secret` gives `github token`; `'bank'` with `note` gives nothing. |
| `does not modify its input`                                      | The input array is unchanged after filtering.                                                                  |

## Details

### Helpers

- `item(id, kind, title)` builds an `ItemSummary` with both times set to 0.
- `titles(items)` maps results to their titles for comparison.

## Dependencies

- **Internal**: [`filterItems.ts`](filterItems.ts.md), [`api/types.ts`](../../api/types.ts.md) (`ItemSummary`).
- **External**: `vitest`.

## Used by

_Not applicable: test file, run by `npm test`._

## Tests

- File under test: [`screens/items/filterItems.ts`](../../../../../../ui/src/screens/items/filterItems.ts).

## Notes

- The `id` tie-break for equal titles is not tested; no two sample items have the same title.
