# [`ui/src/screens/items/kinds.ts`](../../../../../../ui/src/screens/items/kinds.ts)

## Purpose

Display names of the three item kinds, shared by the item screens so that every badge, heading and filter uses the same words and order.

## Public API

| Name           | Kind     | Signature                                              | Description                                         |
| -------------- | -------- | ------------------------------------------------------ | --------------------------------------------------- |
| `ITEM_KINDS`   | constant | `export const ITEM_KINDS: readonly ItemKind[]`          | Every kind, in the order the UI offers them.        |
| `KIND_LABELS`  | constant | `export const KIND_LABELS: Record<ItemKind, string>`    | Singular label, for badges and headings.            |
| `KIND_PLURALS` | constant | `export const KIND_PLURALS: Record<ItemKind, string>`   | Plural label, for the type filter.                  |

## Details

### `ITEM_KINDS`

- `['password', 'note', 'secret']`.

### `KIND_LABELS`

- `password: 'Password'`, `note: 'Note'`, `secret: 'Secret'`.

### `KIND_PLURALS`

- `password: 'Passwords'`, `note: 'Notes'`, `secret: 'Secrets'`.

- **Edge cases and invariants**: both records are typed `Record<ItemKind, string>`, so adding a kind to `ItemKind` without a label fails to compile. `ITEM_KINDS` has no such check; a new kind must be added to it by hand.

## Dependencies

- **Internal**: [`api/types.ts`](../../api/types.ts.md) (`ItemKind`).
- **External**: none.

## Used by

- [`ItemList.tsx`](ItemList.tsx.md): `ITEM_KINDS`, `KIND_LABELS`, `KIND_PLURALS`.
- [`ItemsScreen.tsx`](ItemsScreen.tsx.md): `ITEM_KINDS`, `KIND_LABELS`.
- [`ItemDetails.tsx`](ItemDetails.tsx.md) and [`ItemForm.tsx`](ItemForm.tsx.md): `KIND_LABELS`.

## Tests

_Not applicable: constants only. The labels appear in [`ItemsScreen.test.tsx`](ItemsScreen.test.tsx.md) and [`ItemsScreen.crud.test.tsx`](ItemsScreen.crud.test.tsx.md)._

## Notes

- The records are not `readonly`, unlike `ITEM_KINDS`; nothing in the code mutates them.
