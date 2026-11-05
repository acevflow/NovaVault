# [`ui/src/app/router.test.ts`](../../../../../ui/src/app/router.test.ts)

## Purpose

Unit tests for the pure route reducer in [`router.ts`](router.ts.md). They need no DOM and no fake shell.

## Public API

| Test case                                             | What it checks                                                                                                         |
| ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `routes the startup status to create, unlock or home` | `statusLoaded` with `exists: false` gives `create`; `exists: true, unlocked: false` gives `unlock`; both `true` gives `home`. |
| `navigates only while unlocked`                       | `navigate` to `settings` from `home` gives `settings`; from `unlock` it returns the same route object (`toBe`).         |
| `always lands on unlock after locking`                | `locked` from `home`, `settings` and `create` gives `unlock`.                                                          |
| `reports startup failures and retries`                | `statusFailed` gives `startupError` with the message; `retry` from there gives `initialRoute`.                         |

## Details

Not applicable beyond the table: each test calls `routerReducer` directly and compares the result.

## Dependencies

- **Internal**: [`router.ts`](router.ts.md) (`initialRoute`, `routerReducer`, `Route`).
- **External**: `vitest`.

## Used by

_Not applicable: test file, run by `npm test`._

## Tests

- File under test: [`app/router.ts`](../../../../../ui/src/app/router.ts).

## Notes

- The `unlocked` action is not tested here.
