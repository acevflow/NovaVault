# [`src-tauri/src/clipboard.rs`](../../../../src-tauri/src/clipboard.rs)

## Purpose

Copies item fields to the system clipboard and clears them again. Copying happens in Rust, so a copied secret never passes through the webview: the UI names an item and a field and only learns the clear delay. A single worker thread owns the clipboard handle for the lifetime of the app, because on X11 and Wayland the copying process must keep serving the contents. The worker clears the clipboard after the configured delay, on lock and on exit, but only if it still holds the value NovaVault put there.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `BackendError` | struct | `pub struct BackendError;` | The clipboard could not be opened, read or written. No details. `Debug, Clone, Copy, PartialEq, Eq`. |
| `ClipboardBackend` | trait | `pub trait ClipboardBackend` | Access to a clipboard: `set_text`, `get_text`, `clear`. |
| `ClipboardBackend::set_text` | trait method | `fn set_text(&mut self, text: &str) -> Result<(), BackendError>;` | Replaces the contents, with history opt-out. |
| `ClipboardBackend::get_text` | trait method | `fn get_text(&mut self) -> Result<Option<Zeroizing<String>>, BackendError>;` | Current text, or `None` if not text. |
| `ClipboardBackend::clear` | trait method | `fn clear(&mut self) -> Result<(), BackendError>;` | Empties the clipboard. |
| `SystemClipboard` | struct | `pub struct SystemClipboard(arboard::Clipboard);` | The OS clipboard through `arboard`. |
| `SystemClipboard::open` | function | `pub fn open() -> Result<Self, BackendError>` | Opens the OS clipboard. |
| `ClipboardManager` | struct | `pub struct ClipboardManager` | Handle to the worker; Tauri managed state. |
| `ClipboardManager::start` | function | `pub fn start<B, F>(open: F) -> Self where B: ClipboardBackend, F: FnMut() -> Result<B, BackendError> + Send + 'static` | Spawns the worker. |
| `ClipboardManager::copy` | method | `pub fn copy(&self, text: Zeroizing<String>, clear_after: Duration) -> CommandResult<()>` | Copies and schedules the clear; waits for the outcome. |
| `ClipboardManager::clear` | method | `pub fn clear(&self)` | Clears now if still ours; does not wait. |
| `ClipboardManager::shutdown` | method | `pub fn shutdown(&self)` | Clears if still ours, releases the clipboard, stops the worker. |

## Details

### `BackendError`

A unit struct. Platform error text from `arboard` is dropped on purpose: the UI does not need it, and it is never logged.

### `ClipboardBackend`

