# [`ui/src/recovery/RecoveryCodeFlow.tsx`](../../../../../ui/src/recovery/RecoveryCodeFlow.tsx)

## Purpose

The two-step flow that shows a new emergency recovery code once and makes the user prove they saved it. Step one shows the code, offers printing and requires a "saved" checkbox; step two asks the user to retype it. It is used after vault creation and after regenerating the code in settings. It implements the CLAUDE.md rule that the code is shown once and the user must confirm they saved it.

## Public API

| Name                    | Kind      | Signature | Description |
| ----------------------- | --------- | --------- | ----------- |
| `RecoveryCodeFlowProps` | interface | `export interface RecoveryCodeFlowProps { readonly code: string; readonly onDone: () => void; readonly headingLevel?: 1 \| 2 \| 3; readonly intro?: string; }` | Props of `RecoveryCodeFlow`. |
| `RecoveryCodeFlow`      | component | `function RecoveryCodeFlow({ code, onDone, headingLevel = 1, intro = 'This code opens your vault if you forget your master password.', }: RecoveryCodeFlowProps)` | Show-once, print, confirm-by-retyping flow. |

The step components `ShowStep` and `ConfirmStep` are internal and described below.

## Details

### `RecoveryCodeFlowProps`

- `code`: the code in display form, `XXXX-XXXX-…` (8 groups of 4), as returned by `create_vault` or `regenerate_recovery_code`.
- `onDone`: called once the user has retyped the code correctly. Parents drop the code from their state here.
- `headingLevel`: level of the step headings; default `1`. Settings uses a lower level because the flow sits inside a section.
- `intro`: the first sentence above the code, explaining when this code was issued.

### `RecoveryCodeFlow`

- **State**: `step`, `'show'` or `'confirm'`, starting at `'show'`.
- **Rendered output**: `ShowStep` or `ConfirmStep`. Continue moves to `'confirm'`; "Show the code again" moves back to `'show'`.

### `ShowStep` (internal)

- **State**: `saved` (the checkbox), starting at `false`.
- **Rendered output**, inside `<div className="print-sheet stack">`:
  1. [`ScreenHeading`](../components/ScreenHeading.tsx.md) "Save your recovery code", which takes focus.
  2. A `print-only` paragraph with the print date, from `new Date().toLocaleDateString()`.
  3. The intro, followed by a fixed warning: shown only once, keep it offline, anyone with it can open the vault, print only on a trusted printer.
  4. The code as `<ol className="recovery-code" aria-label="Recovery code">`, one `<li>` per group from `recoveryCodeGroups(code)`, keyed by position and group.
  5. A `no-print` block: a warning [`Alert`](../components/Alert.tsx.md) ("If you lose both your master password and this code, your data cannot be recovered."), the checkbox "I have saved my recovery code somewhere safe", a Print button that calls `window.print()`, and a primary Continue button disabled until the box is checked.
- **Printing**: the `print-sheet`, `print-only` and `no-print` classes (in [`styles.css`](../../../../../ui/src/styles.css)) decide what reaches the printed page: the heading, date, explanation and code, without the controls.

### `ConfirmStep` (internal)

- **State**: `typed` (the input) and `error` (`string | undefined`).
- **Rendered output**: a `<form noValidate>` with the heading "Confirm your recovery code", an explanation, a [`TextField`](../components/TextField.tsx.md) labelled "Recovery code" and the buttons "Show the code again" and Confirm (submit).
- **Input settings**: `autoComplete="off"`, `spellCheck={false}`, `autoCorrect="off"`, `autoCapitalize="characters"`, placeholder `XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX`, description "Dashes, spaces and letter case do not matter."
- **Submit**: prevents the default, then:
  - if [`recoveryCodesMatch(code, typed)`](../../../../../ui/src/recovery/recoveryCode.ts) (uppercase, keep only `0-9A-Z`, compare, and never match an empty code): clears `typed` and calls `onDone()`;
  - otherwise sets the error "That does not match the code shown. Check each group and try again."
- Typing clears the error.

### Flow summary

- **User flows**: read or print the code → tick "saved" → Continue → retype → Confirm → `onDone`. "Show the code again" returns to step one, which remounts with the checkbox cleared.
- **Accessibility**:
  - Each step's heading takes focus when the step appears.
  - The code is a labelled ordered list, so screen readers read it group by group.
  - The checkbox is wrapped in its `<label>`.
  - The confirmation error is linked to the input and marks it `aria-invalid`.
- **Tauri calls**: none. The comparison happens in the UI; the code was already issued by the shell.
- **Security**:
  - The code lives only in the parent's state and in this component's output; the parent drops it in `onDone`.
  - On purpose there is no copy button: the clipboard is shared with every app, and the shell's auto-clearing copy only reads item fields from the vault, so a copied code would stay there indefinitely.
  - Printing is offered with a warning to use a trusted printer.
  - JavaScript strings cannot be zeroized; the code is reclaimed by garbage collection after the parent drops it.

## Dependencies

- **Internal**: [`Alert`](../components/Alert.tsx.md), [`Button`](../components/Button.tsx.md), [`ScreenHeading`](../components/ScreenHeading.tsx.md), [`TextField`](../components/TextField.tsx.md), [`recovery/recoveryCode.ts`](../../../../../ui/src/recovery/recoveryCode.ts) (`recoveryCodeGroups`, `recoveryCodesMatch`).
- **External**: `react`: `useState`, the `SubmitEvent` type.

## Used by

- [`CreateVaultScreen`](../screens/create/CreateVaultScreen.tsx.md): after the vault is created.
- [`RecoveryCodeSection`](../screens/settings/RecoveryCodeSection.tsx.md): after the code is regenerated.

## Tests

- No dedicated test file. Covered through its parents:
  - [`CreateVaultScreen.test.tsx`](../../../../../ui/src/screens/create/CreateVaultScreen.test.tsx): `creates the vault, shows the code once and requires retyping it`, `can go back to the code from the confirmation step`.
  - [`SettingsScreen.test.tsx`](../../../../../ui/src/screens/settings/SettingsScreen.test.tsx): `regenerates and confirms a new code`.
- [`recoveryCode.test.ts`](../../../../../ui/src/recovery/recoveryCode.test.ts) covers the comparison and splitting helpers.

## Notes

- After a failed Confirm, focus stays on the Confirm button and the error is not a live region. A screen-reader user hears the error only after moving back to the input.
- The match is checked only in the UI. It proves the user can reproduce the code, not that the shell would accept it, though the shell issued it moments before.
- The print date uses the webview's locale.
