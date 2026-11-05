# [`src-tauri/src/clipboard.rs`](../../../../src-tauri/src/clipboard.rs#L330) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `clipboard.rs` drives a real `ClipboardManager` worker thread against an in-memory `FakeClipboard`. It checks that copied secrets are cleared after the delay, on `clear` (lock) and on `shutdown`, that a value the user copied afterwards is never wiped, and that backend failures surface as `ClipboardUnavailable`. No test touches the system clipboard.

## Public API

| Name                                        | Kind   | Signature                                              | Description                                                              |
| ------------------------------------------- | ------ | ------------------------------------------------------ | ------------------------------------------------------------------------ |
| `FakeClipboard`                             | helper | `struct FakeClipboard { contents: Arc<Mutex<Option<String>>>, fail_writes: Arc<Mutex<bool>> }` | In-memory `ClipboardBackend`; clones share state with the test. |
| `FakeClipboard::contents`                   | helper | `fn contents(&self) -> Option<String>`                 | Current clipboard text, as seen by the test.                             |
| `FakeClipboard::user_copies`                | helper | `fn user_copies(&self, text: &str)`                    | Simulates another app or the user copying something.                     |
| `manager`                                   | helper | `fn manager(fake: &FakeClipboard) -> ClipboardManager` | Starts a manager whose `open` always returns a clone of `fake`.          |
| `secret`                                    | helper | `fn secret(text: &str) -> Zeroizing<String>`           | Wraps test text the way `copy` expects it.                               |
| `eventually`                                | helper | `fn eventually(condition: impl Fn() -> bool)`          | Polls every 5 ms until `condition` holds; fails after 5 s.               |
| `SHORT`, `LONG`                             | consts | `const SHORT: Duration` (50 ms), `const LONG: Duration` (60 s) | A delay that expires during the test, and one that never does.   |
| `copied_text_is_cleared_after_the_delay`    | test   | `fn copied_text_is_cleared_after_the_delay()`          | A 50 ms copy is placed, then cleared.                                    |
| `something_copied_afterwards_is_left_alone` | test   | `fn something_copied_afterwards_is_left_alone()`       | A later user copy survives the deadline.                                 |
| `a_new_copy_replaces_the_schedule`          | test   | `fn a_new_copy_replaces_the_schedule()`                | The second copy's delay replaces the first; `clear` still removes it.    |
| `clear_on_lock_only_removes_our_value`      | test   | `fn clear_on_lock_only_removes_our_value()`            | `clear` and `shutdown` leave a later user copy alone.                    |
| `shutdown_clears_and_stops`                 | test   | `fn shutdown_clears_and_stops()`                       | Shutdown clears, later copies fail with `Internal`, shutdown is idempotent. |
| `write_failures_are_reported_and_retried`   | test   | `fn write_failures_are_reported_and_retried()`         | A failed write gives `ClipboardUnavailable`; the next copy succeeds.     |
| `a_missing_clipboard_is_reported`           | test   | `fn a_missing_clipboard_is_reported()`                 | An `open` that always fails gives `ClipboardUnavailable`.                |

## Details

### Helpers

`FakeClipboard` implements `ClipboardBackend`: `set_text` stores the text unless `fail_writes` is set (then `Err(BackendError)`), `get_text` returns a `Zeroizing` copy, `clear` sets the contents to `None`. Because both fields are `Arc<Mutex<…>>`, the clone moved into the worker and the one kept by the test see the same clipboard. `eventually` exists so timing-based assertions do not become flaky on slow CI machines; the 5 s limit is far above the 50 ms delay.

### [`copied_text_is_cleared_after_the_delay`](../../../../src-tauri/src/clipboard.rs#L395)

Copies `hunter2` with `SHORT`. `copy` returns only after the worker wrote it, so the clipboard holds `hunter2` immediately; it then becomes empty within the timeout.

### [`something_copied_afterwards_is_left_alone`](../../../../src-tauri/src/clipboard.rs#L404)

Copies `hunter2` with `SHORT`, then the user copies `shopping list`. After four times the delay the clipboard still holds `shopping list`: the worker compares the current text with what it copied and only clears its own value.

### [`a_new_copy_replaces_the_schedule`](../../../../src-tauri/src/clipboard.rs#L414)

Copies `first` with `SHORT`, then `second` with `LONG`. After `SHORT * 4` the clipboard holds `second` (the first deadline no longer applies). `clear()`, which the shell calls on lock, then empties it at once.

### [`clear_on_lock_only_removes_our_value`](../../../../src-tauri/src/clipboard.rs#L427)

Copies `hunter2` with `LONG`, the user copies `mine`, then `clear()` and `shutdown()` run. The clipboard still holds `mine`.

### [`shutdown_clears_and_stops`](../../../../src-tauri/src/clipboard.rs#L438)

Copies `hunter2` with `LONG` and shuts down: the clipboard is empty when `shutdown` returns (it waits for the worker). A later `copy` returns `Err(CommandError::Internal)` because the worker is gone, and a second `shutdown` neither panics nor blocks.

### [`write_failures_are_reported_and_retried`](../../../../src-tauri/src/clipboard.rs#L453)

With `fail_writes` on, `copy` returns `Err(CommandError::ClipboardUnavailable)`. With it off again, the next `copy` succeeds and the clipboard holds the text: the worker reopens the backend on the next request.

### [`a_missing_clipboard_is_reported`](../../../../src-tauri/src/clipboard.rs#L467)

Starts a manager whose `open` always returns `Err(BackendError)`. `copy` returns `Err(CommandError::ClipboardUnavailable)`.

## Dependencies

- **Internal**: [`clipboard`](clipboard.rs.md) (`super::*`): `ClipboardManager`, `ClipboardBackend`, `BackendError`; [`error`](error.rs.md): `CommandError`.
- **External**: `zeroize::Zeroizing` (via `super::*`); the standard library for threads, timing and `Arc<Mutex<…>>`.

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`src-tauri/src/clipboard.rs`](../../../../src-tauri/src/clipboard.rs), documented in [`clipboard.rs.md`](clipboard.rs.md).

How to run:

```sh
cargo test -p novavault clipboard::tests
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- `SystemClipboard` (the `arboard` backend and its per-OS history opt-outs) is not tested; the tests cover only the worker logic.
- The case where a copy succeeds and a later copy's write fails is not tested; [`clipboard.rs.md`](clipboard.rs.md) describes the resulting gap, where the first value may never be cleared.
- The tests rely on wall-clock sleeps (`SHORT * 4` = 200 ms in two tests); `eventually` bounds the positive waits, but the "left alone" checks are only as strong as that fixed sleep.
