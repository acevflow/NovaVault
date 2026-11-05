# [`crates/novavault-core/src/test_util.rs`](../../../../../crates/novavault-core/src/test_util.rs)

## Purpose

Helpers shared by the crate's unit tests. The module is declared `#[cfg(test)] mod test_util;` in [`lib.rs`](lib.rs.md), so it is compiled only for tests and never ships in the library. Its single type, `TempDir`, gives each test a fresh, uniquely named directory that is removed when the test ends.

## Public API

Crate-visible, test-only items:

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `TempDir` | struct | `pub(crate) struct TempDir(PathBuf);` | A temporary directory, removed with its contents on drop. |
| `TempDir::new` | function | `pub(crate) fn new() -> Self` | Creates the directory. Panics on failure. |
| `TempDir::path` | function | `pub(crate) fn path(&self) -> &Path` | The directory's path. |
| `impl Drop for TempDir` | trait impl | `fn drop(&mut self)` | Removes the directory recursively, ignoring errors. |

## Details

### `TempDir`

Wraps the path of a directory created under `std::env::temp_dir()`.

### `TempDir::new`

1. Draws 8 random bytes with `getrandom::fill`.
2. Formats them as 16 lower-case hexadecimal digits.
3. Creates `<system temp dir>/novavault-test-<hex>` with `create_dir`, which fails if the directory already exists.

- **Returns**: the new `TempDir`.
- **Errors**: none returned. Both steps use `unwrap()` and panic on failure, which the doc comment states is fine in tests. The project rule against `unwrap()` applies to non-test code only.
- **Edge cases and invariants**: the random name lets tests run in parallel without sharing directories.

### `TempDir::path`

Borrows the directory path. Tests join file names onto it.

### `Drop for TempDir`

`std::fs::remove_dir_all` on the path, ignoring the result, so a cleanup failure never masks the test's own outcome.

- **Security**: the test files are written by the tests themselves and use fake values only (for example the password `correct horse battery staple`). Nothing secret is stored.

## Dependencies

- **Internal**: none.
- **External**: `getrandom`, already a crate dependency, for the random directory name.

## Used by

- [`atomic`](atomic.rs.md) tests.
- [`vault`](vault.rs.md) tests.

## Tests

_Not applicable: this file is a test helper and has no tests of its own._

## Notes

- **Leftover directories.** A test that aborts the process, rather than unwinding, skips `Drop`, so its directory can remain in the system temporary directory.
- **Default permissions.** The directory is created with the default permissions of the system temporary directory, not owner-only.
