# [`.github/dependabot.yml`](../../../.github/dependabot.yml)

## Purpose

Configures Dependabot version updates for every dependency manifest in the repository: the Cargo workspace, the UI npm package, the root npm package (the Tauri CLI) and the GitHub Actions pinned in [`ci.yml`](workflows/ci.yml.md). Minor and patch updates are grouped into one pull request per ecosystem to keep the volume manageable for a solo maintainer; major updates arrive one per pull request.

## Public API

### Top level

| Key       | Type    | Value | Purpose                              |
| --------- | ------- | ----- | ------------------------------------ |
| `version` | integer | `2`   | Dependabot configuration format.     |
| `updates` | list    | 4 entries | One entry per ecosystem and directory. |

### `updates` entries

| `package-ecosystem` | `directory` | Manifest covered                                        | `commit-message.prefix` | Group name                 |
| ------------------- | ----------- | ------------------------------------------------------- | ----------------------- | -------------------------- |
| `cargo`             | `/`         | Workspace [`Cargo.toml`](../Cargo.toml.md) and `Cargo.lock` (both crates) | `chore(deps)` | `cargo-minor-and-patch`    |
| `npm`               | `/ui`       | [`ui/package.json`](../ui/package.json.md) and `ui/package-lock.json` | `chore(deps)` | `npm-minor-and-patch`      |
| `npm`               | `/`         | Root [`package.json`](../package.json.md) and `package-lock.json` (the Tauri CLI only) | `chore(deps)` | `npm-root-minor-and-patch` |
| `github-actions`    | `/`         | Workflows under `.github/workflows/`                    | `ci(deps)`              | `actions-minor-and-patch`  |

### Settings shared by every entry

| Key                         | Value            | Purpose                                                                  |
| --------------------------- | ---------------- | ------------------------------------------------------------------------ |
| `schedule.interval`         | `weekly`         | Checks for new versions once a week.                                     |
| `open-pull-requests-limit`  | `5`              | At most five open version-update pull requests per entry.               |
| `groups.<name>.update-types`| `[minor, patch]` | Combines all minor and patch updates of the entry into one pull request. |

## Details

### Grouping

Each entry has one group matching `minor` and `patch` updates. All such updates found in a run land in a single pull request named after the group. Updates not matched by the group, that is major version bumps, are opened as individual pull requests, so each breaking change can be reviewed and tested on its own.

### Commit messages

Dependency bumps use `chore(deps)` and action bumps use `ci(deps)`, matching the Conventional Commits types required by [`CLAUDE.md`](../../../CLAUDE.md).

### Security-relevant settings

- **Pinned actions stay current.** [`ci.yml`](workflows/ci.yml.md) pins every action to a full commit SHA with a version comment. The `github-actions` entry lets Dependabot propose new SHAs, so pinning does not freeze the project on old, possibly vulnerable, action versions.
- **Every manifest is covered.** The Cargo workspace, both npm packages and the workflows each have an entry, so no dependency set is left without update pull requests.
- **Updates go through CI.** Dependabot opens ordinary pull requests, so every bump runs the full CI on all three platforms before it can be merged. Cargo builds use `--locked`, so a bump takes effect only through the updated `Cargo.lock` in the pull request.

## Dependencies

- **Internal**: the manifests and lockfiles listed above, and the workflow [`ci.yml`](workflows/ci.yml.md).
- **External**: GitHub Dependabot.

## Used by

- GitHub Dependabot reads this file from the default branch.

## Tests

_Not applicable: a configuration file has no tests. Each update pull request is checked by [CI](workflows/ci.yml.md)._

## Notes

- The groups do not set `applies-to`, so they apply to version updates only. Dependabot security updates, if enabled in the repository settings, are not configured here and are not grouped.
- `open-pull-requests-limit` counts version updates per entry; with grouping, one group pull request plus up to four major updates can be open per ecosystem.
- **Open question:** whether Dependabot updates the `dtolnay/rust-toolchain` pin, whose comment is `stable branch` rather than a version (see the Notes in [`ci.yml.md`](workflows/ci.yml.md)).
- `ignore` rules are not used: every dependency, including `tauri` and the cryptography crates, receives update pull requests.
