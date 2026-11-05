# [`ui/src/app/AppShell.tsx`](../../../../../ui/src/app/AppShell.tsx)

## Purpose

The app frame drawn around every screen: a skip link, a header with the product name, and the `main` landmark that holds the current screen. While the vault is unlocked the header also shows the main navigation and the Lock button. It is purely presentational; routing and locking are done by [`App`](../App.tsx.md).

## Public API

| Name            | Kind      | Signature | Description |
| --------------- | --------- | --------- | ----------- |
| `AppShellProps` | interface | `export interface AppShellProps { readonly current: UnlockedScreen \| null; readonly onNavigate: (screen: UnlockedScreen) => void; readonly onLock: () => void; readonly locking?: boolean; readonly children: ReactNode; }` | Props of `AppShell`. |
| `AppShell`      | component | `function AppShell({ current, onNavigate, onLock, locking = false, children, }: AppShellProps)` | Skip link, header with navigation and Lock button, and the main region. |

## Details

### `AppShellProps`

- `current`: the unlocked screen being shown (`'home'` or `'settings'`), or `null` while locked. Navigation and the Lock button only render when it is set.
- `onNavigate`: called with the target screen when a navigation button is pressed.
- `onLock`: called when the Lock button is pressed.
- `locking`: optional, defaults to `false`; whether a lock request is in progress.
- `children`: the current screen.

### `AppShell`

- **State**: none.
- **Rendered output**:
  1. `<a class="skip-link" href="#main">Skip to main content</a>`.
  2. `<header>` with the brand text "NovaVault".
  3. When `current !== null`: a `<nav aria-label="Main">` list with two buttons from the module constant `NAVIGATION`, "Vault" (`home`) and "Settings" (`settings`), followed by a secondary [`Button`](../components/Button.tsx.md) labelled "Lock", or "Locking…" with `busy` while `locking` is true.
  4. `<main id="main">` containing `children`.
- **User flows**: pressing "Vault" or "Settings" calls `onNavigate`; pressing "Lock" calls `onLock`. While busy, `Button` disables itself, so repeated lock presses are ignored.
- **Accessibility**:
  - The skip link lets keyboard users jump past the header to `#main`.
  - The navigation is a labelled `nav` landmark; the active entry carries `aria-current="page"`.
  - Navigation entries are `<button type="button">` rather than links, because there are no URLs.
  - The Lock button's busy state is exposed through `Button` (`aria-busy`, `disabled`).
- **Tauri calls**: none directly. The Lock button's `onLock` handler in `App` calls `lock`.
- **Security**: hiding navigation while `current` is `null` is cosmetic; the real guard is the router, which ignores `navigate` unless the route is already unlocked.

## Dependencies

- **Internal**: [`Button`](../components/Button.tsx.md) for the Lock button; [`app/router.ts`](../../../../../ui/src/app/router.ts) for the `UnlockedScreen` type.
- **External**: `react`: the `ReactNode` type.

## Used by

- [`App.tsx`](../App.tsx.md): wraps every screen.

## Tests

- No dedicated test file. [`App.test.tsx`](../../../../../ui/src/App.test.tsx) checks the skip link target, `aria-current` on the Vault entry, and that the Lock button and navigation disappear after locking.

## Notes

- Not applicable: no known limitations.
