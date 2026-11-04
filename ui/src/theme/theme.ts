/**
 * Colour theme preference: follow the system, or force light or dark.
 *
 * The preference is not secret, so it is kept in `localStorage`. Storage
 * can be unavailable (blocked or private mode); every access is guarded
 * and the app falls back to the system theme.
 */

/** The user's theme choice. */
export type ThemePreference = 'system' | 'light' | 'dark';

/** Every preference, in display order. */
export const THEME_PREFERENCES: readonly ThemePreference[] = ['system', 'light', 'dark'];

const STORAGE_KEY = 'novavault.theme';

function isThemePreference(value: unknown): value is ThemePreference {
  return value === 'system' || value === 'light' || value === 'dark';
}

/** Reads the saved preference, or `system` if none is saved or storage fails. */
export function loadThemePreference(): ThemePreference {
  try {
    const saved = window.localStorage.getItem(STORAGE_KEY);
    return isThemePreference(saved) ? saved : 'system';
  } catch {
    return 'system';
  }
}

/** Saves the preference; a storage failure only means it is not remembered. */
export function saveThemePreference(preference: ThemePreference): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, preference);
  } catch {
    // Not remembered across restarts; the current session still applies it.
  }
}

/**
 * Applies the preference to the document. `system` removes the override
 * so the stylesheet's `prefers-color-scheme` rules decide.
 */
export function applyThemePreference(preference: ThemePreference): void {
  const root = document.documentElement;
  if (preference === 'system') {
    delete root.dataset.theme;
  } else {
    root.dataset.theme = preference;
  }
}
