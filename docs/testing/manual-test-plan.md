# Manual test plan

The automated tests cannot check everything: the real window, the OS clipboard, auto-lock on suspend and screen lock, native file dialogs, files at rest, accessibility and per-OS behaviour all need a person at the keyboard. This plan lists the four manual test guides, gives one table to sum up a full run, and a template for reporting bugs. Automated results are in the [test report](test-report.md).

## How to run

1. Run the guides in order, A to D. Guide A builds the app and creates the test vault; guides B, C and D reuse its procedures and test files.
2. Fill in the **Recording results** table at the top of each guide, then mark every case **Pass** or **Fail** and write what you saw in **Notes**.
3. Copy the totals of each area into the [results summary](#results-summary) below.
4. Report every failure with the [bug report template](#bug-report-template).

Never use real passwords, real files or personal data while testing. Use made-up values and a vault that holds nothing of value.

| | |
| --- | --- |
| Tester | |
| Date | |
| Commit tested | |
| Build | Development (`npm run dev`) / Release (installed bundle) |
| OS and version | |
| Desktop session (Linux only) | For example GNOME on Wayland, KDE Plasma on X11 |

## Guides

| Guide | What it covers | Cases |
| ----- | -------------- | ----- |
| [A: setup and core flows](manual/01-setup-and-core-flows.md) | Running the app per OS, where the vault is stored, the clean-state procedure, vault creation, unlock and lock, master password change and recovery code regeneration, items. | 64 |
| [B: system behavior](manual/02-system-behavior.md) | Helper commands and test files, clipboard auto-clear, auto-lock, encrypted attachments, persistence, damaged files and unknown versions. | 74 |
| [C: security, UI and platforms](manual/03-security-ui-platforms.md) | Files at rest, logs and console, webview hardening, keyboard navigation, themes, window size, error messages, Windows, macOS and Linux behaviour, edge cases. | 104 |
| [D: additional cases](manual/04-additional-cases.md) | Startup error screen, unusual unlock input, settings and recovery code paths, item list and form details, attachment errors, hibernation, remote sessions and two copies of the app. | 35 |
| **Total** | | **277** |

## Results summary

One row per group of case IDs. **Total** is the number of cases in the guide. Fill in **Passed**, **Failed** and **Notes** (for example skipped cases and bug links). The per-OS rows of guide C apply only on that OS; write "not run" in **Notes** for the others.

| Area | Total | Passed | Failed | Notes |
| ---- | ----- | ------ | ------ | ----- |
| A-CR: vault creation | 16 | | | |
| A-UL: unlock and lock | 12 | | | |
| A-KS: master password change and recovery code | 11 | | | |
| A-IT: items | 25 | | | |
| B-CB: clipboard | 19 | | | |
| B-AL: auto-lock | 18 | | | |
| B-AT: attachments | 15 | | | |
| B-PS: persistence | 22 | | | |
| C-SC: security checks | 20 | | | |
| C-UI: UI and accessibility | 25 | | | |
| C-WIN: Windows | 8 | | | |
| C-MAC: macOS | 7 | | | |
| C-LNX: Linux | 14 | | | |
| C-ED: edge cases | 30 | | | |
| D-ST: startup and unlock | 6 | | | |
| D-SE: settings and recovery code | 12 | | | |
| D-IT: items and the item list | 6 | | | |
| D-AT: attachments | 6 | | | |
| D-OS: hibernation and remote sessions | 2 | | | |
| D-MI: two copies at once | 3 | | | |
| **Total** | **277** | | | |

## Bug report template

Open one GitHub issue per bug and paste the template below. Do **not** report security vulnerabilities in a public issue; follow [SECURITY.md](../../SECURITY.md) instead. Never paste real passwords, recovery codes or vault contents.

Severity:

- **Critical**: data loss, a secret exposed (on disk, in logs, in the clipboard after the timeout), or the vault cannot be opened with a valid secret.
- **High**: a core feature does not work (create, unlock, lock, items, attachments, password change, recovery code, auto-lock, clipboard clear).
- **Medium**: a feature works only partly, or there is a workaround.
- **Low**: wording, layout or cosmetic issues.

```markdown
**Title:** <case ID>: <short description of what went wrong>

| | |
| --- | --- |
| Case ID | e.g. B-AL-05 |
| Severity | Critical / High / Medium / Low |
| Build | Development / Release |
| Commit | |
| OS and version | |
| Desktop session (Linux) | |
| Frequency | Always / Sometimes (n of m tries) / Once |

**Preconditions**
State of the app and vault before the steps (fresh install, test vault, settings changed).

**Steps to reproduce**
1.
2.
3.

**Expected result**
What the guide says should happen.

**Actual result**
What happened instead. Copy any error message word for word.

**Attachments**
Screenshots, console output or log excerpts, with no secrets in them.

**Notes**
Anything else: workarounds, whether other OSes are affected, related cases.
```
