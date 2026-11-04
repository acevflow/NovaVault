//! Linux: systemd-logind and screen saver signals over D-Bus.
//!
//! - System bus, `org.freedesktop.login1.Manager.PrepareForSleep(true)`:
//!   the system is about to suspend or hibernate.
//! - System bus, `org.freedesktop.login1.Session.Lock` on the app's own
//!   session: the session is being locked (`loginctl lock-session`, most
//!   desktop environments).
//! - Session bus, `ActiveChanged(true)` on `org.freedesktop.ScreenSaver`
//!   (KDE and others) and `org.gnome.ScreenSaver` (GNOME): the screen
//!   saver or lock screen became active.
//!
//! Every source is optional: a system without logind or without a screen
//! saver service simply provides fewer triggers. Failures are reported on
//! stderr with a fixed message. Unlike Windows and macOS, this uses only
//! safe Rust.

use futures_util::StreamExt;
use zbus::Connection;

use super::{LockMonitor, LockSink, MonitorError};
use crate::autolock::LockReason;

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1",
    gen_blocking = false
)]
trait LoginManager {
    /// Object path of the session called `id` (`"auto"`: the caller's).
    fn get_session(&self, id: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// Emitted with `true` before sleep and `false` after resume.
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    gen_blocking = false
)]
trait LoginSession {
    /// Emitted when the session should lock its screen.
    #[zbus(signal)]
    fn lock(&self) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.ScreenSaver",
    default_service = "org.freedesktop.ScreenSaver",
    default_path = "/org/freedesktop/ScreenSaver",
    gen_blocking = false
)]
trait FreedesktopScreenSaver {
    /// Emitted with `true` when the screen saver or lock screen activates.
    #[zbus(signal)]
    fn active_changed(&self, active: bool) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.gnome.ScreenSaver",
    default_service = "org.gnome.ScreenSaver",
    default_path = "/org/gnome/ScreenSaver",
    gen_blocking = false
)]
trait GnomeScreenSaver {
    /// Emitted with `true` when the GNOME lock screen activates.
    #[zbus(signal)]
    fn active_changed(&self, active: bool) -> zbus::Result<()>;
}

/// Listens for sleep and screen-lock events on Linux.
pub struct LinuxMonitor;

impl LockMonitor for LinuxMonitor {
    fn start(&self, sink: LockSink) -> Result<(), MonitorError> {
        // Connecting is asynchronous; each source runs as its own task on
        // Tauri's runtime and reports its own failure.
        let system = sink.clone();
        tauri::async_runtime::spawn(async move {
            if watch_logind(&system).await.is_err() {
                eprintln!("NovaVault: logind is unavailable; no lock on suspend or session lock");
            }
        });
        tauri::async_runtime::spawn(async move {
            if watch_screen_savers(&sink).await.is_err() {
                eprintln!("NovaVault: the session bus is unavailable; no lock on screen saver");
            }
        });
        Ok(())
    }
}

/// Forwards logind's sleep and session-lock signals.
async fn watch_logind(sink: &LockSink) -> zbus::Result<()> {
    let connection = Connection::system().await?;
    let manager = LoginManagerProxy::new(&connection).await?;
    let session_path = manager.get_session("auto").await?;
    let session = LoginSessionProxy::builder(&connection)
        .path(session_path)?
        .build()
        .await?;

    let sleeps = manager.receive_prepare_for_sleep().await?.map(
        |signal| -> zbus::Result<Option<LockReason>> {
            Ok(signal.args()?.start().then_some(LockReason::Suspend))
        },
    );
    let locks = session
        .receive_lock()
        .await?
        .map(|_| -> zbus::Result<Option<LockReason>> { Ok(Some(LockReason::ScreenLock)) });
    let mut events = futures_util::stream::select(sleeps, locks);
    while let Some(event) = events.next().await {
        if let Some(reason) = event? {
            sink(reason);
        }
    }
    Err(zbus::Error::Failure("logind signal streams ended".into()))
}

/// Forwards `ActiveChanged(true)` from whichever screen saver services
/// exist on the session bus.
async fn watch_screen_savers(sink: &LockSink) -> zbus::Result<()> {
    let connection = Connection::session().await?;
    let freedesktop = FreedesktopScreenSaverProxy::new(&connection).await?;
    let gnome = GnomeScreenSaverProxy::new(&connection).await?;
    // Subscribing works even if a service is not running: the match rule
    // is registered with the bus, and signals arrive once it starts.
    let freedesktop = freedesktop
        .receive_active_changed()
        .await?
        .map(|signal| signal.args().map(|args| *args.active()));
    let gnome = gnome
        .receive_active_changed()
        .await?
        .map(|signal| signal.args().map(|args| *args.active()));
    let mut changes = futures_util::stream::select(freedesktop, gnome);
    while let Some(active) = changes.next().await {
        if active? {
            sink(LockReason::ScreenLock);
        }
    }
    Ok(())
}
