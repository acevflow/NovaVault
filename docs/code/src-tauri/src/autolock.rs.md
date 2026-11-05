# [`src-tauri/src/autolock.rs`](../../../../src-tauri/src/autolock.rs)

## Purpose

Automatic locking: after inactivity, and on the OS events reported by [`os_lock`](os_lock/mod.rs.md). Whatever the trigger, `lock_and_reset_ui` locks the session (wiping the vault key), clears a copied value from the clipboard, and reloads the main webview. The reload discards everything the UI holds in memory and returns it to its start screen, without the UI listening for events, so the webview needs no event permission.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `LockReason` | enum | `pub enum LockReason` | Why the vault was locked automatically. `Debug, Clone, Copy, PartialEq, Eq`. |
| `LockReason::describe` | method | `pub const fn describe(self) -> &'static str` | Name for diagnostics. |
| `lock_and_reset_ui` | function | `pub fn lock_and_reset_ui(app: &AppHandle, reason: LockReason)` | Locks for `reason` and resets the UI if it locked. |
| `start_idle_timer` | function | `pub fn start_idle_timer(app: AppHandle)` | Starts the inactivity thread. |

## Details

### `LockReason`

| Variant | Meaning | `describe()` | Raised by |
| ------- | ------- | ------------ | --------- |
| `Inactivity` | No command for the configured timeout. | `inactivity` | `start_idle_timer`. |
| `Suspend` | The system is about to suspend or hibernate. | `system suspend` | The suspend source of each OS monitor. |
| `ScreenLock` | The screen was locked or the user session was switched away. | `screen lock` | The screen-lock sources of each OS monitor. |

### `lock_and_reset_ui`

1. Gets `AppState` from Tauri's managed state.
2. Locks:
   - `Inactivity`: `state.with_session(|s| s.lock_if_idle(Instant::now()))`. It locks only if the session has really been idle for the timeout. It never interrupts a transfer, because a transfer in progress holds the session mutex and counts as activity.
   - `Suspend` and `ScreenLock`: `state.with_session_interrupting_transfers(Session::lock)`. It locks unconditionally and first asks a running attachment transfer to stop at its next chunk, so the lock is not delayed by it.
3. If nothing was locked (already locked, or not idle long enough), it returns. The clipboard is not touched and the UI is not reloaded.
4. Calls `ClipboardManager::clear()`, which does not wait.
5. Reloads the webview window labelled `main`. If the reload fails, it prints `NovaVault: vault locked (<reason>), UI reset failed`. The key is already wiped at that point; only the UI's stale view remains.

- **Parameters**: `app` is the Tauri handle; `reason` is the trigger.
- **Security**: the key is wiped before anything else happens. The reload also drops secrets the user had revealed in the UI, along with titles and form contents. The stderr message contains only the fixed reason name.
- **Edge cases and invariants**:
  - It runs on the caller's thread: the idle thread, the Windows listener thread, the macOS main thread or a Linux async task. On Windows and macOS this makes the suspend lock synchronous with the OS notification.
  - It blocks while another command holds the session mutex. For OS events, the interrupt flag bounds that wait to one chunk of an attachment transfer. A long Argon2id derivation (unlock, password change) is not interrupted.
  - It panics if `AppState` or `ClipboardManager` is not managed. `main.rs` manages both before starting either trigger.

### `start_idle_timer`

Spawns the thread `novavault-idle-lock`. Every `CHECK_INTERVAL` (5 seconds) it calls `lock_and_reset_ui(&app, LockReason::Inactivity)`, forever.

- **Security**: the thread holds only the `AppHandle`, no secret.
- **Errors**: none returned. If the thread cannot be spawned, it prints `NovaVault: could not start the inactivity auto-lock`. Manual and OS-event locking still work.
- **Edge cases and invariants**:
  - The lock can happen up to 5 seconds after the timeout.
  - Inactivity means "no command" ([`Session::touch`](session.rs.md#sessiontouch-and-sessionis_unlocked) is called by every command that goes through `run`). Mouse or keyboard activity without a command does not count.
  - The idle clock is `Instant`, which is monotonic. On platforms where it pauses during sleep, the suspend monitor covers that case by locking before the system sleeps.

## Platform differences

This file has no platform-specific code. The behaviour differs only through the triggers:

- Windows and macOS call `lock_and_reset_ui` synchronously from the OS notification.
- Linux calls it from an async task.
- On other targets only `Inactivity` exists.

See the table in [`os_lock/mod.rs.md`](os_lock/mod.rs.md#platform-differences).

## Dependencies

- **Internal**:
  - [`commands`](commands.rs.md): `AppState::with_session` and `with_session_interrupting_transfers`.
  - [`session`](session.rs.md): `Session::lock` and `lock_if_idle`.
  - [`clipboard`](clipboard.rs.md): `ClipboardManager::clear`.
- **External**: `tauri`: `AppHandle`, managed state and `get_webview_window` / `reload`.

## Used by

- [`main.rs`](main.rs.md): calls `start_idle_timer` at setup, and builds the OS monitor sink around `lock_and_reset_ui`.
- The [`os_lock`](os_lock/mod.rs.md) modules use `LockReason`.

## Tests

_Not applicable: this file has no tests. The idle decision is tested through [`Session::lock_if_idle`](session.rs.md#tests) (`idle_sessions_lock_after_the_timeout`, `auto_lock_timeout_is_configurable_and_persisted`), and the clipboard clear through [`clipboard.rs`](clipboard.rs.md#tests)._

## Notes

- A manual lock (the `lock` command) does not go through this function. It clears the clipboard and locks, but does not reload the webview: the UI handles its own Lock button. [`docs/threat-model.md`](../../../threat-model.md) says "Every lock clears a copied value and reloads the webview". For manual locks that is inaccurate. [`docs/security-design.md`](../../../security-design.md) correctly limits the reload to automatic locks.
- An OS event while the vault is already locked does nothing, including no clipboard clear. That is harmless, because every earlier lock already cleared it.
