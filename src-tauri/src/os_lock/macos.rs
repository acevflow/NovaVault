//! macOS: AppKit workspace and distributed notifications.
//!
//! - `NSWorkspaceWillSleepNotification` is posted on the workspace's
//!   notification center before the system sleeps.
//! - `NSWorkspaceSessionDidResignActiveNotification` is posted when the
//!   user switches to another login session (fast user switching).
//! - `com.apple.screenIsLocked` is posted on the distributed notification
//!   center when the screen locks. It is not formally documented, but is
//!   the long-standing signal used by macOS apps for this purpose.
//!
//! Observers are registered from the main thread (the Tauri setup hook),
//! whose run loop delivers the notifications.

// objc2 exposes these AppKit statics and the block-based observer API only
// as `unsafe`. Every `unsafe` block below is one call with its argument.
#![allow(unsafe_code)]

use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use block2::RcBlock;
use objc2_app_kit::{
    NSWorkspace, NSWorkspaceSessionDidResignActiveNotification, NSWorkspaceWillSleepNotification,
};
use objc2_foundation::{
    NSDistributedNotificationCenter, NSNotification, NSNotificationCenter, NSString,
};

use super::{LockMonitor, LockSink, MonitorError};
use crate::autolock::LockReason;

/// Set once the observers are registered; they are process-global.
static STARTED: AtomicBool = AtomicBool::new(false);

/// Name of the distributed notification posted when the screen locks.
const SCREEN_LOCKED: &str = "com.apple.screenIsLocked";

/// Listens for sleep and screen-lock events on macOS.
pub struct MacMonitor;

impl LockMonitor for MacMonitor {
    /// Registers the three observers and returns. Must be called on the
    /// main thread, whose run loop delivers the notifications.
    fn start(&self, sink: LockSink) -> Result<(), MonitorError> {
        if STARTED.swap(true, Ordering::SeqCst) {
            return Err(MonitorError::AlreadyStarted);
        }
        let workspace = NSWorkspace::sharedWorkspace().notificationCenter();
        // SAFETY: these are immutable `NSString` constants exported by
        // AppKit, valid for the whole process lifetime.
        let (will_sleep, resign_active) = unsafe {
            (
                NSWorkspaceWillSleepNotification,
                NSWorkspaceSessionDidResignActiveNotification,
            )
        };
        observe(&workspace, will_sleep, LockReason::Suspend, &sink);
        observe(&workspace, resign_active, LockReason::ScreenLock, &sink);

        let distributed = NSDistributedNotificationCenter::defaultCenter();
        let screen_locked = NSString::from_str(SCREEN_LOCKED);
        observe(&distributed, &screen_locked, LockReason::ScreenLock, &sink);
        Ok(())
    }
}

/// Calls `sink(reason)` whenever `center` posts a notification `name`.
fn observe(center: &NSNotificationCenter, name: &NSString, reason: LockReason, sink: &LockSink) {
    let sink = Arc::clone(sink);
    let block = RcBlock::new(move |_notification: NonNull<NSNotification>| sink(reason));
    // SAFETY: no sender filter is given, so there is no object whose type
    // could be wrong. With no queue, the block runs synchronously on the
    // posting thread (the main thread here); it captures only a
    // `Send + Sync` sink, so it is safe to call from any thread.
    let observer = unsafe {
        center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &block)
    };
    // The observer must stay registered until the process exits, so its
    // token is intentionally never released.
    core::mem::forget(observer);
}
