# [`ui/vite.config.ts`](../../../ui/vite.config.ts)

## Purpose

Configures Vite (dev server and production bundle) and Vitest (test runner) for the UI. The dev server settings match what the Tauri shell expects from [`tauri.conf.json`](../src-tauri/tauri.conf.json.md), and the test settings set up the jsdom environment that every UI test runs in.

## Public API

| Key                           | Type     | Value                     | Purpose                                                                 |
| ----------------------------- | -------- | ------------------------- | ----------------------------------------------------------------------- |
| `plugins`                     | array    | `[react()]`               | JSX transform and Fast Refresh.                                         |
| `clearScreen`                 | boolean  | `false`                   | Keeps Rust compiler output visible in the terminal during `tauri dev`.  |
| `server.port`                 | number   | `5173`                    | Fixed dev port.                                                         |
| `server.strictPort`           | boolean  | `true`                    | Fails instead of moving to another port.                                |
| `server.host`                 | string   | `'localhost'`             | Binds the dev server to localhost only.                                 |
| `envPrefix`                   | string[] | `['VITE_', 'TAURI_ENV_']` | Environment variables exposed to client code through `import.meta.env`. |
| `build.target`                | string   | `'es2022'`                | Output language level.                                                  |
| `build.sourcemap`             | boolean  | `false`                   | No source maps in release bundles.                                      |
| `build.chunkSizeWarningLimit` | number   | `1500`                    | Chunk size in kB above which Vite warns.                                |
| `test.environment`            | string   | `'jsdom'`                 | DOM implementation for tests.                                           |
| `test.setupFiles`             | string[] | `['./src/test/setup.ts']` | Runs before every test file.                                            |
| `test.restoreMocks`           | boolean  | `true`                    | Restores every mock after each test.                                    |

## Details

### Dev server

- `server.port` and `strictPort` must match `build.devUrl` (`http://localhost:5173`) in `tauri.conf.json` and the `ws://localhost:5173 http://localhost:5173` sources in its `devCsp`. If the port were taken and Vite moved to another one, the shell would load nothing, so Vite fails instead.
- **Security**: `host: 'localhost'` keeps the dev server off the local network, so other machines cannot load the development UI.

### `envPrefix`

- Only variables starting with `VITE_` or `TAURI_ENV_` are inlined into client code; nothing else in the environment reaches the bundle. No file under `ui/src` reads `import.meta.env` today.

### `build`

- `target: 'es2022'` matches the `ES2022` target in [`tsconfig.app.json`](tsconfig.app.json.md). The comment in the file says every webview Tauri targets supports it.
- `sourcemap: false`: release builds do not ship the original sources.
- `chunkSizeWarningLimit: 1500`: the zxcvbn dictionaries, loaded lazily by [`password/strength.ts`](src/password/strength.ts.md), form one chunk of about 1.2 MB, above Vite's 500 kB default. The webview reads the chunk from the local bundle; it is never downloaded.

### `test`

- The `/// <reference types="vitest/config" />` line types the `test` key.
- [`src/test/setup.ts`](src/test/setup.ts.md) registers the jest-dom matchers and cleans up after each test.
- `restoreMocks: true` restores spies after each test, so one test's IPC mock cannot leak into the next.

## Dependencies

- **Internal**: [`src/test/setup.ts`](src/test/setup.ts.md).
- **External**: `vite` (`defineConfig`), `@vitejs/plugin-react` (`react`), `vitest` (types for `test`).

## Used by

- `npm run dev`, `npm run build`, `npm run preview` and `npm test`, see [`package.json`](package.json.md).
- [`tsconfig.node.json`](tsconfig.node.json.md): type-checks this file.

## Tests

Not applicable: configuration file. Every UI test runs with the `test` settings above.

## Notes

- [`docs/development.md`](../../development.md) notes that Vitest must run from `ui/`: run from the repository root, it does not load this file, and most tests fail without the jsdom environment and setup file.
