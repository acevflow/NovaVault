# [`src-tauri/tauri.conf.json`](../../../src-tauri/tauri.conf.json)

## Purpose

The Tauri 2 configuration of the desktop app. It names the app, tells the Tauri CLI how to run and bundle the React UI, defines the single `main` window, and sets the webview's security policy: the Content Security Policy for production and development, prototype freezing, no global Tauri object, and the one capability the window gets. [`build.rs`](build.rs.md) validates it and `tauri::generate_context!` in [`main.rs`](src/main.rs.md) embeds it in the binary.

## Public API

### Top level

| Key           | Type   | Value                              | Purpose                                                                 |
| ------------- | ------ | ---------------------------------- | ----------------------------------------------------------------------- |
| `$schema`     | string | `https://schema.tauri.app/config/2` | Editor validation only; not read at runtime.                           |
| `productName` | string | `NovaVault`                        | Display and bundle name.                                                |
| `version`     | string | `0.1.0`                            | App version; matches the workspace version.                             |
| `identifier`  | string | `io.github.acevflow.novavault`     | Reverse-DNS app id. Also decides the app data directory where the vault lives. |

### `build`

| Key                  | Type   | Value                          | Purpose                                                        |
| -------------------- | ------ | ------------------------------ | -------------------------------------------------------------- |
| `devUrl`             | string | `http://localhost:5173`        | Where `tauri dev` loads the UI (the Vite dev server).          |
| `frontendDist`       | string | `../ui/dist`                   | Built UI embedded in release binaries.                         |
| `beforeDevCommand`   | string | `npm --prefix ui run dev`      | Starts Vite before `tauri dev`.                                |
| `beforeBuildCommand` | string | `npm --prefix ui run build`    | Builds the UI before `tauri build`.                            |

### `app`

