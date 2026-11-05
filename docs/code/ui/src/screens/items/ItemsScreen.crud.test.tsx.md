# [`ui/src/screens/items/ItemsScreen.crud.test.tsx`](../../../../../../ui/src/screens/items/ItemsScreen.crud.test.tsx)

## Purpose

Component tests for the item operations of [`ItemsScreen`](ItemsScreen.tsx.md): view, copy, create, edit and delete. A small in-memory fake vault answers the item commands, and the tests check both the UI and the exact arguments sent to the shell. A recurring check is that secret fields are never revealed unless the user asks.

## Public API

| Test case                                                     | What it checks                                                                                                                                                                                                              |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ItemsScreen: view` › `shows plain fields and keeps secret fields hidden` | Opening `GitHub` focuses its level-2 heading, shows `alice` and the URL, shows the password as `Hidden`, marks the list row `aria-current="true"`, calls `get_item` once and never `reveal_item_field`. |
| `ItemsScreen: copy` › `copies plain and secret fields through the shell` | `Copy username` shows `Copied. The clipboard clears in 1 minute.`; `Copy username` and `Copy password` send `copy_item_field` for each field, and `reveal_item_field` is never called.                  |
| `ItemsScreen: create` › `creates a note and shows it`         | The `New note` form sends `{ item: { kind: 'note', title: 'Diary', body: 'dear diary' } }`, shows `Item added.`, and opens and selects the new item.                                                                       |
| `ItemsScreen: create` › `sends every field of a new password` | The `New password` form sends title, username, URL, password and notes in one `create_item` call.                                                                                                                           |
| `ItemsScreen: create` › `requires a title`                    | Saving a new secret without a title shows `Enter a title.`, focuses the title field and calls nothing.                                                                                                                      |
| `ItemsScreen: create` › `can be cancelled`                    | `Cancel` closes the `New note` form.                                                                                                                                                                                        |
| `ItemsScreen: edit` › `keeps secret fields that were not touched` | The edit form starts with an empty password field and the hint `Leave empty to keep the current password.`, and offers no reveal for the empty notes. Changing only the username sends `password: null` and `notes: null`, shows `Changes saved.` and never calls `reveal_item_field`. |
| `ItemsScreen: edit` › `loads the current secret on request, so it can be cleared` | `Reveal current password` loads the value into the field; clearing it and saving sends `password: ''`, with `notes: null`.                                                                  |
| `ItemsScreen: delete` › `asks for confirmation and can be cancelled` | `Delete` opens an `alertdialog` named `Delete this item?` described as `“GitHub” will be deleted…`, with focus on `Cancel`. Both `Cancel` and Escape close it, and `delete_item` is never called.                   |
| `ItemsScreen: delete` › `deletes after confirmation`          | Confirming sends `delete_item` with `{ id }`, shows `Item deleted.` and then the empty-vault message.                                                                                                                       |

## Details

### `setUp()`

- Builds a fake vault in a `Map<string, ItemDetails>` with one password item, `GitHub` (username `alice`, URL `https://github.com`, a password, no notes).
- Fakes `list_items` (summaries derived from the map), `get_item`, `reveal_item_field` (always `hunter2`), `copy_item_field` (60-second delay), `create_item` (stores a note under `'f'.repeat(32)`), `update_item` (updates title and username of a password item) and `delete_item`.
- Renders `ItemsScreen` and returns the recorded calls and a `userEvent` instance.

### Helpers

- `argsOf(calls, command)`: the arguments of every call to one command.
- `openGitHub(user)`: clicks the `GitHub` row and returns queries scoped to the `Item details` region.

- **Security**: all values are fake (`hunter2`, `s3cret`, `sk-123`). The empty `reveal_item_field` call lists show that viewing, copying and editing never fetch a secret the user did not ask to see.

## Dependencies

- **Internal**: [`ItemsScreen.tsx`](ItemsScreen.tsx.md), [`api/types.ts`](../../api/types.ts.md) (`ItemDetails`, `ItemSummary`), [`test/ipc.ts`](../../test/ipc.ts.md) (`mockCommands`, `CommandArgs`).
- **External**: `@testing-library/react` (`render`, `screen`, `within`), `@testing-library/user-event`, `vitest`.

## Used by

_Not applicable: test file, run by `npm test`._

## Tests

- Files under test: [`screens/items/ItemsScreen.tsx`](../../../../../../ui/src/screens/items/ItemsScreen.tsx), and through it [`ItemDetails.tsx`](../../../../../../ui/src/screens/items/ItemDetails.tsx), [`ItemForm.tsx`](../../../../../../ui/src/screens/items/ItemForm.tsx), [`ConfirmDelete.tsx`](../../../../../../ui/src/screens/items/ConfirmDelete.tsx), [`ItemList.tsx`](../../../../../../ui/src/screens/items/ItemList.tsx), [`SecretField.tsx`](../../../../../../ui/src/components/SecretField.tsx) and [`CopyButton.tsx`](../../../../../../ui/src/components/CopyButton.tsx).

## Notes

- The fake `update_item` only applies `title` and `username`, and only to password items; the tests do not need more.
