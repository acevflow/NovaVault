# Manual test guide C: security, UI and platforms

Manual test procedure for the checks that need a person looking at the real app and its files: that nothing secret is written to disk, to a log or to the developer console in clear; that the UI can be used with the keyboard, in both themes and at any window size, and reports errors clearly; how the app installs and behaves on Windows, macOS and Linux (including X11 and Wayland); and how it copes with empty, very long, non-Latin and rapidly repeated input. It covers only features that exist in the code today.

This guide builds on the two previous ones. Read them first; their procedures are not repeated here:

- [Manual test guide A: setup and core flows](01-setup-and-core-flows.md): building and starting the app (section 1), the release bundles per OS (section 1.3), where the vault is stored (section 1.4), and how to reset, back up and restore a test vault (sections 2.2 and 2.3).
- [Manual test guide B: system behavior](02-system-behavior.md): the working folder and the `$VAULT` variable (section 2.1), checksums (section 2.2), test files (section 2.3), and the clipboard and auto-lock cases that the per-OS cases below refer to.

The design behind the cases is in the [threat model](../../threat-model.md), the [security design](../../security-design.md), the [vault format specification](../../vault-format.md), [Clipboard behaviour](../../clipboard.md) and the [API reference](../../api-reference.md). Automated results are in the [Rust test report](../reports/rust.md) and the [UI test report](../reports/ui.md).

## Recording results

Fill in this table once per test run, then mark each case **Pass** or **Fail** and write what you saw in **Notes** (always for a failure: what happened instead, and any error message, word for word).

| | |
| --- | --- |
| Tester | |
| Date | |
| Commit tested | |
| Build | Development (`npm run dev`) / Release (installed bundle) |
| OS and version | |
| Webview and version | WebView2 (Windows), WKWebView / Safari version (macOS), WebKitGTK (Linux) |
| Desktop session (Linux only) | For example GNOME on Wayland, KDE Plasma on X11 |
| Display scaling | For example 100 %, 150 %, Retina |

Never use real passwords, real files or personal data while testing. Use the made-up values below and a vault that holds nothing of value. Several cases search files for your test secrets: a real secret typed into a terminal ends up in the shell history.

Mark each case by ticking one box (`☒ Pass` or `☒ Fail`), and write what you saw in **Notes**. Text in quotes is the exact wording the app should show.

## 1. Preconditions

Before every run:

- The preconditions of guide A, section 2.1, are met: the app builds and starts, you use a test account or machine, and no other copy of NovaVault is running.
- The working folder `nv-test` and the `$VAULT` variable from guide B, section 2.1, are set up.
- You have taken a backup of the data directory (guide A, section 2.3) if it holds a vault you want to keep.

### 1.1 Marker vault

The security cases in section 3 look for known text in the files NovaVault writes. Each value below contains a distinctive marker that cannot occur by chance, so finding it means the value was stored in clear.

1. Reset to a fresh install (guide A, section 2.2) and create a vault with the master password `Marker-Master-Q7v2-correct-horse`. Write down the recovery code exactly as shown.
2. Create these items:

   | Kind | Field | Value |
   | --- | --- | --- |
   | Password | Title | `MarkerTitleLogin` |
   | | Username | `marker-user@example.com` |
   | | Website | `https://marker-site.example.com` |
   | | Password | `MarkerPassword-Zx81` |
   | | Notes | `MarkerLoginNotes` |
   | Note | Title | `MarkerTitleNote` |
   | | Text | `MarkerNoteBody` and, on a second line, `Café-Ünïcode-Marker` |
   | Secret | Title | `MarkerTitleSecret` |
   | | Value | `MarkerSecretValue-9k` |
   | | Notes | `MarkerSecretNotes` |

3. In `nv-test`, create a text file `marker-attachment.txt` containing `MarkerAttachmentContent`, and attach it to `MarkerTitleNote`.
4. Quit the app.

Keep this vault small: do not attach the 1 GB test file from guide B, since the search commands below read every file in full.

### 1.2 Searching files for text

A value can be stored as UTF-8 (one byte per ASCII character) or as UTF-16 (two bytes per character, common on Windows). The helpers below look for both encodings in every file of a folder, including hidden files.

