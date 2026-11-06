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

## 3. Settings and recovery code

Start: the test vault, unlocked, on **Settings**. Write down every new password and recovery code.

### 3.1 Master password and recovery code

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| D-SE-01 | Regeneration needs the current secret | 1. In **Recovery code**, leave the current-secret field empty and select **Generate new recovery code**. | "Enter your current master password or recovery code." Focus moves to the empty field, and no new code is generated: the current code still unlocks. | ☐ Pass ☐ Fail | |
| D-SE-02 | Regenerate using the recovery code itself | 1. In **Recovery code**, choose **Recovery code** under "Confirm it is you with" and type the current code ("old code").<br>2. Select **Generate new recovery code**, then save, tick the box, retype and confirm the new code.<br>3. Lock and unlock with the old code, then with the new code. | Step 2: "Generating…", then the new code, then "Your new recovery code is saved and confirmed." Step 3: the old code gives "That recovery code is not correct. It may have been replaced by a newer one."; the new code unlocks. | ☐ Pass ☐ Fail | |
| D-SE-03 | Malformed recovery code as the current secret | 1. In **Master password**, choose **Recovery code**, type `ABCD-EFGH`, fill both new password fields with `orbit-lantern-violet-canyon-42`, and select **Change master password**.<br>2. In **Recovery code**, choose **Recovery code**, type `ABCD-EFGH` and select **Generate new recovery code**. | Both times: "That recovery code is not valid. Check it for typos: it has 8 groups of 4 characters." The current-secret field is emptied, and nothing changes: the master password and the recovery code both still unlock. | ☐ Pass ☐ Fail | |
| D-SE-04 | New password equal to the current one | 1. Type `correct-horse-battery-staple-7` as the current password and as both new passwords.<br>2. Select **Change master password**, then lock and unlock with it. | "Master password changed. Use the new password next time you unlock." The same password still unlocks. Reusing the current password is allowed. | ☐ Pass ☐ Fail | |
| D-SE-05 | Leaving Settings during the new-code flow | 1. Write down the current code ("old code"). Generate a new code and write it down, but stay on "Save your recovery code".<br>2. Select **Vault** in the header, then **Settings** again.<br>3. Lock and unlock with the old code, then with the code from step 1. | Step 2: the app leaves Settings without a warning. Back on **Settings**, **Recovery code** shows the **Generate new recovery code** form again; the new code is not shown again. Step 3: the old code gives "That recovery code is not correct. It may have been replaced by a newer one."; the code from step 1 unlocks. The new code is active as soon as it is generated. | ☐ Pass ☐ Fail | |
| D-SE-06 | Auto-lock during the first recovery code step | Run after a backup, on a fresh install (guide A, section 2.2).<br>1. Create a vault and write down the code shown on "Save your recovery code". Look at the header.<br>2. Leave the app alone on that screen for 5 minutes 10 seconds.<br>3. Unlock with the master password, lock, and unlock with the code from step 1. | Step 1: the header shows no **Vault**, **Settings** or **Lock**, although the vault is already unlocked. Step 2: the window reloads to "Unlock your vault" after the 5-minute inactivity timeout; the code is not shown again. Step 3: both unlock. Restore the backup afterwards. | ☐ Pass ☐ Fail | |
| D-SE-07 | Focus after a wrong current secret | Keyboard only.<br>1. In **Master password**, type a wrong current password and two matching strong new passwords, and press Enter.<br>2. Repeat in **Recovery code** with a wrong password. | Each time the error message appears and the current-secret field is emptied. The app tries to move focus back to that field; whether it lands there in the real webview is an open question in the code. Record where focus is (start typing and see where the text goes). | ☐ Pass ☐ Fail | |

### 3.2 Settings file

