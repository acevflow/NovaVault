# [`src-tauri/src/os_lock/mod.rs`](../../../../../src-tauri/src/os_lock/mod.rs)

## Purpose

Defines the common interface for the operating-system lock triggers, system suspend and screen lock, and picks one implementation per OS at compile time. Monitors only report events, as a `LockReason`, to a sink. The sink decides what to do: in practice, [`autolock::lock_and_reset_ui`](../autolock.rs.md#lock_and_reset_ui). If a monitor cannot start, the inactivity timer and manual locking keep working.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `LockSink` | type alias | `pub type LockSink = Arc<dyn Fn(LockReason) + Send + Sync>;` | Receives lock events, from any thread. |
| `LockMonitor` | trait | `pub trait LockMonitor` | A source of OS lock events. |
| `LockMonitor::start` | trait method | `fn start(&self, sink: LockSink) -> Result<(), MonitorError>;` | Starts delivering events for the rest of the process. |
| `MonitorError` | enum | `pub enum MonitorError` | Why a monitor could not start. `Debug, Clone, Copy, PartialEq, Eq, Error`. |
| `platform_monitor` | function | `pub fn platform_monitor() -> Box<dyn LockMonitor>` | The monitor for the build target. |

## Details

### `LockSink`

An `Arc`'d closure, so each platform can clone it into observers or tasks. It is called on whichever thread the platform delivers events on, so it must be cheap and thread-safe. The sink built in `main.rs` takes the session mutex, which may wait for the command currently running.

### `LockMonitor::start`

- **Errors**: `MonitorError` if the platform API cannot be set up synchronously. Some failures only surface later, on stderr: on Linux, a missing D-Bus service is detected by the watcher tasks after `start` returns.
- **Edge cases and invariants**: platform hooks are process-global. Each implementation returns `AlreadyStarted` on a second call.

### `MonitorError`

| Variant | Message | Raised by |
| ------- | ------- | --------- |
| `AlreadyStarted` | `the OS lock monitor is already running` | Every implementation, on a second `start`. |
| `Unavailable(&'static str)` | `OS lock events are unavailable: {0}` | Windows (thread, window or subscription setup) and the unsupported fallback. |

- **Security**: the payload is a `&'static str` naming the failed step. It never contains data.
- **Edge cases and invariants**: on macOS and Linux `Unavailable` is never constructed, so it carries `#[cfg_attr(any(target_os = "macos", target_os = "linux"), expect(dead_code, …))]`.

### `platform_monitor`

Returns the boxed monitor for the build target. Start it once.

## Platform differences

| | Windows ([`windows.rs`](windows.rs.md)) | macOS ([`macos.rs`](macos.rs.md)) | Linux ([`linux.rs`](linux.rs.md)) | Other ([`unsupported.rs`](unsupported.rs.md)) |
| - | - | - | - | - |
| `cfg` | `target_os = "windows"` | `target_os = "macos"` | `target_os = "linux"` | none of the three |
| Suspend | `WM_POWERBROADCAST` / `PBT_APMSUSPEND` | `NSWorkspaceWillSleepNotification` | logind `Manager.PrepareForSleep(true)` (system bus) | none |
| Screen lock | `WM_WTSSESSION_CHANGE` / `WTS_SESSION_LOCK` | `com.apple.screenIsLocked` (distributed, undocumented); `NSWorkspaceSessionDidResignActiveNotification` (fast user switching) | logind `Session.Lock` on the app's own session; `ActiveChanged(true)` from `org.freedesktop.ScreenSaver` or `org.gnome.ScreenSaver` (session bus) | none |
| Delivery | A dedicated thread `novavault-os-lock` with a hidden top-level window and its own message loop | Observers on the main thread's run loop | Two `zbus` tasks on Tauri's async runtime | none |
| Lock runs | Synchronously in the window procedure | Synchronously in the observer block | In the async task, as each signal arrives | none |
| `start` fails synchronously | Yes, `Unavailable` if the thread, window class, window or WTS subscription fails | Never (except `AlreadyStarted`) | Never (except `AlreadyStarted`); later failures go to stderr | Always, `Unavailable("not supported on this platform")` |
| `unsafe` | Yes, one FFI call per block | Yes, one call per block | None | None |

The module doc states that the Windows and macOS modules are the only places in NovaVault allowed to use `unsafe`. Each enables it with a file-level `#![allow(unsafe_code)]`.

## Dependencies

- **Internal**: [`autolock`](../autolock.rs.md): `LockReason`.
- **External**: `thiserror`: `MonitorError`'s `Display` and `Error`.

## Used by

- [`main.rs`](../main.rs.md): `start_os_lock_monitor` builds the sink and calls `platform_monitor().start(sink)`, printing any error.
- The four platform modules implement `LockMonitor`.

## Tests

_Not applicable: this file has no tests. The Windows message mapping is tested in [`windows.rs`](windows.rs.md#tests)._

## Notes

- The module doc table lists exactly the events the platform modules handle. It matches [`docs/security-design.md`](../../../../security-design.md) and [`docs/architecture.md`](../../../../architecture.md#auto-lock).
- None of the monitors handles resume, unlock or session logoff events. Only the events in the table lock the vault.
