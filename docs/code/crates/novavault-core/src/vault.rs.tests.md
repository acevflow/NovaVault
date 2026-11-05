# [`crates/novavault-core/src/vault.rs`](../../../../../crates/novavault-core/src/vault.rs#L877) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `vault.rs` is the crate's end-to-end suite. It drives the public `Vault` API against real files in a temporary directory: create, open with either secret, item CRUD and saving, generic credential errors, rejection of foreign, newer and corrupted files, record replay, password change and recovery code regeneration, format version 0 migration, and the whole attachment lifecycle (add, export, remove, pruning, cancellation, damaged blobs, export target checks).

## Public API

### Helpers

| Name                | Kind     | Signature                                                                 | Description                                                              |
| ------------------- | -------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `fast_params`       | helper   | `fn fast_params() -> KdfParams`                                           | Lowest accepted cost: 19 MiB, 2 passes, 1 lane.                          |
| `pw`                | helper   | `fn pw(text: &str) -> SecretSlice<u8>`                                    | Wraps a test password's UTF-8 bytes.                                     |
| `s`                 | helper   | `fn s(text: &str) -> SecretString`                                        | Wraps a test field value.                                                |
| `PASSWORD`          | constant | `const PASSWORD: &str = "correct horse battery staple"`                   | The fake master password of every test vault.                            |
| `login`, `note`, `token` | helpers | `fn login() -> ItemContent`, `fn note(body: &str) -> ItemContent`, `fn token() -> ItemContent` | One fake item of each kind.            |
| `new_vault`         | helper   | `fn new_vault(dir: &TempDir) -> (Vault, RecoveryCode)`                    | Creates `test.nvault` in `dir` with `PASSWORD`.                          |
| `open`              | helper   | `fn open(path: &Path) -> VaultResult<Vault>`                              | Opens with `PASSWORD`.                                                   |
| `note_body`         | helper   | `fn note_body(item: &Item) -> String`                                     | Returns a note's body; panics for other kinds.                           |
| `open_bytes`        | helper   | `fn open_bytes(dir: &TempDir, bytes: &[u8]) -> VaultResult<Vault>`        | Writes `bytes` to `modified.nvault` and opens it with `PASSWORD`.        |
| `version_zero_file` | helper   | `fn version_zero_file(vault: &Vault) -> Vec<u8>`                          | Re-encodes a vault's contents as a format version 0 file.                |
| `file_names`        | helper   | `fn file_names(dir: &Path) -> Vec<String>`                                | Sorted file names in `dir`, empty if it does not exist.                  |
| `keep_going`        | helper   | `fn keep_going(_: u64) -> ControlFlow<()>`                                | Progress callback that never cancels.                                    |
| `content`           | helper   | `fn content(len: usize) -> Vec<u8>`                                       | `len` bytes of `(index × 31 mod 251)` test data.                         |
| `attach`            | helper   | `fn attach(vault: &mut Vault, item: ItemId, name: &str, data: &[u8]) -> Attachment` | Adds an attachment from memory.                                 |
| `export`            | helper   | `fn export(vault: &Vault, item: ItemId, attachment: AttachmentId, dir: &TempDir) -> VaultResult<Vec<u8>>` | Exports to `exported.bin` and reads it back. |
| `blob_names`        | helper   | `fn blob_names(vault: &Vault) -> Vec<String>`                             | File names in the vault's attachments directory.                         |

### Tests

