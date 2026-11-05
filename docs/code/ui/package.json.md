# [`ui/package.json`](../../../ui/package.json)

## Purpose

The npm manifest of the React + TypeScript front end. It defines the scripts run by the root check ([`scripts/check.mjs`](../../../scripts/check.mjs)), CI and the Tauri CLI, and declares the UI's runtime and development dependencies. It is a separate package from the root [`package.json`](../../../package.json), which only holds the Tauri CLI.

## Public API

| Key                    | Type    | Value                         | Purpose                                                                            |
| ---------------------- | ------- | ----------------------------- | ---------------------------------------------------------------------------------- |
| `name`                 | string  | `"novavault-ui"`              | Package name; never published.                                                     |
| `private`              | boolean | `true`                        | Prevents an accidental `npm publish`.                                              |
| `version`              | string  | `"0.1.0"`                     | UI version; matches the root package.                                              |
| `license`              | string  | `"GPL-3.0-or-later"`          | Project license.                                                                   |
| `type`                 | string  | `"module"`                    | Treats `.js` files (such as [`eslint.config.js`](eslint.config.js.md)) as ES modules. |
| `scripts.dev`          | string  | `"vite"`                      | Starts the Vite dev server on port 5173.                                           |
| `scripts.build`        | string  | `"tsc -b && vite build"`      | Type-checks the project references, then bundles to `ui/dist`.                     |
| `scripts.preview`      | string  | `"vite preview"`              | Serves the built bundle locally.                                                   |
| `scripts.lint`         | string  | `"eslint . --max-warnings 0"` | Lints; any warning fails.                                                          |
| `scripts.typecheck`    | string  | `"tsc -b"`                    | Type-checks without bundling.                                                      |
| `scripts.format`       | string  | `"prettier --write ."`        | Formats the UI in place.                                                           |
| `scripts.format:check` | string  | `"prettier --check ."`        | Fails if any file is not formatted.                                                |
| `scripts.test`         | string  | `"vitest run"`                | Runs the Vitest suite once.                                                        |
| `dependencies`         | object  | see Details                   | Packages shipped in the bundle.                                                    |
| `devDependencies`      | object  | see Details                   | Build, lint, format and test tooling.                                              |

## Details

### `scripts`

- [`src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json.md) runs `npm --prefix ui run dev` as `beforeDevCommand` and `npm --prefix ui run build` as `beforeBuildCommand`, so `dev` and `build` are the contract with the Tauri shell.
- [`scripts/check.mjs`](../../../scripts/check.mjs) and [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml) run `format:check`, `lint`, `typecheck`, `test` and `build`, in that order. A comment in the CI file states that these five scripts must exist.
- `build` runs `tsc -b` first because Vite strips types without checking them.
- `lint` uses `--max-warnings 0`, so warnings fail like errors.

### `dependencies`

| Package                      | Range     | Why                                                                                       |
| ---------------------------- | --------- | ----------------------------------------------------------------------------------------- |
| `@tauri-apps/api`            | `^2.12.4` | `invoke` for the Tauri commands, imported only by [`api/commands.ts`](src/api/commands.ts.md). |
| `@zxcvbn-ts/core`            | `^4.2.0`  | Master password strength estimate in [`password/strength.ts`](src/password/strength.ts.md). |
| `@zxcvbn-ts/language-common` | `^4.1.3`  | Common-password and keyboard-pattern dictionaries for zxcvbn.                             |
| `@zxcvbn-ts/language-en`     | `^4.1.1`  | English word and name dictionaries for zxcvbn.                                            |
| `react`, `react-dom`         | `^19.3.1` | UI library and DOM renderer.                                                              |

### `devDependencies`

| Package                                                                                                                                                                      | Why                                                         |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `typescript`                                                                                                                                                                 | Compiler and type checker (`tsc -b`).                       |
| `vite`, `@vitejs/plugin-react`                                                                                                                                               | Dev server and bundler, with the JSX transform and Fast Refresh. |
| `vitest`, `jsdom`                                                                                                                                                            | Test runner and its DOM environment.                        |
| `@testing-library/react`, `@testing-library/dom`, `@testing-library/user-event`, `@testing-library/jest-dom`                                                                 | Rendering, user interaction and DOM matchers in tests.      |
| `eslint`, `@eslint/js`, `typescript-eslint`, `eslint-plugin-react-hooks`, `eslint-plugin-react-refresh`, `eslint-plugin-jsx-a11y`, `eslint-config-prettier`, `globals`       | Linting; see [`eslint.config.js`](eslint.config.js.md).     |
| `prettier`                                                                                                                                                                   | Formatting; see [`.prettierrc.json`](.prettierrc.json.md).  |
| `@types/node`, `@types/react`, `@types/react-dom`                                                                                                                            | Type definitions.                                           |

- **Security**: only `dependencies` reach the shipped bundle. None of them needs network access, and the CSP in `tauri.conf.json` blocks remote connections anyway.

## Dependencies

- **Internal**: none.
- **External**: the packages listed above.

## Used by

- [`src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json.md): runs `dev` and `build`.
- [`scripts/check.mjs`](../../../scripts/check.mjs) and [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml): run the check scripts.

## Tests

Not applicable: configuration file. CI runs `npm ci`, which fails if this manifest and [`package-lock.json`](../../../ui/package-lock.json) disagree.

## Notes

- Ranges use `^`; the installed versions are fixed by `ui/package-lock.json`, which is committed because `npm ci` and the CI npm cache require it.
- [`docs/dependencies.md`](../../dependencies.md) lists the zxcvbn packages at `4.2.0`, `4.1.3` and `4.1.1`, the lower bounds of the ranges here.
