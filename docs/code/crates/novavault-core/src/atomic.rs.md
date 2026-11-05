# [`crates/novavault-core/src/atomic.rs`](../../../../../crates/novavault-core/src/atomic.rs)

## Purpose

Crate-internal (`mod atomic`, not `pub`). Replaces files atomically: new contents go to a temporary file in the same directory, which is flushed to disk and then renamed over the target. The target path therefore always holds either the complete old file or the complete new one. The crate uses this for every write it makes: the vault file, migration backups, attachment blobs and exported attachments.

## Public API

Crate-visible items:

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `write_atomic` | function | `pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()>` | Atomically replaces or creates a file with `bytes`. |
| `AtomicFile` | struct | `pub(crate) struct AtomicFile { file: Option<File>, temp: PathBuf, target: PathBuf, dir: PathBuf }` | A streaming writer that becomes the target file only on `commit`. |
| `AtomicFile::create` | function | `pub(crate) fn create(path: &Path) -> io::Result<Self>` | Creates the temporary file next to `path`. |
| `AtomicFile::commit` | function | `pub(crate) fn commit(mut self) -> io::Result<()>` | Flushes, renames over `path`, syncs the directory. |
| `impl Write for AtomicFile` | trait impl | `fn write(&mut self, buf: &[u8]) -> io::Result<usize>`, `fn flush(&mut self) -> io::Result<()>` | Writes to the temporary file. |
| `impl Drop for AtomicFile` | trait impl | `fn drop(&mut self)` | Deletes an uncommitted temporary file. |

## Details

### `write_atomic`

`AtomicFile::create(path)`, then `write_all(bytes)`, then `commit()`.

- **Parameters**:
  - `path`: the target.
  - `bytes`: the full contents.
- **Returns**: `()` once the new file is in place.
- **Errors**: any I/O error while creating, writing, flushing or renaming. The temporary file is then removed and any existing file at `path` is untouched. The exception is the Unix directory sync, described under `commit`.
- **Security**: on Unix the file is created with mode `0600` (owner read and write only).

### `AtomicFile`

The temporary file is named `.<target file name>.<16 lower-case hex digits>.tmp` and sits in the target's directory, or in `.` when the path has no parent. [`vault`](vault.rs.md) matches this pattern to clean up interrupted blob writes.

- **Security**: dropping an uncommitted `AtomicFile` closes and deletes its temporary file. An error or a cancellation partway through therefore never leaves a partial file at the target path, nor a partial temporary file. This matters for attachment exports, which write plaintext before the whole blob has been authenticated.

### `AtomicFile::create`

1. Takes the file name of `path`, failing with `InvalidInput` if there is none.
2. Picks the directory: the parent, or `.` for a bare file name.
3. Draws 8 random bytes for the suffix.
4. Opens `.<name>.<suffix>.tmp` with `write(true).create_new(true)`, plus `mode(0o600)` on Unix.

- **Errors**:
  - `io::ErrorKind::InvalidInput`: `path` has no file name.
  - An `io::Error` of kind `Other`: `getrandom` failed.
  - Any error of `open`, for example `NotFound` when the directory does not exist, or `AlreadyExists` on a suffix collision.
- **Security**: `create_new` refuses to open an existing path or follow a link planted there, so a file pre-created at the temporary name cannot capture the contents. The random suffix makes the name unpredictable.

### `AtomicFile::commit`

1. Takes the open file. Calling `commit` after a commit is impossible, because `commit` consumes `self`.
2. Calls `sync_all`, so the data is on disk before the rename makes it visible under the real name.
3. Closes the file and renames the temporary file over `target`.
4. If step 2 or 3 failed, removes the temporary file (best effort) and returns the original error.
5. Syncs the directory (`sync_dir`).

- **Errors**:
  - Any error of `sync_all` or `rename`. The temporary file is then removed and the target is untouched.
  - On Unix, an error while syncing the directory after the rename. The target already holds the new contents; only whether the rename survives a crash is in doubt.
- **Edge cases and invariants**: `fs::rename` replaces an existing target on Windows, macOS and Linux. Renaming over a directory fails on every platform, and that failure leaves the target untouched.

### `Write for AtomicFile`

Forwards to the temporary file. After the file has been taken it returns `io::Error::other("file already committed")`. That case cannot happen through the public flow, because `commit` consumes `self`.

### `Drop for AtomicFile`

If the file is still open (not committed), closes it and then deletes the temporary file, ignoring errors. It closes first because Windows cannot always delete an open file.

### `closed` and `sync_dir` (private)

`closed` builds the "file already committed" error. `sync_dir` opens the directory and calls `sync_all` on Unix. On other platforms it is a no-op, because the standard library cannot open a directory for syncing on Windows.

## Dependencies

- **Internal**: none.
- **External**:
  - `getrandom`: the random temporary-file suffix.
  - Standard library only for file I/O.

## Used by

- [`vault`](vault.rs.md):
  - `write_atomic` for `save` and the migration backup.
  - `AtomicFile` for attachment blobs (`add_attachment`) and exports (`export_attachment`).
- [`test_util`](test_util.rs.md) supplies the temporary directories used by the tests here.

## Tests

- [`creates_and_replaces_without_leftovers`](../../../../../crates/novavault-core/src/atomic.rs#L177): create, replace with longer and then empty contents; only the target remains in the directory.
- [`failure_leaves_original_and_no_temp_file`](../../../../../crates/novavault-core/src/atomic.rs#L192): a rename onto a directory fails, the other file is unchanged and no temporary file remains.
- [`missing_directory_is_an_error`](../../../../../crates/novavault-core/src/atomic.rs#L207): a missing parent directory gives `NotFound`.
- [`uncommitted_files_leave_nothing_behind`](../../../../../crates/novavault-core/src/atomic.rs#L217): dropping an `AtomicFile` keeps the previous file and leaves no temporary file; committing replaces it.
- [`file_is_private_to_its_owner`](../../../../../crates/novavault-core/src/atomic.rs#L236) (Unix only): the written file has mode `0600`.

## Notes

- **Plaintext can persist on disk after deletion.** Deleting an uncommitted export temporary file unlinks it but does not overwrite its blocks. Plaintext from a cancelled or failed export may therefore stay recoverable on the storage medium. This is outside what the module claims, which is that no partial file is left at the target or temporary path.
- **No directory sync on Windows.** On Windows the directory is never synced, so a power loss right after `commit` may in principle lose the rename. The code comment and [security-design.md](../../../../security-design.md#per-os-notes) both document this.
- **Mode applies to new files only.** The `0600` mode is set when the temporary file is created. Because a rename replaces the target, the final file always has the new mode, and the target's previous permissions are not carried over.