Each case edits `settings.json` in the data directory with the app closed. The settings file is read only when the app starts. Restore the original file (or the backup) after each case.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| D-SE-08 | Values at the limits | 1. Write `{"autoLockMinutes": 60, "clipboardClearSeconds": 5}`, start, unlock and open **Settings**.<br>2. Copy a password on **Vault**.<br>3. Repeat with `"clipboardClearSeconds": 300`.<br>4. Repeat with `"clipboardClearSeconds": 4`, then `301`, then `"autoLockMinutes": 61`. | Step 1: "1 hour" and "5 seconds" are selected; "5 seconds" is added to the choices. Step 2: "Copied. The clipboard clears in 5 seconds." Step 3: "5 minutes" is selected and listed among the choices. Step 4: each value is out of range, so the whole file is ignored and both defaults apply ("5 minutes" and "30 seconds"). | ☐ Pass ☐ Fail | |
| D-SE-09 | Missing or unknown keys | 1. Write `{"autoLockMinutes": 15}`, start, unlock and open **Settings**.<br>2. Write `{"autoLockMinutes": 15, "clipboardClearSeconds": 60, "theme": "dark"}` and repeat. | Step 1: "15 minutes" and "30 seconds": a missing clipboard delay takes its default and the timeout is kept. Step 2: "5 minutes" and "30 seconds": a file with an unknown key is ignored as a whole. | ☐ Pass ☐ Fail | |
| D-SE-10 | Editing the file while the app runs | 1. Start the app and unlock.<br>2. In `settings.json`, set `"clipboardClearSeconds": 120` and save the file.<br>3. Copy a password.<br>4. In **Settings**, change **Lock after inactivity** to "15 minutes" and select **Save**. Open `settings.json` again. | Step 3: "The clipboard clears in 30 seconds." (or whatever was set when the app started): the edit is not read. Step 4: the file holds `"autoLockMinutes": 15` and the clipboard delay the app started with; the hand edit is overwritten. | ☐ Pass ☐ Fail | |
| D-SE-11 | The settings file cannot be written | 1. Quit. Delete `settings.json` if it exists and create an empty folder named `settings.json` in its place.<br>2. Start, unlock and open **Settings**.<br>3. Choose "15 minutes" and select **Save**.<br>4. Quit, delete the folder and any `settings.json.tmp` file. | Step 2: the defaults are shown. Step 3: "The vault file could not be read or written. Check the disk and try again." (the same message as for the vault file). **Save** stays enabled and the timeout in force is still 5 minutes. A `settings.json.tmp` file may be left in the data directory; record whether it is. | ☐ Pass ☐ Fail | |
| D-SE-12 | Unknown theme value in storage | Development build.<br>1. Unlock, open the developer console (guide C, section 1.4) and run `localStorage.setItem('novavault.theme', 'purple')`.<br>2. Reload the window (F5 or Ctrl+R) and open **Settings**. | **Match system** is selected under **Appearance**, and the app follows the OS theme. Choosing any theme writes a valid value again. | ☐ Pass ☐ Fail | |

## 4. Items and the item list

Start: the test vault, unlocked, on **Vault**, with at least three items, one of them a password item `Mail account`.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| D-IT-01 | Count wording | 1. Type a search that matches exactly one item.<br>2. In a vault with a single item (delete the others after a backup, or use a fresh vault), clear the search. | Step 1: "1 of &lt;total&gt; items". Step 2: "1 item". | ☐ Pass ☐ Fail | |
| D-IT-02 | Website is free text, not a link | 1. Select **New password**, title `Not a URL`, Website `not a url`, and save.<br>2. Click the website value in the details. | Step 1: "Item added." The website is saved as typed; there is no URL check. Step 2: the value is plain text. Clicking it opens nothing, in the app or in a browser. | ☐ Pass ☐ Fail | |
| D-IT-03 | New item hidden by the filter | 1. Choose **Notes** in **Type**.<br>2. Select **New password**, title `Filtered out`, and save. | "Item added." and the new item's details are shown, but the list shows notes only, without `Filtered out`, and the count reads "&lt;notes&gt; of &lt;total&gt; items" with the total increased by one. Choosing **All types** lists it. | ☐ Pass ☐ Fail | |
| D-IT-04 | Leaving the vault screen resets it | 1. Type a search, choose a type and select an item.<br>2. Select **Settings**, then **Vault**. | The search is empty, **All types** is selected, no item is selected and "Select an item to see its details, or add a new one." is shown. | ☐ Pass ☐ Fail | |
| D-IT-05 | Unlocking always opens the vault screen | 1. Open **Settings** and select **Lock**.<br>2. Unlock. | "Your vault" is shown, not **Settings**. | ☐ Pass ☐ Fail | |
| D-IT-06 | Escape closes both confirmations | 1. Select an item that has an attachment (attach `one.bin` first if needed).<br>2. Select **Delete** (item), then **Remove** next to the attachment, so that both questions are shown.<br>3. Press Escape once. | Both "Delete this item?" and "Remove this attachment?" close, and nothing is deleted. | ☐ Pass ☐ Fail | |

