import { act, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { useTheme } from './ThemeContext';
import { ThemeProvider } from './ThemeProvider';

function Probe() {
  const { preference, setPreference } = useTheme();
  return (
    <button
      type="button"
      onClick={() => {
        setPreference('dark');
      }}
    >
      {preference}
    </button>
  );
}

describe('ThemeProvider', () => {
  it('follows the system theme by default', () => {
    render(
      <ThemeProvider>
        <Probe />
      </ThemeProvider>,
    );
    expect(screen.getByRole('button')).toHaveTextContent('system');
    expect(document.documentElement.dataset.theme).toBeUndefined();
  });

  it('applies and remembers a forced theme', () => {
    const { unmount } = render(
      <ThemeProvider>
        <Probe />
      </ThemeProvider>,
    );
    act(() => {
      screen.getByRole('button').click();
    });
    expect(document.documentElement.dataset.theme).toBe('dark');
    unmount();

    render(
      <ThemeProvider>
        <Probe />
      </ThemeProvider>,
    );
    expect(screen.getByRole('button')).toHaveTextContent('dark');
  });

  it('restores a saved light theme', () => {
    window.localStorage.setItem('novavault.theme', 'light');
    render(
      <ThemeProvider>
        <Probe />
      </ThemeProvider>,
    );
    expect(document.documentElement.dataset.theme).toBe('light');
  });
});
