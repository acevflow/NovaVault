# [`package.json`](../../package.json)

## Purpose

The root npm package. It is the project's task runner: its scripts are the entry points for the full local check (`npm run check`), development mode and release builds. Its only dependency is the Tauri CLI. The UI has its own, separate package in [`ui/package.json`](ui/package.json.md).

## Public API

### Package fields

| Key       | Type    | Value                | Purpose                                            |
| --------- | ------- | -------------------- | -------------------------------------------------- |
| `name`    | string  | `novavault`          | Package name (not published).                      |
| `private` | boolean | `true`               | Makes `npm publish` refuse to publish the package. |
| `version` | string  | `0.1.0`              | Matches the workspace version in [`Cargo.toml`](Cargo.toml.md). |
| `license` | string  | `GPL-3.0-or-later`   | Project license.                                   |

### `scripts`

| Script  | Command                    | Purpose                                                                                       |
| ------- | -------------------------- | --------------------------------------------------------------------------------------------- |
| `check` | `node scripts/check.mjs`   | The single full local check: format, lint, typecheck, test and build for the UI and Rust ([`check.mjs`](scripts/check.mjs.md)). |
| `tauri` | `tauri`                    | Passes arguments to the Tauri CLI, for example `npm run tauri -- --version`.                  |
| `dev`   | `tauri dev`                | Starts the Vite dev server and the Tauri shell in development mode.                            |
| `build` | `tauri build`              | Builds the UI and a release bundle of the app.                                                 |

### `devDependencies`

| Package           | Requirement | Locked | Purpose                                                       |
| ----------------- | ----------- | ------ | ------------------------------------------------------------- |
| `@tauri-apps/cli` | `^2.12.4`   | 2.12.4 | Provides the `tauri` binary used by `tauri`, `dev` and `build`. |

## Details

### `check`

Runs [`scripts/check.mjs`](scripts/check.mjs.md) with Node. The script needs no npm dependencies, so `npm run check` works without first running `npm ci` at the root. It installs the UI's dependencies itself if `ui/node_modules` is missing.

### `dev` and `build`

Both call the Tauri CLI, which reads [`src-tauri/tauri.conf.json`](src-tauri/tauri.conf.json.md):

- `tauri dev` first runs `beforeDevCommand` (`npm --prefix ui run dev`, the Vite dev server) and loads the UI from `devUrl` (`http://localhost:5173`) under the development CSP.
- `tauri build` first runs `beforeBuildCommand` (`npm --prefix ui run build`), embeds `frontendDist` (`../ui/dist`) and produces the platform bundle under the production CSP.

The Tauri CLI is a root dev dependency, so `npm ci` at the root is needed before `dev`, `build` or `tauri` (see [`docs/development.md`](../development.md#commands)).

### Security-relevant settings

- **`private: true`** prevents accidental publication of the package to the npm registry.
- **One dependency.** The root package installs only the Tauri CLI; `package-lock.json` (lockfile version 3) pins it and its platform binaries. [Dependabot](.github/dependabot.yml.md) watches it as the `npm` entry with directory `/`.

## Dependencies

- **Internal**: [`scripts/check.mjs`](scripts/check.mjs.md); [`src-tauri/tauri.conf.json`](src-tauri/tauri.conf.json.md) (read by the Tauri CLI); [`ui/package.json`](ui/package.json.md) (its `dev` and `build` scripts, through the Tauri hooks).
- **External**: `@tauri-apps/cli`: the Tauri 2 command-line tool.

## Used by

- Developers, through `npm run check`, `npm run dev`, `npm run build` and `npm run tauri`, as listed in [`docs/development.md`](../development.md#commands) and [`CONTRIBUTING.md`](../../CONTRIBUTING.md).
- [`dependabot.yml`](.github/dependabot.yml.md) (root `npm` entry).

## Tests

_Not applicable: a manifest has no tests._

## Notes

- CI does not use this file: [`ci.yml`](.github/workflows/ci.yml.md) runs the individual check steps itself and never installs the root package, so the `dev` and `build` scripts and the Tauri CLI are not exercised in CI.
- `0.1.0` is written in three places kept in sync by hand: this file, the Cargo workspace and `tauri.conf.json` (the version the bundle uses). Nothing enforces that they match.
- No `engines` field: the required Node.js version is stated only in [`docs/development.md`](../development.md#prerequisites-all-platforms) (Node.js LTS).
