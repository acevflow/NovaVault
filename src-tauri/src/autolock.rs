//! Automatic locking: on inactivity, and on the OS events reported by
//! `os_lock`.
//!
//! Whatever the trigger, [`lock_and_reset_ui`] locks the session (wiping the
//! vault key) and then reloads the main webview. The reload discards
//! everything the UI holds in memory (item titles, form contents) and brings
//! it back to its start screen, without the UI having to listen for events,
//! so no event permission has to be granted to the webview.
//!
//! Inactivity is measured from the last command (see `Session::touch`).
//! The idle clock is monotonic; on platforms where it pauses during sleep,
//! the suspend monitor covers that case by locking before the system sleeps.

use std::thread;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

use crate::commands::AppState;

/// How long the vault may stay unlocked without any command.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// How often the idle timer is checked; bounds how late the lock can be.
const CHECK_INTERVAL: Duration = Duration::from_secs(5);

/// Why the vault was locked automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockReason {
    /// No command for [`IDLE_TIMEOUT`].
    Inactivity,
    /// The system is about to suspend or hibernate.
    Suspend,
    /// The screen was locked or the user session was switched away.
    ScreenLock,
}

impl LockReason {
    /// Human-readable name, for diagnostics.
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Inactivity => "inactivity",
            Self::Suspend => "system suspend",
            Self::ScreenLock => "screen lock",
        }
    }
}

/// Locks the vault for `reason` and, if it was unlocked, resets the UI.
///
/// For [`LockReason::Inactivity`] the session locks only if it has really
/// been idle for [`IDLE_TIMEOUT`]; OS events lock unconditionally.
pub fn lock_and_reset_ui(app: &AppHandle, reason: LockReason) {
    let locked = app
        .state::<AppState>()
        .with_session(|session| match reason {
            LockReason::Inactivity => session.lock_if_idle(Instant::now(), IDLE_TIMEOUT),
            LockReason::Suspend | LockReason::ScreenLock => session.lock(),
        });
    if !locked {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        if window.reload().is_err() {
            // The key is already wiped; only the UI's stale view remains.
            eprintln!(
                "NovaVault: vault locked ({}), UI reset failed",
                reason.describe()
            );
        }
    }
}

/// Starts the background thread that enforces [`IDLE_TIMEOUT`].
///
/// A failure to start is reported on stderr and is not fatal: the vault
/// still locks on user request and on OS events.
pub fn start_idle_timer(app: AppHandle) {
    let spawned = thread::Builder::new()
        .name("novavault-idle-lock".into())
        .spawn(move || {
            loop {
                thread::sleep(CHECK_INTERVAL);
                lock_and_reset_ui(&app, LockReason::Inactivity);
            }
        });
    if spawned.is_err() {
        eprintln!("NovaVault: could not start the inactivity auto-lock");
    }
}
