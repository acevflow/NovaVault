import { useCallback, useLayoutEffect, useMemo, useState, type ReactNode } from 'react';

import {
  applyThemePreference,
  loadThemePreference,
  saveThemePreference,
  type ThemePreference,
} from './theme';
import { ThemeContext } from './ThemeContext';

/** Props of {@link ThemeProvider}. */
export interface ThemeProviderProps {
  /** The app. */
  readonly children: ReactNode;
}

/** Loads the saved theme preference, applies it and shares it with the app. */
export function ThemeProvider({ children }: ThemeProviderProps) {
  const [preference, setPreferenceState] = useState<ThemePreference>(loadThemePreference);

  // Layout effect: apply before paint so the app never flashes the wrong theme.
  useLayoutEffect(() => {
    applyThemePreference(preference);
  }, [preference]);

  const setPreference = useCallback((next: ThemePreference) => {
    saveThemePreference(next);
    setPreferenceState(next);
  }, []);

  const value = useMemo(() => ({ preference, setPreference }), [preference, setPreference]);
  return <ThemeContext value={value}>{children}</ThemeContext>;
}