| Key                         | Type    | Value                        | Purpose                                                       |
| --------------------------- | ------- | ---------------------------- | ------------------------------------------------------------- |
| `withGlobalTauri`           | boolean | `false`                      | No `window.__TAURI__` object in the webview.                  |
| `windows[0].label`          | string  | `main`                       | Window label; the capability is scoped to it.                 |
| `windows[0].title`          | string  | `NovaVault`                  | Window title.                                                 |
| `windows[0].width`/`height` | number  | `1024` × `720`               | Initial size.                                                 |
| `windows[0].minWidth`/`minHeight` | number | `640` × `480`         | Minimum size.                                                 |
| `security.csp`              | string  | see [Content Security Policy](#content-security-policy) | Production CSP.                     |
| `security.devCsp`           | string  | see [Content Security Policy](#content-security-policy) | CSP used by `tauri dev`.            |
| `security.freezePrototype`  | boolean | `true`                       | Freezes `Object.prototype`.                                   |
| `security.capabilities`     | array   | `["default"]`                | The only capability enabled: [`default.json`](capabilities/default.json.md). |

### `bundle`

| Key                | Type   | Value                                                              | Purpose                                      |
| ------------------ | ------ | ------------------------------------------------------------------ | -------------------------------------------- |
| `active`           | boolean | `true`                                                            | `tauri build` produces installers.           |
| `targets`          | string | `all`                                                              | Every installer format of the host platform. |
| `icon`             | array  | `icons/32x32.png`, `icons/128x128.png`, `icons/128x128@2x.png`, `icons/icon.icns`, `icons/icon.ico` | App icons per platform. |
| `category`         | string | `Utility`                                                          | Store / desktop-entry category.              |
| `shortDescription` | string | `Local-first encrypted vault`                                      | Installer summary.                           |
| `license`          | string | `GPL-3.0-or-later`                                                 | License recorded in packages.                |

## Details

### Content Security Policy

The CSP is what limits a script injected into the webview (for example through a rendering bug) to the app's own code and the IPC channel. Keys never cross IPC, so the CSP is a second line of defence, not the only one.

Production policy (`csp`), directive by directive:

| Directive                                 | Allows                                   | Denies, and why                                                                                                 |
| ----------------------------------------- | ---------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `default-src 'self'`                      | Same-origin resources                    | Any resource type not listed below from any other origin. The fallback for everything unlisted.                  |
| `script-src 'self'`                       | The bundled scripts                      | Inline scripts, `eval` and `new Function` (no `'unsafe-inline'`/`'unsafe-eval'`), and remote scripts: an injected `<script>` or `onclick` cannot run. |
| `style-src 'self'`                        | The bundled stylesheet                   | Inline `<style>` and `style=""` attributes, and remote stylesheets, which could be used for CSS-based data exfiltration. |
| `img-src 'self' data:`                    | Bundled images and `data:` URIs          | Remote images, so an injected `<img src="https://…">` cannot leak data in a URL.                                 |
| `font-src 'self'`                         | Bundled fonts                            | Remote fonts.                                                                                                   |
| `connect-src ipc: http://ipc.localhost`   | The Tauri IPC channel only               | `fetch`, XHR, WebSocket and `EventSource` to anywhere else: the UI has no network access. `ipc:` is the IPC scheme on macOS and Linux; `http://ipc.localhost` is how it appears on Windows (WebView2). |
| `object-src 'none'`                       | Nothing                                  | `<object>` and `<embed>` plugins.                                                                               |
| `base-uri 'none'`                         | Nothing                                  | `<base>`, so injected markup cannot redirect relative URLs.                                                     |
| `form-action 'none'`                      | Nothing                                  | Form submissions to any URL; forms are handled in React and sent over IPC.                                      |
| `frame-ancestors 'none'`                  | Nothing                                  | Embedding the app page in a frame.                                                                              |

`frame-src` and `worker-src` are not set, so they fall back to `default-src 'self'`.

Development policy (`devCsp`), used only by `tauri dev`, differs in exactly two directives:

| Directive     | Added                                         | Why                                                                                      |
| ------------- | --------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `script-src`  | `'unsafe-inline'`                             | Vite's React plugin injects an inline preamble for fast refresh.                         |
| `style-src`   | `'unsafe-inline'`                             | Vite injects CSS as inline `<style>` elements during development.                        |
| `connect-src` | `ws://localhost:5173`, `http://localhost:5173` | The Vite dev server and its hot-module-replacement WebSocket.                           |

Release builds never use `devCsp`; the extra sources cannot reach a shipped binary. Port 5173 matches `server.port` with `strictPort` in `ui/vite.config.ts`, which also binds Vite to `localhost` only.

### `freezePrototype`

Tauri freezes `Object.prototype` before the app's scripts run, so an injected script or a compromised dependency cannot add or replace properties on it (prototype pollution) to tamper with objects the UI builds, including the arguments it passes to `invoke`. Other built-in prototypes (for example `Array.prototype`) are not frozen by this setting.

### `withGlobalTauri`

With `false`, Tauri does not expose `window.__TAURI__`. The UI reaches the shell only through the `invoke` function imported from `@tauri-apps/api`, so script that is not part of the bundle has no ready-made handle to the IPC API.

### `capabilities`

Only the capability with identifier `default` is enabled for the app. Tauri would otherwise enable every capability file found in `capabilities/`; listing it explicitly means a stray file added there grants nothing. What the capability grants is documented in [`default.json.md`](capabilities/default.json.md).

### Settings not present

Some security-relevant settings are absent and therefore take Tauri's defaults: no `assetProtocol` (the `asset:` scheme for reading local files is off), no `dangerousDisableAssetCspModification` (Tauri may add nonces or hashes for its own injected scripts to the CSP), no `plugins` section (the dialog plugin needs no configuration), and no remote URLs in windows (the UI is loaded from the embedded `frontendDist`).

## Dependencies

- **Internal**: [`capabilities/default.json`](capabilities/default.json.md) (by identifier); `ui/dist` (built UI) and `ui/vite.config.ts` (dev server port); the icons under `src-tauri/icons/`.
- **External**: the Tauri 2 CLI and `tauri-build`, which read this file.

## Used by

- [`build.rs`](build.rs.md) (`tauri_build::try_build`) and `tauri::generate_context!` in [`main.rs`](src/main.rs.md).
- The Tauri CLI (`tauri dev`, `tauri build`), as run by the project scripts.

## Tests

_Not applicable: no test reads this file. `tauri-build` rejects an invalid config at build time, so every `cargo build` and `cargo test` of the shell validates it._

## Notes

- **Minor inconsistency:** `docs/architecture.md` says `freezePrototype` stops script from tampering with "built-in prototypes" (plural); Tauri's setting freezes `Object.prototype` only.
- `docs/architecture.md` and `docs/threat-model.md` describe the production CSP as allowing "no inline scripts or styles and no `eval`" and "only IPC connections"; this matches the file. `docs/development.md` says `devCsp` adds the Vite dev server and inline HMR code, which also matches.
- **Open question:** `img-src` allows `data:`, but the UI currently contains no images or `data:` URIs. It may be kept for small assets Vite inlines as `data:` URIs; the file does not say why.
- `font-src 'self'` has no consumer either (the UI ships no font files); it is harmless.
- `version` here and in the workspace `Cargo.toml` must be bumped together; nothing checks it.
