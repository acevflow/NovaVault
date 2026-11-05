# [`ui/src/screens/settings/ClipboardSection.tsx`](../../../../../../ui/src/screens/settings/ClipboardSection.tsx)

## Purpose

The Clipboard card of the settings screen: it sets how many seconds a copied field stays on the clipboard before the shell clears it. It loads the current value with `get_settings` and saves with `set_clipboard_clear_delay`.

## Public API

| Name                    | Kind      | Signature | Description |
| ----------------------- | --------- | --------- | ----------- |
| `ClipboardSectionProps` | interface | `export interface ClipboardSectionProps { readonly onLocked: () => void; }` | Props of `ClipboardSection`. |
| `ClipboardSection`      | component | `function ClipboardSection({ onLocked }: ClipboardSectionProps)` | Clipboard clear delay picker. |

Internal: `CLEAR_CHOICES` (`[10, 30, 60, 120]` seconds).

## Details

### `ClipboardSectionProps`

- `onLocked`: called if saving reports the vault is locked.

### `ClipboardSection`

- **State**: `saved`, `selected`, `busy`, `error`, `done`, with the same meaning as in [`AutoLockSection`](AutoLockSection.tsx.md).
- **Load effect** (mount only): `getSettings()` sets `saved` and `selected` to `clipboardClearSeconds`. A failure sets `error`.
- **Choices**: `CLEAR_CHOICES`, plus the saved value (sorted in) if it is not one of them. Labels come from [`formatSeconds`](../../../../../../ui/src/format/duration.ts): "10 seconds", "30 seconds", "1 minute", "2 minutes".
- **Rendered states**, inside `<section className="card">` with `<h2>Clipboard</h2>` and the explanation "Copied passwords and other fields are removed from the clipboard after this delay, and when the vault locks, unless you have copied something else since.":
  - **Loading**: `<p role="status">Loading settings…</p>`.
  - **Load failed**: an error [`Alert`](../../components/Alert.tsx.md), and no form.
  - **Loaded**: a form with the labelled select "Clear the clipboard after" (disabled while busy), an error `Alert` if any, the success message "Copied fields are now cleared after <duration>." after a save, and a primary Save [`Button`](../../components/Button.tsx.md), disabled while unchanged and busy while saving.
- **Save**: as in `AutoLockSection`, with `setClipboardClearDelay(selected)`.
- **Accessibility**: a native labelled select; polite status messages for loading and success, alerts for errors.
- **Tauri calls**:
  - `getSettings` → `get_settings` on mount;
  - `setClipboardClearDelay` → `set_clipboard_clear_delay` with `{ seconds }`. The shell accepts 5 to 300 ([`settings.rs`](../../../../src-tauri/src/settings.rs.md)).
  See [`commands.rs`](../../../../src-tauri/src/commands.rs.md).
- **Security**: the delay is not secret. Clearing, including the "unless you have copied something else since" check, is done by the shell ([`clipboard.rs`](../../../../src-tauri/src/clipboard.rs.md)).

## Dependencies

- **Internal**: [`api/commands.ts`](../../../../../../ui/src/api/commands.ts) (`getSettings`, `setClipboardClearDelay`), [`api/errors.ts`](../../../../../../ui/src/api/errors.ts), [`Alert`](../../components/Alert.tsx.md), [`Button`](../../components/Button.tsx.md), [`format/duration.ts`](../../../../../../ui/src/format/duration.ts).
- **External**: `react`: `useEffect`, `useId`, `useState`, the `SubmitEvent` type.

## Used by

- [`SettingsScreen`](SettingsScreen.tsx.md).

## Tests

- [`SettingsScreen.test.tsx`](../../../../../../ui/src/screens/settings/SettingsScreen.test.tsx): `loads and saves the clipboard clear delay`: shows the loaded `30`, saves "1 minute" as `{ seconds: 60 }` and announces "after 1 minute".

## Notes

- The code is nearly identical to [`AutoLockSection`](AutoLockSection.tsx.md), including the handling of a `locked` error during the initial load (shown as an error, not routed to `onLocked`).
- The UI offers 10 to 120 seconds, while the shell accepts 5 to 300; values outside the list can only come from an existing settings file.
