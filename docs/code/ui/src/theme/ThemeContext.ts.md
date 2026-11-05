# [`ui/src/theme/ThemeContext.ts`](../../../../../ui/src/theme/ThemeContext.ts)

## Purpose

The React context that shares the theme preference and its setter, and the `useTheme` hook to read it. [`ThemeProvider`](ThemeProvider.tsx.md) provides the value; it lives in its own file so the provider file exports only a component, as Fast Refresh requires (see [`eslint.config.js`](../../eslint.config.js.md)).

## Public API

| Name                | Kind      | Signature                                                                                                          | Description                                       |
| ------------------- | --------- | ------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------- |
| `ThemeContextValue` | interface | `export interface ThemeContextValue { readonly preference: ThemePreference; readonly setPreference: (preference: ThemePreference) => void; }` | Current preference and setter. |
| `ThemeContext`      | constant  | `export const ThemeContext = createContext<ThemeContextValue \| null>(null)`                                        | The context; `null` outside a provider.           |
| `useTheme`          | hook      | `export function useTheme(): ThemeContextValue`                                                                    | Returns the preference and its setter.            |

## Details

### `ThemeContextValue`

- `preference`: the active preference. `setPreference`: applies and saves a new one.

### `useTheme`

- Reads the context with `useContext`.
- **Errors**: throws `Error('useTheme must be used inside a ThemeProvider')` if there is no provider, so a missing provider fails loudly instead of silently ignoring theme changes.

## Dependencies

- **Internal**: [`theme.ts`](theme.ts.md) (`ThemePreference`).
- **External**: `react` (`createContext`, `useContext`).

## Used by

- [`ThemeProvider.tsx`](ThemeProvider.tsx.md): renders `ThemeContext` with the value.
- [`AppearanceSection.tsx`](../screens/settings/AppearanceSection.tsx.md): `useTheme`.
- [`ThemeProvider.test.tsx`](ThemeProvider.test.tsx.md): `useTheme` in its probe component.

## Tests

- [`ThemeProvider.test.tsx`](ThemeProvider.test.tsx.md): reads and changes the preference through `useTheme`.

## Notes

- The error thrown outside a provider is not tested.
- The source does not say why the context is in its own file; the Fast Refresh reason above is inferred from the `react-refresh` lint rule.
