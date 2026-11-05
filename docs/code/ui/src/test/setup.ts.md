# [`ui/src/test/setup.ts`](../../../../../ui/src/test/setup.ts)

## Purpose

The Vitest setup file, run before every test file through `test.setupFiles` in [`vite.config.ts`](../../vite.config.ts.md). It registers the jest-dom matchers and resets shared state after each test, so tests cannot affect each other.

## Public API

_Not applicable: this file exports nothing; it runs for its side effects._

## Details

### Module body

1. `import '@testing-library/jest-dom/vitest'` adds matchers such as `toBeInTheDocument` and `toHaveAccessibleDescription` to Vitest's `expect`.
2. Registers an `afterEach` hook that:
   1. `cleanup()`: unmounts everything rendered with Testing Library;
   2. `clearMocks()`: removes the fake IPC installed by [`mockCommands`](ipc.ts.md);
   3. deletes `document.documentElement.dataset.theme`, which [`theme.ts`](../theme/theme.ts.md) sets;
   4. `window.localStorage.clear()`: forgets any saved theme preference.

- **Edge cases and invariants**: Vitest's `restoreMocks` (also in `vite.config.ts`) separately restores `vi.spyOn` spies such as the `window.print` spy.

## Dependencies

- **Internal**: none.
- **External**: `@testing-library/jest-dom` (matchers), `@tauri-apps/api/mocks` (`clearMocks`), `@testing-library/react` (`cleanup`), `vitest` (`afterEach`).

## Used by

- [`vite.config.ts`](../../vite.config.ts.md): `test.setupFiles: ['./src/test/setup.ts']`.

## Tests

_Not applicable: test infrastructure. [`ThemeProvider.test.tsx`](../theme/ThemeProvider.test.tsx.md) relies on the theme and storage reset: its first test expects no saved preference._

## Notes

- None.
