# [`ui/src/App.tsx`](../../../../ui/src/App.tsx)

## Purpose

The root React component. It asks the shell for the vault status at startup, then routes between the loading, startup-error, create, unlock, items and settings screens using the reducer in [`app/router.ts`](../../../../ui/src/app/router.ts). It owns the manual lock action and wraps everything in the theme provider and the app frame.

## Public API

| Name  | Kind      | Signature         | Description                                                   |
| ----- | --------- | ----------------- | ------------------------------------------------------------- |
| `App` | component | `function App()` (no props) | Root component: startup status check, routing and manual lock. |

The internal `Screen` component and its `ScreenProps` interface are not exported; they are described below because they decide what is rendered.

## Details

### `App`

- **Props**: none.
- **State**:
  - `route` (`useReducer(routerReducer, initialRoute)`): the current [`Route`](../../../../ui/src/app/router.ts). Starts as `{ name: 'loading' }`.
  - `locking` (`useState(false)`): whether a manual lock request is in flight; drives the header button's busy state.
- **Startup effect**: runs whenever `route.name` changes, but only acts while it is `'loading'`.
  1. Calls `vaultStatus()`.
  2. On success dispatches `statusLoaded`; the reducer moves to `create` (no vault), `home` (already unlocked) or `unlock`.
  3. On failure dispatches `statusFailed` with `errorMessage(error)`, which shows the startup-error screen.
  4. A local `active` flag, cleared by the effect cleanup, drops results that arrive after the route has changed or the component unmounted.
- **`handleLock`** (stable `useCallback`): sets `locking`, calls `lock()`, ignores any rejection, then clears `locking` and dispatches `locked`. The unlock screen is shown even if the call failed; the source comment says this is acceptable because `lock` only drops the key from memory and every later `unlock` locks first.
- **`handleLocked`** (stable `useCallback`): dispatches `locked` without calling the shell. Screens call it when a command reports the vault is already locked. It is stable so screens can list it as an effect dependency.
- **Rendered output**: `ThemeProvider` › `AppShell` › `Screen`.
  - `AppShell` receives `current` = the route name when [`isUnlockedRoute`](../../../../ui/src/app/router.ts) is true, otherwise `null`, so navigation and the Lock button only appear while unlocked.
  - `onNavigate` dispatches `navigate`; the reducer ignores it unless the current route is unlocked.
- **User flows**:
  - First run: loading → create screen → (`onCreated`) → items.
  - Existing locked vault: loading → unlock screen → (`onUnlocked`) → items.
  - Startup failure: loading → error screen → Retry (`retry` resets to `loading`, re-running the effect).
  - Manual lock: header Lock button → `lock` → unlock screen.
  - Auto-lock: per the component's doc comment, the shell reloads the webview, so the app starts over at `loading` and the status check lands on the unlock screen. This matches [`autolock.rs`](../../src-tauri/src/autolock.rs.md).
- **Tauri calls**:
  - `vaultStatus()` → `vault_status`, on every entry into the `loading` route.
  - `lock()` → `lock`, on the header Lock button.
  See [`commands.rs`](../../src-tauri/src/commands.rs.md).
- **Accessibility**: delegated to `AppShell` (skip link, labelled navigation, `aria-current`) and to each screen.
- **Security**: the component holds no secrets. A screen that needs an unlocked vault is only reachable through an `unlocked` transition or a `statusLoaded` with `unlocked: true`; a `locked` action always lands on the unlock screen.

### `Screen` (internal)

```ts
interface ScreenProps {
  readonly route: Route;
  readonly onRetry: () => void;
  readonly onUnlocked: () => void;
  readonly onLocked: () => void;
}
```

An exhaustive `switch` on `route.name`:

| Route          | Renders                                                     |
| -------------- | ----------------------------------------------------------- |
| `loading`      | `LoadingScreen`                                             |
| `startupError` | `StartupErrorScreen` with `route.message` and `onRetry`     |
| `home`         | `ItemsScreen` with `onLocked`                               |
| `create`       | `CreateVaultScreen` with `onCreated={onUnlocked}`           |
| `unlock`       | `UnlockScreen` with `onUnlocked`                            |
| `settings`     | `SettingsScreen` with `onLocked`                            |

## Dependencies

- **Internal**:
  - [`api/commands.ts`](../../../../ui/src/api/commands.ts): `vaultStatus`, `lock`.
  - [`api/errors.ts`](../../../../ui/src/api/errors.ts): `errorMessage` for the startup error text.
  - [`app/router.ts`](../../../../ui/src/app/router.ts): `routerReducer`, `initialRoute`, `isUnlockedRoute`, `Route`.
  - [`AppShell`](app/AppShell.tsx.md), [`ThemeProvider`](theme/ThemeProvider.tsx.md), [`StatusScreens`](screens/StatusScreens.tsx.md), [`CreateVaultScreen`](screens/create/CreateVaultScreen.tsx.md), [`UnlockScreen`](screens/unlock/UnlockScreen.tsx.md), [`ItemsScreen`](screens/items/ItemsScreen.tsx.md), [`SettingsScreen`](screens/settings/SettingsScreen.tsx.md).
- **External**: `react`: `useReducer`, `useState`, `useEffect`, `useCallback`.

## Used by

- [`ui/src/main.tsx`](../../../../ui/src/main.tsx): renders `<App />` inside `StrictMode` into `#root`.

## Tests

- [`App.test.tsx`](../../../../ui/src/App.test.tsx):
  - `shows the unlocked vault with navigation when already unlocked`: an exists-and-unlocked status lands on the focused "Your vault" heading, the Vault button has `aria-current="page"` and the skip link targets `#main`.
  - `shows a startup error and retries`: a `storage` failure shows the alert; "Try again" re-runs the status check and reaches the vault.
  - `locks the vault from the header`: the Lock button calls `lock` and removes the Lock button and navigation.
- [`app/router.test.ts`](../../../../ui/src/app/router.test.ts) covers the reducer transitions `App` relies on.

## Notes

- Under `StrictMode` in development the startup effect runs twice; the `active` flag discards the first run's result, but `vault_status` is still invoked twice.
- A manual lock does not reload the webview, unlike auto-lock. State held by unmounted screens is dropped by React when the route changes to `unlock`.
