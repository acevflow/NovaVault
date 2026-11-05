# [`src-tauri/src/settings.rs`](../../../../src-tauri/src/settings.rs#L151) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `settings.rs` checks the bounds of the two settings (auto-lock timeout and clipboard clear delay), the JSON format of the settings file, the save/load round trip, and that a missing, malformed or out-of-bounds file falls back to the defaults so auto-lock and clipboard clearing stay on.

## Public API

| Name                                       | Kind   | Signature                                     | Description                                                     |
| ------------------------------------------ | ------ | --------------------------------------------- | --------------------------------------------------------------- |
| `temp_path`                                | helper | `fn temp_path() -> std::path::PathBuf`        | `settings.json` in a new, uniquely named temp subdirectory.     |
| `bounds_are_enforced`                      | test   | `fn bounds_are_enforced()`                    | Auto-lock minutes outside 1..=60 are rejected and not applied.  |
| `clipboard_delay_bounds_are_enforced`      | test   | `fn clipboard_delay_bounds_are_enforced()`    | Clear delays outside 5..=300 s are rejected and not applied.    |
| `files_without_the_clipboard_delay_still_load` | test | `fn files_without_the_clipboard_delay_still_load()` | Older files without `clipboardClearSeconds` load with the default. |
| `serializes_both_settings_in_camel_case`   | test   | `fn serializes_both_settings_in_camel_case()` | Defaults serialize to `{ "autoLockMinutes": 5, "clipboardClearSeconds": 30 }`. |
| `saves_and_loads`                          | test   | `fn saves_and_loads()`                        | A saved file loads back equal.                                  |
| `invalid_files_fall_back_to_defaults`      | test   | `fn invalid_files_fall_back_to_defaults()`    | Missing file and seven invalid contents load as defaults.       |

## Details

### Helpers

`temp_path` names the directory `novavault-settings-test-<random ItemId>` under `std::env::temp_dir()`, so parallel tests never share a file. It does not create the directory; tests that write first create it (or rely on `save` doing so) and remove it with `fs::remove_dir_all` at the end, ignoring errors.

### [`bounds_are_enforced`](../../../../src-tauri/src/settings.rs#L164)

1. A default `Settings` has `DEFAULT_AUTO_LOCK_MINUTES` (5).
2. `set_auto_lock_minutes` with `0`, `61` and `u32::MAX` returns `Err(CommandError::InvalidInput)`, and the value stays 5.
3. `1`, `15` and `60` are accepted in turn; after the last one `auto_lock_timeout()` is 3600 s.

### [`clipboard_delay_bounds_are_enforced`](../../../../src-tauri/src/settings.rs#L182)

1. The default is `DEFAULT_CLIPBOARD_CLEAR_SECONDS` (30).
2. `0`, `4` and `301` are rejected with `InvalidInput`, and the value stays 30.
3. `300` is accepted and `clipboard_clear_delay()` is 300 s.

### [`files_without_the_clipboard_delay_still_load`](../../../../src-tauri/src/settings.rs#L209)

Writes `{"autoLockMinutes":15}`, the format from before the clipboard setting existed. `load` returns a 900 s timeout and the default 30 s delay, through the serde default on `clipboard_clear_seconds`.

### [`serializes_both_settings_in_camel_case`](../../../../src-tauri/src/settings.rs#L223)

`Settings::default()` serializes to exactly `{ "autoLockMinutes": 5, "clipboardClearSeconds": 30 }`, the shape of both the file and the IPC response.

### [`saves_and_loads`](../../../../src-tauri/src/settings.rs#L232)

Sets 30 minutes and 10 seconds, saves to a path whose directory does not exist yet (so `save` creates it), and `load` returns equal settings.

### [`invalid_files_fall_back_to_defaults`](../../../../src-tauri/src/settings.rs#L243)

A missing file loads as the defaults. Then each of these contents is written and loads as the defaults:

| Content                                              | Why it is invalid                      |
| ---------------------------------------------------- | -------------------------------------- |
| `not json`                                           | Not JSON.                              |
| `{"autoLockMinutes":0}`                              | Timeout below 1 minute.                |
| `{"autoLockMinutes":999}`                            | Timeout above 60 minutes.              |
| `{"autoLockMinutes":"5"}`                            | Wrong type.                            |
| `{"autoLockMinutes":5,"extra":true}`                 | Unknown field (`deny_unknown_fields`). |
| `{"autoLockMinutes":5,"clipboardClearSeconds":1}`    | Delay below 5 s.                       |
| `{"autoLockMinutes":5,"clipboardClearSeconds":3600}` | Delay above 300 s.                     |

The whole file is rejected, not only the bad field: an otherwise valid `autoLockMinutes` is not kept.

## Dependencies

- **Internal**: [`settings`](settings.rs.md) (`super::*`): `Settings` and the bound constants; [`error`](error.rs.md): `CommandError`.
- **External**: `serde_json`; `novavault-core::ItemId` (only as a random name source); `std::fs`.

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`src-tauri/src/settings.rs`](../../../../src-tauri/src/settings.rs), documented in [`settings.rs.md`](settings.rs.md).

How to run:

```sh
cargo test -p novavault settings::tests
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- The tests write real files under the system temp directory. A test that fails before its `remove_dir_all` leaves its directory behind.
- `save` failures (unwritable directory, failed rename) are not tested.
- Persistence through `Session` (that a setting changed by a command survives a restart) is tested in [`session.rs.tests.md`](session.rs.tests.md).
