# [`ui/src/screens/unlock/UnlockScreen.tsx`](../../../../../../ui/src/screens/unlock/UnlockScreen.tsx)

## Purpose

The unlock screen, shown when a vault exists but is locked (at startup, after a manual lock, or after an auto-lock reload). The user unlocks with the master password or, if it is forgotten, with the emergency recovery code. The two methods are tabs. The shell checks the secret; the UI only sends it and shows the outcome.

## Public API

| Name                | Kind      | Signature | Description |
| ------------------- | --------- | --------- | ----------- |
| `UnlockScreenProps` | interface | `export interface UnlockScreenProps { readonly onUnlocked: () => void; }` | Props of `UnlockScreen`. |
| `UnlockScreen`      | component | `function UnlockScreen({ onUnlocked }: UnlockScreenProps)` | Password or recovery code unlock. |

Internal: `METHODS`, the two tabs: `password` ("Master password") and `recoveryCode` ("Recovery code").

## Details

### `UnlockScreenProps`

- `onUnlocked`: called once the vault is unlocked. [`App`](../../App.tsx.md) dispatches `unlocked`, which opens the items screen.

### `UnlockScreen`

- **State**: `method` (`'password'` by default), `secret`, `busy`, `error`, a `useId` base id, refs to the tab buttons and to the secret input.
- **`selectMethod(next)`**: sets the method, clears the secret ("never carry a typed secret over to the other method") and clears the error.
- **Tab keyboard handling**: ArrowRight and ArrowLeft move to the next or previous tab (wrapping), Home and End go to the first or last. The key press selects that method and focuses its tab (automatic activation). Other keys are ignored.
- **Submit**: prevents the default. With an empty secret, shows "Enter your master password." or "Enter your recovery code.", focuses the input and stops. Otherwise calls `attempt()`.
- **`attempt()`**:
  1. Sets `busy` and clears `error`.
  2. Calls `unlock({ method, secret })`.
  3. Success: clears the secret and calls `onUnlocked()`.
  4. Failure: clears the secret ("so a wrong secret does not linger") and sets the error:
     - `invalid_credentials` with a password: "That master password is not correct."
     - `invalid_credentials` with a code: "That recovery code is not correct. It may have been replaced by a newer one."
     - anything else: `errorMessage`, for example the `invalid_recovery_code`, `unsupported_vault` or `corrupted` texts.
     Then clears `busy` and focuses the input.
- **Rendered output**: `<section className="screen screen--narrow">` with:
  1. The [`ScreenHeading`](../../components/ScreenHeading.tsx.md) "Unlock your vault" (takes focus).
  2. `<div role="tablist" aria-label="Unlock with">` with two `role="tab"` buttons, each with `aria-selected`, `aria-controls` pointing at the panel, and a roving `tabIndex` (0 for the selected tab, -1 otherwise).
  3. `<div role="tabpanel" aria-labelledby={selected tab}>` with a `<form noValidate>`:
     - for `password`: a [`PasswordField`](../../components/PasswordField.tsx.md) "Master password", `autoComplete="current-password"`;
     - for `recoveryCode`: a [`TextField`](../../components/TextField.tsx.md) "Recovery code" with `field--code`, `autoComplete="off"`, `spellCheck={false}`, `autoCorrect="off"`, `autoCapitalize="characters"`, and the description "Forgot your master password? Unlock with your recovery code, then set a new password in Settings.";
     - both disabled while busy, and keyed per method so switching remounts the input;
     - an error [`Alert`](../../components/Alert.tsx.md) when `error` is set;
     - the primary submit [`Button`](../../components/Button.tsx.md) "Unlock", shown as "Unlocking…" while busy.
- **User flows**: type the password → Enter or Unlock → items screen. Forgot it: Recovery code tab → type the code → Unlock → items → Settings → change the master password using the recovery code as the current credential ([`ChangePasswordSection`](../settings/ChangePasswordSection.tsx.md)).
- **Accessibility**:
  - Follows the WAI-ARIA tabs pattern: tablist, tabs, tabpanel, roving tab index, arrow, Home and End keys.
  - The heading takes focus on arrival.
  - Errors are alerts, and focus returns to the input after a failure or an empty submit.
  - The recovery code hint is linked to its input with `aria-describedby`.
- **Tauri calls**: `unlock` → `unlock` with `{ credential: { method, secret } }`. The shell derives the key-slot key with Argon2id, so the call takes noticeable time. See [`commands.rs`](../../../../src-tauri/src/commands.rs.md).
- **Security**:
  - The secret is sent once and cleared from state on success and on failure.
  - Switching method clears it.
  - Spell checking is off for both inputs.
  - Error texts never echo the secret.

## Dependencies

- **Internal**: [`api/commands.ts`](../../../../../../ui/src/api/commands.ts) (`unlock`), [`api/errors.ts`](../../../../../../ui/src/api/errors.ts), [`api/types.ts`](../../../../../../ui/src/api/types.ts) (`CredentialMethod`), [`Alert`](../../components/Alert.tsx.md), [`Button`](../../components/Button.tsx.md), [`PasswordField`](../../components/PasswordField.tsx.md), [`ScreenHeading`](../../components/ScreenHeading.tsx.md), [`TextField`](../../components/TextField.tsx.md).
- **External**: `react`: `useId`, `useRef`, `useState`, the `KeyboardEvent` and `SubmitEvent` types.

## Used by

- [`App.tsx`](../../App.tsx.md): for the `unlock` route.

## Tests

- [`UnlockScreen.test.tsx`](../../../../../../ui/src/screens/unlock/UnlockScreen.test.tsx):
  - `unlocks with the master password`.
  - `unlocks with the recovery code from its tab`.
  - `reports a wrong password and clears the field`: shows the message, empties the field, keeps focus on it.
  - `explains %s errors` (parameterized): `invalid_recovery_code`, `unsupported_vault` and `corrupted` show their texts.
  - `asks for a secret before calling the shell`.
  - `goes from unlock to the vault and back on lock`.

## Notes

- Unlike create and change-password, there is no status message explaining the key-derivation wait; only the button label changes to "Unlocking…".
- The method switch is implemented here as tabs, while the settings re-authentication uses radios ([`CurrentCredentialFields`](../settings/CurrentCredentialFields.tsx.md)). The two are not shared.
- Open question: after a failure, `input.current?.focus()` runs in the same tick as `setBusy(false)`, before React has re-enabled the input. The test passes in jsdom; whether focus reliably lands on a just-disabled input in the real webview is not verified here. The same pattern appears in [`ChangePasswordSection`](../settings/ChangePasswordSection.tsx.md) and [`RecoveryCodeSection`](../settings/RecoveryCodeSection.tsx.md), where `busy` is cleared in `finally`, after the focus call.
