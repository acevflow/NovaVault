# Clipboard behaviour

NovaVault can copy any field of an item (user name, password, URL, notes, note text, secret value) to the system clipboard. This document describes what happens on each operating system, and what NovaVault cannot control.

## How copying works

- **Copy happens in Rust.** The UI calls `copy_item_field` with an item identifier and a field name. The shell decrypts the item, copies the field to the clipboard itself and returns only the delay before it is cleared (`{ clearAfterSeconds }`). The value never passes through the webview, even for secret fields that are not revealed.
- **One clipboard owner.** A single worker thread (`src-tauri/src/clipboard.rs`) owns the clipboard handle for the lifetime of the app, using the [`arboard`](https://crates.io/crates/arboard) crate. The handle is opened on first use, so the app starts even where no clipboard is available, and is reopened after a failure.
- **Automatic clearing.** The clipboard is cleared:
  - after the configured delay (Settings › Clipboard: 10 seconds, 30 seconds, 1 minute or 2 minutes; the shell accepts 5 to 300 seconds, default 30);
  - when the vault locks, whether by the Lock button, inactivity, system suspend or screen lock;
  - when the app exits normally.
- **Only our value is cleared.** Before clearing, NovaVault reads the clipboard and clears it only if it still holds the value it copied, so something the user copied afterwards is left alone. If the clipboard cannot be read, it is cleared anyway: losing an unrelated clipboard entry is better than leaving a secret behind.
- **A new copy replaces the previous one.** Only the most recent copy is tracked, and its timer starts again.
- **History opt-outs.** Each copy carries the platform's "do not record this" marker (below). These are conventions that well-behaved clipboard tools honour, not guarantees.
- **Failures.** If the clipboard cannot be written, the command fails with `clipboard_unavailable` and the UI says so. Nothing is retried in the background.

## Windows

- The value is placed in the standard clipboard together with two of the [clipboard formats Windows defines for this purpose](https://learn.microsoft.com/windows/win32/dataxchg/clipboard-formats#cloud-clipboard-and-clipboard-history-formats): `CanIncludeInClipboardHistory = 0` keeps it out of Windows clipboard history (Win+V), and `CanUploadToCloudClipboard = 0` keeps it out of cloud clipboard sync across devices.
- The Windows clipboard keeps its contents after the copying app exits, so clearing on exit matters. If NovaVault is killed (Task Manager, crash, power loss) the value stays on the clipboard until something else replaces it.
- Third-party clipboard managers that ignore these formats may still record the value.

## macOS

- The value is placed on the general pasteboard together with the `org.nspasteboard.ConcealedType` marker, the convention clipboard history tools (for example Alfred, Raycast, Maccy, Paste) use to skip passwords.
- The general pasteboard keeps its contents after the app exits. As on Windows, a killed app cannot clear it.
- Universal Clipboard (Handoff) may offer the pasteboard to the user's other Apple devices nearby. NovaVault cannot opt out of it per item; users who copy secrets often may want to turn Handoff off.
- Any app running as the user can read the general pasteboard. Recent macOS versions can show a notice when an app reads it, but do not prevent the read.

## Linux

On Linux there is no single clipboard service: the clipboard belongs to the display server session, and the **app that copied a value must keep running to serve it** to whoever pastes. NovaVault therefore keeps its clipboard handle open on a dedicated thread for as long as it runs.

NovaVault writes only the `CLIPBOARD` selection (Ctrl+C / Ctrl+V). It never sets the `PRIMARY` selection (middle-click paste), so selecting text elsewhere is unaffected and secrets do not end up in `PRIMARY`.

Each copy carries the `x-kde-passwordManagerHint: secret` MIME type, which KDE Klipper and several other clipboard managers honour by not recording the value.

### X11

- NovaVault owns the `CLIPBOARD` selection and answers paste requests from its worker thread.
- Clearing removes ownership, so nothing is left to paste.
- **On exit, NovaVault clears first and then releases the clipboard.** If it released a value instead, an X11 clipboard manager (one that implements the `CLIPBOARD_MANAGER` protocol) could be asked to keep a copy after NovaVault quits; clearing first avoids handing it the secret.
- If NovaVault is killed, it can no longer serve the value, so the value usually disappears with it. A clipboard manager that copied it while NovaVault was running may still hold it.
- Any X11 client in the session can read the selection; X11 has no isolation between apps.

### Wayland

- NovaVault uses the **data-control** protocol (`ext-data-control-v1` or the older `wlr-data-control-unstable-v1`) when `WAYLAND_DISPLAY` is set and the compositor supports it. wlroots-based compositors (Sway, Hyprland and others) and KDE Plasma support it; GNOME (Mutter) does not in many releases still in use. With data-control, copying and clearing work immediately, even when NovaVault's window does not have focus.
- If the compositor has no data-control protocol, NovaVault falls back to the X11 clipboard through **XWayland**. XWayland passes the clipboard between X11 and Wayland apps only when an X11 window has keyboard focus. In that case:
  - a copied value may become available to native Wayland apps only once NovaVault's window has been focused (it normally is, because the user just clicked Copy);
  - **a timed clear may not reach native Wayland apps until NovaVault's window is focused again.** Until then, they may still be able to paste the old value. This is a known residual risk (see the [threat model](threat-model.md#auto-lock-and-clipboard)).
- As on X11, the app that copied a value serves it, so a killed NovaVault usually takes the value with it. Clipboard managers that recorded it earlier may still hold it.
- Flatpak or other sandboxes may block data-control access; NovaVault then uses the XWayland fallback above, or reports `clipboard_unavailable` if neither works.

## What NovaVault cannot protect against

- Malware or any app running as the user can read the clipboard while a value is on it. Keep the clear delay short.
- Clipboard managers and sync tools that ignore the opt-out markers may keep their own copies.
- A crash, forced termination or power loss skips the clear on exit (on Windows and macOS the value then stays on the clipboard).
- Pasting puts the value into the target app, which may keep it in its own undo history or autosave.
