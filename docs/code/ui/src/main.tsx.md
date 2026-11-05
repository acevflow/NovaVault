# [`ui/src/main.tsx`](../../../../ui/src/main.tsx)

## Purpose

The UI entry point, loaded by [`index.html`](../index.html.md). It imports the global stylesheet, finds the `#root` element and renders [`App`](App.tsx.md) inside React's `StrictMode`.

## Public API

_Not applicable: this file exports nothing. It runs once, for its side effects, when the page loads._

## Details

### Module body

1. Imports [`styles.css`](styles.css.md), which Vite bundles as a separate stylesheet.
2. Looks up `document.getElementById('root')`.
3. Throws `Error('Root element #root not found in index.html')` if the element is missing, so a broken page fails loudly instead of rendering nothing.
4. Calls `createRoot(rootElement).render(<StrictMode><App /></StrictMode>)`.

- **Security**: handles no secrets. `StrictMode` affects development builds only; it renders components twice and runs effects twice to expose impure code.
- **Edge cases and invariants**: the id must match the `<div id="root">` in `index.html`.

## Dependencies

- **Internal**: [`App.tsx`](App.tsx.md) (`App`), [`styles.css`](styles.css.md).
- **External**: `react` (`StrictMode`), `react-dom/client` (`createRoot`).

## Used by

- [`index.html`](../index.html.md): loads it with `<script type="module" src="/src/main.tsx">`.

## Tests

_Not applicable: no test imports this file; tests render `App` directly, as in [`App.test.tsx`](App.test.tsx.md)._

## Notes

- None.
