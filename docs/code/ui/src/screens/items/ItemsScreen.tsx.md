# [`ui/src/screens/items/ItemsScreen.tsx`](../../../../../../ui/src/screens/items/ItemsScreen.tsx)

## Purpose

The main screen of an unlocked vault (route `home`). It loads the item list, shows it with search and type filter in [`ItemList`](ItemList.tsx.md), and drives a side panel that shows the selected item ([`ItemDetails`](ItemDetails.tsx.md)), an edit form, or a create form ([`ItemForm`](ItemForm.tsx.md)). After every change it reloads the list.

## Public API

| Name               | Kind      | Signature | Description |
| ------------------ | --------- | --------- | ----------- |
| `ItemsScreenProps` | interface | `export interface ItemsScreenProps { readonly onLocked: () => void; }` | Props of `ItemsScreen`. |
| `ItemsScreen`      | component | `function ItemsScreen({ onLocked }: ItemsScreenProps)` | Item list and the panel for the selected item. |

The `Panel` type and the `PanelContent` component are internal and described below.

## Details

### `ItemsScreenProps`

- `onLocked`: called when a command reports that the vault is locked. [`App`](../../App.tsx.md) passes a stable callback that routes to the unlock screen. It is passed down to every child that calls the shell.

### `Panel` (internal type)

```ts
type Panel =
  | { readonly mode: 'none' }
  | { readonly mode: 'view'; readonly id: string }
  | { readonly mode: 'edit'; readonly details: Details }
  | { readonly mode: 'create'; readonly kind: ItemKind };
```

### `ItemsScreen`

- **State**:
  - `items` (`readonly ItemSummary[] | null`; `null` until the first load succeeds);
  - `loadError` (`string | null`);
  - `panel` (`Panel`, starting at `none`);
  - `notice` (success text shown above the panel, or `null`);
  - `revision` (a counter bumped to reload the list).
- **Load effect** (on `revision` and `onLocked`): calls `listItems()`. On success stores the items. On a `locked` error calls `onLocked()`. On any other error sets `loadError`. A local `active` flag drops results from a superseded run.
- **`reload`** (stable): clears `loadError` and bumps `revision`. Used by "Try again" and after every create, update and delete. The previous list stays on screen while it reloads.
- **`show(next, message = null)`**: sets `notice` to `message` and `panel` to `next`, so every panel change replaces or clears the notice.
- **`selectedId`**: the id being viewed or edited, passed to `ItemList` to mark the current entry.
- **Rendered output**: `<div className="screen screen--wide">` with:
  1. The [`ScreenHeading`](../../components/ScreenHeading.tsx.md) "Your vault" (level 1, takes focus).
  2. On a load error, an error [`Alert`](../../components/Alert.tsx.md) and a "Try again" button.
  3. Before the first load: `<p role="status">Loading items…</p>` (hidden when there is a load error).
  4. Once loaded: one "New password", "New note" and "New secret" button (password is primary), then the `ItemList` next to the detail panel. The panel shows the success `notice`, then `PanelContent`.

### `PanelContent` (internal)

| `panel.mode` | Renders | Transitions |
| ------------ | ------- | ----------- |
| `none`   | "Select an item to see its details, or add a new one." | — |
| `view`   | `ItemDetails` keyed by `id` | Edit → `edit` with the loaded details; deleted → reload, `none`, "Item deleted." |
| `edit`   | `ItemForm` with `existing` = details, keyed by id | Cancel → `view`; saved → reload, `view`, "Changes saved." |
| `create` | `ItemForm` with `existing={null}`, keyed by kind | Cancel → `none`; saved → reload, `view` of the new id, "Item added." |

The `key` on `ItemDetails` and `ItemForm` makes React remount them for another item or kind, which drops revealed values and form drafts and moves focus to the new heading.

### Flow summary

- **User flows**: select an item → view it → Edit → Save or Cancel; New password / note / secret → fill → Save; view → Delete → confirm. Search and type filter live in `ItemList`.
- **Accessibility**: the screen heading takes focus on arrival, and each panel brings its own focused heading. The loading text and success notices are polite status messages; load errors are alerts.
- **Tauri calls**: `listItems` → `list_items` on mount and after every reload. Children call `get_item`, `reveal_item_field`, `copy_item_field`, `create_item`, `update_item`, `delete_item`, `add_attachment`, `export_attachment` and `delete_attachment`. See [`commands.rs`](../../../../src-tauri/src/commands.rs.md).
- **Security**: `list_items` returns summaries without secret fields. Titles stay in the webview to draw and filter the list. Remounting the detail panel per item discards anything revealed for the previous one.

## Dependencies

- **Internal**: [`api/commands.ts`](../../../../../../ui/src/api/commands.ts) (`listItems`), [`api/errors.ts`](../../../../../../ui/src/api/errors.ts), [`api/types.ts`](../../../../../../ui/src/api/types.ts), [`Alert`](../../components/Alert.tsx.md), [`Button`](../../components/Button.tsx.md), [`ScreenHeading`](../../components/ScreenHeading.tsx.md), [`ItemDetails`](ItemDetails.tsx.md), [`ItemForm`](ItemForm.tsx.md), [`ItemList`](ItemList.tsx.md), [`kinds.ts`](../../../../../../ui/src/screens/items/kinds.ts) (`ITEM_KINDS`, `KIND_LABELS`).
- **External**: `react`: `useState`, `useEffect`, `useCallback`.

## Used by

- [`App.tsx`](../../App.tsx.md): for the `home` route.

## Tests

- [`ItemsScreen.test.tsx`](../../../../../../ui/src/screens/items/ItemsScreen.test.tsx): `lists every item sorted by title with its type`, `searches by title`, `filters by type`, `explains an empty vault`, `reports a failure to load and retries`, `returns to the unlock screen if the vault is locked`.
- [`ItemsScreen.crud.test.tsx`](../../../../../../ui/src/screens/items/ItemsScreen.crud.test.tsx): `shows plain fields and keeps secret fields hidden`, `copies plain and secret fields through the shell`, `creates a note and shows it`, `sends every field of a new password`, `requires a title`, `can be cancelled`, `keeps secret fields that were not touched`, `loads the current secret on request, so it can be cleared`, `asks for confirmation and can be cancelled`, `deletes after confirmation`.

## Notes

- If a reload fails after a successful first load, the error alert and "Try again" appear above the previous, possibly stale, list.
- Switching panels while a create or update request is in flight unmounts the form; the request still completes in the shell, but its `onSaved` then reloads the list and switches the panel again.
