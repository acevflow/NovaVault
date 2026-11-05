# [`src-tauri/src/session.rs`](../../../../src-tauri/src/session.rs#L516) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `session.rs` is the shell's main integration suite. It runs the logic of every Tauri command through `Session` against a real vault file in a temporary directory, with the real core crypto at the lowest accepted Argon2id cost. It covers the vault lifecycle, both key slots, item CRUD, the secret-handling rules of the IPC DTOs, auto-lock, persisted settings and encrypted attachments.

## Public API

| Name                       | Kind   | Signature                                                                    | Description                                                     |
| -------------------------- | ------ | ---------------------------------------------------------------------------- | --------------------------------------------------------------- |
| `TempDir`                  | helper | `struct TempDir(PathBuf)`                                                    | Unique temp directory, removed on drop.                         |
| `TempDir::new`             | helper | `fn new() -> Self`                                                           | Creates `novavault-shell-test-<random ItemId>` under the temp dir. |
| `TempDir::path`            | helper | `fn path(&self) -> &Path`                                                    | The directory path.                                             |
| `PASSWORD`                 | const  | `const PASSWORD: &str = "correct horse battery staple"`                      | Master password used by most tests.                             |
| `s`                        | helper | `fn s(text: &str) -> SecretString`                                           | Wraps text as a `SecretString`.                                 |
| `fast_params`              | helper | `fn fast_params() -> KdfParams`                                              | 19 MiB, 2 iterations, 1 lane: the lowest accepted cost.         |
| `session`                  | helper | `fn session(dir: &TempDir) -> Session`                                       | Session for `<dir>/data/vault.nvault` (directory not yet created). |
| `note`                     | helper | `fn note(title: &str, body: &str) -> ItemInput`                              | A note input.                                                   |
| `note_update`              | helper | `fn note_update(title: &str, body: Option<&str>) -> ItemUpdate`              | A note update; `None` keeps the body.                           |
| `login`                    | helper | `fn login(password: &str, notes: &str) -> ItemInput`                         | A password item `Bank` / `alice` / `https://bank.example`.      |
| `revealed`                 | helper | `fn revealed(session: &Session, id: &str, field: ItemField) -> CommandResult<String>` | `reveal_field`, serialized, reading `value`.           |
| `with_password`            | helper | `fn with_password(password: &str) -> Credential`                             | A password credential.                                          |
| `code_text`                | helper | `fn code_text(code: &RecoveryCodeDto) -> String`                             | The `recoveryCode` string of a DTO.                             |
| `titles`                   | helper | `fn titles(session: &Session) -> Vec<String>`                                | Sorted titles from `list_items`.                                |
| `keep_going`               | helper | `fn keep_going(_: u64) -> ControlFlow<()>`                                   | Progress callback that never cancels.                           |
| `user_file`                | helper | `fn user_file(dir: &TempDir, name: &str, contents: &[u8]) -> PathBuf`        | Writes a "user" file to attach.                                 |
| `create_lock_and_unlock_with_either_secret` | test | `fn create_lock_and_unlock_with_either_secret()` | Create, lock, and unlock with the password or a loosely typed recovery code. |
| `wrong_or_malformed_secrets_leave_the_session_locked` | test | `fn wrong_or_malformed_secrets_leave_the_session_locked()` | Wrong or malformed secrets give typed errors and leave the session locked. |
| `status_reports_existence_and_lock_state` | test | `fn status_reports_existence_and_lock_state()` | `(exists, unlocked)` follows create and lock. |
| `missing_and_existing_vaults_are_reported` | test | `fn missing_and_existing_vaults_are_reported()` | `VaultNotFound` before a vault exists, `VaultExists` on a second create. |
| `empty_passwords_are_rejected` | test | `fn empty_passwords_are_rejected()` | Empty passwords give `InvalidInput` on create and password change. |
| `data_commands_require_an_unlocked_vault` | test | `fn data_commands_require_an_unlocked_vault()` | Data and key slot commands return `Locked` after `lock`. |
| `item_edits_are_saved_immediately` | test | `fn item_edits_are_saved_immediately()` | Item edits survive a lock and unlock. |
| `listing_never_serializes_secret_fields` | test | `fn listing_never_serializes_secret_fields()` | The `list_items` JSON holds titles but no field values. |
| `item_details_have_plain_fields_and_flags_only` | test | `fn item_details_have_plain_fields_and_flags_only()` | `get_item` returns plain fields and `has*` flags, never secrets. |
| `secret_fields_are_revealed_one_at_a_time` | test | `fn secret_fields_are_revealed_one_at_a_time()` | `reveal_field` returns secret fields only and requires an unlocked vault. |
| `updates_keep_secret_fields_that_are_left_out` | test | `fn updates_keep_secret_fields_that_are_left_out()` | `None` keeps a secret field, an empty string clears it. |
| `updates_cannot_change_the_kind_of_an_item` | test | `fn updates_cannot_change_the_kind_of_an_item()` | An update of another kind gives `InvalidInput`. |
| `unknown_or_malformed_item_ids_are_reported` | test | `fn unknown_or_malformed_item_ids_are_reported()` | `ItemNotFound` for unknown ids, `InvalidInput` for malformed ones. |
| `changing_the_master_password` | test | `fn changing_the_master_password()` | Only the password slot changes; the recovery code still works. |
| `regenerating_the_recovery_code_invalidates_the_old_one` | test | `fn regenerating_the_recovery_code_invalidates_the_old_one()` | The old code stops working; the new code and password unlock. |
| `key_slot_changes_require_the_current_secret` | test | `fn key_slot_changes_require_the_current_secret()` | Key slot changes need the current password or recovery code. |
| `idle_sessions_lock_after_the_timeout` | test | `fn idle_sessions_lock_after_the_timeout()` | The session locks after the idle timeout; activity restarts it. |
| `clipboard_delay_is_configurable_and_persisted` | test | `fn clipboard_delay_is_configurable_and_persisted()` | The clipboard delay is validated and persisted. |
| `auto_lock_timeout_is_configurable_and_persisted` | test | `fn auto_lock_timeout_is_configurable_and_persisted()` | The auto-lock timeout is validated, applied at once and persisted. |
| `attachments_are_added_exported_and_deleted` | test | `fn attachments_are_added_exported_and_deleted()` | Attachments are added, survive reopening, export identically and delete once. |
| `deleting_an_item_deletes_its_attachment_data` | test | `fn deleting_an_item_deletes_its_attachment_data()` | Deleting an item removes its blobs. |
| `attachment_commands_report_their_errors` | test | `fn attachment_commands_report_their_errors()` | Attachment commands map each failure to its error code. |
| `attachment_dtos_never_contain_paths` | test | `fn attachment_dtos_never_contain_paths()` | Attachment DTOs contain no file system paths. |
| `debug_shows_only_lock_state` | test | `fn debug_shows_only_lock_state()` | `Debug` shows the lock state, not the vault path. |

