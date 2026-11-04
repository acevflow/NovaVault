/**
 * Screen routing as a typed state machine.
 *
 * The app has a handful of screens and no URLs to deep-link to, so a reducer
 * is simpler and safer than a router library: a screen that needs an
 * unlocked vault can only be reached through an `unlocked` transition, and
 * a `locked` action always lands on the unlock screen.
 */
import type { VaultStatus } from '../api/types';

/** The screen being shown. */
export type Route =
  | { readonly name: 'loading' }
  | { readonly name: 'startupError'; readonly message: string }
  | { readonly name: 'create' }
  | { readonly name: 'unlock' }
  | { readonly name: 'home' }
  | { readonly name: 'settings' };

/** Screens reachable from the navigation while the vault is unlocked. */
export type UnlockedScreen = 'home' | 'settings';

/** Something that changes the route. */
export type RouterAction =
  | { readonly type: 'statusLoaded'; readonly status: VaultStatus }
  | { readonly type: 'statusFailed'; readonly message: string }
  | { readonly type: 'retry' }
  | { readonly type: 'unlocked' }
  | { readonly type: 'locked' }
  | { readonly type: 'navigate'; readonly to: UnlockedScreen };

/** The route before the vault status is known. */
export const initialRoute: Route = { name: 'loading' };

/** Whether `route` is a screen that requires an unlocked vault. */
export function isUnlockedRoute(route: Route): route is { readonly name: UnlockedScreen } {
  return route.name === 'home' || route.name === 'settings';
}

/** Computes the next route. */
export function routerReducer(route: Route, action: RouterAction): Route {
  switch (action.type) {
    case 'statusLoaded':
      if (!action.status.exists) {
        return { name: 'create' };
      }
      return action.status.unlocked ? { name: 'home' } : { name: 'unlock' };
    case 'statusFailed':
      return { name: 'startupError', message: action.message };
    case 'retry':
      return initialRoute;
    case 'unlocked':
      return { name: 'home' };
    case 'locked':
      return { name: 'unlock' };
    case 'navigate':
      // Navigation is only offered while unlocked; ignore it otherwise so a
      // stale click can never reveal an unlocked screen.
      return isUnlockedRoute(route) ? { name: action.to } : route;
  }
}
