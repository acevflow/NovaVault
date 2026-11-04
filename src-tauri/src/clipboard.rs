//! Copying item fields to the system clipboard, and clearing them again.
//!
//! Copy happens in Rust so that a copied secret never passes through the
//! webview: the UI asks for "the password of item X" and only learns how
//! long the clipboard will hold it.
//!
//! One worker thread owns the clipboard. On X11 and Wayland the copying
//! process must stay alive and serve the contents to whoever pastes, so the
//! clipboard handle lives as long as the app does. The worker clears the
//! clipboard after the configured delay, when the vault locks and when the
//! app exits, but only if the clipboard still holds the value NovaVault put
//! there: something the user copied afterwards is left alone.
//!
//! Per-OS behaviour, including the X11 and Wayland differences, is
//! documented in `docs/clipboard.md`.

use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use zeroize::Zeroizing;

use crate::error::{CommandError, CommandResult};

/// How long the app waits at exit for the worker to clear the clipboard.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// The clipboard could not be opened, read or written. Carries no details:
/// platform error text is not needed by the UI and is never logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendError;

/// Access to a system clipboard. Implemented by [`SystemClipboard`] and,
/// in tests, by a fake.
pub trait ClipboardBackend {
    /// Replaces the clipboard's contents with `text`, asking clipboard
    /// history tools not to record it.
    ///
    /// # Errors
    ///
    /// [`BackendError`] if the clipboard cannot be written.
    fn set_text(&mut self, text: &str) -> Result<(), BackendError>;

    /// The clipboard's text, or `None` if it holds no text (for example an
    /// image).
    ///
    /// # Errors
    ///
    /// [`BackendError`] if the clipboard cannot be read.
    fn get_text(&mut self) -> Result<Option<Zeroizing<String>>, BackendError>;

    /// Empties the clipboard.
    ///
    /// # Errors
    ///
    /// [`BackendError`] if the clipboard cannot be written.
    fn clear(&mut self) -> Result<(), BackendError>;
}

/// The operating system's clipboard, through `arboard`.
pub struct SystemClipboard(arboard::Clipboard);

impl SystemClipboard {
    /// Opens the system clipboard.
    ///
    /// # Errors
    ///
    /// [`BackendError`] if there is no clipboard (for example no display
    /// server on Linux).
    pub fn open() -> Result<Self, BackendError> {
        arboard::Clipboard::new()
            .map(Self)
            .map_err(|_| BackendError)
    }
}

impl ClipboardBackend for SystemClipboard {
    fn set_text(&mut self, text: &str) -> Result<(), BackendError> {
        let set = self.0.set();
        // Ask clipboard history and sync tools to skip the value. These are
        // conventions that well-behaved tools honour, not guarantees.
        #[cfg(target_os = "windows")]
        let set = {
            use arboard::SetExtWindows;
            set.exclude_from_history().exclude_from_cloud()
        };
        #[cfg(target_os = "macos")]
        let set = {
            use arboard::SetExtApple;
            set.exclude_from_history()
        };
        #[cfg(target_os = "linux")]
        let set = {
            use arboard::SetExtLinux;
            set.exclude_from_history()
        };
        set.text(text).map_err(|_| BackendError)
    }

    fn get_text(&mut self) -> Result<Option<Zeroizing<String>>, BackendError> {
        match self.0.get_text() {
            Ok(text) => Ok(Some(Zeroizing::new(text))),
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(_) => Err(BackendError),
        }
    }

    fn clear(&mut self) -> Result<(), BackendError> {
        self.0.clear().map_err(|_| BackendError)
    }
}

/// A request to the clipboard worker.
enum Message {
    /// Copy `text`, clear it after `clear_after`, and report the outcome.
    Copy {
        text: Zeroizing<String>,
        clear_after: Duration,
        reply: Sender<Result<(), BackendError>>,
    },
    /// Clear now (the vault locked).
    Clear,
    /// Clear now, release the clipboard and stop.
    Shutdown { reply: Sender<()> },
}

/// What the worker copied, and when to clear it.
struct Pending {
    text: Zeroizing<String>,
    deadline: Instant,
}

/// Handle to the clipboard worker thread. Managed as Tauri state.
pub struct ClipboardManager {
    // `None` once shut down (or if the worker never started), so later
    // calls fail fast instead of queueing messages nobody reads.
    sender: Mutex<Option<Sender<Message>>>,
}

