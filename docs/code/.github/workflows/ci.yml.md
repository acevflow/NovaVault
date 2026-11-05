# [`.github/workflows/ci.yml`](../../../../../.github/workflows/ci.yml)

## Purpose

The only GitHub Actions workflow. It runs the same checks as the full local check ([`scripts/check.mjs`](../../scripts/check.mjs.md)) on Linux, Windows and macOS: UI formatting, lint, typecheck, tests and build, then Rust formatting, Clippy and tests. Each group of steps runs only if its project files exist, a leftover from before the workspace and UI were scaffolded.

## Public API

### Workflow-level settings

| Key                                 | Type    | Value                                   | Purpose                                                              |
| ----------------------------------- | ------- | --------------------------------------- | -------------------------------------------------------------------- |
| `name`                              | string  | `CI`                                    | Name shown in the Actions tab and on pull request checks.            |
| `on.push.branches`                  | list    | `[main]`                                | Runs on every push to `main` (including merges).                     |
| `on.pull_request`                   | event   | all branches, default activity types    | Runs on pull requests into any branch.                               |
| `on.workflow_dispatch`              | event   | no inputs                               | Allows a manual run from the Actions tab.                            |
| `permissions.contents`              | string  | `read`                                  | Least privilege for `GITHUB_TOKEN`; every other scope is `none`.     |
| `concurrency.group`                 | string  | `ci-${{ github.ref }}`                  | One run per ref at a time.                                           |
| `concurrency.cancel-in-progress`    | boolean | `true`                                  | A new push to the same ref cancels the older run.                    |
| `env.CARGO_TERM_COLOR`              | string  | `always`                                | Colored Cargo output in logs.                                        |

### Job `check`

| Key                          | Value                                             | Purpose                                                          |
| ---------------------------- | ------------------------------------------------- | ---------------------------------------------------------------- |
| `name`                       | `Check (${{ matrix.os }})`                        | One check per OS, for example `Check (ubuntu-latest)`.           |
| `runs-on`                    | `${{ matrix.os }}`                                | Runner image from the matrix.                                    |
| `strategy.matrix.os`         | `[ubuntu-latest, windows-latest, macos-latest]`   | The three supported desktop platforms.                           |
| `strategy.fail-fast`         | `false`                                           | A failure on one OS does not cancel the others.                  |
| `defaults.run.shell`         | `bash`                                            | Same shell on all three runners (Git Bash on Windows).           |

### Steps

| #  | Step                                         | Runs when                                | What it does or checks                                                                                  |
| -- | -------------------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| 1  | Checkout                                     | always                                   | `actions/checkout` v7.0.1 with `persist-credentials: false`.                                            |
| 2  | Detect project components (`id: detect`)     | always                                   | Writes `rust`, `tauri` and `ui` outputs (`true`/`false`).                                               |
| 3  | Set up Node.js                               | `ui == 'true'`                           | `actions/setup-node` v7.1.0, `node-version: lts/*`, npm cache keyed on `ui/package-lock.json`.          |
| 4  | Install UI dependencies                      | `ui == 'true'`                           | `npm ci` in `ui/`: installs exactly what the lockfile records, fails if it is out of sync.              |
| 5  | Check UI formatting                          | `ui == 'true'`                           | `npm run format:check` (Prettier `--check`).                                                            |
| 6  | Lint UI                                      | `ui == 'true'`                           | `npm run lint` (ESLint with `--max-warnings 0`).                                                        |
| 7  | Typecheck UI                                 | `ui == 'true'`                           | `npm run typecheck` (`tsc -b`).                                                                         |
| 8  | Test UI                                      | `ui == 'true'`                           | `npm test` (`vitest run`).                                                                              |
| 9  | Build UI                                     | `ui == 'true'`                           | `npm run build` (`tsc -b && vite build`), produces `ui/dist`.                                           |
| 10 | Install Rust toolchain                       | `rust == 'true'`                         | `dtolnay/rust-toolchain`, `toolchain: stable`, components `rustfmt` and `clippy`.                       |
| 11 | Cache Rust build                             | `rust == 'true'`                         | `Swatinem/rust-cache` v2.9.3 with default settings.                                                     |
| 12 | Install Tauri system dependencies (Linux)    | `tauri == 'true'` and `runner.os == 'Linux'` | `apt-get install` of the WebKitGTK 4.1 stack and build tools.                                       |
| 13 | Check Rust formatting                        | `rust == 'true'`                         | `cargo fmt --all -- --check`.                                                                           |
| 14 | Run Clippy                                   | `rust == 'true'`                         | `cargo clippy --workspace --all-targets --locked -- -D warnings`.                                       |
| 15 | Run Rust tests                               | `rust == 'true'`                         | `cargo test --workspace --locked`.                                                                      |
| 16 | Summary                                      | `always()`                               | Writes which components were checked or skipped to the job summary.                                     |

