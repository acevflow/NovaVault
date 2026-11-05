# Rust test report

Results of the Rust format, lint and test checks, run locally on one Windows machine. No application code was changed for this report.

| | |
| --- | --- |
| Date | 2026-11-05 |
| Commit | `339935d` (`main`, merge of PR #32) |
| Result | `cargo fmt --check`: **pass** · `cargo clippy`: **pass** · `cargo test --workspace`: **fail** (environment, not a test failure) |

## Environment

| Tool | Version |
| ---- | ------- |
| OS | Windows 11 Home 10.0.26200, x86_64 |
| git | 2.55.0.windows.3 |
| rustc | 1.99.1 (58cf8f9c3 2026-10-13), host `x86_64-pc-windows-msvc`, LLVM 23.1.1 |
| cargo | 1.99.1 (5f94df478 2026-08-27) |
| rustup | 1.29.0, toolchain `stable-x86_64-pc-windows-msvc` (default) |
| rustfmt | 1.10.0-stable |
| clippy | 0.1.99 |
| Node.js / npm | 24.19.0 / 12.0.2 |
| Tauri CLI (`@tauri-apps/cli`) | 2.12.4 |
| Locked Tauri crates | `tauri` 2.11.6, `tauri-build` 2.6.3, `wry` 0.55.1, `tao` 0.35.3 |

### Tauri 2 prerequisites (Windows)

Checked with `npx tauri info`, `vswhere` and the registry.

| Prerequisite | Status |
| ------------ | ------ |
| Microsoft C++ Build Tools (MSVC) | Present: Visual Studio Community 2022 17.14, with the VC x86/x64 tools |
| Windows SDK | Present: 10.0.26100.0 |
| WebView2 Runtime | Present: 156.0.4314.58 |
| Rust MSVC toolchain | Present: `stable-x86_64-pc-windows-msvc` |
| Node.js and npm (for the UI and the Tauri CLI) | Present |
| Tauri CLI | Present as the npm dev dependency (`npx tauri`). `cargo tauri` (the `tauri-cli` crate) is not installed. The project does not need it, because its scripts use the npm CLI. |

**Missing:** nothing the project needs. `npx tauri info` run from the repository root lists `@tauri-apps/api` and the JS plugins as "not installed". That is expected, because they are dependencies of `ui/`, and `ui/node_modules/@tauri-apps/api` is present. It also says newer Tauri crate versions exist. That is information, not a missing prerequisite.

## Commands

Run from the repository root in this order. Each command ran even when an earlier one failed.

| # | Command | Exit code | Result |
| - | ------- | --------- | ------ |
| 1 | `cargo fmt --check` | 0 | Pass, no diff |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | Pass, no warnings (1 min 05 s) |
| 3 | `cargo test --workspace` | 101 | **Fail**: the core unit-test binary was blocked by the OS (see [Failures](#failures)) |

Two more runs were made only to diagnose the failure. They are not a substitute for command 3:

| # | Command | Exit code | Purpose |
| - | ------- | --------- | ------- |
| 3a | `cargo test --workspace --no-fail-fast` | 101 | Run every test target, not just the targets before the first failure |
| 3b | `cargo test -p novavault-core` | 0 | Find out whether the core tests pass when they are built on their own |

## Results per crate

### `cargo test --workspace` (command 3)

| Crate | Target | Passed | Failed | Ignored | Status |
| ----- | ------ | ------ | ------ | ------- | ------ |
| `novavault` (`src-tauri`) | unit tests (`src\main.rs`) | 51 | 0 | 0 | Pass |
| `novavault-core` | unit tests (`src\lib.rs`) | 0 | 0 | 0 | **Not executed**: the OS blocked the binary |
| `novavault-core` | doc tests | – | – | – | Not reached, because cargo stops at the first failing target |

With `--no-fail-fast` (3a), the results are the same, and the core doc tests run with 0 tests (the core has no runnable doc examples, only `text` blocks).

### `cargo test -p novavault-core` (diagnostic 3b)

| Crate | Target | Passed | Failed | Ignored | Status |
| ----- | ------ | ------ | ------ | ------- | ------ |
| `novavault-core` | unit tests (`src\lib.rs`) | 128 | 0 | 0 | Pass (51.7 s) |
| `novavault-core` | doc tests | 0 | 0 | 0 | Pass (no doc tests) |

The core source has 129 `#[test]` functions. On Windows, 128 of them are compiled. `atomic::tests::file_is_private_to_its_owner` is `#[cfg(unix)]` and checks that vault files get mode `0600`, so it never runs on Windows.

### Totals

| | Tests passed | Tests failed | Not run |
| - | ------------ | ------------ | ------- |
| Required run (command 3) | 51 | 0 | 128 core unit tests (blocked) |
| Including diagnostic 3b | 179 | 0 | 1 (`#[cfg(unix)]`, Windows only) |

No test assertion failed in any run.

## Failures

### `cargo test --workspace`: core test binary blocked by Windows Application Control

```text
     Running unittests src\lib.rs (target\debug\deps\novavault_core-bee834a697104ae6.exe)
error: test failed, to rerun pass `-p novavault-core --lib`

Caused by:
  could not execute process `D:\Projects\NovaVault\target\debug\deps\novavault_core-bee834a697104ae6.exe` (never executed)

Caused by:
  Una directiva de Control de aplicaciones bloqueó este archivo. (os error 4551)
```

(The OS message is Spanish for "An Application Control policy has blocked this file.")

- **Cause:** a local Windows Application Control (WDAC / Smart App Control) policy refused to start the freshly built test executable. The process never ran, so no test reported a failure. This is a local OS policy, not a defect in NovaVault, and it was not worked around.
- **Why the core passes on its own:** the workspace build and the `-p novavault-core` build produce different test binaries (`…-bee834a697104ae6.exe` and `…-f4e0d4685a6ea509.exe`), because cargo unifies dependency features differently across the workspace. The policy blocks one and allows the other. Earlier local runs blocked other binaries (build scripts, a proc-macro DLL, and the shell test binary), so which file gets blocked changes from run to run.
- **Effect:** the required command fails locally. CI runs the same command (`cargo test --workspace --locked`) on Windows, macOS and Linux. The latest CI run on `main` (run 42098475466, for PR #32) passed.

## Warnings

- `cargo fmt --check`: none.
- `cargo clippy --workspace --all-targets -- -D warnings`: none. Workspace lints `missing_docs = "warn"`, `unwrap_used` and `expect_used = "deny"` are on.
- `cargo test`: no compiler warnings.
- Tooling note: `npx tauri info` says that newer versions of `tauri`, `tauri-build`, `wry`, `tao`, `tauri-plugin-dialog` and `tauri-plugin-fs` exist. Upgrading them was not part of this task.

## Untested areas

These are from reading which files define `#[test]` functions. No coverage tool was run.

### `src-tauri` (shell)

| Area | Lines | Tests | Notes |
| ---- | ----- | ----- | ----- |
| `commands.rs` (all 19 `#[tauri::command]` handlers, `AppState`, `run`, the `pick_file`/`save_file` dialogs) | 712 | 0 | The logic behind the commands is tested through `Session` in `session.rs`. The Tauri wiring is not tested: argument names, `AppHandle` and state access, blocking-task dispatch, transfer interruption and the native dialogs. `main.rs` checks only that the capability file grants exactly the declared commands. |
| `autolock.rs` (`lock_and_reset_ui`, `start_idle_timer`, `LockReason`) | 99 | 0 | The idle-timeout logic is tested in `session.rs`. The timer thread and the UI reset event are not. |
| `os_lock/mod.rs` (`platform_monitor`, `LockMonitor`, `MonitorError`) | 83 | 0 | |
| `os_lock/windows.rs` | 185 | 1 | Only the message-to-reason mapping (`reason_for`) is tested. The hidden listener window and the message loop are not. |
| `os_lock/linux.rs` (logind and screen-saver D-Bus watchers) | 169 | 0 | Not compiled on Windows. Untested on every OS. |
| `os_lock/macos.rs` (NSWorkspace / distributed notifications) | 82 | 0 | Not compiled on Windows. Untested on every OS. |
| `os_lock/unsupported.rs` | 16 | 0 | Fallback for other OSes. |
| Real clipboard | – | – | `clipboard.rs` tests use a fake clipboard. Behaviour against the real OS clipboard (Windows, macOS, X11, Wayland) is not tested automatically. |

### `novavault-core`

Every module except `lib.rs` (re-exports only) and `test_util.rs` (test helper) has unit tests: `aead` 8, `atomic` 5, `attachment` 12, `error` 3, `format` 9, `header` 10, `item` 16, `kdf` 6, `keys` 6, `recovery` 11, `slots` 15, `vault` 28. Gaps:

- **No integration tests.** There is no `crates/novavault-core/tests/` directory, so the public API is tested only from inside the crate.
- **No runnable doc examples.** All 0 doc tests: the code blocks are `text` diagrams.
- **Unix file permissions are not tested on Windows** (`file_is_private_to_its_owner` is `#[cfg(unix)]`). There is no matching test for how vault files are protected on Windows.
- **Not used:** no fuzzing of the vault, header or attachment parsers, no property-based tests, and no coverage measurement.

### Locally

- The core unit tests did not run as part of the required workspace command, because of the failure above.
- `cargo test --workspace --locked` and `cargo clippy --locked`, the exact CI variants, were not run locally. The commands above were run as requested, without `--locked`.

## Bugs found

None. No test failed, and no defect was seen while preparing this report.
