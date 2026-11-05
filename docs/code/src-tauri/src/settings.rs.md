# [`src-tauri/src/settings.rs`](../../../../src-tauri/src/settings.rs)

## Purpose

Defines the app settings that live outside the vault: the inactivity auto-lock timeout and the clipboard clear delay, each with its bounds and default. The settings must be readable while the vault is locked, so they are kept in a small plain JSON file (`settings.json`, next to the vault file). That file is neither secret nor authenticated, and it is only advisory: anything unusable falls back to the defaults.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `MIN_AUTO_LOCK_MINUTES` | constant | `pub const MIN_AUTO_LOCK_MINUTES: u32 = 1;` | Shortest accepted timeout. |
| `MAX_AUTO_LOCK_MINUTES` | constant | `pub const MAX_AUTO_LOCK_MINUTES: u32 = 60;` | Longest accepted timeout. |
| `DEFAULT_AUTO_LOCK_MINUTES` | constant | `pub const DEFAULT_AUTO_LOCK_MINUTES: u32 = 5;` | Default timeout. |
| `MIN_CLIPBOARD_CLEAR_SECONDS` | constant | `pub const MIN_CLIPBOARD_CLEAR_SECONDS: u32 = 5;` | Shortest accepted clear delay. |
| `MAX_CLIPBOARD_CLEAR_SECONDS` | constant | `pub const MAX_CLIPBOARD_CLEAR_SECONDS: u32 = 300;` | Longest accepted clear delay. |
| `DEFAULT_CLIPBOARD_CLEAR_SECONDS` | constant | `pub const DEFAULT_CLIPBOARD_CLEAR_SECONDS: u32 = 30;` | Default clear delay. |
| `Settings` | struct | `pub struct Settings` | The persisted settings. `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize`; private fields. |
| `impl Default for Settings` | trait impl | `fn default() -> Self` | 5 minutes, 30 seconds. |
| `Settings::auto_lock_timeout` | method | `pub fn auto_lock_timeout(self) -> Duration` | The timeout as a `Duration`. |
| `Settings::set_auto_lock_minutes` | method | `pub fn set_auto_lock_minutes(&mut self, minutes: u32) -> CommandResult<()>` | Sets the timeout, checking bounds. |
| `Settings::clipboard_clear_delay` | method | `pub fn clipboard_clear_delay(self) -> Duration` | The clear delay as a `Duration`. |
| `Settings::clipboard_clear_seconds` | method | `pub const fn clipboard_clear_seconds(self) -> u32` | The clear delay in seconds. |
| `Settings::set_clipboard_clear_seconds` | method | `pub fn set_clipboard_clear_seconds(&mut self, seconds: u32) -> CommandResult<()>` | Sets the clear delay, checking bounds. |
| `Settings::load` | function | `pub fn load(path: &Path) -> Self` | Reads the file, or returns the defaults. Never fails. |
| `Settings::save` | method | `pub fn save(self, path: &Path) -> io::Result<()>` | Writes the file through a temporary file and a rename. |

## Details

### Constants

The bounds are inclusive. The source justifies them as follows: a timeout above 60 minutes would leave an unattended vault open too long, 5 seconds is enough to switch windows and paste, and copied secrets should not stay on the clipboard for more than 300 seconds.

### `Settings`

`#[serde(rename_all = "camelCase", deny_unknown_fields)]`. The same JSON is used for the file and for IPC (`get_settings`):

```json
{ "autoLockMinutes": 5, "clipboardClearSeconds": 30 }
```

- `autoLockMinutes` (`u32`): required.
- `clipboardClearSeconds` (`u32`): `#[serde(default = "default_clipboard_clear_seconds")]`, so files written before this setting existed still load.
- **Edge cases and invariants**: the fields are private. A `Settings` built through `Default` or the setters is always within bounds. One built by deserialization is checked by `load` through the private `is_valid`. `get_settings` serializes an existing value and never deserializes one from the UI.

