# Test report

Summary of the latest automated test runs. It brings together the [Rust test report](reports/rust.md) and the [UI test report](reports/ui.md); those two files have the full command output, environment and per-test results. Manual testing is planned in the [manual test plan](manual-test-plan.md).

| | |
| --- | --- |
| Rust run | 2026-11-05, commit `339935d` (`main`, merge of PR #32) |
| UI run | 2026-11-06, commit `c80307a` (`main`, merge of PR #33) |
| Machine | Windows 11 Home 10.0.26200, x86_64, rustc 1.99.1, Node.js 24.19.0 |
| Scope | Local runs on one Windows machine. No application code was changed for either report. |

## Overall status

**No defect was found in the application code, and no test assertion failed in a normal run.**

- Every formatting, lint and type check passed, for Rust and for the UI.
- All 179 Rust tests and all 81 UI tests passed when they could run.
- Two commands failed locally because a Windows Application Control (WDAC) policy blocked freshly built executables (os error 4551). These are environment failures, not code failures, and they were not worked around.
- One problem was found in the test code: three UI tests can time out on a busy machine (see [Bugs found](#bugs-found)).
- CI runs the Rust and UI checks on Windows, macOS and Linux. The latest CI run on `main` at the time of the Rust report (run 42098475466, for PR #32) passed.

## Results per suite

| Suite | Command | Result | Passed | Failed | Details |
| ----- | ------- | ------ | ------ | ------ | ------- |
| Rust format | `cargo fmt --check` | Pass | – | – | [Rust report](reports/rust.md#commands) |
| Rust lint | `cargo clippy --workspace --all-targets -- -D warnings` | Pass | – | – | [Rust report](reports/rust.md#commands) |
| Rust tests, workspace | `cargo test --workspace` | **Fail** (environment) | 51 (shell) | 0 | Core test binary blocked by WDAC, 128 core tests not run. [Rust report](reports/rust.md#cargo-test---workspace-command-3) |
| Rust tests, core only (diagnostic) | `cargo test -p novavault-core` | Pass | 128 | 0 | 1 `#[cfg(unix)]` test is not compiled on Windows. [Rust report](reports/rust.md#cargo-test--p-novavault-core-diagnostic-3b) |
| UI format | `npm run format:check` | Pass | – | – | [UI report](reports/ui.md#commands) |
| UI lint | `npm run lint` | Pass | – | – | [UI report](reports/ui.md#commands) |
| UI type check | `npm run typecheck` | Pass | – | – | [UI report](reports/ui.md#commands) |
| UI unit tests | `npm test` | Pass, **flaky** under load | 81 | 0 | 78/81 in one run while Rust compiled in parallel. [UI report](reports/ui.md#unit-test-results) |
| UI production build | `npm run build` (in `ui/`) | Pass | – | – | [UI report](reports/ui.md#ui-production-build-command-7) |
| Full app build | `npm run build` (root, `tauri build`) | **Fail** (environment) | – | – | Build script blocked by WDAC, no installer produced. [UI report](reports/ui.md#full-app-build-command-8) |

### Totals

| Area | Tests | Passed | Failed | Not run |
| ---- | ----- | ------ | ------ | ------- |
| Rust (`src-tauri` + `novavault-core`) | 180 | 179 | 0 | 1 (Unix-only test on Windows) |
| UI (Vitest, 14 files) | 81 | 81 | 0 | 0 |
| **Total** | **261** | **260** | **0** | **1** |

The Rust totals include the diagnostic core-only run, because the required workspace run could not execute the core tests.

## Failures

| # | Failure | Cause | Effect | Details |
| - | ------- | ----- | ------ | ------- |
| 1 | `cargo test --workspace`: the core unit-test binary never started | Local WDAC policy (os error 4551). Which binary gets blocked changes from run to run. | The required command fails locally. The same tests pass with `cargo test -p novavault-core` and in CI. | [Rust report](reports/rust.md#failures) |
| 2 | `tauri build`: the `anyhow` build script never started | Same WDAC policy | No `novavault.exe` and no MSI or NSIS installer were built or tested locally. | [UI report](reports/ui.md#full-app-build-command-8) |
| 3 | 3 Vitest tests timed out at 5 s | High CPU load while Rust compiled in parallel; see bug 1 below | The tests passed in the two runs without load. | [UI report](reports/ui.md#unit-tests-time-out-under-load-run-6a) |

## Bugs found

| # | Area | Description | Status |
| - | ---- | ----------- | ------ |
| 1 | UI test code | `CreateVaultScreen.test.tsx:12` and `SettingsScreen.test.tsx:34` and `:69` wait up to 10 s for the strength meter, but `ui/vite.config.ts` sets no `test.testTimeout`, so Vitest's default of 5 s ends the test first. The slowest test takes 4.4 s without load, so a busy machine or slow CI runner can fail it. Fix: raise the timeout to at least 10 s, or make the strength estimator faster to load in tests. | Open, not fixed (reports change no code) |

No bugs were found in the application code.

### Warnings worth tracking

- `npm audit` in `ui/`: 7 moderate vulnerabilities from one advisory in `minimatch` ([GHSA-g3hv-75wg-m249](https://github.com/advisories/GHSA-g3hv-75wg-m249)), reached through `typescript-eslint`. Development dependency only, not in the shipped bundle. The only offered fix is a breaking downgrade.
- `eslint@9.39.5` is reported as no longer supported.
- Newer versions of `tauri`, `tauri-build`, `wry`, `tao` and the Tauri plugins exist. Upgrading was not part of the reports.

## Untested areas

From reading the test files; no coverage tool is configured. The last column says where a [manual test guide](manual-test-plan.md#guides) checks the area by hand.

| Area | Gap | Manual coverage |
| ---- | --- | --------------- |
| Tauri commands (`commands.rs`) | The 19 command handlers, state access, transfer interruption and native file dialogs are not tested. Their logic is tested through `Session`. | Guides A, B and D |
| Auto-lock wiring (`autolock.rs`, `os_lock/`) | The idle timer thread, the UI reset event and the OS lock monitors are not tested. Only the Windows message mapping has a test; the macOS and Linux monitors are untested on every OS. | Guide B (auto-lock), guide D (hibernation, remote sessions) |
| Real clipboard | Tests use a fake clipboard. Windows, macOS, X11 and Wayland behaviour is not tested automatically. | Guide B (clipboard), guide C (per-OS) |
| Real Tauri IPC | UI tests replace the shell with `mockIPC`, so a mismatch between UI and Rust command names, arguments or error codes would not be caught. | All guides, by using the real app |
| End-to-end and WebView | The UI never ran inside the real window for these reports. CSP, `freezePrototype` and the capability set were not exercised. | Guide C (webview hardening, developer console) |
| Core library | No integration tests under `tests/`, no runnable doc examples, no fuzzing or property-based tests. Vault file protection on Windows has no test (the permission test is Unix-only). | Guide B (damaged files), guide C (files at rest) |
| UI modules | Components, password strength, recovery code flow, item and settings sections are covered only indirectly through the screen tests. | Guides A and D |
| Accessibility and visuals | Only static lint checks and role queries. No screen reader, keyboard-only, contrast, theme or layout testing. | Guide C (UI and accessibility) |
| Platforms and packaging | Local runs were Windows only. macOS and Linux run only in CI, and CI does not run `tauri build` or build installers. | Guide C (per-OS behaviour), guide A (release build) |
| Coverage | No line or branch coverage is measured for Rust or the UI. | – |
