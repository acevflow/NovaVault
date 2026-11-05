# [`ui/src/components/SecretField.test.tsx`](../../../../../ui/src/components/SecretField.test.tsx)

## Purpose

Component tests for [`SecretField`](SecretField.tsx.md), the read-only display of one secret item field. They check that the value is fetched only on demand, dropped from the DOM when hidden again, copied through the shell without being revealed, and that errors and a locked vault are handled.

## Public API

| Test case                                               | What it checks                                                                                                                                                                                                    |
| ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `is hidden by default and fetches nothing`              | The field shows `Hidden`, the `Reveal password` toggle has `aria-pressed="false"`, and no command is called.                                                                                                      |
| `reveals the value on demand and drops it when hidden`  | Clicking the toggle calls `reveal_item_field` once with `{ id, field: 'password' }`, shows the value, and sets `aria-pressed="true"` and the label `Hide`. Clicking again removes the value from the DOM and shows `Hidden`. |
| `copies through the shell without revealing the value`  | `Copy password` calls `copy_item_field` only; a status says `Copied. The clipboard clears in 30 seconds.`, and the field stays `Hidden`.                                                                         |
| `reports a clipboard that is not available`             | A `clipboard_unavailable` rejection is announced in the status region.                                                                                                                                            |
| `says when the field is empty and offers no reveal`     | With `hasValue={false}` the field shows `Empty` and renders no button.                                                                                                                                            |
| `reports a failure to reveal`                           | An `item_not_found` rejection shows an alert containing `no longer exists`.                                                                                                                                       |
| `returns to the unlock screen if the vault is locked`   | A `locked` rejection calls `onLocked`.                                                                                                                                                                            |

## Details

### `renderField` (local helper)

`function renderField(handlers: CommandHandlers, hasValue = true, onLocked = vi.fn())`

- Installs the fake commands with [`mockCommands`](../test/ipc.ts.md) and renders `SecretField` for the `password` field of item `'a'.repeat(32)`, inside a `<dl>` because the component renders description-list content.
- Returns `{ calls, onLocked, view, user }`, where `user` comes from `userEvent.setup()`.
- **Security**: uses the obviously fake value `hunter2`.

## Dependencies

- **Internal**: [`SecretField.tsx`](SecretField.tsx.md), [`test/ipc.ts`](../test/ipc.ts.md) (`mockCommands`, `shellError`, `CommandHandlers`).
- **External**: `@testing-library/react`, `@testing-library/user-event`, `vitest` (`vi.fn`, `vi.waitFor`).

## Used by

_Not applicable: test file, run by `npm test`._

## Tests

- Files under test: [`components/SecretField.tsx`](../../../../../ui/src/components/SecretField.tsx), and through it [`CopyButton.tsx`](../../../../../ui/src/components/CopyButton.tsx) and [`api/commands.ts`](../../../../../ui/src/api/commands.ts).

## Notes

- The test checks that the revealed value leaves the DOM. It cannot check that the JavaScript string is gone from memory; see the garbage-collection note on the [`SecretField`](SecretField.tsx.md) page.