### Pinned actions

| Action                   | Pinned SHA                                  | Comment         |
| ------------------------ | ------------------------------------------- | --------------- |
| `actions/checkout`       | `3d3c42e5aac5ba805825da76410c181273ba90b1`  | `v7.0.1`        |
| `actions/setup-node`     | `949feb2413d6458794dcd2491c4babbbce0c15c1`  | `v7.1.0`        |
| `dtolnay/rust-toolchain` | `b725eda525362e7cf34af9a7243cac1e3f01a7ec`  | `stable branch` |
| `Swatinem/rust-cache`    | `d1527c8f67ef9af357290475d4af4b0fab8f12e0`  | `v2.9.3`        |

## Details

### Triggers and concurrency

The workflow runs on pushes to `main`, on every pull request and on manual dispatch. Runs are grouped by `github.ref`, so pushing again to a pull request branch cancels the run still in progress for that branch. Pushes to `main` share the group `ci-refs/heads/main`, so a quick second merge also cancels the first merge's run.

### Component detection

The `detect` step defines a small shell function `detect <name> <path>` that prints `<name>=true` if `<path>` exists and `<name>=false` otherwise, and appends the three lines to `$GITHUB_OUTPUT` (and the log, through `tee`):

| Output  | Path tested            | Gates                                         |
| ------- | ---------------------- | --------------------------------------------- |
| `rust`  | `Cargo.toml`           | Toolchain, cache, fmt, Clippy, tests.         |
| `tauri` | `src-tauri/Cargo.toml` | The Linux system dependencies step only.      |
| `ui`    | `ui/package.json`      | Node setup and all UI steps.                  |

All three paths exist in the repository today, so every step runs on every OS (step 12 on Linux only).

### Step order

The UI steps run before the Rust steps because the Tauri shell embeds `ui/dist` at compile time: Clippy and the tests compile `src-tauri`, which fails if `ui/dist` is missing. The order matches [`scripts/check.mjs`](../../scripts/check.mjs.md).

The workflow's comment states a contract for `ui/package.json`: it must define `format:check`, `lint`, `typecheck`, `test` and `build`, and `ui/package-lock.json` must be committed. Both hold today (see [`ui/package.json.md`](../../ui/package.json.md)).

### Caching

- **npm**: `actions/setup-node` caches the npm download cache, keyed on the hash of `ui/package-lock.json`. `node_modules` itself is not cached; `npm ci` still runs every time.
- **Cargo**: `Swatinem/rust-cache` with defaults caches the Cargo registry, git dependencies and the `target/` directory, keyed on the OS, the Rust toolchain and the hashes of the Cargo manifests and `Cargo.lock`.

### Linux system dependencies

