# [`ui/src/screens/items/filterItems.ts`](../../../../../../ui/src/screens/items/filterItems.ts)

## Purpose

Searches, filters and sorts the item list. It runs entirely in the UI over the titles returned by `listItems`. Those titles are already in the webview to draw the list, and no secret field is ever searched.

## Public API

| Name          | Kind     | Signature                                                                                         | Description                                      |
| ------------- | -------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| `KindFilter`  | type     | `export type KindFilter = 'all' \| ItemKind`                                                      | One kind of item, or all of them.                |
| `filterItems` | function | `export function filterItems(items: readonly ItemSummary[], query: string, kind: KindFilter): ItemSummary[]` | Matching items, sorted by title.     |

## Details

### `filterItems`

1. Trims `query` and normalizes it: NFD decomposition, diacritics removed (`\p{Diacritic}`), lower case.
2. Keeps items whose `kind` equals `kind`, or every item if `kind` is `'all'`.
3. Keeps items whose normalized title contains the normalized query, or every item if the query is empty.
4. Sorts by title with `localeCompare(…, undefined, { sensitivity: 'base' })`, which ignores case and accents. Equal titles are ordered by `id`, so the order is stable.

- **Parameters**: `items`, the summaries from `listItems`; `query`, the search box text; `kind`, the type filter.
- **Returns**: a new array. The input is not modified, because `filter` creates a copy before `sort`.
- **Security**: only titles are searched. `ItemSummary` has no secret fields, so a search can never match on, or reveal, a password, note body or secret value.
- **Edge cases and invariants**: "cafe" finds "Café", and "  GITHUB " finds "GitHub". A query that is only spaces matches every item.

### `normalize` (private)

`function normalize(text: string): string`: lower case without diacritics, used on both the query and each title.

## Dependencies

- **Internal**: [`api/types.ts`](../../api/types.ts.md) (`ItemKind`, `ItemSummary`).
- **External**: none.

## Used by

- [`ItemList.tsx`](ItemList.tsx.md): the search box and type filter.

## Tests

- [`filterItems.test.ts`](filterItems.test.ts.md): sorting, case- and accent-insensitive search, kind filter and input immutability.

## Notes

- Sorting uses the webview's default locale, so the order of titles with special characters may vary with the system language.
