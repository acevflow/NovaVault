# [`src-tauri/src/os_lock/windows.rs`](../../../../../src-tauri/src/os_lock/windows.rs)

## Purpose

The Windows `LockMonitor`. It listens for system suspend and session lock through a hidden window. Power broadcasts (`WM_POWERBROADCAST`) are delivered only to top-level windows, and session changes (`WM_WTSSESSION_CHANGE`) only to windows registered with `WTSRegisterSessionNotification`. So a dedicated thread creates one invisible top-level window for both and runs its message loop, leaving the app's own windows and event loop untouched. Compiled only for `target_os = "windows"`.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `WindowsMonitor` | struct | `pub struct WindowsMonitor;` | The Windows monitor. |
| `impl LockMonitor for WindowsMonitor` | trait impl | `fn start(&self, sink: LockSink) -> Result<(), MonitorError>` | Spawns the listener thread and waits until it is subscribed. |

`WindowsMonitor` is `pub` within the private `os_lock::windows` module, so it is reachable only through [`platform_monitor`](mod.rs.md#platform_monitor).

## Details

### `WindowsMonitor::start`

1. Stores `sink` in the process-global `SINK: OnceLock<LockSink>`. If it is already set, returns `MonitorError::AlreadyStarted`.
2. Spawns the thread `novavault-os-lock`. The window belongs to this thread, so the message loop must run there too. On that thread:
   1. `create_listener_window()`;
   2. send the result back over a channel;
   3. on success, `run_message_loop()` until the process exits.
3. Waits for that result and returns it.

- **Errors**:
  - `AlreadyStarted`: a second call.
  - `Unavailable("could not start the listener thread")`: the thread could not be spawned.
  - `Unavailable("the listener thread stopped")`: the thread ended without replying.
  - `Unavailable("could not create the listener window")`: `GetModuleHandleW`, `RegisterClassW` or `CreateWindowExW` failed.
  - `Unavailable("could not subscribe to session changes")`: `WTSRegisterSessionNotification` failed.
- **Edge cases and invariants**: setup failures are reported to the caller instead of being lost on the thread.

### `reason_for` (private)

Maps a window message and its `wparam` to a lock reason:

| Message | `wparam` | Reason |
| ------- | -------- | ------ |
| `WM_POWERBROADCAST` | `PBT_APMSUSPEND` | `LockReason::Suspend` |
| `WM_WTSSESSION_CHANGE` | `WTS_SESSION_LOCK` | `LockReason::ScreenLock` |
| anything else | | `None` |

The `wparam` is first converted to `u32`. A value that does not fit gives `None`.

### `window_proc` (private)

`extern "system"` window procedure. If `reason_for` gives a reason and the sink is set, it calls `sink(reason)` synchronously, then always forwards to `DefWindowProcW` with the unchanged arguments. The source states that Windows waits for this handler before it suspends, so the vault is locked before the system sleeps.

### `create_listener_window` (private)

1. `GetModuleHandleW(null)`: the executable's own module handle.
2. `RegisterClassW` with `lpfnWndProc = window_proc`, that module and the class name `NovaVaultLockListener`.
3. `CreateWindowExW` with style `WS_OVERLAPPED` and without `WS_VISIBLE`, size 0×0, no parent, no menu, title `NovaVault`. This creates a top-level window that is never shown.
4. `WTSRegisterSessionNotification(window, NOTIFY_FOR_THIS_SESSION)`.

The window is never destroyed and stays alive for the rest of the process.

### `run_message_loop` (private)

Runs `GetMessageW` and `DispatchMessageW` while `GetMessageW` returns a positive value. `0` (`WM_QUIT`) and `-1` (error) both end the loop.

### `unsafe` usage

The file opts out of the workspace's `unsafe_code` lint with `#![allow(unsafe_code)]`, because Win32 offers no safe API for window classes, window procedures or message loops. Each `unsafe` block is one FFI call with a `SAFETY` comment:

- `DefWindowProcW`;
- `GetModuleHandleW`;
- `RegisterClassW`;
- `CreateWindowExW`;
- `WTSRegisterSessionNotification`;
- `GetMessageW`;
- `DispatchMessageW`.

## Dependencies

- **Internal**:
  - [`os_lock`](mod.rs.md): `LockMonitor`, `LockSink` and `MonitorError`.
  - [`autolock`](../autolock.rs.md): `LockReason`.
- **External**: `windows`: the Win32 bindings (`Foundation`, `LibraryLoader`, `RemoteDesktop`, `WindowsAndMessaging`), including the `w!` wide-string macro.

## Used by

- [`os_lock/mod.rs`](mod.rs.md): `platform_monitor` returns `WindowsMonitor` on Windows.

## Tests

- [`maps_suspend_and_lock_messages`](../../../../../src-tauri/src/os_lock/windows.rs#L165): `PBT_APMSUSPEND` maps to `Suspend` and `WTS_SESSION_LOCK` to `ScreenLock`. `PBT_APMRESUMEAUTOMATIC`, `WTS_SESSION_UNLOCK`, and a lock code on an unrelated message map to `None`. Window creation and the message loop are not unit-tested.

## Notes

- Only `WTS_SESSION_LOCK` is handled from session changes. Other `WM_WTSSESSION_CHANGE` codes (for example console or remote disconnect, or logoff) do not lock the vault. Whether switching users on Windows always sends `WTS_SESSION_LOCK` first is not verified by this code or its tests. Open question.
- If spawning the thread fails after `SINK` was set, a later `start` call returns `AlreadyStarted` instead of retrying. `main.rs` calls `start` only once, so this has no effect today.
- The listener thread runs until the process exits. Nothing posts `WM_QUIT` to it.
