# [`Cargo.toml`](../../Cargo.toml)

## Purpose

The root Cargo manifest. It is a virtual workspace (no `[package]` of its own) that groups the two Rust crates, [`novavault-core`](crates/novavault-core/Cargo.toml.md) and the Tauri shell [`novavault`](src-tauri/Cargo.toml.md), so they share one `Cargo.lock`, one `target/` directory, common package metadata and common lint levels.

## Public API

### `[workspace]`

| Key        | Type   | Value                                   | Purpose                                                        |
| ---------- | ------ | --------------------------------------- | -------------------------------------------------------------- |
| `resolver` | string | `"3"`                                   | Cargo's feature and dependency resolver version 3.             |
| `members`  | list   | `["crates/novavault-core", "src-tauri"]` | The two crates built by `--workspace`.                         |

### `[workspace.package]`

Inherited by members that write `<key>.workspace = true`; both members inherit all six.

| Key            | Type    | Value                                    | Purpose                                         |
| -------------- | ------- | ---------------------------------------- | ----------------------------------------------- |
| `version`      | string  | `"0.1.0"`                                | Shared version of both crates.                  |
| `edition`      | string  | `"2024"`                                 | Rust edition.                                   |
| `rust-version` | string  | `"1.85"`                                 | Minimum supported Rust version (MSRV).          |
| `license`      | string  | `"GPL-3.0-or-later"`                     | Project license (SPDX expression).              |
| `repository`   | string  | `"https://github.com/acevflow/NovaVault"` | Source location.                               |
| `publish`      | boolean | `false`                                  | Prevents accidental `cargo publish` to crates.io. |

### `[workspace.lints.rust]`

| Lint          | Level    | Purpose                                                                 |
| ------------- | -------- | ----------------------------------------------------------------------- |
| `unsafe_code` | `"deny"` | Rejects `unsafe` blocks, functions and impls unless explicitly allowed. |
| `missing_docs`| `"warn"` | Warns on undocumented public items.                                     |

### `[workspace.lints.clippy]`

| Lint          | Level    | Purpose                                     |
| ------------- | -------- | ------------------------------------------- |
| `unwrap_used` | `"deny"` | Rejects `.unwrap()` on `Option` and `Result`. |
| `expect_used` | `"deny"` | Rejects `.expect()` on `Option` and `Result`. |

Both members opt in with `[lints] workspace = true`.

## Details

### Workspace layout

A virtual manifest means commands run from the root apply to the members: `cargo test --workspace`, `cargo clippy --workspace` and `cargo fmt --all` cover both crates, and `-p novavault-core` or `-p novavault` selects one. Both crates share the root `Cargo.lock` (tracked, see [`.gitignore`](.gitignore.md)) and the root `target/` directory.

### Resolver and edition

Resolver 3 is the default for edition 2024 and makes dependency resolution aware of `rust-version` (it prefers dependency versions compatible with the MSRV). Edition 2024 itself requires Rust 1.85, which is the declared `rust-version`. A virtual workspace has no edition to infer the resolver from, so it is set explicitly.

### Security-relevant settings

- **`unsafe_code = "deny"`.** The manifest comment says unsafe code is not needed anywhere and any exception must be justified explicitly. `deny` (rather than `forbid`) can be overridden locally: the shell's OS lock monitors for [Windows](src-tauri/src/os_lock/windows.rs.md) and [macOS](src-tauri/src/os_lock/macos.rs.md) use `#![allow(unsafe_code)]` for their FFI. [`novavault-core`](crates/novavault-core/src/lib.rs.md) tightens it to `#![forbid(unsafe_code)]`, which cannot be overridden, so the cryptography crate contains no `unsafe` at all.
- **`unwrap_used` and `expect_used` denied.** The comment states the reason: panicking on a secret-handling path is a correctness and availability bug. Errors must be propagated as typed errors instead. Tests are exempted by [`clippy.toml`](../../clippy.toml) (`allow-unwrap-in-tests`, `allow-expect-in-tests`).
- **`missing_docs = "warn"`** enforces the "document every public item" rule. It is a warning locally, but CI and `npm run check` pass `-D warnings` to Clippy, so an undocumented public item fails the check.
- **`publish = false`** keeps the security-sensitive crates off crates.io, where they would appear to be a supported, general-purpose library.

## Dependencies

- **Internal**: the member manifests [`crates/novavault-core/Cargo.toml`](crates/novavault-core/Cargo.toml.md) and [`src-tauri/Cargo.toml`](src-tauri/Cargo.toml.md); [`clippy.toml`](../../clippy.toml) for the test exemptions.
- **External**: none. The workspace declares no `[workspace.dependencies]`; each member lists its own.

## Used by

- Cargo, for every build, lint and test from the repository root.
- [`ci.yml`](.github/workflows/ci.yml.md) detects the Rust workspace by the presence of this file and runs fmt, Clippy and tests.
- [`scripts/check.mjs`](scripts/check.mjs.md) runs the same Rust commands.
- [`dependabot.yml`](.github/dependabot.yml.md) watches this workspace (`cargo`, directory `/`).

## Tests

_Not applicable: a manifest has no tests. `cargo test --workspace --locked` builds and tests every member._

## Notes

- **Inconsistency:** the comment above `unsafe_code` says any exception "must be justified explicitly at the crate level", but the two exceptions are module-level `#![allow(unsafe_code)]` attributes in `src-tauri/src/os_lock/windows.rs` and `macos.rs`, each with a per-block justification.
- The MSRV `1.85` is declared but not tested: CI uses the latest stable toolchain (see [`ci.yml.md`](.github/workflows/ci.yml.md)).
- There is no `[profile.*]` section, so release builds use Cargo's default release profile (for example no `panic = "abort"`, no LTO, symbols not stripped).