**Windows (PowerShell).** Paste this function into the session:

```powershell
# Lists every file under $Folder that contains $Text, encoded as UTF-8 or UTF-16LE.
function Find-TestText([string]$Folder, [string]$Text) {
  # Latin-1 maps each byte to one character, so a byte sequence can be searched as a string.
  $latin1 = [Text.Encoding]::GetEncoding(28591)
  $needles = @{
    'UTF-8'    = $latin1.GetString([Text.Encoding]::UTF8.GetBytes($Text))
    'UTF-16LE' = $latin1.GetString([Text.Encoding]::Unicode.GetBytes($Text))
  }
  $found = $false
  foreach ($file in Get-ChildItem -LiteralPath $Folder -Recurse -File -Force) {
    $haystack = $latin1.GetString([IO.File]::ReadAllBytes($file.FullName))
    foreach ($encoding in $needles.Keys) {
      if ($haystack.IndexOf($needles[$encoding], [StringComparison]::Ordinal) -ge 0) {
        "FOUND ($encoding): $($file.FullName)"
        $found = $true
      }
    }
  }
  if (-not $found) { 'not found' }
}
```

Usage: `Find-TestText "$env:APPDATA\io.github.acevflow.novavault" "MarkerPassword-Zx81"`.

**macOS and Linux (bash).** Paste this function into a `bash` session (on macOS, start `bash` first if your shell is `zsh`):

```sh
# Lists every file under $1 that contains the text $2, encoded as UTF-8 or UTF-16LE.
find_text() {
  utf16=$(printf '%s' "$2" | iconv -f UTF-8 -t UTF-16LE | od -A n -t x1 -v | tr -d ' \n')
  found=0
  while IFS= read -r -d '' file; do
    if LC_ALL=C grep -q -a -F -- "$2" "$file"; then
      echo "FOUND (UTF-8): $file"; found=1
    fi
    if od -A n -t x1 -v "$file" | tr -d ' \n' | grep -q -F -- "$utf16"; then
      echo "FOUND (UTF-16LE): $file"; found=1
    fi
  done < <(find "$1" -type f -print0)
  [ "$found" -eq 0 ] && echo "not found"
}
```

Usage: `find_text "$(dirname "$VAULT")" "MarkerPassword-Zx81"`.

The UTF-16 check compares hexadecimal text, so in rare cases it can report a match that is shifted by half a byte. Treat any `FOUND` as a failure to investigate, never as a pass.

The data directory is called `$DATA` in the cases below:

| OS | Command |
| --- | --- |
| Windows (PowerShell) | `$DATA = "$env:APPDATA\io.github.acevflow.novavault"` |
| macOS | `DATA=~/Library/Application\ Support/io.github.acevflow.novavault` |
| Linux | `DATA="${XDG_DATA_HOME:-$HOME/.local/share}/io.github.acevflow.novavault"` |

### 1.3 Viewing a file in hexadecimal

A hex viewer shows every byte of a file, with the printable characters next to it.

