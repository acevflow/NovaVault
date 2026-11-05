# [`ui/src/App.test.tsx`](../../../../ui/src/App.test.tsx)

## Purpose

Tests the root [`App`](App.tsx.md) component end to end with a fake Tauri shell: the startup status check, the unlocked shell with its navigation, error recovery and locking from the header.

## Public API

| Test case                                                    | What it checks                                                                                                                                                                                            |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `shows the unlocked vault with navigation when already unlocked` | With `vault_status` returning `{ exists: true, unlocked: true }`, the `Your vault` level-1 heading appears and receives focus, the `Vault` navigation button has `aria-current="page"`, and a `Skip to main content` link points to `#main`. |
| `shows a startup error and retries`                          | A first `vault_status` call that rejects with `storage` shows an alert containing `could not be read or written`; clicking `Try again` calls `vault_status` again and shows the vault.                     |
| `locks the vault from the header`                            | Clicking `Lock` calls the `lock` command, and the `Lock` button and the navigation disappear.                                                                                                             |

## Details

### Setup

- Each test installs fake commands with [`mockCommands`](test/ipc.ts.md). `list_items` returns `[]`, so the item list is empty. Any command without a handler rejects with `internal`, so an unexpected call would surface as an error.
- The retry test throws [`shellError('storage')`](test/ipc.ts.md) on the first call only, using a local counter.
- The lock test checks the recorded calls (`calls.map((call) => call.command)`) for `lock`.
- **Security**: no secrets are used; the vault is reported as already unlocked.

## Dependencies

- **Internal**: [`App.tsx`](App.tsx.md), [`test/ipc.ts`](test/ipc.ts.md) (`mockCommands`, `shellError`).
- **External**: `@testing-library/react` (`render`, `screen`), `@testing-library/user-event`, `vitest`.

## Used by

_Not applicable: test file, run by `npm test`._

## Tests

- Files under test: [`App.tsx`](../../../../ui/src/App.tsx) (page: [`App.tsx.md`](App.tsx.md)), and through it [`AppShell.tsx`](../../../../ui/src/app/AppShell.tsx), [`StatusScreens.tsx`](../../../../ui/src/screens/StatusScreens.tsx) and [`api/errors.ts`](../../../../ui/src/api/errors.ts), which supplies the `storage` message.

## Notes

- None.
