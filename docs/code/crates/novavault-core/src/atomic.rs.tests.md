# [`crates/novavault-core/src/atomic.rs`](../../../../../crates/novavault-core/src/atomic.rs#L162) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `atomic.rs` checks atomic file replacement on the real file system: `write_atomic` creates and replaces files without leaving temporary files, a failed write leaves the original intact, an uncommitted `AtomicFile` deletes its temporary file, and on Unix the result is owner-only. Each test works in its own temporary directory.

## Public API

| Name                                       | Kind   | Signature                                         | Description                                                                 |
| ------------------------------------------ | ------ | ------------------------------------------------- | --------------------------------------------------------------------------- |
| `entries`                                  | helper | `fn entries(dir: &Path) -> Vec<String>`           | Sorted file names in `dir`, used to assert no temporary file is left.       |
| `creates_and_replaces_without_leftovers`   | test   | `fn creates_and_replaces_without_leftovers()`     | Create, grow, empty: contents are exact and only the target remains.        |
| `failure_leaves_original_and_no_temp_file` | test   | `fn failure_leaves_original_and_no_temp_file()`   | A failed rename removes the temporary file and leaves other files alone.    |
| `missing_directory_is_an_error`            | test   | `fn missing_directory_is_an_error()`              | A target in a missing directory fails with `NotFound`.                      |
| `uncommitted_files_leave_nothing_behind`   | test   | `fn uncommitted_files_leave_nothing_behind()`     | Dropping an uncommitted `AtomicFile` keeps the old file; committing replaces it. |
| `file_is_private_to_its_owner`             | test   | `fn file_is_private_to_its_owner()`               | Unix only: the written file has mode `0o600`.                               |

## Details

### `entries`

Reads the directory with `fs::read_dir`, collects each entry's file name (lossy UTF-8) and sorts them. Tests compare the result with an exact list, so any leftover `.<name>.<16 hex>.tmp` file makes them fail.

### `creates_and_replaces_without_leftovers`

In a fresh directory, writes `test.nvault` three times: `b"first"`, then the longer `b"second, longer contents"`, then `b""`. After each write the file holds exactly the new bytes (so a shorter write does not keep a tail of the old one). At the end the directory holds only `test.nvault`.

### `failure_leaves_original_and_no_temp_file`

Writes `b"original"` to `test.nvault`, creates a directory `blocked`, then calls `write_atomic(&blocked, b"new")`. Renaming a file over a directory fails on every platform, so the call returns an error. Afterwards `test.nvault` still holds `b"original"` and the directory holds only `blocked` and `test.nvault`: the temporary file was removed on the error path of `AtomicFile::commit`.

### `missing_directory_is_an_error`

Targets `missing/test.nvault` inside the temp directory, where `missing` does not exist. `write_atomic` fails with `io::ErrorKind::NotFound` (from creating the temporary file), so no directory is created implicitly.

### `uncommitted_files_leave_nothing_behind`

1. Writes `b"previous"` to `export.bin`.
2. Creates an `AtomicFile` for it, writes `b"partial"` and drops it without committing. `export.bin` still holds `b"previous"` and is the only entry: the `Drop` impl deleted the temporary file.
3. Creates another `AtomicFile`, writes `b"complete"` and commits. `export.bin` now holds `b"complete"` and is still the only entry.

This is the property exports and attachment writes rely on: a cancelled or failed stream leaves no partial plaintext on disk.

### `file_is_private_to_its_owner`

Compiled only with `#[cfg(unix)]`. Writes `test.nvault` and asserts `mode & 0o777 == 0o600`, the mode `AtomicFile::create` sets with `OpenOptionsExt::mode`.

## Dependencies

- **Internal**: [`atomic`](atomic.rs.md) (`super::*`): `write_atomic`, `AtomicFile`, and the `std::fs`, `std::io` and `std::path` imports; [`test_util`](test_util.rs.md): `TempDir`, a self-deleting temporary directory.
- **External**: none beyond the standard library (`std::os::unix::fs::PermissionsExt` on Unix).

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`crates/novavault-core/src/atomic.rs`](../../../../../crates/novavault-core/src/atomic.rs), documented in [`atomic.rs.md`](atomic.rs.md).

How to run:

```sh
cargo test -p novavault-core atomic::tests
cargo test -p novavault-core atomic::tests::uncommitted_files_leave_nothing_behind
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too. CI runs on Windows, macOS and Linux, which is the only place `file_is_private_to_its_owner` and the Unix directory sync run.

## Notes

- The module has five tests, but `file_is_private_to_its_owner` is not compiled on Windows, so `cargo test -- --list` there shows four.
- No test exercises the Unix-only `sync_dir` failure path or a `sync_all` failure; both are hard to provoke portably.
