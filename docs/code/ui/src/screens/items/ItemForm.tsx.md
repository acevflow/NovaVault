# [`ui/src/screens/items/ItemForm.tsx`](../../../../../../ui/src/screens/items/ItemForm.tsx)

## Purpose

The form that creates an item of a given kind, or edits an existing one. When editing, secret fields start empty and are never pre-filled: an untouched secret field is sent as `null`, so the shell keeps the stored value without the UI ever reading it. The user can load the current value on request to change or clear it.

## Public API

| Name            | Kind      | Signature | Description |
| --------------- | --------- | --------- | ----------- |
| `ItemFormProps` | interface | `export interface ItemFormProps { readonly kind: ItemKind; readonly existing: ItemDetails \| null; readonly onSaved: (id: string) => void; readonly onCancel: () => void; readonly onLocked: () => void; }` | Props of `ItemForm`. |
| `ItemForm`      | component | `function ItemForm({ kind, existing, onSaved, onCancel, onLocked }: ItemFormProps)` | Create or edit form for one item. |

Internal items: the `SecretDraft` interface, `SECRET_FIELDS`, `hasStored`, `initialSecrets` and the `SecretInput` component, described below.

## Details

### `ItemFormProps`

- `kind`: the kind to create, or of the item being edited.
- `existing`: the item being edited (plain fields and `has…` flags from `get_item`), or `null` to create.
- `onSaved`: called with the saved item's id.
- `onCancel`: leaves the form without saving.
- `onLocked`: called if a command reports the vault is locked.

### `SecretDraft` and the keep/clear rule

```ts
interface SecretDraft {
  readonly value: string;
  readonly loaded: boolean;
}
```

`initialSecrets(isNew)` gives every secret field (`password`, `notes`, `body`, `value`) `{ value: '', loaded: isNew }`.

| Mode   | Field state                      | Sent value  | Shell effect             |
| ------ | -------------------------------- | ----------- | ------------------------ |
| Create | any                              | `value`     | stored as typed (`''` = empty) |
| Edit   | not loaded, empty                | `null`      | keeps the stored value   |
| Edit   | not loaded, typed                | `value`     | replaces it              |
| Edit   | loaded (revealed), any           | `value`     | replaces it; `''` clears it |

So in edit mode a stored secret can only be cleared after "Reveal current …" loads it.

`SECRET_FIELDS` lists each kind's secret fields and labels: password → Password, Notes; note → Text (`body`); secret → Value, Notes. `hasStored(existing, field)` reads the matching `has…` flag.

### `ItemForm`

- **State**: `title`, `username`, `url` (from `existing` for password items, otherwise `''`), `secrets` (drafts), `titleError`, `error`, `busy`, and a ref to the title input.
- **Rendered output**: `<section className="card" aria-label={heading}>` with a level-2 [`ScreenHeading`](../../components/ScreenHeading.tsx.md) ("New password" / "New note" / "New secret", or Edit “<title or Untitled>”), and a `<form noValidate>` with:
  1. [`TextField`](../../components/TextField.tsx.md) "Title" (`required`, `autoComplete="off"`, error from `titleError`).
  2. For passwords: "Username" (`spellCheck` and `autoCapitalize` off) and "Website" (`type="url"`, placeholder `https://`, `spellCheck` off).
  3. One `SecretInput` per secret field of the kind.
  4. An error [`Alert`](../../components/Alert.tsx.md) when `error` is set.
  5. Save (primary submit, "Saving…" with `busy`) and Cancel (disabled while busy) [`Button`](../../components/Button.tsx.md)s.
- **Submit**: prevents the default and clears `error`. If the trimmed title is empty, sets "Enter a title.", focuses the title input and stops. Otherwise clears `titleError` and calls `save()`:
  - create: `createItem(buildInput())` → `onSaved(newId)`;
  - edit: `updateItem(existing.id, buildUpdate())` → `onSaved(existing.id)`;
  - on failure: `report` (calls `onLocked()` for `locked`, otherwise shows `errorMessage`) and clears `busy`. On success `busy` stays set because the parent replaces the form.