### `Settings::auto_lock_timeout` and `Settings::clipboard_clear_delay`

Convert to `Duration`: `minutes * 60` seconds (computed in `u64`, so it cannot overflow) and `seconds`.

### `Settings::set_auto_lock_minutes` and `Settings::set_clipboard_clear_seconds`

Check the value against the inclusive bounds and store it.

- **Errors**: `CommandError::InvalidInput` if out of bounds. The previous value is then kept.

### `Settings::load`

1. Reads the whole file. Any I/O error, a missing file included, gives the defaults.
2. Parses JSON. A syntax error, a wrong type, a missing `autoLockMinutes` or an unknown field gives the defaults.
3. Keeps the result only if `is_valid()`. Otherwise it returns the defaults.

- **Security**: falling back to the defaults keeps auto-lock and clipboard clearing on. An attacker who can write the file can set at most 60 minutes and 300 seconds; out-of-range values are discarded as a whole.
- **Edge cases and invariants**: never fails and never logs. One bad value discards the other, valid one too.

### `Settings::save`

1. Serializes with `serde_json::to_vec_pretty`. A serialization failure becomes `io::Error::other`.
2. Creates the parent directory if needed.
3. Writes `<path with extension "json.tmp">` (for `settings.json`, that is `settings.json.tmp`).
4. Renames it over `path`.

- **Errors**: any I/O error from those steps. The caller ([`Session`](session.rs.md#set_auto_lock_minutes-and-set_clipboard_clear_seconds)) reports it as `CommandError::Storage` and keeps the old in-memory value.
- **Edge cases and invariants**: the rename replaces the file in one step on all three platforms, so a reader never sees a half-written file. The temporary file is left behind if the rename fails.

## Dependencies

- **Internal**: [`error`](error.rs.md): `CommandError::InvalidInput` and `CommandResult`.
- **External**:
  - `serde` and `serde_json`: the file and IPC format.
  - `std::fs`: reading and writing the file.

## Used by

- [`session.rs`](session.rs.md): loads the settings in `Session::new`, applies and saves changes, and reads the timeout and delay.
- [`commands.rs`](commands.rs.md): `get_settings` returns `Settings`. `copy_item_field` reads the clear delay.

## Tests

- [`bounds_are_enforced`](../../../../src-tauri/src/settings.rs#L164): 0, 61 and `u32::MAX` minutes are rejected without changing the value. 1, 15 and 60 are accepted, and 60 minutes is 3600 seconds.
- [`clipboard_delay_bounds_are_enforced`](../../../../src-tauri/src/settings.rs#L182): 0, 4 and 301 seconds are rejected. 300 is accepted.
- [`files_without_the_clipboard_delay_still_load`](../../../../src-tauri/src/settings.rs#L209): a file with only `autoLockMinutes` loads, with the default delay.
- [`serializes_both_settings_in_camel_case`](../../../../src-tauri/src/settings.rs#L223): the default settings serialize to the documented JSON.
- [`saves_and_loads`](../../../../src-tauri/src/settings.rs#L232): a save into a not-yet-existing directory loads back equal.
- [`invalid_files_fall_back_to_defaults`](../../../../src-tauri/src/settings.rs#L243): a missing file, non-JSON, out-of-range values, a string where a number is expected and an unknown field all give the defaults.

## Notes

- The module doc says the file is replaced "in one rename so a crash never leaves a half-written file". The rename is atomic, but the temporary file is not flushed with `fsync` before the rename, unlike the core's `atomic` module. After a power loss the file could be empty or truncated. `load` then falls back to the defaults, so the app still starts.
- The temporary file name is fixed (`settings.json.tmp`). That is harmless today because every save runs under the session mutex.
- On Unix, `settings.json` is created with the default permissions, not `0600`. [`docs/threat-model.md`](../../../threat-model.md) records this.
