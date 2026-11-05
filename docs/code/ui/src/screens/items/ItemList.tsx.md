# [`ui/src/screens/items/ItemList.tsx`](../../../../../../ui/src/screens/items/ItemList.tsx)

## Purpose

The list of the vault's items with a title search and a type filter. Filtering runs in the UI over the summaries already loaded (titles and kinds only), using [`filterItems`](../../../../../../ui/src/screens/items/filterItems.ts). Selecting an entry asks the parent to show its details.

## Public API

| Name            | Kind      | Signature | Description |
| --------------- | --------- | --------- | ----------- |
| `ItemListProps` | interface | `export interface ItemListProps { readonly items: readonly ItemSummary[]; readonly selectedId: string \| null; readonly onSelect: (id: string) => void; }` | Props of `ItemList`. |
| `ItemList`      | component | `function ItemList({ items, selectedId, onSelect }: ItemListProps)` | The items, with search and type filter. |

The helper `countLabel(shown, total)` is internal.

## Details

### `ItemListProps`

- `items`: every item of the vault, as returned by `list_items`.
- `selectedId`: the item whose details are shown, if any.
- `onSelect`: called with an item's id when its entry is pressed.

### `ItemList`

- **State**: `query` (search text, starting empty), `kind` (`KindFilter`, starting `'all'`), plus two `useId` ids.
- **Derived**: `visible = filterItems(items, query, kind)`: items of the chosen kind whose title contains the trimmed query, ignoring case and diacritics ("cafe" finds "Café"), sorted by title (base sensitivity) and then by id.
- **Rendered output**: `<section className="card items__list" aria-labelledby=…>` with:
  1. `<h2>Items</h2>`.
  2. A `<div role="search" aria-label="Filter items">` holding a [`TextField`](../../components/TextField.tsx.md) "Search by title" (`type="search"`, `autoComplete="off"`, `spellCheck={false}`) and a labelled `<select>` "Type" with "All types", "Passwords", "Notes", "Secrets".
  3. `<p role="status">` with the count from `countLabel`: "3 items", "1 item", or "1 of 3 items" when filtered. The noun follows the total.
  4. One of:
     - "Your vault is empty. Add a password, note or secret to get started." when there are no items;
     - "No items match your search." when the filter hides everything;
     - a `<ul>` of `<button type="button">` entries, each with the title (or "Untitled") and the kind label.
- **User flows**: typing or choosing a type filters as you go; pressing an entry calls `onSelect(id)`.
- **Accessibility**:
  - The list is a labelled section, and the filters a labelled `search` landmark.
  - The count is a polite live region, so filter results are announced.
  - The selected entry has `aria-current="true"`.
  - Entries are native buttons, reachable with Tab.
- **Tauri calls**: none.
- **Security**: only titles and kinds are searched; no secret field is in the webview to search. The search box has spell checking and autocomplete off.

## Dependencies

- **Internal**: [`TextField`](../../components/TextField.tsx.md), [`filterItems.ts`](../../../../../../ui/src/screens/items/filterItems.ts) (`filterItems`, `KindFilter`), [`kinds.ts`](../../../../../../ui/src/screens/items/kinds.ts) (`ITEM_KINDS`, `KIND_LABELS`, `KIND_PLURALS`), [`api/types.ts`](../../../../../../ui/src/api/types.ts) (`ItemSummary`).
- **External**: `react`: `useId`, `useState`.

## Used by

- [`ItemsScreen`](ItemsScreen.tsx.md).

## Tests

- [`ItemsScreen.test.tsx`](../../../../../../ui/src/screens/items/ItemsScreen.test.tsx): `lists every item sorted by title with its type`, `searches by title`, `filters by type`, `explains an empty vault`.
- [`filterItems.test.ts`](../../../../../../ui/src/screens/items/filterItems.test.ts) covers the filtering and sorting.

## Notes

- The selected entry uses `aria-current="true"`, while the navigation in [`AppShell`](../../app/AppShell.tsx.md) uses `aria-current="page"`.
- The `<select>` handler casts `event.target.value` to `KindFilter`; the options are generated from `ITEM_KINDS`, so the cast holds.
- Search and filter state is local, so it resets whenever `ItemsScreen` remounts (for example after navigating to Settings and back).
