# [`ui/src/theme/ThemeProvider.test.tsx`](../../../../../ui/src/theme/ThemeProvider.test.tsx)

## Purpose

Component tests for [`ThemeProvider`](ThemeProvider.tsx.md): the default system theme, applying and remembering a forced theme, and restoring a saved one. A small probe component reads the context through [`useTheme`](ThemeContext.ts.md).

## Public API

| Test case                            | What it checks                                                                                                                       |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| `follows the system theme by default` | With nothing saved, the preference is `system` and `data-theme` is not set.                                                         |
| `applies and remembers a forced theme` | Choosing `dark` sets `data-theme="dark"`; after unmounting and rendering a new provider, the preference is still `dark`.            |
| `restores a saved light theme`       | A saved `novavault.theme` of `light` is applied on mount.                                                                            |

## Details

### `Probe`

- A button that shows the current `preference` and calls `setPreference('dark')` when clicked. The click runs inside `act` so the layout effect that applies the theme has run before the assertion.

## Dependencies

- **Internal**: [`ThemeProvider.tsx`](ThemeProvider.tsx.md), [`ThemeContext.ts`](ThemeContext.ts.md) (`useTheme`).
- **External**: `@testing-library/react` (`act`, `render`, `screen`), `vitest`.

## Used by

_Not applicable: test file, run by `npm test`._

## Tests

- Files under test: [`theme/ThemeProvider.tsx`](../../../../../ui/src/theme/ThemeProvider.tsx), [`theme/ThemeContext.ts`](../../../../../ui/src/theme/ThemeContext.ts) and [`theme/theme.ts`](../../../../../ui/src/theme/theme.ts).

## Notes

- The tests depend on [`setup.ts`](../test/setup.ts.md) clearing `localStorage` and `data-theme` after each test.
