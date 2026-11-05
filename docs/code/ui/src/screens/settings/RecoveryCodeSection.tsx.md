# [`ui/src/screens/settings/RecoveryCodeSection.tsx`](../../../../../../ui/src/screens/settings/RecoveryCodeSection.tsx)

## Purpose

The "Recovery code" card of the settings screen. After re-authentication it asks the shell for a new emergency recovery code, which invalidates the previous one, then walks the user through saving and confirming it with [`RecoveryCodeFlow`](../../recovery/RecoveryCodeFlow.tsx.md).

## Public API

| Name                       | Kind      | Signature | Description |
| -------------------------- | --------- | --------- | ----------- |
| `RecoveryCodeSectionProps` | interface | `export interface RecoveryCodeSectionProps { readonly onLocked: () => void; }` | Props of `RecoveryCodeSection`. |
| `RecoveryCodeSection`      | component | `function RecoveryCodeSection({ onLocked }: RecoveryCodeSectionProps)` | Regenerate, show and confirm a new recovery code. |

## Details

### `RecoveryCodeSectionProps`

- `onLocked`: called if the shell reports the vault is locked.

### `RecoveryCodeSection`

- **State**: `current` from [`useCredential()`](../../../../../../ui/src/screens/settings/useCredential.ts), `newCode` (`string | null`), `busy`, `error`, `done`, and a ref to the current-credential input.
- **Rendered states**, inside `<section className="card">` with `<h2>Recovery code</h2>`:
  - **Form** (`newCode === null`): the text "Generate a new code if yours was lost or seen by someone else.", a warning [`Alert`](../../components/Alert.tsx.md) ("Your current recovery code stops working as soon as the new one is generated."), [`CurrentCredentialFields`](CurrentCredentialFields.tsx.md), an error `Alert` if any, a success `Alert` "Your new recovery code is saved and confirmed." after a completed flow, and a danger submit [`Button`](../../components/Button.tsx.md) "Generate new recovery code" ("Generating…" while busy).
  - **Flow** (`newCode` set): `RecoveryCodeFlow` with level-3 headings and the intro "This new code replaces your previous one, which no longer works.". On `onDone` it sets `newCode` to `null` (dropping the code) and `done` to `true`.
- **Submit**: prevents the default and clears `done` and `error`. With no current secret, shows "Enter your current master password or recovery code.", focuses the input and stops. Otherwise calls `regenerate()`.
- **`regenerate()`**: sets `busy` and calls `regenerateRecoveryCode(current.credential)`.
  - Success: clears the current secret and stores the code.
  - `locked`: calls `onLocked()`.
  - `invalid_credentials`: clears the secret, shows "The current password or recovery code is not correct. Your recovery code was not changed." and focuses the input.
  - Other errors: clears the secret, shows `errorMessage`, focuses the input.
  - Always clears `busy`.
- **User flows**: choose a method → type the current secret → Generate → save, print and tick → Continue → retype → Confirm → the success message.
- **Accessibility**:
  - The card is labelled by its heading.
  - The flow's step headings (level 3) take focus as they appear.
  - Errors are alerts; the warning and success messages are polite status messages.
  - Focus returns to the credential input after a missing or wrong credential.
- **Tauri calls**: `regenerateRecoveryCode` → `regenerate_recovery_code` with `{ currentCredential }`, returning `{ recoveryCode }`. See [`commands.rs`](../../../../src-tauri/src/commands.rs.md).
- **Security**:
  - The current secret is cleared after every shell attempt.
  - The new code lives only in `newCode` and is dropped when the flow completes.
  - The danger variant and the warning stress that the old code stops working at once.

## Dependencies

- **Internal**: [`api/commands.ts`](../../../../../../ui/src/api/commands.ts) (`regenerateRecoveryCode`), [`api/errors.ts`](../../../../../../ui/src/api/errors.ts), [`Alert`](../../components/Alert.tsx.md), [`Button`](../../components/Button.tsx.md), [`RecoveryCodeFlow`](../../recovery/RecoveryCodeFlow.tsx.md), [`CurrentCredentialFields`](CurrentCredentialFields.tsx.md), [`useCredential.ts`](../../../../../../ui/src/screens/settings/useCredential.ts).
- **External**: `react`: `useId`, `useRef`, `useState`, the `SubmitEvent` type.

## Used by

- [`SettingsScreen`](SettingsScreen.tsx.md).

## Tests

- [`SettingsScreen.test.tsx`](../../../../../../ui/src/screens/settings/SettingsScreen.test.tsx): `regenerates and confirms a new code` (and checks the code is gone from the page afterwards), `keeps the old code when re-authentication fails`, `requires a current credential before calling the shell`.

## Notes

- The old code is invalid as soon as the shell returns the new one, before the user has confirmed saving it. During the flow, the navigation and Lock button stay available (the route is `settings`). Leaving the screen, locking, or an auto-lock reload loses the new code unconfirmed, and the user must regenerate again.
- Unlike vault creation, there is no step that blocks other actions until the code is confirmed.
