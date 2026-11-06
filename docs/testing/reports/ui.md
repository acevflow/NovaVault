# UI test report

Results of the UI lint, type check, unit tests and production build, and of the full app build, run locally on one Windows machine. No application code was changed for this report.

| | |
| --- | --- |
| Date | 2026-11-06 |
| Commit | `c80307a` (`main`, merge of PR #33) |
| Result | Lint: **pass** · Typecheck: **pass** · Unit tests: **pass** (81/81), but **flaky** under CPU load (3 timeouts in 1 of 3 runs) · UI build: **pass** · Full app build: **fail** (environment, not a code failure) |

## Environment

| Tool | Version |
| ---- | ------- |
| OS | Windows 11 Home 10.0.26200, x86_64 |
| git | 2.55.0.windows.3 |
| Node.js / npm | 24.19.0 / 12.0.2 |
| Package manager | npm (`package-lock.json` at the root for the Tauri CLI, and in `ui/`) |
| TypeScript | 6.0.3 |
| ESLint | 9.39.5 |
| Prettier | 3.9.10 |
| Vitest | 5.1.0 (environment: jsdom) |
| Vite | 8.3.5 |
| Tauri CLI (`@tauri-apps/cli`) | 2.12.4 (`npx tauri`; `cargo tauri` is not installed and not needed) |
| rustc / cargo | 1.99.1 / 1.99.1, host `x86_64-pc-windows-msvc` |

The Tauri 2 Windows prerequisites (MSVC, Windows SDK, WebView2) were checked for the [Rust test report](rust.md) the day before. They were not checked again.

## Commands

Run in this order. Each command ran even when an earlier one failed.

| # | Directory | Command | Exit code | Time | Result |
| - | --------- | ------- | --------- | ---- | ------ |
| 1 | root | `npm ci` | 0 | 10 s | Pass. 3 packages, 0 vulnerabilities |
| 2 | `ui/` | `npm ci` | 0 | 2 min | Pass. 362 packages, 7 moderate vulnerabilities (see [Warnings](#warnings)) |
| 3 | `ui/` | `npm run format:check` (`prettier --check .`) | 0 | 3 s | Pass |
| 4 | `ui/` | `npm run lint` (`eslint . --max-warnings 0`) | 0 | 21 s | Pass, 0 errors, 0 warnings |
| 5 | `ui/` | `npm run typecheck` (`tsc -b`) | 0 | 5 s | Pass, 0 errors |
| 6 | `ui/` | `npm test` (`vitest run`) | 0 | 29 s | Pass, 81/81 |
| 7 | `ui/` | `npm run build` (`tsc -b && vite build`) | 0 | 15 s | Pass |
| 8 | root | `npm run build` (`tauri build`) | 1 | 106 s | **Fail**: the OS blocked a build script (see [Failures](#failures)) |

Command 3 was not part of the request. It is part of the project's `npm run check` and CI, so it was included.

Two more test runs were made to collect per-test results. Their output was written to a JSON file:

| # | Command | Conditions | Exit code | Result |
| - | ------- | ---------- | --------- | ------ |
| 6a | `npx vitest run --reporter=json` | While command 8 compiled Rust in parallel (high CPU load) | 1 | **Fail**: 78 passed, 3 failed (timeouts) |
| 6b | `npx vitest run --reporter=json` | After command 8, no other load | 0 | Pass, 81/81 |

## Unit test results

14 test files, 81 tests. Results from run 6b. No tests were skipped and none are marked `todo`.

| Test file | Tests | Passed | Run 6a (under load) |
| --------- | ----- | ------ | ------------------- |
| `src/App.test.tsx` | 3 | 3 | 3/3 |
| `src/api/commands.test.ts` | 5 | 5 | 5/5 |
| `src/app/router.test.ts` | 4 | 4 | 4/4 |
| `src/components/SecretField.test.tsx` | 7 | 7 | 7/7 |
| `src/format/bytes.test.ts` | 5 | 5 | 5/5 |
| `src/recovery/recoveryCode.test.ts` | 2 | 2 | 2/2 |
| `src/theme/ThemeProvider.test.tsx` | 3 | 3 | 3/3 |
| `src/screens/create/CreateVaultScreen.test.tsx` | 5 | 5 | **4/5** |
| `src/screens/settings/SettingsScreen.test.tsx` | 10 | 10 | **8/10** |
| `src/screens/items/AttachmentList.test.tsx` | 9 | 9 | 9/9 |
| `src/screens/items/filterItems.test.ts` | 4 | 4 | 4/4 |
| `src/screens/items/ItemsScreen.crud.test.tsx` | 10 | 10 | 10/10 |
| `src/screens/items/ItemsScreen.test.tsx` | 6 | 6 | 6/6 |
| `src/screens/unlock/UnlockScreen.test.tsx` | 8 | 8 | 8/8 |
| **Total** | **81** | **81** | **78/81** |

Slowest tests in run 6b. The default Vitest test timeout is 5000 ms.

| Duration | Test |
| -------- | ---- |
| 4389 ms | SettingsScreen: master password › changes the password after re-authentication |
| 3010 ms | CreateVaultScreen › rejects weak passwords without creating a vault |
| 2739 ms | CreateVaultScreen › creates the vault, shows the code once and requires retyping it |
| 2050 ms | SettingsScreen: master password › reports a wrong current password |
| 1953 ms | CreateVaultScreen › reports mismatched confirmations on the field |

## Failures

### Unit tests time out under load (run 6a)

Three tests failed in run 6a, while `tauri build` was compiling Rust on the same machine. All three passed in runs 6 and 6b.

```text
FAIL src/screens/create/CreateVaultScreen.test.tsx
  CreateVaultScreen > rejects weak passwords without creating a vault        5079 ms
FAIL src/screens/settings/SettingsScreen.test.tsx
  SettingsScreen: master password > changes the password after re-authentication  5047 ms
  SettingsScreen: master password > reports a wrong current password              5048 ms

Error: Test timed out in 5000ms.
If this is a long-running test, pass a timeout value as the last argument or configure it globally with "testTimeout".
    at D:/Projects/NovaVault/ui/src/screens/create/CreateVaultScreen.test.tsx:23:3
    (same error at SettingsScreen.test.tsx:38:3 and SettingsScreen.test.tsx:49:3)
```

Cause: see [Bugs found](#bugs-found), item 1. This is not an application failure. The tests type a password with `userEvent` and wait for the lazily loaded strength estimator, which is slow when the CPU is busy.

### Full app build (command 8)

The UI part of the build (`beforeBuildCommand`) passed with the same output as command 7. The Rust compile stopped after 41 crates:

```text
error: failed to run custom build command for `anyhow v1.0.104`

Caused by:
  could not execute process `D:\Projects\NovaVault\target\release\build\anyhow-c92c6b66244c4610\build-script-build` (never executed)

Caused by:
  Una directiva de Control de aplicaciones bloqueó este archivo. (os error 4551)
warning: build failed, waiting for other jobs to finish...
       Error failed to build app: failed to build app
```

The Spanish message means "An Application Control policy blocked this file". A Windows Application Control (WDAC) policy on this machine blocks some compiled build scripts. The [Rust test report](rust.md) records the same cause. Which file gets blocked changes from run to run. The build was not retried, and the policy was not worked around. The installers were not built, so no Windows bundle (MSI or NSIS) was produced or tested on this machine. CI builds the Rust workspace on Windows, macOS and Linux, but it does not run `tauri build` or bundle installers.

## Warnings

- **Flaky timeouts**: 3 of 81 tests are close to the 5 s timeout even without load (slowest: 4.4 s). See [Bugs found](#bugs-found).
- **npm audit** (`ui/`): 7 moderate vulnerabilities, all from one advisory: `minimatch` 7.0.0–11.0.0, [GHSA-g3hv-75wg-m249](https://github.com/advisories/GHSA-g3hv-75wg-m249) (denial of service through glob patterns). It comes in through `typescript-eslint` → `@typescript-eslint/typescript-estree`. These are development dependencies only (lint). They are not in the shipped bundle. npm offers a fix only through `npm audit fix --force`, which would install `typescript-eslint@8.36.0`, a breaking downgrade. It was not applied. The root `npm ci` reported 0 vulnerabilities.
- **Deprecation**: `npm ci` in `ui/` warns `eslint@9.39.5: This version is no longer supported`.
- **Vitest performance hint**: jsdom was created 14 times (68.65 s in total, 47% of tracked time). Vitest suggests `pool: 'vmThreads'` or `isolate: false`. It is only a hint, not a warning about the tests.
- **Large chunk**: `dist-DUoORJZU.js` is 1.2 MB (613 kB gzip). It is the password strength dictionary of `@zxcvbn-ts`, loaded lazily. `ui/vite.config.ts` sets `chunkSizeWarningLimit: 1500` on purpose, so Vite reports no warning.
- Lint, type check, Prettier and Vite build: no warnings.

## Build artifacts

### UI production build (command 7)

Output directory `ui/dist/` (not committed). 7 files, 1,939,406 bytes in total.

| Path | Size (bytes) | Size (Vite, kB) | gzip (kB) | Content |
| ---- | ------------ | --------------- | --------- | ------- |
| `ui/dist/index.html` | 394 | 0.39 | 0.26 | Entry page |
| `ui/dist/assets/index-B4i3c-QS.css` | 11,281 | 11.28 | 2.68 | Styles |
| `ui/dist/assets/index-4jevT0Gs.js` | 266,467 | 266.46 | 81.46 | App code, React, Tauri API |
| `ui/dist/assets/decompress-W8FIgzvw.js` | 151 | 0.15 | 0.15 | Dictionary decompression helper |
| `ui/dist/assets/dist-B72SMmQL.js` | 33,029 | 33.02 | 11.08 | `@zxcvbn-ts` core, lazy |
| `ui/dist/assets/dist-3H1nCAPe.js` | 427,581 | 427.58 | 224.09 | `@zxcvbn-ts` common data (keyboard layouts and more), lazy |
| `ui/dist/assets/dist-DUoORJZU.js` | 1,200,503 | 1,200.50 | 612.74 | `@zxcvbn-ts` dictionaries, lazy |

127 modules were transformed. Source maps are off for release builds (`sourcemap: false`).

### Full app build (command 8)

No artifacts. The build failed before linking `novavault.exe`, so `target/release/novavault.exe` and `target/release/bundle/` do not exist.

## Bugs found

No bugs were found in the application code. One problem was found in the test code. It was recorded and not fixed, as this task asked.

1. **Test timeout lower than the wait inside the tests.** `src/screens/create/CreateVaultScreen.test.tsx:12` and `src/screens/settings/SettingsScreen.test.tsx:34` and `:69` wait up to 10 s for the strength meter (`findByText(..., { timeout: 10_000 })`). But `ui/vite.config.ts` does not set `test.testTimeout`, so each test still fails after Vitest's default of 5 s. The 10 s wait can never be used. Without load, the slowest test took 4.4 s. Under load, three tests went past 5 s and failed (run 6a). A slow CI runner could hit the same failure. Possible fixes: set a per-test or global timeout of at least 10 s, or make the strength estimator faster to load in tests.

## Untested areas

What the unit tests do not cover, or what was not checked locally:

- **Real Tauri IPC.** All tests replace the shell with `mockIPC` (`src/test/ipc.ts`). The real contract between the UI and the Rust commands (names, argument shapes, error codes) is tested only from the UI side. A mismatch with `src-tauri` would not be caught here.
- **No end-to-end or WebView tests.** The UI never ran inside the Tauri WebView2 window for this report, because the app build failed. CSP behaviour, `freezePrototype`, the capability set and the real window were not exercised.
- **No direct tests** for these modules. Most are used by the screen tests, so they are covered only indirectly, through those screens:
  - `app/AppShell.tsx`, `screens/StatusScreens.tsx`, `main.tsx`
  - Components: `Alert`, `Button`, `CopyButton`, `PasswordField`, `ScreenHeading`, `TextAreaField`, `TextField`
  - `password/`: `strength.ts`, `usePasswordStrength.ts`, `StrengthMeter.tsx`, `NewPasswordFields.tsx`, `newPassword.ts`
  - `recovery/RecoveryCodeFlow.tsx`
  - `screens/items/`: `ItemList`, `ItemDetails`, `ItemForm`, `ConfirmDelete`, `kinds.ts`
  - `screens/settings/` sections: `AppearanceSection`, `AutoLockSection`, `ChangePasswordSection`, `ClipboardSection`, `RecoveryCodeSection`, `CurrentCredentialFields`, `useCredential.ts`
  - `format/dateTime.ts`, `format/duration.ts`, `theme/theme.ts`
- **No coverage measurement.** The project does not configure a coverage provider, so no line or branch coverage figures exist.
- **Behaviour that needs the OS or the shell:** auto-lock on inactivity, suspend and screen lock; clipboard auto-clear after the timeout; native file dialogs for attachments. The UI tests only check that the UI calls the right commands.
- **Accessibility** is checked only statically, by `eslint-plugin-jsx-a11y` and the role and label queries in the tests. No screen reader, keyboard-only or contrast audit was done.
- **Visual and layout checks**: none (no screenshot tests, no light/dark theme rendering checks beyond `ThemeProvider.test.tsx`).
- **Other platforms.** Everything above ran on Windows only. macOS and Linux run the UI checks only in CI.
