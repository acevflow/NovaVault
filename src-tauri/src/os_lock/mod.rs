//! Operating-system lock triggers: system suspend and screen lock.
//!
//! Each platform reports these events through its own API, so there is one
//! [`LockMonitor`] implementation per OS behind a common trait, chosen at
//! compile time by [`platform_monitor`]:
//!
//! | OS      | Suspend                               | Screen lock                                                   |
//! | ------- | ------------------------------------- | ------------------------------------------------------------- |
//! | Windows | `WM_POWERBROADCAST` / `PBT_APMSUSPEND` | `WM_WTSSESSION_CHANGE` / `WTS_SESSION_LOCK`                   |
//! | macOS   | `NSWorkspaceWillSleepNotification`    | `com.apple.screenIsLocked`, `NSWorkspaceSessionDidResignActiveNotification` |
//! | Linux   | logind `PrepareForSleep(true)`        | logind session `Lock`, `org.freedesktop.ScreenSaver` / `org.gnome.ScreenSaver` `ActiveChanged(true)` |
//!
//! Monitors only report events; the sink decides what to do (lock the
//! vault and reset the UI, see `autolock`). If a monitor cannot start, the
//! inactivity timer and manual locking keep working.
//!
//! Windows and macOS have no safe bindings for these notifications, so
//! their modules are the only places in NovaVault allowed to use `unsafe`.
//! Each `unsafe` block is a single FFI call with its safety argument next
//! to it. The Linux monitor uses D-Bus through safe Rust.

use std::sync::Arc;

use thiserror::Error;

use crate::autolock::LockReason;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
mod unsupported;
#[cfg(target_os = "windows")]
mod windows;

/// Receives lock events. Called from whichever thread the platform
/// delivers events on, so it must be cheap and thread-safe.
pub type LockSink = Arc<dyn Fn(LockReason) + Send + Sync>;

/// A source of OS lock events.
pub trait LockMonitor {
    /// Starts delivering suspend and screen-lock events to `sink` for the
    /// rest of the process lifetime.
    ///
    /// # Errors
    ///
    /// [`MonitorError`] if the platform API could not be set up. Some
    /// failures may only be detected (and reported on stderr) after this
    /// returns, for example when a D-Bus service is missing.
    fn start(&self, sink: LockSink) -> Result<(), MonitorError>;
}

/// Why a [`LockMonitor`] could not start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MonitorError {
    /// The monitor was already started; platform hooks are process-global.
    #[error("the OS lock monitor is already running")]
    AlreadyStarted,

    /// The platform API failed; the text names the step, never data.
    #[error("OS lock events are unavailable: {0}")]
    Unavailable(&'static str),
}

/// The monitor for the platform this build targets.
pub fn platform_monitor() -> Box<dyn LockMonitor> {
    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsMonitor);
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacMonitor);
    #[cfg(target_os = "linux")]
    return Box::new(linux::LinuxMonitor);
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return Box::new(unsupported::UnsupportedMonitor);
}
