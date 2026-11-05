# [`ui/src/password/StrengthMeter.tsx`](../../../../../ui/src/password/StrengthMeter.tsx)

## Purpose

Shows how strong a new master password is: a `<meter>`, a text label announced politely to screen readers, and zxcvbn's warning and suggestions. Before anything is typed it shows guidance instead. The estimate itself is computed elsewhere, by [`usePasswordStrength`](../../../../../ui/src/password/usePasswordStrength.ts) and [`strength.ts`](../../../../../ui/src/password/strength.ts).

## Public API

| Name                 | Kind      | Signature | Description |
| -------------------- | --------- | --------- | ----------- |
| `StrengthMeterProps` | interface | `export interface StrengthMeterProps { readonly estimate: StrengthEstimate \| null; readonly hasPassword: boolean; }` | Props of `StrengthMeter`. |
| `StrengthMeter`      | component | `function StrengthMeter({ estimate, hasPassword }: StrengthMeterProps)` | Visual and announced password strength with tips. |

## Details

### `StrengthMeterProps`

- `estimate`: the estimate to show, or `null` while none is available.
- `hasPassword`: whether the user has typed anything.

### `StrengthMeter`

- **State**: `useId()` for the label id.
- **Rendered states**:
  - **Nothing typed** (`hasPassword` false): a paragraph: "Use a long passphrase of several unrelated words. It must rate at least “Strong”." The quoted label is `STRENGTH_LABELS[MIN_ACCEPTED_SCORE]`, with `MIN_ACCEPTED_SCORE = 3`.
  - **Checking** (`hasPassword` true, `estimate` null): the meter at `0` and the label "Password strength: checking…". No tips.
  - **Estimated**: the meter at `estimate.score`, the label "Password strength: <label>" (Very weak, Weak, Fair, Strong, Very strong for 0–4), and a list of tips: the warning followed by the suggestions, with `null` and empty strings removed. No list is rendered when there are no tips.
- **Meter**: `<meter min={0} max={4} low={2} high={3} optimum={4} value={score} aria-labelledby={labelId}>`, so browsers colour scores at or below 2 as poor, 3 as acceptable and 4 as optimal.
- **Accessibility**:
  - The meter is named by the visible label through `aria-labelledby`.
  - The label paragraph has `aria-live="polite"`, so the strength is announced as the user types without interrupting.
  - Inside [`NewPasswordFields`](NewPasswordFields.tsx.md), the whole meter is the password input's description, linked with `aria-describedby`.
- **Tauri calls**: none. Estimation runs locally in the webview.
- **Security**: receives only the score and feedback, never the password. `strength.ts` drops the zxcvbn result's echo of the password.

## Dependencies

- **Internal**: [`password/strength.ts`](../../../../../ui/src/password/strength.ts): `MIN_ACCEPTED_SCORE`, `STRENGTH_LABELS`, `StrengthEstimate`.
- **External**: `react`: `useId`.

## Used by

- [`NewPasswordFields`](NewPasswordFields.tsx.md): as the description of the password field.

## Tests

- No dedicated test file. [`CreateVaultScreen.test.tsx`](../../../../../ui/src/screens/create/CreateVaultScreen.test.tsx) (`rejects weak passwords without creating a vault`) goes through it.

## Notes

- While checking, the meter shows `0`, the same position as "Very weak", although the label says "checking…".
- Tips are used as React keys; zxcvbn does not return duplicate feedback strings, but two identical tips would trigger a duplicate-key warning.
- zxcvbn's feedback strings come from its English translations, so they are English only.
