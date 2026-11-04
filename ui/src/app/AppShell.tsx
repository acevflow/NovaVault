import type { ReactNode } from 'react';

import { Button } from '../components/Button';
import type { UnlockedScreen } from './router';

/** Props of {@link AppShell}. */
export interface AppShellProps {
  /**
   * The unlocked screen being shown, or `null` while locked. Navigation and
   * the lock button only appear when this is set.
   */
  readonly current: UnlockedScreen | null;
  /** Opens an unlocked screen. */
  readonly onNavigate: (screen: UnlockedScreen) => void;
  /** Locks the vault. */
  readonly onLock: () => void;
  /** Whether a lock request is in progress. */
  readonly locking?: boolean;
  /** The current screen. */
  readonly children: ReactNode;
}

const NAVIGATION: readonly { readonly screen: UnlockedScreen; readonly label: string }[] = [
  { screen: 'home', label: 'Vault' },
  { screen: 'settings', label: 'Settings' },
];

/** App frame: skip link, header with navigation and lock button, and the main region. */
export function AppShell({
  current,
  onNavigate,
  onLock,
  locking = false,
  children,
}: AppShellProps) {
  return (
    <div className="shell">
      <a className="skip-link" href="#main">
        Skip to main content
      </a>
      <header className="shell__header">
        <span className="shell__brand">NovaVault</span>
        {current === null ? null : (
          <>
            <nav aria-label="Main" className="shell__nav">
              <ul>
                {NAVIGATION.map(({ screen, label }) => (
                  <li key={screen}>
                    <button
                      type="button"
                      className="shell__nav-link"
                      aria-current={current === screen ? 'page' : undefined}
                      onClick={() => {
                        onNavigate(screen);
                      }}
                    >
                      {label}
                    </button>
                  </li>
                ))}
              </ul>
            </nav>
            <Button variant="secondary" busy={locking} onClick={onLock}>
              {locking ? 'Locking…' : 'Lock'}
            </Button>
          </>
        )}
      </header>
      <main id="main" className="shell__main">
        {children}
      </main>
    </div>
  );
}