The seam between the worker and the platform. [`SystemClipboard`](#systemclipboard) implements it for real use; the tests use an in-memory fake.

### `SystemClipboard`

#### `SystemClipboard::open`

`arboard::Clipboard::new()`. Fails with `BackendError` where there is no clipboard, for example no display server on Linux.

#### `set_text`: platform differences

Every copy asks clipboard history and sync tools to skip the value, selected at compile time:

| Target | `arboard` extension | Calls | Effect (per [`docs/clipboard.md`](../../../clipboard.md)) |
| ------ | ------------------- | ----- | -------------------------------------------------------- |
| `windows` | `SetExtWindows` | `exclude_from_history().exclude_from_cloud()` | `CanIncludeInClipboardHistory = 0` and `CanUploadToCloudClipboard = 0`: kept out of Win+V history and cloud clipboard sync. |
| `macos` | `SetExtApple` | `exclude_from_history()` | The `org.nspasteboard.ConcealedType` marker honoured by clipboard history tools. |
| `linux` | `SetExtLinux` | `exclude_from_history()` | The `x-kde-passwordManagerHint: secret` MIME type honoured by Klipper and others. Only the `CLIPBOARD` selection is written, never `PRIMARY`. |
| other | none | none | Plain text copy. |

These are conventions that well-behaved tools honour, not guarantees.

#### `get_text`

Wraps the text in `Zeroizing`, because what is read back is usually the secret NovaVault copied. `arboard::Error::ContentNotAvailable` (for example, an image on the clipboard) becomes `Ok(None)`. Any other error becomes `BackendError`.

#### `clear`

`arboard::Clipboard::clear()`.

#### Platform lifetime differences

The handle's lifetime matters differently per OS:

- **Windows and macOS**: the system keeps the contents after the handle is dropped or the app exits. Clearing on exit is the only protection there; a killed app leaves the value behind.
- **Linux, X11**: the app owns the `CLIPBOARD` selection and serves it from the worker thread. Clearing removes ownership.
- **Linux, Wayland**: `arboard` uses data-control when the compositor supports it, otherwise XWayland. With XWayland, a timed clear may reach native Wayland apps only once NovaVault's window is focused again (see [`docs/clipboard.md`](../../../clipboard.md#wayland)).
- **Linux, both**: the value usually disappears with the process.

### `ClipboardManager`

Holds `sender: Mutex<Option<Sender<Message>>>`. It is `None` if the worker never started or after `shutdown`, so later calls fail fast instead of queueing messages nobody reads.

#### `ClipboardManager::start`

Creates an `mpsc` channel and spawns the thread `novavault-clipboard`, running the worker loop. `open` is not called at start: the worker calls it on the first copy, and again after any failure. The app therefore starts even where no clipboard is available yet. If the thread cannot be spawned, it prints a fixed message to stderr and stores `None`. Copying then fails with `Internal`.

#### `ClipboardManager::copy`

1. Sends `Message::Copy { text, clear_after, reply }`.
2. Blocks until the worker replies.

- **Returns**: `Ok(())` once the clipboard holds `text` and the clear is scheduled. A later copy replaces the schedule.
- **Errors**:
  - `ClipboardUnavailable`: the clipboard could not be opened or written.
  - `Internal`: the worker is gone (stopped or never started), or it dropped the reply.
- **Security**: `text` is a `Zeroizing<String>` moved into the message. It is never serialized for the webview, and it is wiped when the worker replaces or clears it.

#### `ClipboardManager::clear`

Sends `Message::Clear` and returns at once. Errors are ignored: nothing can be pending if the worker is gone.

#### `ClipboardManager::shutdown`

Takes the sender (later calls see `None`), sends `Message::Shutdown`, and waits at most `SHUTDOWN_TIMEOUT` (2 seconds) for the worker to confirm. A second call returns immediately. The order on the worker side, clear first and then release the handle, matters on X11: releasing a value would let a `CLIPBOARD_MANAGER` take a copy after exit.

### Worker internals (private)

- `Message`:
  - `Copy { text, clear_after, reply }`;
  - `Clear` (sent on lock);
  - `Shutdown { reply }`.
- `Pending { text, deadline }`: the value last copied and when to clear it. The text is kept only to compare with the clipboard before clearing.
- `run_worker(open, receiver)`:
  1. With a pending copy, waits for a message until its deadline (`recv_timeout`). Without one, waits indefinitely.
  2. Deadline reached: `clear_if_unchanged`.
  3. `Copy`: `copy_text`. On success, the new value replaces `pending` (the old `Pending` is wiped). Then it replies.
  4. `Clear`: `clear_if_unchanged`.
  5. `Shutdown`: `clear_if_unchanged`, drop the backend, reply, return.
  6. If every sender disappears without a shutdown, it still runs `clear_if_unchanged` before ending.
- `copy_text(backend, open, text)`: opens the clipboard if there is no handle. On a write failure it drops the handle, so the next copy reopens it (for example, after the X11 connection was lost).
- `clear_if_unchanged(backend, copied)`: reads the clipboard and decides whether it still holds NovaVault's value:
  - it matches: clear;
  - it holds no text: leave it;
  - it cannot be read: clear anyway, because losing an unrelated entry is better than leaving a secret behind.

  A failed clear prints a fixed message to stderr. Without a handle or a pending value, it does nothing.

## Dependencies

- **Internal**: [`error`](error.rs.md): `CommandError::ClipboardUnavailable` and `Internal`.
- **External**:
  - `arboard`: cross-platform clipboard access, including the per-OS history opt-outs.
  - `zeroize`: `Zeroizing` for copied and read-back text.

## Used by

- [`main.rs`](main.rs.md): `ClipboardManager::start(SystemClipboard::open)` at setup, and `shutdown` on `RunEvent::Exit`.
- [`commands.rs`](commands.rs.md): `copy_item_field` calls `copy`. The `lock` command calls `clear`.
- [`autolock.rs`](autolock.rs.md): `clear` after an automatic lock.

## Tests

The tests use a `FakeClipboard` shared with the test through `Arc<Mutex<…>>`, with a switch to make writes fail.

- [`copied_text_is_cleared_after_the_delay`](../../../../src-tauri/src/clipboard.rs#L395): a 50 ms copy is placed, then cleared.
- [`something_copied_afterwards_is_left_alone`](../../../../src-tauri/src/clipboard.rs#L404): a later user copy survives the deadline.
- [`a_new_copy_replaces_the_schedule`](../../../../src-tauri/src/clipboard.rs#L414): a second copy with a long delay is not cleared at the first deadline, and `clear` still removes it.
- [`clear_on_lock_only_removes_our_value`](../../../../src-tauri/src/clipboard.rs#L427): `clear` and `shutdown` leave a user's later copy alone.
- [`shutdown_clears_and_stops`](../../../../src-tauri/src/clipboard.rs#L438): shutdown clears, copying afterwards gives `Internal`, and a second shutdown is harmless.
- [`write_failures_are_reported_and_retried`](../../../../src-tauri/src/clipboard.rs#L453): a failed write gives `ClipboardUnavailable`, and the next copy succeeds.
- [`a_missing_clipboard_is_reported`](../../../../src-tauri/src/clipboard.rs#L467): an `open` that always fails gives `ClipboardUnavailable`.

## Notes

- **Possible gap after a failed copy.** If copy A succeeds and a later copy B fails to write, `copy_text` drops the handle, while `pending` still holds A. At A's deadline, on lock and on shutdown, `clear_if_unchanged` returns early because there is no handle, so A is never cleared. On Windows and macOS the clipboard keeps its contents after the handle is dropped, so A may stay on the clipboard. This contradicts the module doc ("the worker clears the clipboard after the configured delay, when the vault locks and when the app exits") and is not covered by a test. Reported here only; this page does not change code.
- `docs/clipboard.md` says "Nothing is retried in the background". That holds: the reopen after a failure happens only on the next copy.
- The per-OS effects of the `arboard` extensions are taken from [`docs/clipboard.md`](../../../clipboard.md). This file only calls the extension methods.
- On targets other than Windows, macOS and Linux, no history opt-out is applied.
