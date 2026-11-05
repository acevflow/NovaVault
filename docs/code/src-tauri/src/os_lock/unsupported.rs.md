# [`src-tauri/src/os_lock/unsupported.rs`](../../../../../src-tauri/src/os_lock/unsupported.rs)

## Purpose

The fallback `LockMonitor` for build targets other than Windows, macOS and Linux. NovaVault does not target such platforms. This file keeps them compiling, leaving only the inactivity timer and manual locking. Compiled only when `not(any(target_os = "windows", target_os = "macos", target_os = "linux"))`.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `UnsupportedMonitor` | struct | `pub struct UnsupportedMonitor;` | A monitor that never reports events. |
| `impl LockMonitor for UnsupportedMonitor` | trait impl | `fn start(&self, _sink: LockSink) -> Result<(), MonitorError>` | Always fails. |

## Details

### `UnsupportedMonitor::start`

Ignores the sink and returns `Err(MonitorError::Unavailable("not supported on this platform"))`. `main.rs` prints it as `NovaVault: OS lock events are unavailable: not supported on this platform`, so the missing triggers are visible.

- **Edge cases and invariants**: unlike the other monitors, a second call does not return `AlreadyStarted`. It fails the same way every time.

## Dependencies

- **Internal**: [`os_lock`](mod.rs.md): `LockMonitor`, `LockSink` and `MonitorError`.
- **External**: none.

## Used by

- [`os_lock/mod.rs`](mod.rs.md): `platform_monitor` returns it on other targets.

## Tests

_Not applicable: this file has no tests, and no CI target compiles it._

## Notes

- CI builds only Windows, macOS and Linux, so this file is not compiled by CI. A change to the `LockMonitor` trait could break it unnoticed.
