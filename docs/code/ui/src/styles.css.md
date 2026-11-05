# [`ui/src/styles.css`](../../../../ui/src/styles.css)

## Purpose

The one global stylesheet of the UI, imported by [`main.tsx`](main.tsx.md). It defines design tokens as custom properties on `:root`, the light and dark themes, focus and reduced-motion behavior, the print layout of the recovery sheet, and the class-based styles of every screen and component.

## Public API

| Name                        | Kind                 | Value or selector                                                          | Description                                                     |
| --------------------------- | -------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------- |
| `--color-*`                 | custom properties    | `bg`, `surface`, `surface-muted`, `text`, `text-muted`, `border`, `accent`, `accent-hover`, `on-accent`, `danger`, `danger-hover`, `success`, `warning`, `focus`, `error-bg`, `success-bg`, `warning-bg`, `info-bg` | Colour tokens, redefined for dark mode. |
| `--radius`                  | custom property      | `0.5rem`                                                                   | Corner radius.                                                  |
| `--space-1` … `--space-8`   | custom properties    | `0.25rem`, `0.5rem`, `0.75rem`, `1rem`, `1.5rem`, `2rem` (1, 2, 3, 4, 6, 8) | Spacing scale.                                                  |
| `--font-mono`               | custom property      | `ui-monospace, 'Cascadia Mono', 'SF Mono', Menlo, Consolas, monospace`     | Monospace stack for secrets and the recovery code.             |
| Dark theme                  | rule sets            | `@media (prefers-color-scheme: dark) :root:not([data-theme='light'])` and `:root[data-theme='dark']` | Dark colour values.                                           |
| `:focus-visible`            | rule                 | `outline: 3px solid var(--color-focus)`                                    | Visible keyboard focus.                                         |
| `h1:focus`                  | rule                 | `outline: none`                                                            | No outline on headings that receive focus programmatically.     |
| Reduced motion              | media query          | `@media (prefers-reduced-motion: reduce)`                                  | Near-zero transition and animation durations.                   |
| Print layout                | media query          | `@media print`                                                             | Prints only `.print-sheet`.                                     |
| `.skip-link`                | class                | —                                                                          | "Skip to main content" link, shown on focus.                    |
| `.visually-hidden`          | class                | —                                                                          | Hides content visually but keeps it for screen readers.         |
| `.print-only`, `.no-print`  | classes              | —                                                                          | Content shown only when printing, or hidden when printing.      |
| Component classes           | classes              | sections `Shell`, `Screens`, `Buttons`, `Fields`, `Alerts`, `Password strength`, `Recovery code`, `Tabs`, `Items` | Layout and look of each component. |

## Details

### Themes

- Light is the default (`color-scheme: light` and the token values on `:root`).
- Dark applies in two cases: the system prefers dark and the user has not forced light (`:root:not([data-theme='light'])` inside the media query), or the user forced dark (`:root[data-theme='dark']`). The two dark blocks hold the same values.
- [`ThemeProvider`](theme/ThemeProvider.tsx.md) sets or removes `data-theme` on `<html>` (see [`theme/theme.ts`](theme/theme.ts.md)).
- `color-scheme` changes too, so native controls and scrollbars follow the theme.

### Focus and motion

- `:focus-visible` draws a 3 px outline in `--color-focus` with a 2 px offset, for keyboard users.
- `h1:focus` removes the outline from screen headings, which are focused programmatically on navigation so screen readers announce the new screen.
- With reduced motion requested, all transitions and animations last `0.01ms`. These are the file's only `!important` declarations.

### Narrow windows

- `@media (max-width: 40rem)` reduces padding and shows the recovery code in two columns. The comment notes that the app's minimum width is 640 px, which matches `minWidth: 640` in [`tauri.conf.json`](../../src-tauri/tauri.conf.json.md). The Items section adds its own breakpoints at `48rem` and `30rem`.

### Print layout

- **Security**: only the recovery sheet is printed. Every element under `body` gets `visibility: hidden`, and only `.print-sheet` and its descendants are made visible again. App chrome and form contents (a master password field, for example) never reach the printer. Inside the sheet, `.no-print` elements are removed, and `.print-only` elements, hidden on screen, are shown.
- The print view forces light colours (white background, black text) and prints the recovery code in four columns with a plain border.

### Compatibility with the CSP

- The stylesheet loads no remote resources: no `url()`, no `@import` and no `@font-face`. Fonts come from the system stacks. This fits the production CSP (`style-src 'self'`, `font-src 'self'`).

## Dependencies

- **Internal**: none.
- **External**: none.

## Used by

- [`main.tsx`](main.tsx.md): imports it once for the whole app.
- Every component, through `className` values.
- [`ThemeProvider`](theme/ThemeProvider.tsx.md): sets the `data-theme` attribute the dark-theme selectors read.

## Tests

_Not applicable: no test imports this file or checks styles._

## Notes

- The dark token values appear twice (inside the media query and under `[data-theme='dark']`). A change to one block must be copied to the other.
