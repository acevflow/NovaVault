# [`ui/src/theme/theme.ts`](../../../../../ui/src/theme/theme.ts)

## Purpose

The colour theme preference: follow the system, or force light or dark. It loads and saves the preference in `localStorage` and applies it to the document as a `data-theme` attribute, which the stylesheet reads. Every storage access is guarded, so the app falls back to the system theme if storage is unavailable.

## Public API

| Name                   | Kind     | Signature                                                              | Description                                  |
| ---------------------- | -------- | ---------------------------------------------------------------------- | -------------------------------------------- |
| `ThemePreference`      | type     | `export type ThemePreference = 'system' \| 'light' \| 'dark'`          | The user's theme choice.                     |
| `THEME_PREFERENCES`    | constant | `export const THEME_PREFERENCES: readonly ThemePreference[]`           | `['system', 'light', 'dark']`, display order. |
| `loadThemePreference`  | function | `export function loadThemePreference(): ThemePreference`               | Reads the saved preference.                  |
| `saveThemePreference`  | function | `export function saveThemePreference(preference: ThemePreference): void` | Saves the preference.                      |
| `applyThemePreference` | function | `export function applyThemePreference(preference: ThemePreference): void` | Applies it to the document.               |

## Details

### `loadThemePreference`

- Reads `localStorage['novavault.theme']` (the private `STORAGE_KEY`).
- **Returns**: the saved value if it is a valid preference; otherwise `'system'`. Also `'system'` if storage throws (blocked or private mode).
- **Edge cases and invariants**: an invalid saved value, for example from an older version, is ignored rather than trusted.

### `saveThemePreference`

- Writes the preference under the same key. A storage failure is ignored: the choice still applies for the current session but is not remembered after a restart.

### `applyThemePreference`

- `'system'` deletes `data-theme` from `<html>`, so the `prefers-color-scheme` rules in [`styles.css`](../styles.css.md) decide. `'light'` or `'dark'` sets `data-theme` to that value.

- **Security**: the preference is not secret, so `localStorage` is acceptable. The source comment states this choice.

## Dependencies

- **Internal**: none.
- **External**: none (browser `localStorage` and DOM).

## Used by

- [`ThemeProvider.tsx`](ThemeProvider.tsx.md): `loadThemePreference`, `saveThemePreference`, `applyThemePreference`.
- [`ThemeContext.ts`](ThemeContext.ts.md): the `ThemePreference` type.
- [`AppearanceSection.tsx`](../screens/settings/AppearanceSection.tsx.md): `THEME_PREFERENCES` and `ThemePreference` for its radio buttons.

## Tests

- [`ThemeProvider.test.tsx`](ThemeProvider.test.tsx.md): the default, saving and restoring.
- [`SettingsScreen.test.tsx`](../screens/settings/SettingsScreen.test.tsx.md): `switches the theme` checks `data-theme` for `Dark` and `Match system`.

## Notes

- The storage failure branches are not tested.
