# [`clippy.toml`](../../clippy.toml)

## Purpose

Workspace-wide Clippy configuration. The workspace manifest [`Cargo.toml`](Cargo.toml.md) denies `clippy::unwrap_used` and `clippy::expect_used` for every member crate, and this file narrows that rule so it applies only to production code: tests may still use `unwrap()` and `expect()`.

## Public API

| Key                     | Type    | Value  | Purpose                                                                  |
| ----------------------- | ------- | ------ | ------------------------------------------------------------------------ |
| `allow-unwrap-in-tests` | boolean | `true` | `unwrap_used` does not fire in `#[test]` functions and `#[cfg(test)]` code. |
| `allow-expect-in-tests` | boolean | `true` | `expect_used` does not fire in `#[test]` functions and `#[cfg(test)]` code. |

## Details

### `allow-unwrap-in-tests` and `allow-expect-in-tests`

The comment at the top of the file gives the reason: `unwrap()`/`expect()` are denied in production code but are idiomatic in tests, where a panic is the desired failure mode. A failed `unwrap()` in a test reports the error and fails that test, which is what a test should do.

- **Scope**: both keys cover `#[test]` functions and every item inside a `#[cfg(test)]` module. That includes the inline `mod tests` of each source file and the test-only helper module [`crates/novavault-core/src/test_util.rs`](crates/novavault-core/src/test_util.rs.md), which `lib.rs` declares with `#[cfg(test)]`.
- **Security**: production code is unaffected. Panicking on a secret-handling path is still denied there by the workspace lints, and errors must be propagated as typed errors (`thiserror`). The exemption cannot reach a release build, because `#[cfg(test)]` code is not compiled into one.
- **Lookup**: Clippy looks for `clippy.toml` in the directory of the crate being linted and then in its parent directories, so this root file applies to both `crates/novavault-core` and `src-tauri`. Neither crate has its own `clippy.toml`.

## Dependencies

- **Internal**: [`Cargo.toml`](Cargo.toml.md): the `[workspace.lints.clippy]` table that denies `unwrap_used` and `expect_used`, inherited by each member through `[lints] workspace = true`.
- **External**: Clippy (`cargo clippy`), from the Rust toolchain.

## Used by

- [`scripts/check.mjs`](scripts/check.mjs.md): the `Rust: clippy` step (`cargo clippy --workspace --all-targets --locked -- -D warnings`).
- [`.github/workflows/ci.yml`](.github/workflows/ci.yml.md): the `Run Clippy` step, with the same command on Windows, macOS and Linux.

## Tests

_Not applicable: a configuration file has no tests. The Clippy step in `npm run check` and CI checks it: a test that uses `unwrap()` would fail that step if these keys were missing._

## Notes

- These keys affect only Clippy. Rust's own lints from `[workspace.lints.rust]` (`unsafe_code`, `missing_docs`) are not changed by this file.
