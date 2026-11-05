# [`src-tauri/src/os_lock/windows.rs`](../../../../../src-tauri/src/os_lock/windows.rs#L154) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `os_lock/windows.rs` checks `reason_for`, the pure function that turns a Win32 window message and its `wparam` into a `LockReason`. It is compiled and run only on Windows, like the rest of the file. No window is created and no Win32 API is called.

## Public API

| Name                         | Kind   | Signature                             | Description                                                     |
| ---------------------------- | ------ | ------------------------------------- | --------------------------------------------------------------- |
| `wparam`                     | helper | `fn wparam(value: u32) -> WPARAM`     | Builds a `WPARAM` from an event code.                           |
| `maps_suspend_and_lock_messages` | test | `fn maps_suspend_and_lock_messages()` | Suspend and screen lock map to reasons; other events do not. |

## Details

### [`maps_suspend_and_lock_messages`](../../../../../src-tauri/src/os_lock/windows.rs#L165)

| Message                | `wparam`                 | Expected                       |
| ---------------------- | ------------------------ | ------------------------------ |
| `WM_POWERBROADCAST`    | `PBT_APMSUSPEND`         | `Some(LockReason::Suspend)`    |
| `WM_WTSSESSION_CHANGE` | `WTS_SESSION_LOCK`       | `Some(LockReason::ScreenLock)` |
| `WM_POWERBROADCAST`    | `PBT_APMRESUMEAUTOMATIC` | `None`                         |
| `WM_WTSSESSION_CHANGE` | `WTS_SESSION_UNLOCK`     | `None`                         |
| `0`                    | `WTS_SESSION_LOCK`       | `None`                         |

The last case checks that the event code alone is not enough: the same number on an unrelated message must not lock the vault.

## Dependencies

- **Internal**: [`os_lock/windows`](windows.rs.md) (`super::*`): `reason_for` and the imported message constants; [`autolock`](../autolock.rs.md): `LockReason`.
- **External**: `windows` (`PBT_APMRESUMEAUTOMATIC`, `WTS_SESSION_UNLOCK`, `WPARAM`).

## Used by

_Not applicable: test module, run by `cargo test` on Windows._

## Tests

Code under test: [`src-tauri/src/os_lock/windows.rs`](../../../../../src-tauri/src/os_lock/windows.rs), documented in [`windows.rs.md`](windows.rs.md).

How to run (Windows only):

```sh
cargo test -p novavault os_lock::windows::tests
```

CI runs it on its Windows runner (`cargo test --workspace --locked`).

## Notes

- `WindowsMonitor::start`, the hidden window, `window_proc` and the message loop (the `unsafe` FFI code) are not unit-tested; they need a real Windows session.
- The `u32::try_from(wparam.0)` failure branch of `reason_for` (a `wparam` above `u32::MAX` on 64-bit Windows) is not tested.
- The Linux and macOS monitors have no tests.
