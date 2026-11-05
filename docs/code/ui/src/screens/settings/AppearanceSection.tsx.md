# [`ui/src/screens/settings/AppearanceSection.tsx`](../../../../../../ui/src/screens/settings/AppearanceSection.tsx)

## Purpose

The Appearance card of the settings screen: a radio group choosing the colour theme ("Match system", "Light" or "Dark"). The choice applies at once and is remembered through the [`ThemeProvider`](../../theme/ThemeProvider.tsx.md); there is nothing to save.

## Public API

| Name                | Kind      | Signature | Description |
| ------------------- | --------- | --------- | ----------- |
| `AppearanceSection` | component | `function AppearanceSection()` (no props) | Theme radio group. |

## Details

### `AppearanceSection`

- **Props**: none.
- **State**: two `useId` ids (heading, radio group name). The preference comes from `useTheme()`, which throws outside a `ThemeProvider`.
- **Rendered output**: `<section className="card" aria-labelledby=…>` with `<h2>Appearance</h2>` and a `<fieldset>` with legend "Theme". It holds one labelled radio per entry of `THEME_PREFERENCES` (`system`, `light`, `dark`), labelled by the module constant `LABELS`, and checked when it matches the current preference.
- **User flows**: choosing a radio calls `setPreference(option)`, which saves to `localStorage` and sets or removes `data-theme` on `<html>` before the next paint.
- **Accessibility**: a native radio group in a `fieldset` with a `legend`; arrow keys move between options. The section is labelled by its heading.
- **Tauri calls**: none.
- **Security**: the theme is not secret and is kept outside the vault, so it works while locked too.

## Dependencies

- **Internal**: [`theme/ThemeContext.ts`](../../../../../../ui/src/theme/ThemeContext.ts) (`useTheme`), [`theme/theme.ts`](../../../../../../ui/src/theme/theme.ts) (`THEME_PREFERENCES`, `ThemePreference`).
- **External**: `react`: `useId`.

## Used by

- [`SettingsScreen`](SettingsScreen.tsx.md).

## Tests

- [`SettingsScreen.test.tsx`](../../../../../../ui/src/screens/settings/SettingsScreen.test.tsx): `switches the theme`: choosing "Dark" sets `data-theme="dark"`, and "Match system" removes it.
- [`ThemeProvider.test.tsx`](../../../../../../ui/src/theme/ThemeProvider.test.tsx) covers loading and saving.

## Notes

- The theme can only be changed from Settings, which needs an unlocked vault, although the preference itself is stored outside the vault.
