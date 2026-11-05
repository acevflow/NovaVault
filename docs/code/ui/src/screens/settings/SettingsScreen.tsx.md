# [`ui/src/screens/settings/SettingsScreen.tsx`](../../../../../../ui/src/screens/settings/SettingsScreen.tsx)

## Purpose

The settings screen of an unlocked vault (route `settings`). It is a plain container: a focused heading followed by five independent sections, for the master password, the recovery code, auto-lock, the clipboard and appearance. Each section loads and saves its own data.

## Public API

| Name                  | Kind      | Signature | Description |
| --------------------- | --------- | --------- | ----------- |
| `SettingsScreenProps` | interface | `export interface SettingsScreenProps { readonly onLocked: () => void; }` | Props of `SettingsScreen`. |
| `SettingsScreen`      | component | `function SettingsScreen({ onLocked }: SettingsScreenProps)` | Heading and the five settings sections. |

## Details

### `SettingsScreenProps`

- `onLocked`: called when a command reports the vault is locked. Passed to every section that calls the shell.

### `SettingsScreen`

- **State**: none.
- **Rendered output**: `<div className="screen">` with the [`ScreenHeading`](../../components/ScreenHeading.tsx.md) "Settings" (level 1, takes focus), then, in order:
  1. [`ChangePasswordSection`](ChangePasswordSection.tsx.md)
  2. [`RecoveryCodeSection`](RecoveryCodeSection.tsx.md)
  3. [`AutoLockSection`](AutoLockSection.tsx.md)
  4. [`ClipboardSection`](ClipboardSection.tsx.md)
  5. [`AppearanceSection`](AppearanceSection.tsx.md) (no `onLocked`; it does not call the shell)
- **User flows**: reached from the "Settings" navigation entry in [`AppShell`](../../app/AppShell.tsx.md). Each section has its own form and messages.
- **Accessibility**: the `h1` takes focus on arrival; each section is a labelled card with an `h2`.
- **Tauri calls**: none directly. The sections call `change_master_password`, `regenerate_recovery_code`, `get_settings` (twice, once each for auto-lock and clipboard), `set_auto_lock_timeout` and `set_clipboard_clear_delay`.

## Dependencies

- **Internal**: [`ScreenHeading`](../../components/ScreenHeading.tsx.md) and the five section components.
- **External**: none.

## Used by

- [`App.tsx`](../../App.tsx.md): for the `settings` route.

## Tests

- [`SettingsScreen.test.tsx`](../../../../../../ui/src/screens/settings/SettingsScreen.test.tsx), across all sections: `changes the password after re-authentication`, `reports a wrong current password`, `accepts the recovery code as the current credential`, `regenerates and confirms a new code`, `keeps the old code when re-authentication fails`, `requires a current credential before calling the shell`, `loads and saves the auto-lock timeout`, `returns to the unlock screen if the vault is locked`, `loads and saves the clipboard clear delay`, `switches the theme`.

## Notes

- [`AutoLockSection`](AutoLockSection.tsx.md) and [`ClipboardSection`](ClipboardSection.tsx.md) each call `get_settings` on mount, so opening the screen makes two identical calls.
