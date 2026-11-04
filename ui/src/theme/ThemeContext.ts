import { createContext, useContext } from 'react';

import type { ThemePreference } from './theme';

/** Current theme preference and a setter, shared through context. */
export interface ThemeContextValue {
  /** The active preference. */
  readonly preference: ThemePreference;
  /** Applies and saves a new preference. */
  readonly setPreference: (preference: ThemePreference) => void;
}

/** Theme context; provided by `ThemeProvider`. */
export const ThemeContext = createContext<ThemeContextValue | null>(null);

/**
 * Returns the theme preference and its setter.
 *
 * @throws Error if called outside a `ThemeProvider`.
 */
export function useTheme(): ThemeContextValue {
  const value = useContext(ThemeContext);
  if (value === null) {
    throw new Error('useTheme must be used inside a ThemeProvider');
  }
  return value;
}
