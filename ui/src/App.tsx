import { useCallback, useEffect, useReducer, useState } from 'react';

import { lock, vaultStatus } from './api/commands';
import { errorMessage } from './api/errors';
import { AppShell } from './app/AppShell';
import { initialRoute, isUnlockedRoute, routerReducer, type Route } from './app/router';
import { CreateVaultScreen } from './screens/create/CreateVaultScreen';
import { HomeScreen } from './screens/HomeScreen';
import { LoadingScreen, StartupErrorScreen } from './screens/StatusScreens';
import { UnlockScreen } from './screens/unlock/UnlockScreen';
import { ThemeProvider } from './theme/ThemeProvider';

/**
 * Root component: reads the vault status at startup, then routes between
 * the create, unlock and unlocked screens.
 *
 * When the shell auto-locks it reloads the webview, so this component
 * starts over and the status check sends the user to the unlock screen.
 */
export function App() {
  const [route, dispatch] = useReducer(routerReducer, initialRoute);
  const [locking, setLocking] = useState(false);

  useEffect(() => {
    if (route.name !== 'loading') {
      return;
    }
    let active = true;
    vaultStatus()
      .then((status) => {
        if (active) dispatch({ type: 'statusLoaded', status });
      })
      .catch((error: unknown) => {
        if (active) dispatch({ type: 'statusFailed', message: errorMessage(error) });
      });
    return () => {
      active = false;
    };
  }, [route.name]);

  const handleLock = useCallback(() => {
    setLocking(true);
    lock()
      // Show the unlock screen even if the call failed: `lock` only drops the
      // key from memory, and every later `unlock` locks first anyway.
      .catch(() => undefined)
      .finally(() => {
        setLocking(false);
        dispatch({ type: 'locked' });
      });
  }, []);

  return (
    <ThemeProvider>
      <AppShell
        current={isUnlockedRoute(route) ? route.name : null}
        onNavigate={(to) => {
          dispatch({ type: 'navigate', to });
        }}
        onLock={handleLock}
        locking={locking}
      >
        <Screen
          route={route}
          onRetry={() => {
            dispatch({ type: 'retry' });
          }}
          onUnlocked={() => {
            dispatch({ type: 'unlocked' });
          }}
        />
      </AppShell>
    </ThemeProvider>
  );
}

interface ScreenProps {
  readonly route: Route;
  readonly onRetry: () => void;
  readonly onUnlocked: () => void;
}

function Screen({ route, onRetry, onUnlocked }: ScreenProps) {
  switch (route.name) {
    case 'loading':
      return <LoadingScreen />;
    case 'startupError':
      return <StartupErrorScreen message={route.message} onRetry={onRetry} />;
    case 'home':
      return <HomeScreen />;
    case 'create':
      return <CreateVaultScreen onCreated={onUnlocked} />;
    case 'unlock':
      return <UnlockScreen onUnlocked={onUnlocked} />;
    case 'settings':
      // Screen added in the following commit.
      return <p>Coming soon.</p>;
  }
}
