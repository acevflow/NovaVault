import { describe, expect, it } from 'vitest';

import { initialRoute, routerReducer, type Route } from './router';

describe('routerReducer', () => {
  it('routes the startup status to create, unlock or home', () => {
    const status = (exists: boolean, unlocked: boolean) =>
      routerReducer(initialRoute, { type: 'statusLoaded', status: { exists, unlocked } }).name;
    expect(status(false, false)).toBe('create');
    expect(status(true, false)).toBe('unlock');
    expect(status(true, true)).toBe('home');
  });

  it('navigates only while unlocked', () => {
    const home: Route = { name: 'home' };
    expect(routerReducer(home, { type: 'navigate', to: 'settings' })).toEqual({
      name: 'settings',
    });
    const unlock: Route = { name: 'unlock' };
    expect(routerReducer(unlock, { type: 'navigate', to: 'settings' })).toBe(unlock);
  });

  it('always lands on unlock after locking', () => {
    for (const name of ['home', 'settings', 'create'] as const) {
      expect(routerReducer({ name }, { type: 'locked' })).toEqual({ name: 'unlock' });
    }
  });

  it('reports startup failures and retries', () => {
    const failed = routerReducer(initialRoute, { type: 'statusFailed', message: 'm' });
    expect(failed).toEqual({ name: 'startupError', message: 'm' });
    expect(routerReducer(failed, { type: 'retry' })).toEqual(initialRoute);
  });
});
