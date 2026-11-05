# [`ui/src/format/duration.ts`](../../../../../ui/src/format/duration.ts)

## Purpose

Formats a number of seconds as readable English text, as in "The clipboard clears in 30 seconds".

## Public API

| Name            | Kind     | Signature                                             | Description                         |
| --------------- | -------- | ----------------------------------------------------- | ----------------------------------- |
| `formatSeconds` | function | `export function formatSeconds(seconds: number): string` | Seconds as `'30 seconds'` or `'2 minutes'`. |

## Details

### `formatSeconds`

1. If `seconds` is at least 60 and a whole number of minutes, returns `'1 minute'` or `'<n> minutes'`.
2. Otherwise returns `'1 second'` or `'<n> seconds'`.

- **Returns**: for example `'1 second'`, `'30 seconds'`, `'1 minute'`, `'2 minutes'`.
- **Edge cases and invariants**: a duration that is not a whole number of minutes stays in seconds, so 90 is `'90 seconds'`, not `'1 minute 30 seconds'`. 0 is `'0 seconds'`.

## Dependencies

- **Internal**: none.
- **External**: none.

## Used by

- [`CopyButton.tsx`](../components/CopyButton.tsx.md): the copy confirmation message.
- [`ClipboardSection.tsx`](../screens/settings/ClipboardSection.tsx.md): the clear delay options.

## Tests

_Not applicable: no test covers this file directly. Its output appears in [`SecretField.test.tsx`](../components/SecretField.test.tsx.md) (`30 seconds`)._

## Notes

- None.
