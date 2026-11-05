# [`ui/src/screens/items/ItemsScreen.test.tsx`](../../../../../../ui/src/screens/items/ItemsScreen.test.tsx)

## Purpose

Component tests for the list part of [`ItemsScreen`](ItemsScreen.tsx.md): loading the items, sorting, search, the type filter, the empty vault, load failures and a locked vault. Creating, viewing, editing and deleting items are tested in [`ItemsScreen.crud.test.tsx`](ItemsScreen.crud.test.tsx.md).

## Public API

| Test case                                               | What it checks                                                                                                                   |
| ------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `ItemsScreen: list` › `lists every item sorted by title with its type` | The `Diary` row shows `Note`; titles are listed as `Diary`, `GitHub`, `GitHub token`; the count says `3 items`.   |
| `ItemsScreen: list` › `searches by title`               | Typing `git` leaves the two GitHub items and shows `2 of 3 items`; `nope` shows `No items match your search.`                   |
| `ItemsScreen: list` › `filters by type`                 | Choosing `Secrets` leaves `GitHub token`; adding the search `diary` leaves nothing.                                              |
| `ItemsScreen: list` › `explains an empty vault`         | No items shows `Your vault is empty` and `0 items`.                                                                              |
| `ItemsScreen: list` › `reports a failure to load and retries` | A first `corrupted` rejection shows an alert containing `damaged`; `Try again` loads the list and removes the alert.       |
| `ItemsScreen: list` › `returns to the unlock screen if the vault is locked` | A `locked` rejection from `list_items` calls `onLocked`.                                                  |

## Details

### Fixtures and helpers

- `ITEMS`: three fake summaries, `GitHub` (password), `Diary` (note) and `GitHub token` (secret).
- `renderItems(handlers = {}, onLocked = vi.fn())`: fakes `list_items` to return `ITEMS`, merged with any extra handlers, and renders the screen.
- `listedTitles()`: reads the `.item-list__title` text of each button in the list, or `[]` if no list is shown.

## Dependencies

- **Internal**: [`ItemsScreen.tsx`](ItemsScreen.tsx.md), [`api/types.ts`](../../api/types.ts.md) (`ItemSummary`), [`test/ipc.ts`](../../test/ipc.ts.md) (`mockCommands`, `shellError`, `CommandHandlers`).
- **External**: `@testing-library/react` (`render`, `screen`, `within`), `@testing-library/user-event`, `vitest`.

## Used by

_Not applicable: test file, run by `npm test`._

## Tests

- Files under test: [`screens/items/ItemsScreen.tsx`](../../../../../../ui/src/screens/items/ItemsScreen.tsx), [`ItemList.tsx`](../../../../../../ui/src/screens/items/ItemList.tsx), [`filterItems.ts`](../../../../../../ui/src/screens/items/filterItems.ts) and [`kinds.ts`](../../../../../../ui/src/screens/items/kinds.ts).

## Notes

- `listedTitles` depends on the CSS class `item-list__title`, so renaming that class in [`ItemList.tsx`](ItemList.tsx.md) breaks the test without any change in behavior.
