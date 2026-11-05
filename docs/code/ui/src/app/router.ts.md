# [`ui/src/app/router.ts`](../../../../../ui/src/app/router.ts)

## Purpose

Screen routing as a typed state machine. The app has a few screens and no URLs to deep-link to, so a reducer replaces a router library. A screen that needs an unlocked vault can only be reached through an `unlocked` transition, and a `locked` action always lands on the unlock screen. [`App`](../App.tsx.md) runs the reducer with `useReducer`.

## Public API

| Name              | Kind     | Signature                                                                              | Description                                            |
| ----------------- | -------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| `Route`           | type     | `export type Route = { readonly name: 'loading' } \| { readonly name: 'startupError'; readonly message: string } \| { readonly name: 'create' } \| { readonly name: 'unlock' } \| { readonly name: 'home' } \| { readonly name: 'settings' }` | The screen being shown. |
| `UnlockedScreen`  | type     | `export type UnlockedScreen = 'home' \| 'settings'`                                    | Screens reachable from the navigation while unlocked.  |
| `RouterAction`    | type     | `export type RouterAction = { readonly type: 'statusLoaded'; readonly status: VaultStatus } \| { readonly type: 'statusFailed'; readonly message: string } \| { readonly type: 'retry' } \| { readonly type: 'unlocked' } \| { readonly type: 'locked' } \| { readonly type: 'navigate'; readonly to: UnlockedScreen }` | Something that changes the route. |
| `initialRoute`    | constant | `export const initialRoute: Route`                                                     | `{ name: 'loading' }`.                                 |
| `isUnlockedRoute` | function | `export function isUnlockedRoute(route: Route): route is { readonly name: UnlockedScreen }` | Whether the route requires an unlocked vault.     |
| `routerReducer`   | function | `export function routerReducer(route: Route, action: RouterAction): Route`             | Computes the next route.                               |

## Details

### `Route`

- `loading`: the vault status is not known yet. `startupError`: the status check failed; `message` is the user-facing text from [`errorMessage`](../api/errors.ts.md). `create`, `unlock`, `home` (the item list) and `settings`: the corresponding screens.

### `initialRoute`

- `{ name: 'loading' }`. [`App`](../App.tsx.md) calls `vaultStatus()` whenever the route is `loading`, so returning to this route triggers a new status check.

### `isUnlockedRoute`

- Returns `true` for `home` and `settings`, and narrows the type so `route.name` is an `UnlockedScreen`. `App` uses it to pass the current screen to the navigation, or `null` to hide it.

### `routerReducer`

| Action         | Next route                                                                               |
| -------------- | ---------------------------------------------------------------------------------------- |
| `statusLoaded` | `create` if `!status.exists`; otherwise `home` if `status.unlocked`, else `unlock`.      |
| `statusFailed` | `{ name: 'startupError', message }`.                                                      |
| `retry`        | `initialRoute`.                                                                          |
| `unlocked`     | `home`, from any route.                                                                  |
| `locked`       | `unlock`, from any route.                                                                |
| `navigate`     | `{ name: to }` if the current route is an unlocked route; otherwise the same route object, unchanged. |

- **Security**: `navigate` is ignored outside the unlocked screens, so a stale click can never reveal an unlocked screen. Returning the same object makes React skip the re-render.
- **Edge cases and invariants**: the reducer is pure and has no default branch. The `switch` covers every action type; if a new action were added without a case, the compiler would report that the function does not always return a `Route`. It does not check the vault itself: `unlocked` is trusted to be dispatched only after a successful `unlock` or `createVault`.

## Dependencies

- **Internal**: [`api/types.ts`](../api/types.ts.md) (`VaultStatus`).
- **External**: none.

## Used by

- [`App.tsx`](../App.tsx.md): `useReducer(routerReducer, initialRoute)`, `isUnlockedRoute`, and the `Route` type for its screen switch.
- [`AppShell.tsx`](AppShell.tsx.md): the `UnlockedScreen` type for its navigation.

## Tests

- [`router.test.ts`](router.test.ts.md): every transition except `unlocked`.
- [`App.test.tsx`](../App.test.tsx.md): the reducer driven through the real app.

## Notes

- The `unlocked` transition itself has no unit test in `router.test.ts`.
