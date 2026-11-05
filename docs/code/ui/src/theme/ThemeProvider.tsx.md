# [`ui/src/theme/ThemeProvider.tsx`](../../../../../ui/src/theme/ThemeProvider.tsx)

## Purpose

Loads the saved colour theme preference (system, light or dark), applies it to the document before paint, and shares it with the app through [`ThemeContext`](../../../../../ui/src/theme/ThemeContext.ts). It wraps the whole app in [`App`](../App.tsx.md); the Appearance settings read and change the preference with `useTheme()`.

## Public API

| Name                 | Kind      | Signature | Description |
| -------------------- | --------- | --------- | ----------- |
| `ThemeProviderProps` | interface | `export interface ThemeProviderProps { readonly children: ReactNode; }` | Props of `ThemeProvider`. |
| `ThemeProvider`      | component | `function ThemeProvider({ children }: ThemeProviderProps)` | Loads, applies and provides the theme preference. |

## Details

### `ThemeProviderProps`

- `children`: the app.

### `ThemeProvider`

- **State**: `preference` (`ThemePreference`), initialized lazily with [`loadThemePreference`](../../../../../ui/src/theme/theme.ts), which reads `localStorage['novavault.theme']` and falls back to `'system'` when nothing valid is saved or storage throws.
- **Effects**: a `useLayoutEffect` on `preference` calls `applyThemePreference`, which sets `document.documentElement.dataset.theme` to `light` or `dark`, or deletes it for `system` so the stylesheet's `prefers-color-scheme` rules decide. A layout effect runs before paint, so the app never flashes the wrong theme.
- **`setPreference`** (stable `useCallback`): saves with `saveThemePreference` (a storage failure is ignored; the choice then lasts only for the session), then updates the state, which re-applies it.
- **Rendered output**: `<ThemeContext value={{ preference, setPreference }}>` around `children`. The value is memoized so consumers re-render only when the preference changes. This uses the React 19 form that renders a context directly as a provider.
- **User flows**: none of its own; [`AppearanceSection`](../screens/settings/AppearanceSection.tsx.md) calls `setPreference`.
- **Accessibility**: not applicable: it renders no UI of its own.
- **Tauri calls**: none.
- **Security**: the preference is not secret, so `localStorage` is acceptable. It is the only thing the UI persists in the webview.

## Dependencies

- **Internal**: [`theme/theme.ts`](../../../../../ui/src/theme/theme.ts) (`loadThemePreference`, `saveThemePreference`, `applyThemePreference`, `ThemePreference`), [`theme/ThemeContext.ts`](../../../../../ui/src/theme/ThemeContext.ts) (`ThemeContext`).
- **External**: `react`: `useState`, `useLayoutEffect`, `useCallback`, `useMemo`, the `ReactNode` type.

## Used by

- [`App.tsx`](../App.tsx.md): wraps the shell and screens.
- Consumers through `useTheme()`: [`AppearanceSection`](../screens/settings/AppearanceSection.tsx.md).

## Tests

- [`ThemeProvider.test.tsx`](../../../../../ui/src/theme/ThemeProvider.test.tsx):
  - `follows the system theme by default`: no saved value leaves `data-theme` unset.
  - `applies and remembers a forced theme`: choosing a theme sets `data-theme` and saves it.
  - `restores a saved light theme`: a saved `light` value is applied on mount.
- [`SettingsScreen.test.tsx`](../../../../../ui/src/screens/settings/SettingsScreen.test.tsx): `switches the theme`.

## Notes

- The webview is reloaded on auto-lock; the theme survives because it is reloaded from `localStorage`.
