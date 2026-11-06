# Manual test guide D: additional cases

Extra manual test cases for user-visible behaviour, error paths and settings that guides A to C do not cover: the startup error screen, unusual unlock input, the less common settings and recovery code paths, list and form details, attachment errors, and what happens with hibernation, remote sessions and two copies of the app. It covers only features that exist in the code today.

This guide builds on the three previous ones. Read them first; their procedures are not repeated here:

- [Manual test guide A: setup and core flows](01-setup-and-core-flows.md): building and starting the app (section 1), where the vault is stored (section 1.4), and how to reset, back up and restore a test vault (sections 2.2 and 2.3).
- [Manual test guide B: system behavior](02-system-behavior.md): the working folder and the `$VAULT` variable (section 2.1), checksums (section 2.2), test files such as `one.bin` and `large.bin` (section 2.3), and the clipboard, auto-lock and attachment cases.
- [Manual test guide C: security, UI and platforms](03-security-ui-platforms.md): the `$DATA` variable (section 1.2) and the developer console (section 1.4).

Automated results are in the [Rust test report](../reports/rust.md) and the [UI test report](../reports/ui.md).

## Recording results

Fill in this table once per test run, then mark each case **Pass** or **Fail** and write what you saw in **Notes** (always for a failure: what happened instead, and any error message, word for word).

| | |
| --- | --- |
| Tester | |
| Date | |
| Commit tested | |
| Build | Development (`npm run dev`) / Release (installed bundle) |
| OS and version | |
| Desktop session (Linux only) | For example GNOME on Wayland, KDE Plasma on X11 |

Never use real passwords, real files or personal data while testing. Use made-up values and a vault that holds nothing of value.

Mark each case by ticking one box (`☒ Pass` or `☒ Fail`), and write what you saw in **Notes**. Text in quotes is the exact wording the app should show. Some cases ask you to **record** a behaviour instead of checking a fixed result: they cover open questions in the code, and any clearly described outcome is a pass.

## 1. Preconditions

Before every run:

- The preconditions of guide A, section 2.1, are met: the app builds and starts, you use a test account or machine, and no other copy of NovaVault is running (except in section 6, which starts two on purpose).
- A test vault exists with the master password `correct-horse-battery-staple-7`, its recovery code written down, and at least one item of each kind. Guide A, sections 3.1 and 3.4, create one.
- The working folder `nv-test` with the test files, `$VAULT` (guide B, section 2.1) and `$DATA` (guide C, section 1.2) are set up.
- You have taken a backup of the data directory (guide A, section 2.3). Several cases change files in it; restore the backup after each of those.
- Settings are at their defaults (**Lock after inactivity** 5 minutes, **Clear the clipboard after** 30 seconds) unless a case says otherwise.

## 2. Startup and unlock

Start: the test vault, app closed.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| D-ST-01 | Startup error screen | macOS and Linux only; not as root.<br>1. Run `chmod 000 "$DATA"`.<br>2. Start the app.<br>3. Run `chmod 700 "$DATA"` and select **Try again**. | Step 2: the heading "NovaVault could not start", the message "The vault file could not be read or written. Check the disk and try again." and a **Try again** button. Step 3: "Unlock your vault", and the master password unlocks the vault. On Linux the webview may keep its own data in the same directory and fail to start instead; record what happened. | ☐ Pass ☐ Fail | |
| D-ST-02 | Vault removed while the app is open | 1. Start the app and stay on "Unlock your vault".<br>2. Move the data directory elsewhere (for example rename it to `nv-moved`).<br>3. Enter the master password and select **Unlock**.<br>4. Quit, start the app again, then quit and move the directory back. | Step 3: "No vault was found. Create a new vault to get started." The app stays on "Unlock your vault" and offers no way to create a vault from there. Step 4: the restarted app opens on "Create your vault"; after the directory is back, the vault unlocks normally. | ☐ Pass ☐ Fail | |
| D-ST-03 | Reloading the window while unlocked | 1. Development build: unlock, select an item and select **Edit**. Type a new title without saving.<br>2. Press F5 or Ctrl+R (Cmd+R on macOS) in the window.<br>3. Release build: try the same keys. | Step 2: the window reloads and shows "Your vault" directly, without asking for the master password: the vault stays unlocked in the Rust core, and only the window was reloaded. The edit is discarded and the item keeps its old title. Step 3: record whether the release build reloads at all. | ☐ Pass ☐ Fail | |
| D-ST-04 | Spaces in the master password count | Run after a backup, on a fresh install (guide A, section 2.2).<br>1. Create a vault with the master password `orbit-lantern-violet-canyon-42 ` (with one space at the end) in both fields.<br>2. Lock and unlock with `orbit-lantern-violet-canyon-42` (no space).<br>3. Unlock with the space. | Step 2: "That master password is not correct." Step 3: the vault unlocks. Spaces are kept as typed, never trimmed. Restore the backup afterwards. | ☐ Pass ☐ Fail | |
| D-ST-05 | Characters outside the recovery code alphabet | 1. Lock. On **Recovery code**, type the code with its first character replaced by `0`, then by `O`, then by `1`, then by `I`, selecting **Unlock** each time.<br>2. Type the correct code with dots instead of dashes (`ABCD.EFGH.…`) and select **Unlock**. | Every attempt shows, at once, "That recovery code is not valid. Check it for typos: it has 8 groups of 4 characters." The code uses neither `0`/`1` nor `O`/`I`, and unlocking ignores only dashes and spaces. Note: the retype step after creating a code ignores every character that is not a letter or digit, so dots are accepted there (A-CR-14) but not when unlocking. | ☐ Pass ☐ Fail | |
| D-ST-06 | Submitting before the strength check is ready | Run on a fresh install, right after starting the app, so that the strength checker has not loaded yet.<br>1. Paste `correct-horse-battery-staple-7` into both fields and press Enter at once. | If the check was still loading: "Password strength is still being checked. Try again in a moment." and no vault is created; pressing Enter again a moment later creates it. On a fast machine the check may finish first and the vault is created straight away. Record which happened. Restore the backup afterwards. | ☐ Pass ☐ Fail | |
