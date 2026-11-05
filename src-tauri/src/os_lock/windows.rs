//! Windows: session and power notifications through a hidden window.
//!
//! Power broadcasts (`WM_POWERBROADCAST`) are delivered only to top-level
//! windows, and session changes (`WM_WTSSESSION_CHANGE`) to windows
//! registered with `WTSRegisterSessionNotification`. A dedicated thread
//! creates one invisible top-level window for both and runs its message
//! loop, so the app's own windows and event loop are left untouched.

// Win32 has no safe API for window classes, window procedures or message
// loops. Every `unsafe` block below is one FFI call with its justification.
#![allow(unsafe_code)]

use std::sync::{OnceLock, mpsc};
use std::thread;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, PBT_APMSUSPEND,
    RegisterClassW, WINDOW_EX_STYLE, WM_POWERBROADCAST, WM_WTSSESSION_CHANGE, WNDCLASSW,
    WS_OVERLAPPED, WTS_SESSION_LOCK,
};
use windows::core::{PCWSTR, w};

use super::{LockMonitor, LockSink, MonitorError};
use crate::autolock::LockReason;

/// The sink, reachable from the window procedure (a plain `extern` fn that
/// cannot capture state). Set once, by [`WindowsMonitor::start`].
static SINK: OnceLock<LockSink> = OnceLock::new();

/// Window class of the hidden listener window.
const CLASS_NAME: PCWSTR = w!("NovaVaultLockListener");

/// Listens for suspend and screen-lock events on Windows.
pub struct WindowsMonitor;

impl LockMonitor for WindowsMonitor {
    /// Spawns the listener thread and waits until its hidden window exists
    /// and is subscribed, so setup failures are reported here rather than
    /// lost on the thread.
    fn start(&self, sink: LockSink) -> Result<(), MonitorError> {
        SINK.set(sink).map_err(|_| MonitorError::AlreadyStarted)?;
        let (ready, started) = mpsc::channel();
        thread::Builder::new()
            .name("novavault-os-lock".into())
            .spawn(move || {
                // The window belongs to this thread, so the message loop
                // must run here too.
                let created = create_listener_window();
                let ok = created.is_ok();
                // The receiver only disappears if `start` already returned.
                let _ = ready.send(created);
                if ok {
                    run_message_loop();
                }
            })
            .map_err(|_| MonitorError::Unavailable("could not start the listener thread"))?;
        started
            .recv()
            .map_err(|_| MonitorError::Unavailable("the listener thread stopped"))?
    }
}

/// Maps a window message to the lock it calls for, if any.
fn reason_for(message: u32, wparam: WPARAM) -> Option<LockReason> {
    // Both event codes are small integers passed in `wparam`.
    let event = u32::try_from(wparam.0).ok()?;
    match (message, event) {
        (WM_POWERBROADCAST, PBT_APMSUSPEND) => Some(LockReason::Suspend),
        (WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK) => Some(LockReason::ScreenLock),
        _ => None,
    }
}

/// Window procedure of the listener window.
extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if let (Some(reason), Some(sink)) = (reason_for(message, wparam), SINK.get()) {
        // Runs synchronously: Windows waits for this handler before it
        // suspends, so the vault is locked before the system sleeps.
        sink(reason);
    }
    // SAFETY: forwards the unmodified arguments Windows passed to this
    // window procedure, which is what default processing expects.
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

/// Registers the window class, creates the hidden window and subscribes
/// it to session notifications.
fn create_listener_window() -> Result<(), MonitorError> {
    const NO_WINDOW: MonitorError =
        MonitorError::Unavailable("could not create the listener window");

    // SAFETY: a null module name asks for the current executable's handle;
    // no pointer is dereferenced.
    let module = unsafe { GetModuleHandleW(PCWSTR::null()) }.map_err(|_| NO_WINDOW)?;
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: module.into(),
        lpszClassName: CLASS_NAME,
        ..WNDCLASSW::default()
    };
    // SAFETY: `class` is fully initialized, its window procedure has the
    // required signature and its class name is a static string.
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err(NO_WINDOW);
    }
    // SAFETY: the class was registered above; the names are static strings
    // and every optional handle or pointer is absent. `WS_OVERLAPPED`
    // without `WS_VISIBLE` creates a top-level window that is never shown.
    let window = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            CLASS_NAME,
            w!("NovaVault"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(module.into()),
            None,
        )
    }
    .map_err(|_| NO_WINDOW)?;
    // SAFETY: `window` is a valid window created on this thread, and it
    // stays alive for the rest of the process.
    unsafe { WTSRegisterSessionNotification(window, NOTIFY_FOR_THIS_SESSION) }
        .map_err(|_| MonitorError::Unavailable("could not subscribe to session changes"))?;
    Ok(())
}

/// Dispatches messages to the listener window until the process exits.
fn run_message_loop() {
    let mut message = MSG::default();
    // SAFETY: `message` is a valid, writable `MSG`. `GetMessageW` returns 0
    // for `WM_QUIT` and -1 on error; both end the loop.
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        // SAFETY: `message` was just filled in by `GetMessageW`.
        unsafe { DispatchMessageW(&message) };
    }
}

#[cfg(test)]
mod tests {
    use windows::Win32::UI::WindowsAndMessaging::{PBT_APMRESUMEAUTOMATIC, WTS_SESSION_UNLOCK};

    use super::*;

    fn wparam(value: u32) -> WPARAM {
        WPARAM(value as usize)
    }

    #[test]
    fn maps_suspend_and_lock_messages() {
        assert_eq!(
            reason_for(WM_POWERBROADCAST, wparam(PBT_APMSUSPEND)),
            Some(LockReason::Suspend)
        );
        assert_eq!(
            reason_for(WM_WTSSESSION_CHANGE, wparam(WTS_SESSION_LOCK)),
            Some(LockReason::ScreenLock)
        );
        assert_eq!(
            reason_for(WM_POWERBROADCAST, wparam(PBT_APMRESUMEAUTOMATIC)),
            None
        );
        assert_eq!(
            reason_for(WM_WTSSESSION_CHANGE, wparam(WTS_SESSION_UNLOCK)),
            None
        );
        // Same event code on an unrelated message.
        assert_eq!(reason_for(0, wparam(WTS_SESSION_LOCK)), None);
    }
}