| Name                                                  | What it checks                                                                 |
| ----------------------------------------------------- | ------------------------------------------------------------------------------ |
| `round_trip_with_password_and_recovery_code`          | Three items survive save and reopen; both secrets open the vault.              |
| `edits_persist_after_save`                            | Updates and removals reach disk only on `save`; `created_at_ms` is kept.       |
| `unknown_items_are_reported`                          | `get`, `update`, `remove` of a missing id return `ItemNotFound`.               |
| `create_never_overwrites`                             | `create` on an existing file returns `AlreadyExists` and leaves it intact.     |
| `wrong_secrets_are_rejected_generically`              | Wrong password and wrong code both return `InvalidCredentials`.                |
| `missing_file_is_an_io_error`                         | Opening a missing file returns `Io`.                                           |
| `unsupported_versions_and_foreign_files_are_rejected` | Newer versions, foreign files, unknown algorithm and flags.                    |
| `corruption_is_always_detected`                       | Truncations and a flipped bit in every field of every section are rejected.    |
| `replaying_an_old_record_is_detected`                 | An older, validly encrypted record spliced into a newer file is rejected.      |
| `verify_password_accepts_only_the_current_password`   | `verify_password` follows password changes.                                    |
| `verify_recovery_code_accepts_only_the_current_code`  | `verify_recovery_code` follows regeneration.                                   |
| `changing_the_password_keeps_data_and_recovery_code`  | Old password fails, new one works, data and recovery code are kept.            |
| `regenerating_the_recovery_code_invalidates_the_old_one` | Old code fails, new code works, data and password are kept.                 |
| `failed_save_keeps_previous_slots`                    | A failed save during a slot change leaves the in-memory slots unchanged.       |
| `debug_hides_contents`                                | `Debug` shows the item count but no field content.                             |
| `version_zero_vaults_are_migrated_on_open`            | A version 0 file is rewritten as the current version and then holds attachments. |
| `version_zero_vaults_open_with_the_recovery_code_too` | Migration also works when unlocking with the recovery code.                    |
| `a_wrong_secret_never_migrates`                       | A wrong password leaves a version 0 file byte-for-byte unchanged.              |
| `rewriting_the_version_field_is_detected`             | Downgrading the version field of a current file returns `Corrupted`.           |
| `attachments_round_trip_through_save_and_reopen`      | Empty and multi-chunk attachments round-trip; blobs hold only ciphertext.      |
| `editing_an_item_keeps_its_attachments`               | `update` keeps the item's attachments.                                         |
| `removed_attachments_are_deleted_only_after_a_save`   | A removed attachment's blob is deleted by the next `save`, not before.         |
| `removing_an_item_deletes_its_blobs_after_a_save`     | Removing an item deletes its blobs on `save` and keeps other items' blobs.     |
| `unsaved_and_interrupted_blobs_are_pruned_on_open`    | Unreferenced blobs and blob temp files are deleted on open; other files stay.  |
| `cancelled_transfers_leave_nothing_behind`            | Cancelled add and export leave no blob, export or temp file.                   |
| `damaged_blobs_never_produce_an_export`               | Tampered, truncated, swapped or missing blobs never replace the export target. |
| `exports_never_overwrite_the_vault`                   | Exporting onto the vault file (any spelling) or a blob returns `InvalidItem`.  |
| `attachment_inputs_are_validated`                     | Bad names, missing items and missing attachments are rejected without writes.  |

## Details

All passwords, items and file contents are fake (`correct horse battery staple`, `hunter2`, `sk-123`, `alice`, `https://bank.example`). Every test creates its own `TempDir`, so tests are independent and leave nothing behind.

### `round_trip_with_password_and_recovery_code`

Creates an empty vault, inserts a login, a note (`dear diary`) and a secret, saves, and reopens with the password. The vault has 3 items; `list()` returns `(Secret, "API")`, `(Password, "Bank")`, `(Note, "Diary")` once sorted by title. The login's id, user name, password and URL, and the secret's value and notes, match; a new item has `created_at_ms == updated_at_ms`. Opening with the recovery code returns the note body.

### `edits_persist_after_save`

Inserts a note and a secret, updates the note to `second` and removes the secret. Before `save`, opening the file shows 0 items (only the empty vault written by `create` is on disk). After `save`: 1 item, body `second`, `created_at_ms` unchanged and `updated_at_ms >= created_at_ms`; the removed id returns `ItemNotFound`.

### `unknown_items_are_reported`

For id `09 × 16`, `get`, `update` and `remove` each return `ItemNotFound`.

### `create_never_overwrites`

With `existing.nvault` holding `precious`, `Vault::create` returns `AlreadyExists` and the file still holds `precious`.

### `wrong_secrets_are_rejected_generically`

Opening with password `wrong` and with a freshly generated code both return `InvalidCredentials`, the single generic unlock error.

### `missing_file_is_an_io_error`

Opening `absent.nvault` returns `VaultError::Io`.

### `unsupported_versions_and_foreign_files_are_rejected`

From a freshly created vault file:

- version field set to `FORMAT_VERSION + 1`, `0x0100` and `u16::MAX`: `UnsupportedVersion { found }` with that value;
- files `""`, `hello`, `PK\x03\x04` and 512 zero bytes: `NotAVault`;
- `aead_id` (byte 10) set to `2`: `UnsupportedAlgorithm { id: 2 }`;
- flags (byte 11) set to `1`: `Corrupted`.

### `corruption_is_always_detected`

Saves a vault with one note. With `slots = 32`, `record = slots + 2 × 101 + 4 = 238`, `sealed = record + 21` and `seal = len − 40`:

1. Prefixes of length 0, 9, 31, 32, `record`, `seal` and `len − 1` all fail to open (any error).
2. One flipped low bit at a representative byte of each field must give `Corrupted` or `InvalidCredentials`:

| Section       | Fields (byte)                                                                          |
| ------------- | -------------------------------------------------------------------------------------- |
| header        | vault id (16)                                                                          |
| password slot | type (32), salt (33), memory (49), nonce (61), tag (132)                               |
| recovery slot | salt (134), tag (233)                                                                  |
| records       | count (234), id (238), type (254), length (255), nonce (`sealed`), ciphertext (`sealed + 24`), tag (`seal − 1`) |
| seal          | nonce (`seal`), tag (`len − 1`)                                                        |

