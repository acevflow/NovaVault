import { useId } from 'react';

import { useTheme } from '../../theme/ThemeContext';
import { THEME_PREFERENCES, type ThemePreference } from '../../theme/theme';

const LABELS: Readonly<Record<ThemePreference, string>> = {
  system: 'Match system',
  light: 'Light',
  dark: 'Dark',
};

/** Chooses the colour theme. Applies at once; nothing to save. */
export function AppearanceSection() {
  const headingId = useId();
  const name = useId();
  const { preference, setPreference } = useTheme();
  return (
    <section className="card" aria-labelledby={headingId}>
      <h2 id={headingId}>Appearance</h2>
      <fieldset className="fieldset">
        <legend>Theme</legend>
        <div className="choice-row">
          {THEME_PREFERENCES.map((option) => (
            <label key={option} className="choice">
              <input
                type="radio"
                name={name}
                value={option}
                checked={preference === option}
                onChange={() => {
                  setPreference(option);
                }}
              />
              {LABELS[option]}
            </label>
          ))}
        </div>
      </fieldset>
    </section>
  );
}