On Linux, Tauri needs WebKitGTK. The step runs `sudo apt-get update` and then installs, with `-y --no-install-recommends`: `libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`, `file`, `libxdo-dev`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`. This is the same list as the Debian/Ubuntu instructions in [`docs/development.md`](../../../development.md#linux).

### What each Rust step checks

- `cargo fmt --all -- --check` fails if any workspace file is not `rustfmt`-formatted.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` lints both crates including tests, with every warning (including `missing_docs` from the [workspace lints](../../Cargo.toml.md)) turned into an error. `--locked` fails if `Cargo.lock` would need to change.
- `cargo test --workspace --locked` runs the unit tests and doctests of both crates.

### Security-relevant settings

- **`permissions: contents: read`.** The token can read the repository and nothing else, so a compromised step cannot push, comment or create releases.
- **`persist-credentials: false`.** `actions/checkout` does not leave the token in `.git/config`, so later steps (including build scripts of dependencies) cannot reuse it for git operations.
- **Actions pinned to full commit SHAs.** A moved tag cannot change what runs. The human-readable version is kept as a trailing comment so [Dependabot](../dependabot.yml.md) can propose updates.
- **No secrets used.** The workflow references no `secrets.*` values.
- **`pull_request`, not `pull_request_target`.** Pull requests from forks run with a read-only token and no secrets.
- **`--locked`.** The exact dependency versions reviewed in `Cargo.lock` are the ones built and tested.
- **Summary inputs via `env`.** The summary step reads step outputs through environment variables (`RUST`, `UI`) instead of interpolating `${{ }}` into the script, the recommended pattern against script injection.

### Summary step

Runs even when an earlier step failed (`if: always()`). It prints a `### CI components on <OS>` heading and, for the Rust workspace and the UI, `checked` or `skipped (not present yet)`. It reports from the detection outputs only, so "checked" means the steps were scheduled, not that they passed. The `tauri` output is not reported.

## Dependencies

- **Internal**: [`scripts/check.mjs`](../../scripts/check.mjs.md) (same steps), [`ui/package.json`](../../ui/package.json.md) (the npm scripts), the workspace [`Cargo.toml`](../../Cargo.toml.md), `Cargo.lock` and `ui/package-lock.json`.
- **External**: the four pinned actions above; the GitHub-hosted `ubuntu-latest`, `windows-latest` and `macos-latest` images; Ubuntu apt packages listed above.

## Used by

- GitHub Actions, on the triggers above. The `Check (<os>)` results appear on every pull request.
- [`.github/dependabot.yml`](../dependabot.yml.md) updates the pinned actions (`github-actions` ecosystem).
- [`CONTRIBUTING.md`](../../../../CONTRIBUTING.md) and [`docs/development.md`](../../../development.md) describe CI as running the same checks as `npm run check`.

## Tests

_Not applicable: a workflow has no tests of its own. It is exercised by every pull request and push to `main`._

## Notes

- **Inconsistency:** the header comment says the steps "start checking them automatically when they land" and the summary prints `skipped (not present yet)`; both describe the pre-scaffold state. All components exist now, so the detection gating is dead weight unless a component is removed.
- **Inconsistency:** the header says CI runs "the same checks as the full local check", and [`docs/development.md`](../../../development.md) says it runs them "in the same order". One difference: `scripts/check.mjs` skips `npm ci` when `ui/node_modules` exists, while CI always runs it.
- CI compiles the Rust code through Clippy and the tests but never runs `cargo build --release` or `tauri build`, so the release bundle is not built or tested in CI. The root `package.json` (the Tauri CLI) is never installed.
- CI uses the latest stable Rust. The declared minimum, `rust-version = "1.85"` in the [workspace manifest](../../Cargo.toml.md), is not tested.
- **Open question:** `dtolnay/rust-toolchain` is pinned to a commit of its `stable` branch, not a release tag. It is unclear whether Dependabot proposes updates for a SHA whose comment is a branch name rather than a version.
- `node-version: lts/*` follows the current Node.js LTS line, so the Node version can change between runs without a commit.
