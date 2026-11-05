# [`ui/src/format/dateTime.ts`](../../../../../ui/src/format/dateTime.ts)

## Purpose

Formats timestamps for UI text, such as the time an item was created or updated, or the time a file was attached.

## Public API

| Name             | Kind     | Signature                                         | Description                                      |
| ---------------- | -------- | ------------------------------------------------- | ------------------------------------------------ |
| `formatDateTime` | function | `export function formatDateTime(ms: number): string` | A timestamp as a medium date and a short time. |

## Details

### `formatDateTime`

- Returns `new Date(ms).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })`.
- **Parameters**: `ms`, milliseconds since the Unix epoch (UTC), as sent by the shell.
- **Returns**: text in the webview's default locale and local time zone, for example `Nov 5, 2026, 3:04 PM` in US English.
- **Edge cases and invariants**: an invalid `ms` (such as `NaN`) yields `'Invalid Date'`.

## Dependencies

- **Internal**: none.
- **External**: none (the built-in `Intl` support of `Date`).

## Used by

- [`ItemDetails.tsx`](../screens/items/ItemDetails.tsx.md): created and updated times.
- [`AttachmentList.tsx`](../screens/items/AttachmentList.tsx.md): when each file was attached.

## Tests

_Not applicable: no test covers this file directly. The output depends on the locale and time zone of the test environment._

## Notes

- The locale is `undefined`, so the format follows the webview's language, not a setting in the app.
