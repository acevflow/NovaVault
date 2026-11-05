# [`ui/src/components/ScreenHeading.tsx`](../../../../../ui/src/components/ScreenHeading.tsx)

## Purpose

The heading of a screen, or of a step inside a multi-step flow. It moves keyboard focus to itself when it appears, so after a screen or step change, screen-reader and keyboard users start at the top of the new content instead of on a control that no longer exists.

## Public API

| Name                 | Kind      | Signature | Description |
| -------------------- | --------- | --------- | ----------- |
| `ScreenHeadingProps` | interface | `export interface ScreenHeadingProps { readonly children: ReactNode; readonly level?: 1 \| 2 \| 3; }` | Props of `ScreenHeading`. |
| `ScreenHeading`      | component | `function ScreenHeading({ children, level = 1 }: ScreenHeadingProps)` | A heading that focuses itself on mount. |

## Details

### `ScreenHeadingProps`

- `children`: heading text.
- `level`: `1` (default) for a screen, `2` or `3` for a step inside a section.

### `ScreenHeading`

- **State**: a `useRef` to the heading element.
- **Effect**: on mount only (empty dependency list), calls `focus()` on the heading. Changing `children` later does not move focus again; a new step must mount a new `ScreenHeading`.
- **Rendered output**: `<h1>`, `<h2>` or `<h3>` with `tabIndex={-1}` (focusable from script, not in the Tab order) and class `screen__heading` for level 1, `step__heading` otherwise.
- **Accessibility**: the focus move is the point of the component. A screen reader reads the new heading, and the next Tab press continues from the top of the new content.
- **Tauri calls**: none.

## Dependencies

- **Internal**: none.
- **External**: `react`: `useEffect`, `useRef`, the `ReactNode` type.

## Used by

[`RecoveryCodeFlow`](../recovery/RecoveryCodeFlow.tsx.md), [`StatusScreens`](../screens/StatusScreens.tsx.md), [`CreateVaultScreen`](../screens/create/CreateVaultScreen.tsx.md), [`UnlockScreen`](../screens/unlock/UnlockScreen.tsx.md), [`ItemsScreen`](../screens/items/ItemsScreen.tsx.md), [`ItemDetails`](../screens/items/ItemDetails.tsx.md), [`ItemForm`](../screens/items/ItemForm.tsx.md), [`SettingsScreen`](../screens/settings/SettingsScreen.tsx.md).

## Tests

- No dedicated test file. [`App.test.tsx`](../../../../../ui/src/App.test.tsx) asserts the "Your vault" `h1` has focus after startup.

## Notes

- If two `ScreenHeading`s mount in the same commit, the one whose effect runs last keeps focus. React runs child effects before parent effects, and siblings in tree order.
