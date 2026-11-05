# [`ui/index.html`](../../../ui/index.html)

## Purpose

The single HTML page of the UI and Vite's entry point. It provides the `#root` element React renders into and loads [`src/main.tsx`](src/main.tsx.md) as an ES module.

## Public API

| Element                                      | Value                                   | Purpose                                 |
| -------------------------------------------- | --------------------------------------- | --------------------------------------- |
| `<html lang>`                                | `"en"`                                  | Document language, used by screen readers. |
| `<meta charset>`                             | `"UTF-8"`                               | Character encoding.                     |
| `<meta name="viewport">`                     | `width=device-width, initial-scale=1.0` | Layout viewport.                        |
| `<title>`                                    | `NovaVault`                             | Document title.                         |
| `<div id="root">`                            | none                                    | React mount point.                      |
| `<script type="module" src="/src/main.tsx">` | none                                    | Loads the app.                          |

## Details

### `<script type="module">`

- At build time, Vite rewrites the path to the hashed bundle in `ui/dist`.
- **Security**: the page has no inline script and no inline style, so it works under the production CSP in [`tauri.conf.json`](../src-tauri/tauri.conf.json.md) (`script-src 'self'`, `style-src 'self'`). The page sets no CSP itself; Tauri applies the policy from its configuration.

## Dependencies

- **Internal**: [`src/main.tsx`](src/main.tsx.md).
- **External**: none.

## Used by

- Vite (dev server and build), which uses `index.html` as the entry point.
- The Tauri webview, which loads the page from the dev server or from `ui/dist`.

## Tests

Not applicable: tests render components directly and never load this page.

## Notes

- `main.tsx` throws if `#root` is missing, so the two files must use the same id.