## 5. Attachments

Start: the test vault, unlocked, with a note `Attachment test`, the test files from guide B, section 2.3, in `nv-test`, and an empty `nv-export` folder. Restore the backup after this section.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| D-AT-01 | Same file attached twice | 1. Attach `chunk.bin` to `Attachment test` twice.<br>2. Remove the first `chunk.bin`.<br>3. Export the remaining one and compare its checksum with the original. | Step 1: both are listed with the same name and size and different times, and two new `.nvatt` files exist. Step 2: "Removed “chunk.bin”." and one entry is left. Step 3: the checksums match. | ☐ Pass ☐ Fail | |
| D-AT-02 | Unreadable source file | 1. Make `one.bin` unreadable: `chmod 000 ~/nv-test/one.bin` (macOS and Linux), or `icacls "$HOME\nv-test\one.bin" /deny "${env:USERNAME}:(R)"` (Windows).<br>2. Attach it.<br>3. Undo step 1 (`chmod 644 …`, or `icacls "$HOME\nv-test\one.bin" /remove:d "$env:USERNAME"`). | Step 2: "The file could not be read or saved there. Check the file or choose another place." Nothing is attached and no new `.nvatt` file appears. | ☐ Pass ☐ Fail | |
| D-AT-03 | Export to a folder you cannot write to | 1. Create `nv-readonly` in `nv-test` and make it read-only: `chmod 500 ~/nv-test/nv-readonly` (macOS and Linux), or `icacls "$HOME\nv-test\nv-readonly" /deny "${env:USERNAME}:(OI)(CI)(W)"` (Windows).<br>2. Export `chunk.bin` into it.<br>3. Undo step 1. | Step 2: either the dialog refuses the folder, or the app shows "The file could not be read or saved there. Check the file or choose another place." No file is written there. Record which. | ☐ Pass ☐ Fail | |
| D-AT-04 | Export into the attachments folder is refused | 1. List `vault.nvault.attachments` and compute the SHA-256 of one `.nvatt` file.<br>2. Export `chunk.bin`, navigate to the attachments folder in the dialog and save it as `chunk.bin`.<br>3. Export it again over the `.nvatt` file from step 1, confirming the replacement if asked.<br>4. List the folder and compute the checksum again. | Steps 2 and 3: "The file could not be read or saved there. Check the file or choose another place." Step 4: no `chunk.bin` in the folder, and the `.nvatt` file's checksum is unchanged. Every attachment still exports. | ☐ Pass ☐ Fail | |
| D-AT-05 | A transfer is not idle time | 1. Set **Lock after inactivity** to "1 minute".<br>2. Select `Attachment test`, select **Attach file** and start timing. Leave the dialog open for about 50 seconds.<br>3. Pick `large.bin` and note when the transfer ends. It must take more than 10 seconds; if it is faster, use a larger file.<br>4. Leave the app alone. | The vault does not lock during the transfer, although the 1-minute mark passes, and `large.bin` is attached. It locks about 60 to 65 seconds after the transfer ends. Set the timeout back to "5 minutes". | ☐ Pass ☐ Fail | |
| D-AT-06 | Attachment names that Windows cannot use | Optional: needs macOS or Linux and a Windows machine, both with builds of the same commit.<br>1. On macOS or Linux, create files named `notes:v2.txt` and `report.` in `nv-test` and attach both.<br>2. Copy the whole data directory to Windows (as in B-PS-05) and unlock.<br>3. Export both into `nv-export`. | Record the file name the save dialog suggests, the name and folder of the saved file, and any error. The file must land only in the folder you chose, never on another drive or in an alternate data stream. How such names are handled on Windows is an open question in the code. | ☐ Pass ☐ Fail | |