Bytes are sampled per field, not every byte, because each open runs Argon2id. Exhaustive byte-level checks of the same structures are in [`format.rs.tests.md`](format.rs.tests.md) and [`slots.rs.tests.md`](slots.rs.tests.md).

### `replaying_an_old_record_is_detected`

Saves a note `old balance: 10`, keeps the file, updates it to `new balance: 99` (same length) and saves again. Copying the old record region (from byte 238 up to the seal) into the new file gives a file whose record is validly encrypted under the same key, but opening it returns `Corrupted`: the seal no longer matches.

### `verify_password_accepts_only_the_current_password`

`verify_password` accepts `PASSWORD` and rejects `wrong` with `InvalidCredentials`. After `change_password` to `new password`, the old password is rejected and the new one accepted.

### `verify_recovery_code_accepts_only_the_current_code`

`verify_recovery_code` accepts the creation code; after `regenerate_recovery_code` it rejects the old code with `InvalidCredentials` and accepts the new one.

### `changing_the_password_keeps_data_and_recovery_code`

Inserts a note `kept` without saving, then changes the password. On disk: the old password returns `InvalidCredentials`; the new one opens the vault and the unsaved note is there (`change_password` saves the whole vault); the original recovery code still opens it.

### `regenerating_the_recovery_code_invalidates_the_old_one`

Inserts a note, regenerates the code. On disk: the old code returns `InvalidCredentials`; the new code opens the vault with the note; the password still opens it.

### `failed_save_keeps_previous_slots`

Moves the vault file to `moved.nvault` and points the vault's private `path` field at a missing directory. `change_password` and `regenerate_recovery_code` both fail. Pointing `path` back at `moved.nvault` and saving writes a file that still opens with the original password and code, which shows the in-memory slots were restored after each failed save.

### `debug_hides_contents`

With one login inserted, `{:?}` of the vault contains `items: 1` and neither `hunter2` nor `Bank`.

### `version_zero_file`

Builds a version 0 file from a live vault the way a version 0 build wrote it: header with version 0, each record's payload decrypted and re-encoded with `encode_payload(0, …)` (no attachment section), re-encrypted with the version 0 record associated data, framed by `encode_body` and sealed. The key slots are reused unchanged, which works because slots authenticate the header with the version zeroed.

### `version_zero_vaults_are_migrated_on_open`

1. Creates a vault with a login and a note, writes its version 0 form to `old.nvault` (bytes 8–9 are `00 00`) and drops the vault.
2. Opening `old.nvault` with the password migrates it: the file on disk now has `FORMAT_VERSION` in bytes 8–9 and no `.bak` file is left in the directory.
3. Both items survive with their fields and `created_at_ms`; the note has no attachments.
4. The recovery code still opens the migrated file.
5. Adding an attachment `new.txt` to the migrated vault and saving works, and the attachment is there after reopening.

### `version_zero_vaults_open_with_the_recovery_code_too`

A version 0 file opened with the recovery code is migrated and returns the note `kept`; the password then opens the migrated file too.

### `a_wrong_secret_never_migrates`

Opening a version 0 file with `wrong` returns `InvalidCredentials` and the file is byte-for-byte unchanged.

### `rewriting_the_version_field_is_detected`

Setting bytes 8–9 of a saved current-version file to `0` passes the slots (they ignore the version) but returns `Corrupted`, because the records and the seal authenticate the full header.

### `attachments_round_trip_through_save_and_reopen`

1. Adds `empty.txt` (0 bytes) and `scan ✓.pdf` (`3 × CHUNK_LEN + 5` = 196 613 bytes) to a note. Progress for the large file reports `65 536`, `131 072`, `196 608` and `196 613`. The returned metadata has the right size and name.
2. After `save`, the attachments directory holds 2 blobs. The large blob's length is `attachment::blob_len(196 613)`, and no 64-byte window of the blob equals the first 64 plaintext bytes (the blob holds ciphertext only).
3. Reopened with the recovery code: the item lists `("empty.txt", 0)` and `("scan ✓.pdf", 196 613)` in order; both export to their exact contents; `attachment()` returns the name.

### `editing_an_item_keeps_its_attachments`

Adds `a.txt` (`data`) to a note, updates the note to `v2`, saves and reopens: the body is `v2`, the attachment id is unchanged and it exports `data`.

### `removed_attachments_are_deleted_only_after_a_save`

With `keep.txt` and `gone.txt` saved, removes `gone.txt`; removing it again returns `AttachmentNotFound`. Its blob still exists (the saved file still refers to it) until the next `save`, which deletes it and keeps `keep.txt`'s blob. Exporting the removed attachment returns `AttachmentNotFound`.

