//! Platforms without a dedicated monitor: no OS lock events.
//!
//! NovaVault targets Windows, macOS and Linux; this keeps other targets
//! compiling, with only the inactivity timer and manual locking.

use super::{LockMonitor, LockSink, MonitorError};

/// A monitor that never reports events.
pub struct UnsupportedMonitor;

impl LockMonitor for UnsupportedMonitor {
    /// Always fails, so the app reports that OS lock events are missing.
    fn start(&self, _sink: LockSink) -> Result<(), MonitorError> {
        Err(MonitorError::Unavailable("not supported on this platform"))
    }
}
