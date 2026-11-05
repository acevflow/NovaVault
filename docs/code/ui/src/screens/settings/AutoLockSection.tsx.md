# [`ui/src/screens/settings/AutoLockSection.tsx`](../../../../../../ui/src/screens/settings/AutoLockSection.tsx)

## Purpose

The Auto-lock card of the settings screen: it sets how many minutes the vault may stay unlocked without activity before the shell locks it. It loads the current value with `get_settings` and saves with `set_auto_lock_timeout`.

## Public API

| Name                   | Kind      | Signature | Description |
| ---------------------- | --------- | --------- | ----------- |
| `AutoLockSectionProps` | interface | `export interface AutoLockSectionProps { readonly onLocked: () => void; }` | Props of `AutoLockSection`. |
| `AutoLockSection`      | component | `function AutoLockSection({ onLocked }: AutoLockSectionProps)` | Inactivity timeout picker. |

Internal: `AUTO_LOCK_CHOICES` (`[1, 5, 15, 30, 60]`) and `minutesLabel(minutes)` ("1 minute", "5 minutes", …, "1 hour" for 60).

## Details

### `AutoLockSectionProps`

- `onLocked`: called if saving reports the vault is locked.

### `AutoLockSection`

- **State**: `saved` (value stored in the shell, `null` until loaded), `selected` (value in the select), `busy`, `error`, `done` (show the success message).
- **Load effect** (mount only): `getSettings()` sets both `saved` and `selected` to `autoLockMinutes`. A failure sets `error`. Results after unmount are dropped.
- **Choices**: `AUTO_LOCK_CHOICES`. If the saved value is not among them, it is added and the list sorted, so a value set elsewhere stays selectable.
- **Rendered states**, inside `<section className="card">` with `<h2>Auto-lock</h2>` and the note "The vault also locks when the computer sleeps or the screen locks.":
  - **Loading**: `<p role="status">Loading settings…</p>`.
  - **Load failed**: an error [`Alert`](../../components/Alert.tsx.md), and no form.
  - **Loaded**: a form with the labelled select "Lock after inactivity" (disabled while busy), an error `Alert` if any, the success message "The vault now locks after <label> without activity." after a save, and a primary Save [`Button`](../../components/Button.tsx.md).
- **Save**: on submit clears `error` and `done`, then calls `setAutoLockTimeout(selected)`. Success updates `saved` and sets `done`. `locked` calls `onLocked()`. Other errors show `errorMessage`.
- **Button state**: Save is disabled while the selection equals the saved value, and busy ("Saving…") while saving. Changing the selection hides the success message.
- **Accessibility**: a native labelled `<select>`; loading and success messages are polite status messages, errors are alerts. The card is labelled by its heading.
- **Tauri calls**:
  - `getSettings` → `get_settings` on mount;
  - `setAutoLockTimeout` → `set_auto_lock_timeout` with `{ minutes }`. The shell accepts 1 to 60 ([`settings.rs`](../../../../src-tauri/src/settings.rs.md)).
  See [`commands.rs`](../../../../src-tauri/src/commands.rs.md).
- **Security**: settings are not secret. The timer itself runs in the shell ([`autolock.rs`](../../../../src-tauri/src/autolock.rs.md)).

## Dependencies

- **Internal**: [`api/commands.ts`](../../../../../../ui/src/api/commands.ts) (`getSettings`, `setAutoLockTimeout`), [`api/errors.ts`](../../../../../../ui/src/api/errors.ts), [`Alert`](../../components/Alert.tsx.md), [`Button`](../../components/Button.tsx.md).
- **External**: `react`: `useEffect`, `useId`, `useState`, the `SubmitEvent` type.

## Used by

- [`SettingsScreen`](SettingsScreen.tsx.md).

## Tests

- [`SettingsScreen.test.tsx`](../../../../../../ui/src/screens/settings/SettingsScreen.test.tsx):
  - `loads and saves the auto-lock timeout`: shows the loaded `5`, saves `15` with `{ minutes: 15 }` and announces "15 minutes".
  - `returns to the unlock screen if the vault is locked`: a `locked` save error calls `onLocked` once.

## Notes

- The note "The vault also locks when the computer sleeps or the screen locks" depends on the platform monitor. On unsupported targets ([`os_lock/unsupported.rs`](../../../../src-tauri/src/os_lock/unsupported.rs.md)) only the inactivity timer applies, and on Linux the lock before suspend is not guaranteed ([`os_lock/linux.rs`](../../../../src-tauri/src/os_lock/linux.rs.md)).
- A `locked` error from the initial `get_settings` is shown as an error, not routed to `onLocked`; per [`commands.ts`](../../../../../../ui/src/api/commands.ts), settings are readable while locked.
- The code is nearly identical to [`ClipboardSection`](ClipboardSection.tsx.md); the two are not factored into a shared component.
