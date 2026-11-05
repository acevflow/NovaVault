# [`src-tauri/src/os_lock/macos.rs`](../../../../../src-tauri/src/os_lock/macos.rs)

## Purpose

The macOS `LockMonitor`. It registers three notification observers that report sleep, fast user switching and screen lock to the sink. The observers are registered from the main thread (the Tauri setup hook), whose run loop delivers the notifications. Compiled only for `target_os = "macos"`.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `MacMonitor` | struct | `pub struct MacMonitor;` | The macOS monitor. |
| `impl LockMonitor for MacMonitor` | trait impl | `fn start(&self, sink: LockSink) -> Result<(), MonitorError>` | Registers the observers and returns. |

`MacMonitor` is reachable only through [`platform_monitor`](mod.rs.md#platform_monitor).

## Details

### `MacMonitor::start`

1. `STARTED.swap(true)`. If it was already set, returns `MonitorError::AlreadyStarted`.
2. On `NSWorkspace::sharedWorkspace().notificationCenter()`, observes:
   - `NSWorkspaceWillSleepNotification` → `LockReason::Suspend`: posted before the system sleeps;
   - `NSWorkspaceSessionDidResignActiveNotification` → `LockReason::ScreenLock`: the user switched to another login session.
3. On `NSDistributedNotificationCenter::defaultCenter()`, observes `com.apple.screenIsLocked` → `LockReason::ScreenLock`. The source describes this as not formally documented, but the long-standing signal macOS apps use for screen lock.
4. Returns `Ok(())`.

- **Errors**: only `AlreadyStarted`. Registration itself cannot fail synchronously.
- **Edge cases and invariants**: must run on the main thread. `main.rs` calls it from Tauri's `setup` hook, which runs there.

### `observe` (private)

Wraps a clone of the sink in an `RcBlock` that calls `sink(reason)`. It registers that block with `addObserverForName_object_queue_usingBlock(Some(name), None, None, &block)`:

- no sender filter;
- no queue, so the block runs synchronously on the posting thread, here the main thread.

The returned observer token is deliberately leaked with `mem::forget`, because the observer must stay registered until the process exits.

### `unsafe` usage

`#![allow(unsafe_code)]` at file level, because objc2 exposes the AppKit notification-name statics and the block-based observer API only as `unsafe`. There are two blocks, each with a `SAFETY` comment:

- reading `NSWorkspaceWillSleepNotification` and `NSWorkspaceSessionDidResignActiveNotification` (immutable constants valid for the whole process);
- `addObserverForName_object_queue_usingBlock` (no sender filter, so no object type can be wrong; the block captures only a `Send + Sync` sink).

## Dependencies

- **Internal**:
  - [`os_lock`](mod.rs.md): `LockMonitor`, `LockSink` and `MonitorError`.
  - [`autolock`](../autolock.rs.md): `LockReason`.
- **External**:
  - `objc2-app-kit`: `NSWorkspace` and the workspace notification names.
  - `objc2-foundation`: `NSDistributedNotificationCenter`, `NSNotificationCenter`, `NSNotification` and `NSString`.
  - `block2`: `RcBlock` for the observer closures.

## Used by

- [`os_lock/mod.rs`](mod.rs.md): `platform_monitor` returns `MacMonitor` on macOS.

## Tests

_Not applicable: this file has no tests. Registering observers needs a running AppKit process._

## Notes

- `NSWorkspaceSessionDidResignActiveNotification` is mapped to `ScreenLock`, but it signals fast user switching, not a screen lock. [`LockReason::ScreenLock`](../autolock.rs.md#lockreason) is documented as covering both ("the screen was locked or the user session was switched away").
- `com.apple.screenIsLocked` is undocumented by Apple. If a future macOS release stops posting it, screen lock would no longer lock the vault, and nothing in the code would detect that. [`docs/threat-model.md`](../../../../threat-model.md) records this risk.
- The macOS module and the API it calls are not covered by the local check on Windows. They are built and tested only by CI's macOS job.