impl ClipboardManager {
    /// Starts the worker. `open` is called on the worker thread the first
    /// time the clipboard is needed, and again after a failure, so the app
    /// starts even where no clipboard is available yet.
    ///
    /// A failure to spawn the thread is not fatal: copying then reports
    /// [`CommandError::ClipboardUnavailable`].
    pub fn start<B, F>(open: F) -> Self
    where
        B: ClipboardBackend,
        F: FnMut() -> Result<B, BackendError> + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel();
        let spawned = thread::Builder::new()
            .name("novavault-clipboard".into())
            .spawn(move || run_worker(open, &receiver));
        let sender = match spawned {
            Ok(_) => Some(sender),
            Err(_) => {
                eprintln!("NovaVault: clipboard worker could not start; copying is disabled");
                None
            }
        };
        Self {
            sender: Mutex::new(sender),
        }
    }

    /// Copies `text` to the clipboard and schedules it to be cleared after
    /// `clear_after`. A later copy replaces the schedule.
    ///
    /// # Errors
    ///
    /// [`CommandError::ClipboardUnavailable`] if the clipboard cannot be
    /// written, and [`CommandError::Internal`] if the worker is gone.
    pub fn copy(&self, text: Zeroizing<String>, clear_after: Duration) -> CommandResult<()> {
        let (reply, outcome) = mpsc::channel();
        self.send(Message::Copy {
            text,
            clear_after,
            reply,
        })?;
        match outcome.recv() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(BackendError)) => Err(CommandError::ClipboardUnavailable),
            Err(_) => Err(CommandError::Internal),
        }
    }

    /// Clears the clipboard now if it still holds what NovaVault copied.
    /// Does not wait for the worker.
    pub fn clear(&self) {
        // Nothing can be pending if the worker is gone.
        let _ = self.send(Message::Clear);
    }

    /// Clears the clipboard if it still holds what NovaVault copied, then
    /// stops the worker. Waits at most [`SHUTDOWN_TIMEOUT`].
    pub fn shutdown(&self) {
        let sender = self
            .sender
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        let Some(sender) = sender else { return };
        let (reply, done) = mpsc::channel();
        if sender.send(Message::Shutdown { reply }).is_ok() {
            let _ = done.recv_timeout(SHUTDOWN_TIMEOUT);
        }
    }

    fn send(&self, message: Message) -> CommandResult<()> {
        self.sender
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .ok_or(CommandError::Internal)?
            .send(message)
            .map_err(|_| CommandError::Internal)
    }
}