### `removing_an_item_deletes_its_blobs_after_a_save`

Two notes with one attachment each are saved. Removing the first note keeps its blob until `save`; afterwards the directory holds only `<kept id>.nvatt`.

### `unsaved_and_interrupted_blobs_are_pruned_on_open`

Saves `saved.txt`, then adds `unsaved.txt` without saving, writes a fake interrupted temp file `.<unsaved id>.nvatt.0123456789abcdef.tmp` and an unrelated `notes.txt` into the attachments directory, and drops the vault. Reopening leaves exactly `<saved id>.nvatt` and `notes.txt`: the unreferenced blob and the temp file are deleted, and files not named like blobs are not touched. The item has 1 attachment.

### `cancelled_transfers_leave_nothing_behind`

1. Adding a `2 × CHUNK_LEN + 1`-byte file with a callback that breaks after the first chunk returns `Cancelled`; no blob exists and the item has no attachments.
2. Exporting a fully added attachment to `out.bin` with a callback that always breaks returns `Cancelled`; `out.bin` does not exist and no `.tmp` file is left in the directory.

### `damaged_blobs_never_produce_an_export`

Saves two attachments `a.bin` and `b.bin` with the same `2 × CHUNK_LEN + 10`-byte content and writes `previous export` to the export destination. Then replaces `a.bin`'s blob with:

| Case             | Blob contents                                         |
| ---------------- | ----------------------------------------------------- |
| last byte        | original with the last byte's low bit flipped         |
| first chunk      | original with byte `48 + 30` flipped                  |
| truncated        | header plus the first sealed chunk only               |
| other attachment | `b.bin`'s blob (valid, but bound to another id)       |

Each export returns `Corrupted` and the destination still holds `previous export`, even when earlier chunks authenticated: the atomic temporary file is discarded. With the blob deleted, export also returns `Corrupted`. No `.tmp` file is left.

### `exports_never_overwrite_the_vault`

Exporting to the vault path, to the same path spelled `<dir>/./test.nvault`, and to the attachment's own blob path each return `InvalidItem`. The vault file is unchanged and the blob still exists.

### `attachment_inputs_are_validated`

Names `""`, `..`, `dir/file` and `dir\file` return `InvalidItem`; adding to the missing item `09 × 16` returns `ItemNotFound`; `attachment()` with an unknown id returns `AttachmentNotFound`. The attachments directory has no blob afterwards.

## Dependencies

- **Internal**: [`vault`](vault.rs.md) (`super::*`): `Vault` and its private fields (`path`, `header`, `records`, `slots`, `key`) and methods (`blob_path`, `decrypt_payload`); [`format`](format.rs.md): `encode_body`, `seal`, `record_ad`, `SEAL_LEN`, `RecordEnvelope`; [`item`](item.rs.md): item types and `encode_payload`; [`attachment`](attachment.rs.md): `CHUNK_LEN`, `SEALED_CHUNK_LEN`, `BLOB_HEADER_LEN`, `blob_len`; [`aead`](aead.rs.md): `encrypt`; [`header`](header.rs.md): `HEADER_LEN`, `FORMAT_VERSION`, `Header::with_version`; [`slots`](slots.rs.md): `SLOT_LEN`; [`kdf`](kdf.rs.md): `KdfParams`; [`recovery`](recovery.rs.md): `RecoveryCode`; [`test_util`](test_util.rs.md): `TempDir`.
- **External**: `secrecy` (`SecretString`, `SecretSlice`, `ExposeSecret`): test secrets and reading decrypted fields.

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`crates/novavault-core/src/vault.rs`](../../../../../crates/novavault-core/src/vault.rs), documented in [`vault.rs.md`](vault.rs.md). Through it, the suite also exercises [`format.rs`](format.rs.md), [`slots.rs`](slots.rs.md), [`item.rs`](item.rs.md), [`attachment.rs`](attachment.rs.md) and [`atomic.rs`](atomic.rs.md) together.

How to run:

```sh
cargo test -p novavault-core vault::tests
cargo test -p novavault-core vault::tests::version_zero_vaults_are_migrated_on_open
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- This is the slowest module in the crate: every `create` runs two Argon2id derivations at 19 MiB and every successful unlock one, and `corruption_is_always_detected` tries to open 24 modified files.
- `failed_save_keeps_previous_slots` writes the private `path` field directly to simulate a failing save; there is no public way to do that.
- The migration tests build version 0 files with the current code (`version_zero_file`), not from a file produced by an actual version 0 build, so they check consistency with the documented version 0 format rather than with historical output.
- `corruption_is_always_detected` accepts either `Corrupted` or `InvalidCredentials` for each field, without pinning which one a given field produces.
