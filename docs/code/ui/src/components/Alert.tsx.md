# [`ui/src/components/Alert.tsx`](../../../../../ui/src/components/Alert.tsx)

## Purpose

A message box used across the UI for errors, confirmations, warnings and information. It picks the ARIA live-region role from its tone so assistive technology announces errors at once and everything else politely.

## Public API

| Name         | Kind       | Signature | Description |
| ------------ | ---------- | --------- | ----------- |
| `AlertTone`  | type alias | `export type AlertTone = 'error' \| 'success' \| 'warning' \| 'info';` | Kind of message an `Alert` shows. |
| `AlertProps` | interface  | `export interface AlertProps { readonly tone: AlertTone; readonly children: ReactNode; }` | Props of `Alert`. |
| `Alert`      | component  | `function Alert({ tone, children }: AlertProps)` | A message box with a tone-dependent live-region role. |

## Details

### `AlertTone`

Four tones. Each maps to a CSS modifier class `alert--<tone>` (colours live in [`styles.css`](../../../../../ui/src/styles.css)).

### `AlertProps`

- `tone`: decides the colour and how assertively the message is announced.
- `children`: the message content.

### `Alert`

- **State**: none.
- **Rendered output**: a single `<div className="alert alert--<tone>">` containing `children`.
- **Accessibility**: `role="alert"` when `tone` is `'error'` (assertive, announced immediately), `role="status"` for every other tone (polite). Because the role is on the element itself, the message is announced when the `Alert` is inserted into the page.
- **Tauri calls**: none.
- **Security**: renders whatever it is given. Callers pass fixed UI strings, mostly from [`errorMessage`](../../../../../ui/src/api/errors.ts), never the shell's raw messages.

## Dependencies

- **Internal**: none.
- **External**: `react`: the `ReactNode` type.

## Used by

[`RecoveryCodeFlow`](../recovery/RecoveryCodeFlow.tsx.md), [`StatusScreens`](../screens/StatusScreens.tsx.md), [`CreateVaultScreen`](../screens/create/CreateVaultScreen.tsx.md), [`UnlockScreen`](../screens/unlock/UnlockScreen.tsx.md), [`ItemsScreen`](../screens/items/ItemsScreen.tsx.md), [`ItemDetails`](../screens/items/ItemDetails.tsx.md), [`ItemForm`](../screens/items/ItemForm.tsx.md), [`AttachmentList`](../screens/items/AttachmentList.tsx.md), [`AutoLockSection`](../screens/settings/AutoLockSection.tsx.md), [`ClipboardSection`](../screens/settings/ClipboardSection.tsx.md), [`ChangePasswordSection`](../screens/settings/ChangePasswordSection.tsx.md), [`RecoveryCodeSection`](../screens/settings/RecoveryCodeSection.tsx.md).

## Tests

- No dedicated test file. Screen tests query the `alert` and `status` roles it produces, for example [`App.test.tsx`](../../../../../ui/src/App.test.tsx).

## Notes

- Not applicable: no known limitations.
