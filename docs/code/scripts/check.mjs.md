# [`scripts/check.mjs`](../../../scripts/check.mjs)

## Purpose

The single full local check required by the project rules, run with `npm run check` from the repository root (see [`package.json`](../package.json.md)). It runs formatting checks, linters, the TypeScript typecheck, the tests and the UI build for both the UI and the Rust workspace, in a fixed order, and stops at the first failure. [CI](../.github/workflows/ci.yml.md) runs the same commands on all three platforms.

## Public API

_Not applicable: the script exports nothing. Its interface is the command line and the exit code._

| Item          | Kind        | Signature / value                                   | Description                                               |
| ------------- | ----------- | --------------------------------------------------- | --------------------------------------------------------- |
| Invocation    | CLI         | `npm run check` (or `node scripts/check.mjs`)       | Takes no arguments; can be run from any working directory. |
| Exit code     | process     | `0` on success, the failing step's status otherwise (`1` if it has none) | Usable by hooks and scripts.                |
| `root`        | module constant | `join(dirname(fileURLToPath(import.meta.url)), '..')` | Repository root, resolved from the script's location. |
| `ui`          | module constant | `join(root, 'ui')`                              | UI package directory.                                     |
| `steps`       | module constant | `{ name: string, command: string, cwd: string }[]` | The ordered list of steps to run.                       |

## Details

### Steps

Built in this order:

| #  | Name                       | Command                                                        | Directory | Checks                                                       |
| -- | -------------------------- | -------------------------------------------------------------- | --------- | ------------------------------------------------------------ |
| 0  | UI: install dependencies   | `npm ci`                                                       | `ui/`     | Only added if `ui/node_modules` does not exist.              |
| 1  | UI: format                 | `npm run format:check`                                         | `ui/`     | Prettier formatting.                                         |
| 2  | UI: lint                   | `npm run lint`                                                 | `ui/`     | ESLint, zero warnings allowed.                               |
| 3  | UI: typecheck              | `npm run typecheck`                                            | `ui/`     | `tsc -b` in strict mode.                                     |
| 4  | UI: test                   | `npm test`                                                     | `ui/`     | Vitest, run once.                                            |
| 5  | UI: build                  | `npm run build`                                                | `ui/`     | Builds `ui/dist`.                                            |
| 6  | Rust: format               | `cargo fmt --all -- --check`                                   | root      | `rustfmt` formatting of the workspace.                       |
| 7  | Rust: clippy               | `cargo clippy --workspace --all-targets --locked -- -D warnings` | root    | Lints, including tests, with warnings as errors and `Cargo.lock` unchanged. |
| 8  | Rust: test                 | `cargo test --workspace --locked`                              | root      | Unit tests and doctests of both crates.                      |

The UI scripts are defined in [`ui/package.json`](../ui/package.json.md); the Rust lint levels come from the [workspace manifest](../Cargo.toml.md).

### Execution

1. Resolve `root` and `ui` from the script's own path, so the result does not depend on the current directory.
2. Build `steps`, prepending `npm ci` when `ui/node_modules` is missing.
3. For each step, print `[<n>/<total>] <name>: <command>` and run it with `spawnSync(command, { cwd, stdio: 'inherit', shell: true })`, so its output streams straight to the terminal.
4. If the exit status is not `0`, print `Check failed at "<name>".` to stderr and exit with that status. `result.status` is `null` when the process was killed by a signal or could not be started; the script then exits with `1`.
5. After the last step, print `All checks passed.`

### Ordering

The UI is built before any Rust step because the Tauri shell embeds `ui/dist` at compile time; Clippy and the tests compile `src-tauri` and fail if `ui/dist` is missing. Cheap checks (formatting, lint) come first so the most common failures are reported quickly.

### Security notes

- **`shell: true`.** Needed so that `npm` resolves to `npm.cmd` on Windows. Every command is a fixed string defined in the script; no argument, environment variable or file content is interpolated into a command, so no untrusted input reaches the shell.
- **No dependencies.** The script imports only Node built-ins (`node:child_process`, `node:fs`, `node:path`, `node:url`), so it runs before anything is installed and adds no supply-chain surface.
- **`npm ci`, not `npm install`.** When it installs, it installs exactly what `ui/package-lock.json` records and never rewrites the lockfile.
- **`--locked`** on the Cargo steps fails instead of silently updating `Cargo.lock`.

## Dependencies

- **Internal**: [`ui/package.json`](../ui/package.json.md) (the `format:check`, `lint`, `typecheck`, `test` and `build` scripts); the workspace [`Cargo.toml`](../Cargo.toml.md).
- **External**: Node built-in modules only; `npm`, `cargo`, `rustfmt` and `clippy` must be on `PATH`.

## Used by

- [`package.json`](../package.json.md): the `check` script.
- [`CONTRIBUTING.md`](../../../CONTRIBUTING.md) and [`docs/development.md`](../../development.md#the-full-local-check), which ask contributors to run it before every push.
- [`ci.yml`](../.github/workflows/ci.yml.md) mirrors its steps and order (it does not call the script).

## Tests

_Not applicable: the script has no tests. CI runs the same commands on every pull request._

## Notes

- **Inconsistency:** the header comment says it runs "tests and builds for the UI and the Rust workspace", and [`CONTRIBUTING.md`](../../../CONTRIBUTING.md) and the [CHANGELOG](../../../CHANGELOG.md) say the same. Only the UI is built (`vite build`); Rust code is compiled by Clippy and the tests, but no `cargo build` or `tauri build` is run.
- Unlike CI, the script skips `npm ci` when `ui/node_modules` exists, so stale local dependencies are not refreshed; delete `ui/node_modules` to force a clean install.
- The steps are duplicated in [`ci.yml`](../.github/workflows/ci.yml.md) rather than shared; a change to one must be made in the other by hand.
- On machines where application control blocks build scripts (for example Windows WDAC), the Rust steps that compile `src-tauri` can fail locally for reasons unrelated to the code (see [Troubleshooting](../../development.md#troubleshooting)).
