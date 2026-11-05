# [`ui/eslint.config.js`](../../../ui/eslint.config.js)

## Purpose

The ESLint flat configuration for the UI. Every `.ts` and `.tsx` file gets type-aware strict TypeScript rules, React Hooks rules and strict accessibility rules. `.js` files get the basic rules. Any rule that would conflict with Prettier is turned off.

## Public API

| Key                                                     | Type     | Value                      | Purpose                                                              |
| ------------------------------------------------------- | -------- | -------------------------- | -------------------------------------------------------------------- |
| `ignores`                                               | string[] | `['dist', 'coverage']`     | Build and coverage output are not linted.                            |
| TS block `files`                                        | string[] | `['**/*.{ts,tsx}']`        | All TypeScript sources and tests.                                    |
| TS block `extends`                                      | config[] | see Details                | Rule sets.                                                           |
| TS block `languageOptions.ecmaVersion`                  | number   | `2022`                     | Syntax level.                                                        |
| TS block `languageOptions.globals`                      | object   | `globals.browser`          | Browser globals.                                                     |
| TS block `parserOptions.projectService`                 | boolean  | `true`                     | Finds the right `tsconfig` for each file, for the type-aware rules.  |
| TS block `parserOptions.tsconfigRootDir`                | string   | `import.meta.dirname`      | Root for the `tsconfig` lookup.                                      |
| TS block `rules['@typescript-eslint/no-explicit-any']` | string   | `'error'`                  | Bans `any`.                                                          |
| JS block `files`                                        | string[] | `['**/*.js']`              | JavaScript files (this file).                                        |
| JS block `extends`                                      | config[] | `[js.configs.recommended]` | Basic rules.                                                         |
| JS block `languageOptions.globals`                      | object   | `globals.node`             | Node globals.                                                        |
| Last entry                                              | config   | `prettier`                 | Turns off the rules Prettier owns.                                   |

## Details

### TypeScript block `extends`

1. `js.configs.recommended`: ESLint's core recommended rules.
2. `tseslint.configs.strictTypeChecked`: typescript-eslint's strictest preset, using type information (for example, no floating promises and no unsafe flows of `any`).
3. `tseslint.configs.stylisticTypeChecked`: consistent TypeScript style.
4. `reactHooks.configs.flat['recommended-latest']`: the Rules of Hooks and exhaustive effect dependencies.
5. `reactRefresh.configs.vite`: a file that exports components should export only components, so Fast Refresh works.
6. `jsxA11y.flatConfigs.strict`: strict accessibility rules for JSX, which support the "accessible components" rule in `CLAUDE.md`.

### `no-explicit-any`

- Set to `'error'` explicitly to enforce the "no `any`" rule in `CLAUDE.md`. `strictTypeChecked` already enables it, but the explicit setting keeps the rule on even if the preset changes.

### `prettier`

- Must stay last, as the comment in the file says. Later entries override earlier ones, so placing it last turns off the formatting rules of every preset above.

## Dependencies

- **Internal**: [`tsconfig.app.json`](tsconfig.app.json.md) and [`tsconfig.node.json`](tsconfig.node.json.md), through `projectService`.
- **External**: `@eslint/js`, `typescript-eslint` (the `tseslint.config` helper and presets), `eslint-plugin-react-hooks`, `eslint-plugin-react-refresh`, `eslint-plugin-jsx-a11y`, `eslint-config-prettier`, `globals`.

## Used by

- `npm run lint` (`eslint . --max-warnings 0`), see [`package.json`](package.json.md).

## Tests

Not applicable: configuration file.

## Notes

- `tseslint.config(...)` is the helper that allows `extends` inside flat config objects.