| OS | First 64 bytes | Printable text runs of 6 characters or more |
| --- | --- | --- |
| Windows (PowerShell 5.1) | `Format-Hex -Path $VAULT \| Select-Object -First 4` | `strings -n 6 $VAULT` with [Sysinternals Strings](https://learn.microsoft.com/sysinternals/downloads/strings), if installed |
| Windows (PowerShell 7) | `Format-Hex -Path $VAULT -Count 64` | As above |
| macOS | `xxd -l 64 "$VAULT"` or `hexdump -C -n 64 "$VAULT"` | `strings -a -n 6 "$VAULT"` |
| Linux | `xxd -l 64 "$VAULT"` or `hexdump -C -n 64 "$VAULT"` | `strings -a -n 6 "$VAULT"`, and `strings -a -n 6 -e l "$VAULT"` for UTF-16 |

On macOS, `strings` comes with the Xcode Command Line Tools (`xcode-select --install`). On Linux it is part of `binutils`, and `xxd` usually comes with `vim`; `hexdump` is part of `util-linux` or `bsdmainutils`.

Encrypted data looks random, so `strings` always prints some short meaningless runs. What matters is that none of them is a word you typed.

### 1.4 Developer console

The developer console (web inspector) is available in development builds only. Start the app with `npm run dev`, then:

| OS | Webview | How to open the console |
| --- | --- | --- |
| Windows | Microsoft Edge WebView2 | Right-click in the window › **Inspect**, or Ctrl+Shift+I. |
| macOS | WKWebView (Safari engine) | Right-click › **Inspect Element**, or Cmd+Option+I. |
| Linux | WebKitGTK | Right-click › **Inspect Element**, or Ctrl+Shift+I. |

The **Console** tab shows messages written by the page; the **Network** tab lists the calls the UI makes to the Rust core (IPC).

## 2. Security checks

### 2.1 Files at rest

Start: the marker vault from section 1.1, app closed, `$VAULT` and `$DATA` set, and the search helper from section 1.2 pasted. "Search for `<text>`" means:

| OS | Command |
| --- | --- |
| Windows (PowerShell) | `Find-TestText $DATA "<text>"` |
| macOS and Linux | `find_text "$DATA" "<text>"` |

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| C-SC-01 | Only the header is readable | 1. Show the first 64 bytes of `vault.nvault` in hexadecimal (section 1.3).<br>2. List its printable text runs with `strings` (section 1.3). | Step 1: the file starts with `4e 4f 56 41 56 4c 54 00` (`NOVAVLT`), then `01 00` (format version 1) and `01` (the encryption algorithm). Step 2: apart from `NOVAVLT`, only short random-looking runs; no word from the marker vault. | ☐ Pass ☐ Fail | |
| C-SC-02 | Item titles and fields are not stored in clear | Search for each of: `MarkerTitleLogin`, `marker-user@example.com`, `marker-site.example.com`, `MarkerPassword-Zx81`, `MarkerLoginNotes`, `MarkerTitleNote`, `MarkerNoteBody`, `Ünïcode-Marker`, `MarkerTitleSecret`, `MarkerSecretValue-9k`, `MarkerSecretNotes`. | Every search prints "not found". Titles, usernames and websites are encrypted like the secret fields: only record identifiers and types are stored in plaintext ([vault format, §7.2](../../vault-format.md#72-record-encoding)). | ☐ Pass ☐ Fail | |
| C-SC-03 | Attachment content and name are not stored in clear | 1. Search for `MarkerAttachmentContent` and for `marker-attachment`.<br>2. List `vault.nvault.attachments`. | Step 1: "not found" for both. Step 2: one `.nvatt` file, named with 32 hexadecimal characters that do not reveal the original name. | ☐ Pass ☐ Fail | |
| C-SC-04 | The master password is not stored | Search for `Marker-Master-Q7v2-correct-horse` and for `Marker-Master-Q7v2`. | "not found" both times. The vault stores only the vault key wrapped under a key derived from the password, never the password or a plain hash of it. | ☐ Pass ☐ Fail | |
| C-SC-05 | The recovery code is not stored | Search for the recovery code in four forms: as shown (with dashes), without dashes, in lower case with dashes, and the 8 characters of its third and fourth groups joined without a dash. | "not found" for every form. The vault never stores the code itself, only the vault key wrapped under a key derived from it ([vault format, §10](../../vault-format.md#10-recovery-code)). | ☐ Pass ☐ Fail | |
| C-SC-06 | Old and new secrets are not stored after a change | 1. Start the app and unlock. In **Settings**, change the master password to `Marker-NewMaster-P3k9-orbit-lantern`.<br>2. Generate a new recovery code (confirming with the new password), and write it down.<br>3. Quit the app.<br>4. Search for the old and the new master password, and for the old and the new recovery code, with and without dashes.<br>5. List the data directory, including hidden files (`Get-ChildItem -Force $DATA`, or `ls -la "$DATA"`). | Step 4: "not found" for every value. Step 5: no `.tmp` file is left behind. From now on the master password is `Marker-NewMaster-P3k9-orbit-lantern`. | ☐ Pass ☐ Fail | |
| C-SC-07 | The settings file holds no secret | 1. Start the app, unlock, set **Lock after inactivity** to "15 minutes" and **Clear the clipboard after** to "1 minute", and quit.<br>2. Open `settings.json` in a text editor. | The file holds only `autoLockMinutes` and `clipboardClearSeconds` with numbers: `{"autoLockMinutes":15,"clipboardClearSeconds":60}`, possibly spread over several lines. No path, title or secret. Set both settings back to "5 minutes" and "30 seconds". | ☐ Pass ☐ Fail | |
| C-SC-08 | File permissions (macOS and Linux) | macOS and Linux only.<br>1. Run `ls -la "$DATA" "$DATA/vault.nvault.attachments"`.<br>2. Start the app, export `marker-attachment.txt` to `nv-export`, quit, and run `ls -l ~/nv-export`. | `vault.nvault`, every `.nvatt` file and the exported file are `-rw-------` (mode `0600`, owner only). The `vault.nvault.attachments` folder is `drwx------` (mode `0700`). `settings.json` may be readable by others: it is not secret. | ☐ Pass ☐ Fail | |
| C-SC-09 | File permissions (Windows) | Windows only.<br>1. Run `icacls "$DATA\vault.nvault"` and `icacls $DATA`. | The entries are inherited from the user profile (marked `(I)`): typically your account, `SYSTEM` and `Administrators`, and no `Everyone` or `Users` entry. NovaVault sets no permissions of its own on Windows ([security design, per-OS notes](../../security-design.md#per-os-notes)). Record the entries. | ☐ Pass ☐ Fail | |

### 2.2 Logs and console

Start: the marker vault, with the master password `Marker-NewMaster-P3k9-orbit-lantern` and the recovery code from C-SC-06, and the settings at their defaults.

The **full flow** used below is:

1. On "Unlock your vault", enter the wrong password `Marker-Wrong-Echo-55`, then the correct one.
2. Select each item and **Reveal** every secret field, then **Copy** the password of `MarkerTitleLogin`.
3. Edit `MarkerTitleSecret`, change **Value** to `MarkerEditedValue-4t` and save.
4. Attach `marker-attachment.txt` to `MarkerTitleLogin`, then export it to `nv-export`.
5. Select **Lock**, unlock with the recovery code, and quit.

NovaVault writes to the terminal only when something fails, always as a fixed sentence that starts with `NovaVault` (for example "NovaVault: the clipboard could not be cleared"). It never logs a value, a path or an error's details. The UI writes nothing to the developer console.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| C-SC-10 | No secrets in the terminal output (development build) | 1. Start the app and save everything it prints:<br>Windows: `npm run dev 2>&1 \| Tee-Object -FilePath "$HOME\nv-test\nv-dev.log"`<br>macOS and Linux: `npm run dev 2>&1 \| tee ~/nv-test/nv-dev.log`<br>2. Do the full flow.<br>3. Search the log for every marker value, both master passwords, `Marker-Wrong-Echo-55`, `MarkerEditedValue-4t` and both recovery codes:<br>Windows: `Select-String -Path "$HOME\nv-test\nv-dev.log" -SimpleMatch "<text>"`<br>macOS and Linux: `grep -F -- "<text>" ~/nv-test/nv-dev.log` | No match for any value. The log holds only build output from npm, Vite and Cargo, and at most the fixed `NovaVault…` sentences. | ☐ Pass ☐ Fail | |
| C-SC-11 | No secrets in the terminal output (release build) | macOS and Linux (Windows release builds print nothing; see C-WIN-04).<br>1. Start the release executable from a terminal with its output saved: `./target/release/novavault 2>&1 \| tee ~/nv-test/nv-release.log`, or the AppImage, or the executable inside `NovaVault.app/Contents/MacOS/`.<br>2. Do the full flow and search the log as in C-SC-10. | No match for any value. On a normal run the log is empty, or holds only fixed `NovaVault…` sentences and messages from the webview or the desktop toolkit (for example GTK warnings) that contain nothing you typed. | ☐ Pass ☐ Fail | |
| C-SC-12 | No secrets or errors in the developer console | Development build.<br>1. Start the app, open the developer console (section 1.4) on **Console**, and show all levels, including verbose or debug.<br>2. Do the full flow, but do not quit until step 3 is done.<br>3. Type each value from C-SC-10 in the console's filter box. | Step 3: no message matches. The console shows no error or warning from NovaVault. Messages from the development tooling are allowed, for example "[vite] connecting..." and "[vite] connected." and React's hint to install its developer tools. Record anything else. | ☐ Pass ☐ Fail | |
| C-SC-13 | Error messages never echo what was typed | 1. On "Unlock your vault", enter `Marker-Wrong-Echo-55` and select **Unlock**.<br>2. Switch to **Recovery code**, enter `Marker-Wrong-Echo-55` and select **Unlock**. | Step 1: "That master password is not correct." Step 2: "That recovery code is not valid. Check it for typos: it has 8 groups of 4 characters." Neither message, nor anything else on the screen, contains the typed text, and the field is emptied after each attempt. | ☐ Pass ☐ Fail | |
| C-SC-14 | IPC responses carry only what the screen shows | Development build. Only if the **Network** tab lists the IPC calls (WebView2 does; record what your webview shows).<br>1. Open **Network** and clear it.<br>2. Unlock, select `MarkerTitleLogin` and look at the responses of `list_items` and `get_item`.<br>3. Select **Copy** on **Password** and look at the response of `copy_item_field`.<br>4. Select **Reveal** on **Password** and look at the response of `reveal_item_field`. | Step 2: titles, username, website, dates and attachment names, and `hasPassword` and `hasNotes` flags; no password, notes, note text or secret value. Step 3: only `{"clearAfterSeconds":30}`: a copied value never reaches the UI. Step 4: `{"value":"MarkerPassword-Zx81"}`, because you asked to see it. The request of `unlock` holds the password you typed, which the window already had. | ☐ Pass ☐ Fail | |
| C-SC-15 | Revealed values leave the page | Development build.<br>1. Reveal the password of `MarkerTitleLogin`.<br>2. In the console, run `document.body.innerText.includes('MarkerPassword-Zx81')`.<br>3. Select **Hide** and run it again.<br>4. Reveal it again, select **Lock**, and run it again. | Step 2: `true`. Steps 3 and 4: `false`: hiding the value or locking the vault removes it from the page. | ☐ Pass ☐ Fail | |
| C-SC-16 | Webview storage holds only the theme | Development build.<br>1. In **Settings**, choose **Dark** under **Appearance**.<br>2. In the console, run `Object.keys(localStorage)`, `Object.keys(sessionStorage)` and `document.cookie`.<br>3. Run `localStorage.getItem('novavault.theme')`. | Step 2: `['novavault.theme']`, `[]` and `''`. Step 3: `'dark'`. The webview keeps nothing else. Choose **Match system** again afterwards. | ☐ Pass ☐ Fail | |

### 2.3 Webview hardening

The window runs under a strict Content Security Policy (CSP) and has no direct access to Tauri's APIs. The development build uses a slightly looser policy that also allows the Vite dev server; the checks below hold for both builds.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| C-SC-17 | No connection to the internet | Development build.<br>1. In the console, run `fetch('https://example.com').then(() => console.log('reached'), (e) => console.log('blocked', e.message))`. | "blocked" is printed, with a CSP error such as "Refused to connect to 'https://example.com/' because it violates the following Content Security Policy directive: …connect-src…". **Network** shows no completed request to `example.com`. | ☐ Pass ☐ Fail | |
| C-SC-18 | No global Tauri API, frozen prototypes | Development build.<br>1. In the console, run `typeof window.__TAURI__`.<br>2. Run `Object.isFrozen(Object.prototype)`. | Step 1: `'undefined'`: the app does not expose Tauri's global API. Step 2: `true`: built-in prototypes are frozen. | ☐ Pass ☐ Fail | |
| C-SC-19 | Only the vault commands are callable | Development build.<br>1. In the console, run `window.__TAURI_INTERNALS__.invoke('plugin:dialog\|open', { options: {} }).then(console.log, console.log)`. | The call is rejected with a message that the command is not allowed (the wording depends on the Tauri version), and no file dialog opens. The window may only call NovaVault's own commands; file dialogs are opened by the Rust core. | ☐ Pass ☐ Fail | |
| C-SC-20 | Release builds have no developer tools | Release build (installed bundle).<br>1. Right-click anywhere in the window.<br>2. Press Ctrl+Shift+I (Windows and Linux) or Cmd+Option+I (macOS), then F12. | No **Inspect** or **Inspect Element** entry, and no shortcut opens the developer tools. Record what the right-click menu shows, if anything. | ☐ Pass ☐ Fail | |

## 3. UI and accessibility

Start: a test vault with a few items of each kind (guide A, section 3.4, or the marker vault), app open on "Unlock your vault". You can use either build; the theme choice is remembered separately by the development and release builds, because they load the page from different origins.

### 3.1 Keyboard navigation

Run these cases without touching the mouse or trackpad. On macOS, turn on **Keyboard navigation** first (System Settings › Keyboard), so that Tab also reaches buttons, radio buttons and checkboxes.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| C-UI-01 | Skip link | 1. On "Unlock your vault", click nowhere and press Tab once.<br>2. Press Enter, then Tab. | Step 1: a "Skip to main content" link appears at the top left. Step 2: focus moves into the main area; the next Tab reaches the first control of the screen, not the header. | ☐ Pass ☐ Fail | |
| C-UI-02 | Unlock with the keyboard only | 1. Tab to the **Master password** tab. Press Right, Left, End and Home.<br>2. Select **Master password**, Tab to the field, type the master password and press Enter. | Step 1: Right and Left move between **Master password** and **Recovery code** (wrapping around), End selects the last tab and Home the first. Only the selected tab is in the Tab order. Step 2: the vault unlocks, and focus lands on the "Your vault" heading. | ☐ Pass ☐ Fail | |
| C-UI-03 | Header order and current screen | 1. While unlocked, press Tab from the top of the window until you reach the main area. | The order is "Skip to main content", **Vault**, **Settings**, **Lock**, then the main area. The current screen's navigation button is marked as current (visually, and as "current page" to a screen reader). | ☐ Pass ☐ Fail | |
| C-UI-04 | Focus is always visible | 1. Tab through every control on **Vault** and on **Settings**, in the light and in the dark theme. | Every control that gets focus shows a clear outline (3 pixels, in the focus colour) that is not hidden by other elements. Clicking a button with the mouse does not leave an outline on it. | ☐ Pass ☐ Fail | |
| C-UI-05 | Find and open an item | 1. On **Vault**, Tab to **Search by title** and type part of a title.<br>2. Tab to **Type** and change it with the arrow keys.<br>3. Tab to an item in the list and press Enter; repeat with Space on another item.<br>4. Tab through the details. | Steps 1–2: the list and the count update as you type or choose. Step 3: the item opens and focus moves to its title heading. Step 4: Tab reaches **Reveal**, **Copy**, **Edit**, **Delete**, **Attach file** and each attachment's **Export** and **Remove**, in reading order. | ☐ Pass ☐ Fail | |
| C-UI-06 | Create and edit an item | 1. Tab to **New password** and press Enter.<br>2. Leave **Title** empty, Tab to **Username** and press Enter.<br>3. Type a title, fill the other fields with Tab between them; in **Notes**, press Enter between two lines.<br>4. Tab to **Save** and press Enter. | Step 1: focus is in **Title**. Step 2: Enter submits the form: "Enter a title." appears under **Title**, which gets focus again. Step 3: Enter in **Notes** adds a new line instead of saving. Step 4: "Item added." and the new item opens. | ☐ Pass ☐ Fail | |
| C-UI-07 | Delete confirmation with the keyboard | 1. Open an item, Tab to **Delete** and press Enter.<br>2. Press Enter.<br>3. Open the confirmation again and press Escape.<br>4. Open it again, Tab to the confirming **Delete** and press Enter. | Step 1: focus is on **Cancel**. Steps 2 and 3 close the question and nothing is deleted. After closing, press Tab and record which control gets focus: the app does not move focus back to **Delete** explicitly, so it may start again from the top of the window. Step 4: "Item deleted." | ☐ Pass ☐ Fail | |
| C-UI-08 | Settings with the keyboard | 1. On **Settings**, Tab to **Theme** and press the arrow keys.<br>2. Tab to **Lock after inactivity**, change it with the arrow keys and press Enter on **Save**.<br>3. Tab to the **Confirm it is you with** choice and switch it with the arrow keys. | Step 1: each arrow press selects the next theme and applies it at once. Step 2: the setting is saved with its confirmation message. Step 3: the current-secret field changes between master password and recovery code and is emptied. Set any changed value back afterwards. | ☐ Pass ☐ Fail | |
| C-UI-09 | Recovery code flow with the keyboard | 1. In **Settings › Recovery code**, enter the master password and press Enter.<br>2. Tab to the "I have saved my recovery code somewhere safe" checkbox, press Space, and press Enter on **Continue**.<br>3. Type the code and press Enter. | The whole flow can be completed without the mouse. Focus moves to the heading of each step ("Save your recovery code", "Confirm your recovery code"). Step 3: "Your new recovery code is saved and confirmed." Write down the new code. | ☐ Pass ☐ Fail | |
| C-UI-10 | Screen reader spot check | Optional. Use Narrator (Windows, Ctrl+Win+Enter), VoiceOver (macOS, Cmd+F5) or Orca (Linux, Super+Alt+S).<br>1. Unlock with a wrong, then the right password.<br>2. Open an item, copy a field, and add an item.<br>3. Type in **Search by title**. | Each screen's heading is read when it appears. Error messages ("That master password is not correct.") are read at once; status messages ("Copied. The clipboard clears in 30 seconds.", "Item added.", the item count) are read without interrupting. Every field and button is read with its label, for example "Copy password" and "Reveal password". | ☐ Pass ☐ Fail | |

### 3.2 Light and dark themes

The theme is chosen in **Settings › Appearance**: **Match system** (the default) follows the OS setting, **Light** and **Dark** override it. The choice applies at once and is remembered across restarts.

How to switch the OS theme: Windows: Settings › Personalization › Colors › **Choose your mode**. macOS: System Settings › Appearance. Linux: GNOME Settings › Appearance, or the desktop's own setting; some desktops only change the GTK theme, which the webview may not report.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| C-UI-11 | Match system follows the OS live | 1. With **Match system** selected, switch the OS from light to dark and back while the app is open. | The app switches theme within a moment each time, without a restart. On Linux, record the desktop and whether the change reached the app. | ☐ Pass ☐ Fail | |
| C-UI-12 | Forced theme overrides the OS and is remembered | 1. Choose **Dark** with the OS in light mode.<br>2. Quit and restart the app.<br>3. Choose **Light** with the OS in dark mode, and restart again. | Step 1: the app is dark at once. Step 2: it starts dark on "Unlock your vault" (record any brief flash of the light theme while the window loads). Step 3: it stays light. Choose **Match system** afterwards. | ☐ Pass ☐ Fail | |
| C-UI-13 | Every screen is readable in both themes | In the light theme and then in the dark theme, visit: "Create your vault" with the strength meter showing (fresh install, or skip), "Save your recovery code", "Unlock your vault" with an error, **Vault** with a selected item and a revealed field, an item form with "Enter a title.", an attachment transfer with its progress bar, the delete confirmation, and **Settings** with a success message. | All text, borders, focus outlines and the selected item are clearly visible. Error, success and information messages are distinguishable. Native controls (drop-down lists, checkboxes, radio buttons, scroll bars, text cursor) follow the theme. | ☐ Pass ☐ Fail | |
| C-UI-14 | Printing is always light | 1. With the **Dark** theme, generate a new recovery code and select **Print** on "Save your recovery code".<br>2. Look at the preview, then cancel. | The preview is black on white, shows only the code and its printed date, and none of the app's chrome. Finish the flow and write down the new code. | ☐ Pass ☐ Fail | |
| C-UI-15 | Reduced motion | Optional.<br>1. Turn on the OS's reduce-motion setting (Windows: Settings › Accessibility › Visual effects › **Animation effects** off; macOS: Accessibility › Display › **Reduce motion**; GNOME: Accessibility › **Reduce Animation**).<br>2. Use the app. | Nothing in the app animates. The app works the same. | ☐ Pass ☐ Fail | |

### 3.3 Window size

The window opens at 1024 × 720 and cannot be made smaller than 640 × 480 (logical pixels; with display scaling the physical size is larger).

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| C-UI-16 | Default and minimum size | 1. Start the app.<br>2. Drag a corner to make the window as small as possible. | Step 1: the window is about 1024 × 720 and titled "NovaVault". Step 2: it stops shrinking at about 640 × 480. | ☐ Pass ☐ Fail | |
| C-UI-17 | Narrow window | 1. Make the window about 700 pixels wide, then as narrow as possible.<br>2. On **Vault**, open an item; on **Settings**, scroll through every section. | Below about 768 pixels the item list moves above the details instead of beside them. At every width: no horizontal scroll bar, no text or button cut off or overlapping, and the header still shows **Vault**, **Settings** and **Lock**. | ☐ Pass ☐ Fail | |
| C-UI-18 | Short window | 1. Make the window as low as possible (480 pixels).<br>2. Scroll through **Vault** with an item open, and **Settings**. | Every control can be reached by scrolling, with the mouse wheel and with Tab. A control that gets focus scrolls into view. | ☐ Pass ☐ Fail | |
| C-UI-19 | Large and maximized window | 1. Maximize the window (or use full screen on macOS).<br>2. Restore it. | The content stays centred with a readable line length, rather than stretching across the screen. Restoring returns to the previous size. | ☐ Pass ☐ Fail | |
| C-UI-20 | Resizing keeps state | 1. Start editing an item and type in a field.<br>2. Resize the window from wide to narrow and back. | The form and what you typed are kept; nothing is reset, and focus stays where it was. | ☐ Pass ☐ Fail | |
| C-UI-21 | Recovery code at minimum width | 1. Make the window as narrow as possible.<br>2. Generate a new recovery code in **Settings** (or create a vault after a fresh install). | The code is shown as complete groups of 4 characters, two groups per row, never split inside a group. Finish the flow and write down the new code. | ☐ Pass ☐ Fail | |

### 3.4 Error messages

Every message from the Rust core is shown in the app's own words; the core's internal reason (a file path, an I/O error) is never displayed. Errors about a single field appear under that field; other errors appear in a message box near the action that failed.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| C-UI-22 | Field errors sit next to their field | 1. On "Unlock your vault", select **Unlock** with an empty field.<br>2. On a new item, select **Save** with an empty **Title**.<br>3. In **Settings › Master password**, type two different new passwords and select **Change master password**. | Step 1: "Enter your master password." Step 2: "Enter a title." under **Title**. Step 3: "The passwords do not match." under the confirmation field. In steps 1 and 2 focus moves to the field to correct. Each message disappears or changes once the input is fixed and submitted again. | ☐ Pass ☐ Fail | |
| C-UI-23 | Errors and confirmations look different | 1. Cause an error (a wrong master password) and a confirmation (save an item: "Item added.").<br>2. Compare them, in both themes. | Errors are styled as errors (colour and border), confirmations as success messages; the difference does not rely on colour alone, since the wording differs. | ☐ Pass ☐ Fail | |
| C-UI-24 | Storage error | 1. Quit the app and back up the data directory.<br>2. Rename `vault.nvault` to `vault.bak` and create an empty folder named `vault.nvault` in its place.<br>3. Start the app and try to unlock.<br>4. Quit, delete the folder, and rename `vault.bak` back. | Step 3: "Unlock your vault" is shown (a vault path exists) and unlocking fails with "The vault file could not be read or written. Check the disk and try again." or, if the system reports it differently, another message from the app's list; never a raw system error or a path. Record the message. Step 4: the vault unlocks normally. | ☐ Pass ☐ Fail | |
| C-UI-25 | Long messages wrap | 1. Make the window as narrow as possible.<br>2. Unlock with a mistyped recovery code. | "That recovery code is not valid. Check it for typos: it has 8 groups of 4 characters." wraps onto several lines inside its box, without being cut off. | ☐ Pass ☐ Fail | |