- **Reveal current value** (`revealCurrent(field)`): only in edit mode; clears `error`, calls `revealItemField(existing.id, field)` and stores the result as `{ value, loaded: true }`. Errors go through `report`.
- **Payloads**: `buildInput()` sends every field as typed. `buildUpdate()` sends plain fields as typed and each secret through `secretOut` (`null` or text, per the table above). The title is sent untrimmed.

### `SecretInput` (internal)

- **Props**: `field`, `label`, `draft`, `canReveal` (editing and a value is stored), `onChange`, `onReveal`.
- **Keeping state** (`canReveal && !draft.loaded`): adds the description "Leave empty to keep the current <label>." and a small "Reveal current <label>" button.
- **Rendering**: `notes` and `body` use [`TextAreaField`](../../components/TextAreaField.tsx.md) (8 rows for `body`, 4 for notes) with the reveal button as `trailing`. `password` and `value` use [`PasswordField`](../../components/PasswordField.tsx.md) (`autoComplete="off"`), masked with a Show toggle, with the reveal button below it.
- Typing keeps the field's `loaded` flag.

### Flow summary

- **User flows**: create: fill → Save → the parent shows the new item with "Item added."; edit: change plain fields, optionally type a new secret or reveal the current one → Save → "Changes saved."; Cancel at any time.
- **Accessibility**:
  - The form's heading takes focus on mount.
  - A missing title marks the field invalid, links the error and moves focus to it.
  - Save errors are alerts.
  - Secret inputs inherit label, description and spell-check handling from the shared fields.
- **Tauri calls**:
  - `createItem` → `create_item` with `{ item }`, returning the new id;
  - `updateItem` → `update_item` with `{ id, item }`;
  - `revealItemField` → `reveal_item_field` with `{ id, field }`, only on "Reveal current …".
  See [`commands.rs`](../../../../src-tauri/src/commands.rs.md).
- **Security**:
  - Existing secrets are never fetched to fill the form; they reach the webview only through an explicit reveal.
  - Drafts live in this component's state and are dropped when it unmounts (the parent keys it per item or kind). JavaScript strings cannot be zeroized.
  - Notes and note text are shown in plain text in their textareas once loaded or typed; password and value fields stay masked until the Show toggle is pressed.

## Dependencies

- **Internal**: [`api/commands.ts`](../../../../../../ui/src/api/commands.ts) (`createItem`, `updateItem`, `revealItemField`), [`api/errors.ts`](../../../../../../ui/src/api/errors.ts), [`api/types.ts`](../../../../../../ui/src/api/types.ts), [`Alert`](../../components/Alert.tsx.md), [`Button`](../../components/Button.tsx.md), [`PasswordField`](../../components/PasswordField.tsx.md), [`ScreenHeading`](../../components/ScreenHeading.tsx.md), [`TextAreaField`](../../components/TextAreaField.tsx.md), [`TextField`](../../components/TextField.tsx.md), [`kinds.ts`](../../../../../../ui/src/screens/items/kinds.ts).
- **External**: `react`: `useRef`, `useState`, the `SubmitEvent` type.

## Used by

- [`ItemsScreen`](ItemsScreen.tsx.md): in `create` and `edit` modes.

## Tests

- [`ItemsScreen.crud.test.tsx`](../../../../../../ui/src/screens/items/ItemsScreen.crud.test.tsx): `creates a note and shows it`, `sends every field of a new password`, `requires a title`, `can be cancelled`, `keeps secret fields that were not touched` (sends `null`), `loads the current secret on request, so it can be cleared`.

## Notes

- "Reveal current …" replaces whatever the user had already typed in that field.
- Inputs stay editable while a save is in flight; only Save and Cancel are disabled, so edits made during the request are not sent.
- The "Reveal current …" button has no busy state while the reveal request runs.
- An empty title is rejected in the UI; titles made only of spaces are rejected too, but a valid title is sent with its surrounding spaces.
