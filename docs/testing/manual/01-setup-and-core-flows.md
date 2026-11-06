# Manual test guide A: setup and core flows

Manual test procedure for running NovaVault and checking its core flows: vault creation, unlocking and locking, master password change, recovery code regeneration, and items (passwords, notes and secrets). It covers only features that exist in the code today. Attachments, auto-lock, clipboard auto-clear and appearance settings are not covered here.

Automated results are in the [Rust test report](../reports/rust.md) and the [UI test report](../reports/ui.md). This guide checks what those tests cannot: the real app, on a real OS, used by a person.

## Recording results

Fill in this table once per test run, then mark each case **Pass** or **Fail** and write what you saw in **Notes** (always for a failure: what happened instead, and any error message, word for word).

| | |
| --- | --- |
| Tester | |
| Date | |
| Commit tested | |
| Build | Development (`npm run dev`) / Release (installed bundle) |
| OS and version | |

Never use real passwords or personal data while testing. Use made-up values such as those suggested in the steps, and a vault that holds nothing of value.

## 1. Running the app

### 1.1 Prerequisites

Install Git, the Rust stable toolchain, Node.js LTS and the Tauri 2 system prerequisites for your OS, as described in the [development guide](../../development.md#prerequisites-all-platforms). Then get the code and install the dependencies from the repository root:

```sh
git clone https://github.com/acevflow/NovaVault.git
cd NovaVault
npm ci
npm ci --prefix ui
```

The commands in this section are the same on Windows (PowerShell or Command Prompt), macOS and Linux.

### 1.2 Development build

```sh
npm run dev
```

This starts the Vite dev server on `http://localhost:5173` and then compiles and opens the Tauri window. The first run compiles all Rust dependencies and can take several minutes. Changes to the UI reload in place; changes to Rust code rebuild and restart the app. Stop it with `Ctrl+C` in the terminal or by closing the window.

The development build uses a more permissive Content Security Policy (it allows the dev server), so security-related checks should also be repeated on a release build.

### 1.3 Release build

```sh
npm run build
```

This builds the UI into `ui/dist`, compiles the app in release mode and creates installers under `target/release/bundle/`. The unbundled executable is `target/release/novavault` (`novavault.exe` on Windows).

| OS | Bundles created | How to install and start |
| --- | --- | --- |
| Windows | `bundle/msi/*.msi` and `bundle/nsis/*-setup.exe` | Run either installer, then start **NovaVault** from the Start menu. |
| macOS | `bundle/macos/NovaVault.app` and `bundle/dmg/*.dmg` | Open the `.dmg` and drag NovaVault to Applications, or open `NovaVault.app` directly. The app is not signed, so the first start may need right-click, **Open**. |
| Linux | `bundle/deb/*.deb`, `bundle/rpm/*.rpm` and `bundle/appimage/*.AppImage` | Install the `.deb` or `.rpm` with your package manager, or make the AppImage executable (`chmod +x`) and run it. |

The bundles are not code-signed. Expect OS warnings (Windows SmartScreen, macOS Gatekeeper) and accept them only for builds you made yourself.

If a build fails on Windows with `os error 4551`, a Windows Application Control policy is blocking build scripts; see [Troubleshooting](../../development.md#troubleshooting) in the development guide.

### 1.4 Where the vault is stored

NovaVault manages one vault per user account. The shell chooses its location; the UI never sends a path. The data directory is the app data directory for the identifier `io.github.acevflow.novavault`:

| OS | Data directory |
| --- | --- |
| Windows | `%APPDATA%\io.github.acevflow.novavault\` |
| macOS | `~/Library/Application Support/io.github.acevflow.novavault/` |
| Linux | `~/.local/share/io.github.acevflow.novavault/` (or `$XDG_DATA_HOME/io.github.acevflow.novavault/`) |

It can contain:

| File | Contents |
| --- | --- |
| `vault.nvault` | The encrypted vault: key slots and all items. Created when the vault is created. |
| `vault.nvault.attachments/` | Encrypted attachment files, if any were added. |
| `settings.json` | Auto-lock timeout and clipboard clear delay. Not secret. Missing until a setting is changed; defaults are used then. |
| `.vault.nvault.<hex>.tmp` | Temporary file written during a save and renamed over `vault.nvault`. It should only exist for an instant; one left behind after the app has closed is worth a note. |

The development and release builds use the **same** data directory, so they open the same vault.

The theme preference (light, dark or system) is not in this directory: it is kept in the webview's local storage under the key `novavault.theme`.

## 2. Preconditions and clean state

### 2.1 Preconditions

Before every run:

- The app builds and starts on this machine (section 1).
- You use a test OS account or a machine with no vault you care about. If a vault already exists, back it up first (section 2.3).
- No other copy of NovaVault is running. Close every NovaVault window, and stop `npm run dev` with `Ctrl+C`.
- You have something to write on (paper or a plain text file kept only for testing) to record the test master passwords and recovery codes. Never reuse them anywhere.

Suggested test data:

| Purpose | Value |
| --- | --- |
| Master password | `correct-horse-battery-staple-7` |
| New master password (section 3.3) | `orbit-lantern-violet-canyon-42` |
| Weak password | `password1` |
| Accented title (search) | `Café Wi-Fi` |

### 2.2 Reset to a fresh install

A fresh install means the data directory does not exist, so the app opens on **Create your vault**.

1. Quit NovaVault.
2. Back up the data directory if it holds a vault you want to keep (section 2.3).
3. Delete the data directory:

   | OS | Command |
   | --- | --- |
   | Windows (PowerShell) | `Remove-Item -Recurse -Force "$env:APPDATA\io.github.acevflow.novavault"` |
   | macOS | `rm -rf ~/Library/Application\ Support/io.github.acevflow.novavault` |
   | Linux | `rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/io.github.acevflow.novavault"` |

4. Optional: to also reset the theme preference, open **Settings** in the unlocked vault and choose **Match system** under **Appearance** before step 1. Deleting the data directory does not reset it.
5. Start the app. **Expected:** the **Create your vault** screen.

Deleting the data directory deletes the vault and every attachment for good. NovaVault keeps no other copy.

### 2.3 Back up a test vault

Use this to keep a vault in a known state (for example with sample items) and return to it between cases.

1. Quit NovaVault, so no save is in progress.
2. Copy the whole data directory (not only `vault.nvault`: attachments and settings live next to it) to a folder outside it, for example `nv-backup-<date>`:

   | OS | Command |
   | --- | --- |
   | Windows (PowerShell) | `Copy-Item -Recurse "$env:APPDATA\io.github.acevflow.novavault" "$HOME\nv-backup"` |
   | macOS | `cp -R ~/Library/Application\ Support/io.github.acevflow.novavault ~/nv-backup` |
   | Linux | `cp -R "${XDG_DATA_HOME:-$HOME/.local/share}/io.github.acevflow.novavault" ~/nv-backup` |

3. Write down the master password and recovery code that open this backup; they are part of its state.

To restore, quit the app, delete the data directory (section 2.2, step 3), copy the backup back to the data directory path, and start the app. **Expected:** the **Unlock your vault** screen, and the recorded master password opens the vault.

A backup taken before a master password change or recovery code regeneration still opens with the old secrets: restoring it undoes the change.

### 2.4 Delete a test vault

When a run is finished, delete the test vault with section 2.2, step 3, and delete any backups and notes of test secrets you made.

## 3. Test cases

Run the groups in order: each one starts from the state the previous one leaves. Text in quotes is the exact wording the app should show. "Fresh install" means section 2.2 was just done.

Mark each case by ticking one box (`☒ Pass` or `☒ Fail`), and write what you saw in **Notes**.

### 3.1 Vault creation

Start: fresh install, app open on **Create your vault**.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| A-CR-01 | First start shows the create screen | 1. Start the app after a fresh install. | Heading "Create your vault", fields **Master password** and **Confirm master password**, and a **Create vault** button. Below the first field: "Use a long passphrase of several unrelated words. It must rate at least “Strong”." No navigation and no **Lock** button in the header. | ☐ Pass ☐ Fail | |
| A-CR-02 | Empty password is rejected | 1. Leave both fields empty.<br>2. Select **Create vault**. | Error "Enter a master password." No vault file is created in the data directory. | ☐ Pass ☐ Fail | |
| A-CR-03 | Strength feedback updates while typing | 1. Type `password1` in **Master password**, one character at a time.<br>2. Clear it and type `correct-horse-battery-staple-7`. | A strength meter and "Password strength: …" appear as soon as you type, briefly "checking…" the first time. `password1` rates "Very weak" or "Weak" with a warning or tips below. The long passphrase rates "Strong" or "Very strong". The label updates as you type. | ☐ Pass ☐ Fail | |
| A-CR-04 | Weak password is rejected | 1. Type `password1` in both fields.<br>2. Select **Create vault**. | Error "The password must rate at least “Strong”. Make it longer or add unrelated words." No vault is created. | ☐ Pass ☐ Fail | |
| A-CR-05 | Mismatched confirmation is rejected | 1. Type `correct-horse-battery-staple-7` in **Master password** and `correct-horse-battery-staple-8` in **Confirm master password**.<br>2. Select **Create vault**. | "The passwords do not match." under the confirmation field. No vault is created. | ☐ Pass ☐ Fail | |
| A-CR-06 | Show and hide the password | 1. With text in **Master password**, select **Show** in the field.<br>2. Select **Hide**. | **Show** displays the password as plain text and the button reads **Hide**; **Hide** masks it again. Each field has its own toggle. | ☐ Pass ☐ Fail | |
| A-CR-07 | No way to cancel the password step | 1. Look for a cancel, back or close action on the create screen. | There is none: the only action is **Create vault**. Creating a vault is the only way forward from a fresh install. | ☐ Pass ☐ Fail | |
| A-CR-08 | Vault is created | 1. Type `correct-horse-battery-staple-7` in both fields.<br>2. Select **Create vault**. | The button reads "Creating vault…", "Deriving encryption keys. This takes a few seconds." appears and the fields are disabled. Then the heading "Save your recovery code" appears. `vault.nvault` now exists in the data directory. | ☐ Pass ☐ Fail | |
| A-CR-09 | Recovery code is shown in a readable format | 1. Look at the code on "Save your recovery code".<br>2. Write it down. | 8 groups of 4 characters, using only the digits `2`–`9` and capital letters other than `I` and `O`. The text says it is shown only this once, and the warning reads "If you lose both your master password and this code, your data cannot be recovered." There is no copy button. **Print** and **Continue** are shown. | ☐ Pass ☐ Fail | |
| A-CR-10 | Continue requires confirming the code was saved | 1. Try **Continue** without ticking the checkbox.<br>2. Tick "I have saved my recovery code somewhere safe". | **Continue** is disabled until the box is ticked, then enabled. | ☐ Pass ☐ Fail | |
| A-CR-11 | Print opens the print dialog | 1. Select **Print**.<br>2. Look at the preview, then cancel printing. | The OS print dialog opens. The preview shows the code and "NovaVault emergency recovery code, printed on &lt;date&gt;.", without the buttons, checkbox and warning. The screen is unchanged after cancelling. | ☐ Pass ☐ Fail | |
| A-CR-12 | Wrong retyped code is rejected | 1. Select **Continue**.<br>2. On "Confirm your recovery code", type the code with one character changed.<br>3. Select **Confirm**. | "That does not match the code shown. Check each group and try again." The flow stays on the confirm step. The error disappears as soon as you edit the field. | ☐ Pass ☐ Fail | |
| A-CR-13 | Back to the code | 1. On the confirm step, select **Show the code again**. | The same code is shown again, and the "I have saved…" box is unticked, so **Continue** is disabled again. | ☐ Pass ☐ Fail | |
| A-CR-14 | Retyped code accepts any case and separators | 1. Tick the box, select **Continue**.<br>2. Type the code in lower case, with spaces instead of dashes.<br>3. Select **Confirm**. | Accepted. The app opens "Your vault" with "Your vault is empty. Add a password, note or secret to get started." and "0 items". The header shows **Vault**, **Settings** and **Lock**. | ☐ Pass ☐ Fail | |
| A-CR-15 | The code is never shown again | 1. Browse the vault screen and **Settings**. | No screen shows the recovery code. **Settings** only offers to generate a new one. | ☐ Pass ☐ Fail | |
| A-CR-16 | Quitting during the recovery step | 1. Do a fresh install and repeat A-CR-08, writing the code down.<br>2. On "Save your recovery code", quit the app.<br>3. Start it again. | The app opens on "Unlock your vault", not on the create screen: the vault was created when the password was accepted. The password from step 1 unlocks it, and so does the code shown in step 1 (check with A-UL-04). | ☐ Pass ☐ Fail | |

### 3.2 Unlock and lock

Start: the vault from 3.1, master password `correct-horse-battery-staple-7`, recovery code written down. Lock it with **Lock** if it is open.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| A-UL-01 | Lock from the header | 1. While unlocked, select **Lock** in the header. | The button briefly reads "Locking…", then "Unlock your vault" appears. **Vault**, **Settings** and **Lock** disappear from the header. | ☐ Pass ☐ Fail | |
| A-UL-02 | Unlock with the correct master password | 1. On the **Master password** tab, type `correct-horse-battery-staple-7`.<br>2. Select **Unlock** (or press Enter). | The button reads "Unlocking…" for about a second, then "Your vault" shows the vault's items. | ☐ Pass ☐ Fail | |
| A-UL-03 | Wrong master password | 1. Lock.<br>2. Type `correct-horse-battery-staple-8` and select **Unlock**. | "That master password is not correct." The field is emptied and focused. The vault stays locked. | ☐ Pass ☐ Fail | |
| A-UL-04 | Unlock with the recovery code | 1. Select the **Recovery code** tab.<br>2. Type the recovery code (any case, with or without dashes or spaces).<br>3. Select **Unlock**. | The hint under the field reads "Forgot your master password? Unlock with your recovery code, then set a new password in Settings." The vault unlocks. | ☐ Pass ☐ Fail | |
| A-UL-05 | Mistyped recovery code | 1. Lock. On **Recovery code**, type the code with one character changed.<br>2. Select **Unlock**. | Almost immediately: "That recovery code is not valid. Check it for typos: it has 8 groups of 4 characters." The checksum catches the typo before the slow key derivation. The vault stays locked. | ☐ Pass ☐ Fail | |
| A-UL-06 | Incomplete recovery code | 1. Type only the first 4 groups and select **Unlock**. | The same "not valid" message as A-UL-05. | ☐ Pass ☐ Fail | |
| A-UL-07 | Wrong but well-formed recovery code | Run as part of A-KS-08, step 4: a replaced code is well-formed but no longer opens the vault. | "That recovery code is not correct. It may have been replaced by a newer one." | ☐ Pass ☐ Fail | |
| A-UL-08 | Empty field | 1. Leave the field empty and select **Unlock**, on each tab. | "Enter your master password." on the password tab; "Enter your recovery code." on the recovery code tab. | ☐ Pass ☐ Fail | |
| A-UL-09 | Repeated failures | 1. Enter a wrong master password and select **Unlock** 5 times in a row.<br>2. Enter the correct password. | Each attempt shows "That master password is not correct." after about the same time. There is no lockout and no growing delay; this is by design (see the known limitations in the [threat model](../../threat-model.md)). The correct password then unlocks the vault. | ☐ Pass ☐ Fail | |
| A-UL-10 | Switching method clears the field | 1. Type something on the **Master password** tab.<br>2. Switch to **Recovery code**, then back.<br>3. With a tab focused, press the Left and Right arrow keys. | The typed text is gone after each switch, and any error message disappears. The arrow keys move between the tabs. | ☐ Pass ☐ Fail | |
| A-UL-11 | Show the master password | 1. Type a password and select **Show**, then **Hide**. | The password is shown as plain text, then masked again. | ☐ Pass ☐ Fail | |
| A-UL-12 | Restart locks the vault | 1. Unlock.<br>2. Quit the app and start it again. | The app opens on "Unlock your vault". | ☐ Pass ☐ Fail | |

### 3.3 Master password change and recovery code regeneration

Start: unlocked vault from 3.2, on **Settings**. Write down every new password and code.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| A-KS-01 | Change requires the current secret | 1. In **Master password**, leave **Current master password** empty and fill both new password fields with `orbit-lantern-violet-canyon-42`.<br>2. Select **Change master password**. | "Enter your current master password or recovery code." Nothing changes. | ☐ Pass ☐ Fail | |
| A-KS-02 | Wrong current password | 1. Type `wrong-password-here` as the current password and the new password in both fields.<br>2. Select **Change master password**. | "The current password or recovery code is not correct. Nothing was changed." The current field is emptied. The password is unchanged: A-KS-04 then succeeds with the same current password. | ☐ Pass ☐ Fail | |
| A-KS-03 | New password rules apply | 1. Type the correct current password.<br>2. Try `password1` in both new fields, then two different strong passwords. | A strength meter shows under **New master password**. The weak password gives the "must rate at least “Strong”" error; different passwords give "The passwords do not match." Nothing changes. | ☐ Pass ☐ Fail | |
| A-KS-04 | Change the master password | 1. Current: `correct-horse-battery-staple-7`. New and confirmation: `orbit-lantern-violet-canyon-42`.<br>2. Select **Change master password**. | "Changing password…", then "Master password changed. Use the new password next time you unlock." All fields are emptied. | ☐ Pass ☐ Fail | |
| A-KS-05 | Old password stops working, new one works | 1. Lock.<br>2. Try `correct-horse-battery-staple-7`.<br>3. Try `orbit-lantern-violet-canyon-42`. | Step 2: "That master password is not correct." Step 3: the vault unlocks with all items unchanged. | ☐ Pass ☐ Fail | |
| A-KS-06 | Recovery code survives a password change | 1. Lock and unlock with the recovery code. | The vault unlocks: changing the password did not change the recovery code. | ☐ Pass ☐ Fail | |
| A-KS-07 | Change the password using the recovery code | 1. In **Master password**, choose **Recovery code** under "Confirm it is you with" and type the recovery code.<br>2. Set the new password back to `correct-horse-battery-staple-7`.<br>3. Lock and unlock with it. | The password is changed, and the new password unlocks the vault. Switching between **Master password** and **Recovery code** clears the current-secret field. | ☐ Pass ☐ Fail | |
| A-KS-08 | Regenerate the recovery code; the old one stops working | 1. Note the current code as "old code".<br>2. In **Recovery code**, read the warning, type the current master password and select **Generate new recovery code**.<br>3. Save, tick the box, retype and confirm the new code.<br>4. Lock and unlock with the old code.<br>5. Unlock with the new code. | Step 2: the warning reads "Your current recovery code stops working as soon as the new one is generated."; then a new code with "This new code replaces your previous one, which no longer works." Step 3: "Your new recovery code is saved and confirmed." Step 4: "That recovery code is not correct. It may have been replaced by a newer one." Step 5: the vault unlocks. | ☐ Pass ☐ Fail | |
| A-KS-09 | Regeneration with a wrong current secret | 1. Type a wrong current password and select **Generate new recovery code**. | "The current password or recovery code is not correct. Your recovery code was not changed." The current code still unlocks. | ☐ Pass ☐ Fail | |
| A-KS-10 | Old code cannot authorize a change | 1. In **Master password**, choose **Recovery code** and type the old code from A-KS-08.<br>2. Fill a new strong password in both fields and select **Change master password**. | "The current password or recovery code is not correct. Nothing was changed." | ☐ Pass ☐ Fail | |
| A-KS-11 | Leaving the new-code flow early | 1. Generate another code (A-KS-08, step 2) and write it down, then quit the app on "Save your recovery code" without confirming.<br>2. Start again and try the previous code, then the code just shown. | The previous code is rejected and the code just shown unlocks: the change takes effect when the code is generated, not when it is confirmed. | ☐ Pass ☐ Fail | |

### 3.4 Items

Start: unlocked, empty vault (a fresh vault from 3.1, or one with all items deleted), on **Vault**.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| A-IT-01 | Empty vault | 1. Open **Vault**. | "Your vault is empty. Add a password, note or secret to get started.", "0 items", and "Select an item to see its details, or add a new one." Buttons **New password**, **New note** and **New secret**. | ☐ Pass ☐ Fail | |
| A-IT-02 | Title is required | 1. Select **New password**.<br>2. Leave **Title** empty (or type only spaces) and select **Save**. | "Enter a title." on the title field, which gets focus. Nothing is added. | ☐ Pass ☐ Fail | |
| A-IT-03 | Cancel creating | 1. Select **New note**, type a title, select **Cancel**. | The form closes and "Select an item to see its details, or add a new one." is shown. No item is added. | ☐ Pass ☐ Fail | |
| A-IT-04 | Create a password | 1. Select **New password**.<br>2. Title `Mail account`, Username `tester@example.com`, Website `https://mail.example.com`, Password `S3cret-pass!`, Notes `Line one` and `Line two` on two lines.<br>3. Select **Save**. | "Item added." The item is listed with the label **Password** and selected. The details show Username and Website in plain text with **Copy**; Password and Notes masked (`••••••••`) with **Reveal** and **Copy**; and "Last updated" with the current time. | ☐ Pass ☐ Fail | |
| A-IT-05 | Create a note | 1. Select **New note**, title `Café Wi-Fi`, Text `Network: TestNet` and a second line.<br>2. Select **Save**. | "Item added." The details show only **Text** (masked) and "Last updated". | ☐ Pass ☐ Fail | |
| A-IT-06 | Create a secret | 1. Select **New secret**, title `API token`, Value `tok_1234567890`, leave **Notes** empty.<br>2. Select **Save**. | "Item added." **Value** is masked; **Notes** shows "Empty" with no **Reveal** button. | ☐ Pass ☐ Fail | |
| A-IT-07 | Secret inputs are masked while typing | 1. Open **New password**, then **New secret**. | **Password** and **Value** mask what you type and have **Show**/**Hide**. **Notes** and the note's **Text** are ordinary multi-line fields. | ☐ Pass ☐ Fail | |
| A-IT-08 | Reveal and hide a secret field | 1. Select `Mail account`.<br>2. Select **Reveal** on Password, then on Notes.<br>3. Select **Hide** on both. | Each **Reveal** shows only that field's value (notes keep their line breaks) and the button reads **Hide**. **Hide** masks it again. | ☐ Pass ☐ Fail | |
| A-IT-09 | Revealed values do not persist | 1. Reveal the password of `Mail account`.<br>2. Select another item, then `Mail account` again.<br>3. Reveal again, then **Lock** and unlock. | After steps 2 and 3, every secret field is masked again. | ☐ Pass ☐ Fail | |
| A-IT-10 | Edit keeps secret fields left empty | 1. Select `Mail account`, select **Edit**.<br>2. Change Username to `tester2@example.com` and leave Password and Notes empty.<br>3. Select **Save**, then reveal Password. | The heading reads "Edit “Mail account”". Password and Notes start empty, with "Leave empty to keep the current password." (and the same for notes) and **Reveal current password** / **Reveal current notes**. After saving: "Changes saved.", the new username, and the original password `S3cret-pass!`. | ☐ Pass ☐ Fail | |
| A-IT-11 | Edit replaces a secret value | 1. Edit `API token`, type `tok_new` in **Value**, save.<br>2. Reveal **Value**. | "Changes saved." The value is `tok_new` and "Last updated" has changed. | ☐ Pass ☐ Fail | |
| A-IT-12 | Load the current value while editing, and clear it | 1. Edit `Mail account` and select **Reveal current notes**.<br>2. Delete all the text and save. | Step 1 loads the stored notes into the field and the reveal button disappears. After saving, **Notes** shows "Empty". | ☐ Pass ☐ Fail | |
| A-IT-13 | Edit a note's title and text | 1. Edit `Café Wi-Fi`, change the title to `Café Wi-Fi guest`, type new text, save.<br>2. Reveal **Text**. | "Changes saved." The list and heading show the new title; the text is the new text. | ☐ Pass ☐ Fail | |
| A-IT-14 | Cancel editing | 1. Edit any item, change the title, select **Cancel**. | The item's details are shown again with the old title. Nothing is saved. | ☐ Pass ☐ Fail | |
| A-IT-15 | Delete asks for confirmation | 1. Select `API token` and select **Delete**. | "Delete this item?" and "“API token” will be deleted from the vault. This cannot be undone." with **Delete** and **Cancel**. Focus is on **Cancel**, so pressing Enter deletes nothing. | ☐ Pass ☐ Fail | |
| A-IT-16 | Cancel deletion with Cancel and with Escape | 1. Select **Cancel**.<br>2. Select **Delete** again and press Escape. | Both times the question closes and the item is still there. | ☐ Pass ☐ Fail | |
| A-IT-17 | Delete each kind | 1. Delete `API token` by selecting **Delete** in the confirmation.<br>2. Repeat for a note and for a password item created for this purpose. | "Deleting…", then "Item deleted.", and the item is gone from the list and the count. It is still gone after locking and unlocking. | ☐ Pass ☐ Fail | |
| A-IT-18 | Items persist across restarts | 1. Make sure there is at least one item of each kind.<br>2. Quit and restart the app, then unlock. | All items are listed with their titles, and their fields reveal the saved values. | ☐ Pass ☐ Fail | |
| A-IT-19 | List order | 1. Create items titled `beta`, `Alpha` and `gamma`. | The list is sorted by title, ignoring case: `Alpha` before `beta` before `gamma`. | ☐ Pass ☐ Fail | |
| A-IT-20 | Search by title | 1. Type `mail` in **Search by title**.<br>2. Type `MAIL`, then `mail` with spaces before and after. | Only items whose title contains "mail" are listed, whatever the case and ignoring surrounding spaces. The count reads "N of M items". | ☐ Pass ☐ Fail | |
| A-IT-21 | Search ignores accents | 1. Type `cafe`. | `Café Wi-Fi guest` is listed. | ☐ Pass ☐ Fail | |
| A-IT-22 | Search does not look inside fields | 1. Type `tester2` (a username), then `S3cret` (a password). | No item matches by username, website or any secret field: "No items match your search." | ☐ Pass ☐ Fail | |
| A-IT-23 | Filter by type | 1. Clear the search. In **Type**, choose **Passwords**, then **Notes**, then **Secrets**, then **All types**. | Each choice lists only that kind, with a matching count; **All types** lists everything. | ☐ Pass ☐ Fail | |
| A-IT-24 | Search and filter together | 1. Choose **Notes** and type `mail`.<br>2. Clear the search and choose **All types**. | Step 1: only notes whose title contains "mail", or "No items match your search." if there are none. Step 2: every item is listed again. | ☐ Pass ☐ Fail | |
| A-IT-25 | Copy a field | 1. Select a password item and select **Copy** on Password.<br>2. Paste into a text editor. | "Copied. The clipboard clears in …" appears, and the pasted text is the password. The automatic clearing is not covered by this guide. | ☐ Pass ☐ Fail | |