/// The worker loop: serves requests and clears the clipboard on time.
fn run_worker<B, F>(mut open: F, receiver: &Receiver<Message>)
where
    B: ClipboardBackend,
    F: FnMut() -> Result<B, BackendError>,
{
    let mut backend: Option<B> = None;
    let mut pending: Option<Pending> = None;
    loop {
        let message = match &pending {
            Some(copied) => {
                let wait = copied.deadline.saturating_duration_since(Instant::now());
                match receiver.recv_timeout(wait) {
                    Ok(message) => Some(message),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
            None => match receiver.recv() {
                Ok(message) => Some(message),
                Err(_) => break,
            },
        };
        match message {
            // The deadline passed.
            None => clear_if_unchanged(&mut backend, pending.take()),
            Some(Message::Copy {
                text,
                clear_after,
                reply,
            }) => {
                let outcome = copy_text(&mut backend, &mut open, &text);
                if outcome.is_ok() {
                    pending = Some(Pending {
                        text,
                        deadline: Instant::now() + clear_after,
                    });
                }
                let _ = reply.send(outcome);
            }
            Some(Message::Clear) => clear_if_unchanged(&mut backend, pending.take()),
            Some(Message::Shutdown { reply }) => {
                clear_if_unchanged(&mut backend, pending.take());
                drop(backend.take());
                let _ = reply.send(());
                return;
            }
        }
    }
    // Every sender is gone without a shutdown: still do not leave a copied
    // secret behind.
    clear_if_unchanged(&mut backend, pending.take());
}

/// Copies `text`, opening the clipboard if needed. After a failure the
/// handle is dropped so the next copy reopens it (for example after the
/// X11 connection was lost).
fn copy_text<B, F>(backend: &mut Option<B>, open: &mut F, text: &str) -> Result<(), BackendError>
where
    B: ClipboardBackend,
    F: FnMut() -> Result<B, BackendError>,
{
    let clipboard = match backend {
        Some(clipboard) => clipboard,
        None => backend.insert(open()?),
    };
    let outcome = clipboard.set_text(text);
    if outcome.is_err() {
        *backend = None;
    }
    outcome
}

/// Clears the clipboard if it still holds `copied`.
///
/// If the clipboard cannot be read it is cleared anyway: losing what the
/// user copied afterwards is better than leaving a secret behind.
fn clear_if_unchanged<B: ClipboardBackend>(backend: &mut Option<B>, copied: Option<Pending>) {
    let (Some(clipboard), Some(copied)) = (backend.as_mut(), copied) else {
        return;
    };
    let still_ours = match clipboard.get_text() {
        Ok(Some(current)) => *current == *copied.text,
        Ok(None) => false,
        Err(BackendError) => true,
    };
    if still_ours && clipboard.clear().is_err() {
        eprintln!("NovaVault: the clipboard could not be cleared");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// A clipboard in memory, shared with the test.
    #[derive(Clone, Default)]
    struct FakeClipboard {
        contents: Arc<Mutex<Option<String>>>,
        fail_writes: Arc<Mutex<bool>>,
    }

    impl FakeClipboard {
        fn contents(&self) -> Option<String> {
            self.contents.lock().unwrap().clone()
        }

        fn user_copies(&self, text: &str) {
            *self.contents.lock().unwrap() = Some(text.to_owned());
        }
    }

    impl ClipboardBackend for FakeClipboard {
        fn set_text(&mut self, text: &str) -> Result<(), BackendError> {
            if *self.fail_writes.lock().unwrap() {
                return Err(BackendError);
            }
            *self.contents.lock().unwrap() = Some(text.to_owned());
            Ok(())
        }

        fn get_text(&mut self) -> Result<Option<Zeroizing<String>>, BackendError> {
            Ok(self.contents().map(Zeroizing::new))
        }

        fn clear(&mut self) -> Result<(), BackendError> {
            *self.contents.lock().unwrap() = None;
            Ok(())
        }
    }

    fn manager(fake: &FakeClipboard) -> ClipboardManager {
        let fake = fake.clone();
        ClipboardManager::start(move || Ok(fake.clone()))
    }

    fn secret(text: &str) -> Zeroizing<String> {
        Zeroizing::new(text.to_owned())
    }

    /// Waits until `condition` holds, failing after a generous timeout so
    /// slow CI machines do not make the tests flaky.
    fn eventually(condition: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !condition() {
            assert!(Instant::now() < deadline, "condition not met in time");
            thread::sleep(Duration::from_millis(5));
        }
    }

    const SHORT: Duration = Duration::from_millis(50);
    const LONG: Duration = Duration::from_secs(60);

    #[test]
    fn copied_text_is_cleared_after_the_delay() {
        let fake = FakeClipboard::default();
        let clipboard = manager(&fake);
        clipboard.copy(secret("hunter2"), SHORT).unwrap();
        assert_eq!(fake.contents().as_deref(), Some("hunter2"));
        eventually(|| fake.contents().is_none());
    }

    #[test]
    fn something_copied_afterwards_is_left_alone() {
        let fake = FakeClipboard::default();
        let clipboard = manager(&fake);
        clipboard.copy(secret("hunter2"), SHORT).unwrap();
        fake.user_copies("shopping list");
        thread::sleep(SHORT * 4);
        assert_eq!(fake.contents().as_deref(), Some("shopping list"));
    }

    #[test]
    fn a_new_copy_replaces_the_schedule() {
        let fake = FakeClipboard::default();
        let clipboard = manager(&fake);
        clipboard.copy(secret("first"), SHORT).unwrap();
        clipboard.copy(secret("second"), LONG).unwrap();
        thread::sleep(SHORT * 4);
        assert_eq!(fake.contents().as_deref(), Some("second"));
        // Locking still clears it at once.
        clipboard.clear();
        eventually(|| fake.contents().is_none());
    }

    #[test]
    fn clear_on_lock_only_removes_our_value() {
        let fake = FakeClipboard::default();
        let clipboard = manager(&fake);
        clipboard.copy(secret("hunter2"), LONG).unwrap();
        fake.user_copies("mine");
        clipboard.clear();
        clipboard.shutdown();
        assert_eq!(fake.contents().as_deref(), Some("mine"));
    }

    #[test]
    fn shutdown_clears_and_stops() {
        let fake = FakeClipboard::default();
        let clipboard = manager(&fake);
        clipboard.copy(secret("hunter2"), LONG).unwrap();
        clipboard.shutdown();
        assert_eq!(fake.contents(), None);
        assert_eq!(
            clipboard.copy(secret("x"), LONG),
            Err(CommandError::Internal)
        );
        // A second shutdown is harmless.
        clipboard.shutdown();
    }

    #[test]
    fn write_failures_are_reported_and_retried() {
        let fake = FakeClipboard::default();
        let clipboard = manager(&fake);
        *fake.fail_writes.lock().unwrap() = true;
        assert_eq!(
            clipboard.copy(secret("hunter2"), LONG),
            Err(CommandError::ClipboardUnavailable)
        );
        *fake.fail_writes.lock().unwrap() = false;
        clipboard.copy(secret("hunter2"), LONG).unwrap();
        assert_eq!(fake.contents().as_deref(), Some("hunter2"));
    }

    #[test]
    fn a_missing_clipboard_is_reported() {
        let clipboard = ClipboardManager::start(|| Err::<FakeClipboard, _>(BackendError));
        assert_eq!(
            clipboard.copy(secret("hunter2"), LONG),
            Err(CommandError::ClipboardUnavailable)
        );
    }
}