## Details

### Vault lifecycle and key slots

- [`create_lock_and_unlock_with_either_secret`](../../../../src-tauri/src/session.rs#L625): `create` leaves the session unlocked; `lock` returns `true` once and `false` the second time; the password unlocks; the recovery code unlocks too when typed in lowercase with spaces instead of dashes.
- [`wrong_or_malformed_secrets_leave_the_session_locked`](../../../../src-tauri/src/session.rs#L646): a wrong password gives `InvalidCredentials`; `not a code` gives `InvalidRecoveryCode`; a well-formed code of another vault gives `InvalidCredentials`. The session stays locked after each.
- [`status_reports_existence_and_lock_state`](../../../../src-tauri/src/session.rs#L669): `(exists, unlocked)` goes `(false, false)` → `(true, true)` after `create` → `(true, false)` after `lock`.
- [`missing_and_existing_vaults_are_reported`](../../../../src-tauri/src/session.rs#L684): unlocking before any vault exists gives `VaultNotFound`; a second `create` gives `VaultExists`, and the original vault still opens with the original password. (`create` locks first, so the failed second call leaves the session locked.)
- [`empty_passwords_are_rejected`](../../../../src-tauri/src/session.rs#L701): an empty password is rejected with `InvalidInput` by `create` and as the new password of `change_master_password`.
- [`changing_the_master_password`](../../../../src-tauri/src/session.rs#L934): after a change, the old password gives `InvalidCredentials`, the new one opens the vault with its items, and the original recovery code still works (only the password slot was re-wrapped).
- [`regenerating_the_recovery_code_invalidates_the_old_one`](../../../../src-tauri/src/session.rs#L955): the new code differs from the old; afterwards the old code gives `InvalidCredentials`, the new code and the password both unlock.
- [`key_slot_changes_require_the_current_secret`](../../../../src-tauri/src/session.rs#L977): with a wrong password, `change_master_password` and `regenerate_recovery_code` give `InvalidCredentials`; with a malformed code, `InvalidRecoveryCode`. The vault stays unlocked and unchanged. Then the recovery code authorizes a password change (the forgotten-password flow).

### Locked state

- [`data_commands_require_an_unlocked_vault`](../../../../src-tauri/src/session.rs#L716): after `lock`, `list_items`, `create_item`, `update_item`, `delete_item`, `change_master_password` and `regenerate_recovery_code` all return `Locked`.

### Items and what crosses the IPC boundary

- [`item_edits_are_saved_immediately`](../../../../src-tauri/src/session.rs#L746): creates a note and a secret, renames the note to `Journal`, deletes the secret, then locks and unlocks: only `Journal` remains, with the same id and kind `note`. Every edit reached the disk.
- [`listing_never_serializes_secret_fields`](../../../../src-tauri/src/session.rs#L773): the JSON of `list_items` contains the title `Bank` but none of `alice`, `hunter2`, `bank.example` or `PIN`. The listing carries no username or URL either.
- [`item_details_have_plain_fields_and_flags_only`](../../../../src-tauri/src/session.rs#L794): `get_item` of a login returns `id`, `kind`, `title`, `username`, `url`, `hasPassword: true`, `hasNotes: false`, and never the password. A note with an empty body has `hasBody: false`.
- [`secret_fields_are_revealed_one_at_a_time`](../../../../src-tauri/src/session.rs#L816): `reveal_field` returns each secret field (login password and notes, note body, secret value and notes). It returns `InvalidInput` for plain fields (`Username`, `Url`) and for fields of another kind (`Body` on a login, `Password` on a note, `Body` on a secret). `field_text` (used by copy) does return the plain username. After `lock`, `reveal_field` and `get_item` return `Locked`.
- [`updates_keep_secret_fields_that_are_left_out`](../../../../src-tauri/src/session.rs#L870): an update with `password: None` and `notes: Some("")` changes title, username and URL, clears the notes and keeps the password, as seen after a lock and unlock.
- [`updates_cannot_change_the_kind_of_an_item`](../../../../src-tauri/src/session.rs#L898): a note update applied to a login gives `InvalidInput` and leaves the password intact.
- [`unknown_or_malformed_item_ids_are_reported`](../../../../src-tauri/src/session.rs#L914): 32 zeros give `ItemNotFound` for update and delete; `not-an-id` gives `InvalidInput`.

### Auto-lock and settings

- [`idle_sessions_lock_after_the_timeout`](../../../../src-tauri/src/session.rs#L1012): the default timeout is 300 s. A locked session never "locks". With activity at `start`, 299 s is not enough; activity at 200 s restarts the timer so 300 s is not enough either; 500 s locks. A clock reading before the last activity neither panics nor locks.
- [`clipboard_delay_is_configurable_and_persisted`](../../../../src-tauri/src/session.rs#L1035): `set_clipboard_clear_seconds` gives `Locked` before unlock and `InvalidInput` for 1 s; 10 s is accepted and a new `Session` for the same path reads 10.
- [`auto_lock_timeout_is_configurable_and_persisted`](../../../../src-tauri/src/session.rs#L1055): `set_auto_lock_minutes` gives `Locked` before unlock and `InvalidInput` for 0; 1 minute is accepted, takes effect at once (60 s idle locks), and a new `Session` reads 60 s.

### Attachments

- [`attachments_are_added_exported_and_deleted`](../../../../src-tauri/src/session.rs#L1094): attaches a 200 000-byte file named `return 2026.pdf`; the progress callback reaches the full size; the DTO has the name and size. After lock and unlock the attachment is listed and `attachment_info` returns `(name, size)`. Export writes identical bytes. Delete works once, then gives `AttachmentNotFound`, and the list is empty after reopening.
- [`deleting_an_item_deletes_its_attachment_data`](../../../../src-tauri/src/session.rs#L1143): the `vault.nvault.attachments` directory holds one blob after attaching and none after deleting the item.
- [`attachment_commands_report_their_errors`](../../../../src-tauri/src/session.rs#L1157): `check_item` is `Ok` or `ItemNotFound`; a missing source file gives `FileAccess`; an unknown attachment id gives `AttachmentNotFound`, a malformed one `InvalidInput`; a callback returning `Break` gives `Cancelled`. Exporting onto the vault file itself gives `FileAccess` and leaves it byte-for-byte unchanged. After `lock`, `check_item`, `add_attachment`, `export_attachment` and `delete_attachment` give `Locked`.
- [`attachment_dtos_never_contain_paths`](../../../../src-tauri/src/session.rs#L1215): the JSON of `add_attachment` and `get_item` contains neither the temp directory path nor `nvault`.

### Debug output

- [`debug_shows_only_lock_state`](../../../../src-tauri/src/session.rs#L1233): `format!("{session:?}")` contains `unlocked: true` and not `vault.nvault`.

## Dependencies

- **Internal**: [`session`](session.rs.md) (`super::*`): `Session`; [`dto`](dto.rs.md): `ItemInput`, `ItemUpdate`, `ItemField`, `Credential`, `RecoveryCodeDto`; [`error`](error.rs.md): `CommandError`; [`settings`](settings.rs.md) through `Session::settings`.
- **External**: `novavault-core` (`KdfParams`, `RecoveryCode`, `ItemId`, and the vault itself); `secrecy::SecretString`; `serde_json` (to inspect DTOs as the UI would receive them).

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`src-tauri/src/session.rs`](../../../../src-tauri/src/session.rs), documented in [`session.rs.md`](session.rs.md), and through it [`dto.rs`](dto.rs.md), [`settings.rs`](settings.rs.md) and `novavault-core`.

How to run:

```sh
cargo test -p novavault session::tests
cargo test -p novavault session::tests::secret_fields_are_revealed_one_at_a_time
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- Each vault creation and unlock runs Argon2id at 19 MiB; tests that lock and unlock several times are the slowest in the shell, especially in debug builds.
- The tests write real vault files under the system temp directory; `TempDir` removes them on drop, including when a test panics.
- Save failures (the `Storage` path where an edit stays in memory) are not tested.
- `reveal_field` and `field_text` are tested for values, not for zeroization of the returned buffers.
