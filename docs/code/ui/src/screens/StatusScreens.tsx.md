# [`ui/src/screens/StatusScreens.tsx`](../../../../../ui/src/screens/StatusScreens.tsx)

## Purpose

The two screens shown before the vault status is known: a loading screen while `vault_status` runs at startup, and an error screen with a retry button when it fails. [`App`](../App.tsx.md) renders them for the `loading` and `startupError` routes.

## Public API

| Name                      | Kind      | Signature | Description |
| ------------------------- | --------- | --------- | ----------- |
| `LoadingScreen`           | component | `function LoadingScreen()` (no props) | Shown while the vault status is being read. |
| `StartupErrorScreenProps` | interface | `export interface StartupErrorScreenProps { readonly message: string; readonly onRetry: () => void; }` | Props of `StartupErrorScreen`. |
| `StartupErrorScreen`      | component | `function StartupErrorScreen({ message, onRetry }: StartupErrorScreenProps)` | Shown when the vault status cannot be read. |

## Details

### `LoadingScreen`

- **Props / state**: none.
- **Rendered output**: `<section className="screen screen--narrow" aria-busy="true">` containing `<p role="status">Opening NovaVault…</p>`.
- **Accessibility**: the status role announces the text politely; `aria-busy` marks the region as loading. There is no heading, so focus is not moved.
- **Tauri calls**: none; the call is made by `App`.

### `StartupErrorScreenProps`

- `message`: user-facing error text; `App` passes `errorMessage(error)` for the failed `vault_status` call.
- `onRetry`: tries to read the vault status again.

### `StartupErrorScreen`

- **State**: none.
- **Rendered output**: a narrow section with a [`ScreenHeading`](../components/ScreenHeading.tsx.md) "NovaVault could not start" (takes focus), an error [`Alert`](../components/Alert.tsx.md) with `message`, and a primary "Try again" [`Button`](../components/Button.tsx.md).
- **User flows**: "Try again" calls `onRetry`; `App` dispatches `retry`, which returns to the loading route and re-runs `vault_status`.
- **Accessibility**: the heading takes focus and the alert is announced at once (`role="alert"`).
- **Tauri calls**: none directly; retrying makes `App` call `vault_status` again.

## Dependencies

- **Internal**: [`Alert`](../components/Alert.tsx.md), [`Button`](../components/Button.tsx.md), [`ScreenHeading`](../components/ScreenHeading.tsx.md).
- **External**: none.

## Used by

- [`App.tsx`](../App.tsx.md): for the `loading` and `startupError` routes.

## Tests

- [`App.test.tsx`](../../../../../ui/src/App.test.tsx): `shows a startup error and retries` checks the alert text and that "Try again" reaches the vault.

## Notes

- The Try again button is not disabled while retrying, but it is not visible either: retrying replaces this screen with `LoadingScreen` straight away.
