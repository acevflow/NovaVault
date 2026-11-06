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

## 4. Per-OS behaviour

Run the subsection for the OS under test, on a release build installed from its bundle (guide A, section 1.3) unless a case says otherwise. The general clipboard and auto-lock cases are in guide B, sections 3 and 4; the cases below only add what differs per OS. Record the OS version and the webview version for every run.

The bundles are not code-signed, so every OS warns before the first start. Accept the warning only for a build you made yourself.

### 4.1 Windows

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| C-WIN-01 | Install with the MSI package | 1. Run `target\release\bundle\msi\*.msi`. If SmartScreen shows "Windows protected your PC", select **More info** › **Run anyway**.<br>2. Complete the installer and start **NovaVault** from the Start menu. | The installer completes without errors. **NovaVault** is in the Start menu and in Settings › Apps › Installed apps, and starts on "Create your vault" or "Unlock your vault". | ☐ Pass ☐ Fail | |
| C-WIN-02 | Install with the NSIS installer | 1. Uninstall the MSI version first (C-WIN-03).<br>2. Run `target\release\bundle\nsis\*-setup.exe` and complete it.<br>3. Start NovaVault from the Start menu. | As C-WIN-01. If WebView2 is missing (rare on Windows 10 and 11), the installer downloads and installs it, which needs an internet connection; record whether this happened. | ☐ Pass ☐ Fail | |
| C-WIN-03 | Uninstalling keeps the vault | 1. Create or unlock a vault, then quit.<br>2. Uninstall from Settings › Apps › Installed apps. If the uninstaller offers to delete the application data, leave it unticked.<br>3. Check the data directory.<br>4. Reinstall and start the app. | Step 3: `%APPDATA%\io.github.acevflow.novavault\` still holds `vault.nvault`. Step 4: "Unlock your vault", and the same master password opens it. Record whether the uninstaller offered to delete the data. | ☐ Pass ☐ Fail | |
| C-WIN-04 | No console window | 1. Start the installed app from the Start menu.<br>2. Start `target\release\novavault.exe` from PowerShell. | Only the NovaVault window opens, with no console window behind it. Step 2: PowerShell returns to its prompt at once and the app prints nothing there, even on errors: release builds have no console output on Windows. | ☐ Pass ☐ Fail | |
| C-WIN-05 | WebView2 version | 1. Run `(Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue).pv`. If it prints nothing, try the same path under `HKCU:\Software\Microsoft\EdgeUpdate\Clients\`.<br>2. Run the cases of section 3.2 and 3.3 quickly in the installed app. | A version is printed; record it. The app looks and behaves as described in section 3, with Windows-style form controls and scroll bars. | ☐ Pass ☐ Fail | |
| C-WIN-06 | Clipboard history and cloud clipboard | 1. Run B-CB-15 (clipboard history).<br>2. If **Sync across your devices** is on (Settings › System › Clipboard), copy a password and check another device signed in to the same account. | Step 1: as in B-CB-15. Step 2: the password does not reach the other device. If sync is off or unavailable, write "not tested". | ☐ Pass ☐ Fail | |
| C-WIN-07 | Lock events | For each event, unlock first and copy a password:<br>1. Win+L.<br>2. Ctrl+Alt+Del › **Lock**.<br>3. Start › Power › **Sleep**.<br>4. On a laptop, close the lid (if set to sleep).<br>5. Start › your account › **Switch user**, then switch back. | After each event, NovaVault shows "Unlock your vault" and the clipboard is empty. On devices with Modern Standby, sleep is often preceded by a screen lock, which is the event that locks the vault; record which devices you tested. | ☐ Pass ☐ Fail | |
| C-WIN-08 | Display scaling and monitors | 1. Set Settings › System › Display › **Scale** to 150 % (and 100 % if it was different).<br>2. With two monitors at different scales, move the window between them. | Text and controls are sharp, not blurry, at every scale. Moving between monitors resizes the content to the new scale without breaking the layout. | ☐ Pass ☐ Fail | |

### 4.2 macOS

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| C-MAC-01 | Install from the disk image | 1. Open `target/release/bundle/dmg/*.dmg` and drag **NovaVault** to **Applications**.<br>2. Start it from Applications. When macOS refuses to open it: on macOS 14 and earlier, right-click the app › **Open** › **Open**; on macOS 15 and later, open System Settings › Privacy & Security and select **Open Anyway**, then confirm. | The app starts, and starts again later without a warning. It appears in Launchpad and Spotlight as "NovaVault" with its icon. Record the macOS version and the steps Gatekeeper required. | ☐ Pass ☐ Fail | |
| C-MAC-02 | Uninstalling keeps the vault | 1. Quit the app and move `NovaVault.app` from Applications to the Bin.<br>2. Check `~/Library/Application Support/io.github.acevflow.novavault/`.<br>3. Install again and start. | Step 2: `vault.nvault` is still there. Step 3: "Unlock your vault", and the same master password opens it. | ☐ Pass ☐ Fail | |
| C-MAC-03 | WKWebView version | 1. Run `defaults read /Applications/Safari.app/Contents/Info.plist CFBundleShortVersionString`. | Record the Safari version: the app's webview uses the same engine. The app looks and behaves as described in section 3, with macOS form controls. | ☐ Pass ☐ Fail | |
| C-MAC-04 | Menu bar, shortcuts and quitting | 1. With the app focused, look at the menu bar.<br>2. In a text field, use Cmd+A, Cmd+C, Cmd+V, Cmd+X and Cmd+Z.<br>3. Copy a password in the app, then quit with Cmd+Q and paste in a text editor.<br>4. Start the app again and close the window with its red button. | Step 1: an application menu named NovaVault. Step 2: the editing shortcuts work in text fields. Step 3: the app quits and the clipboard is empty (as B-CB-13). Step 4: record whether the app quits or stays in the Dock; either way, starting it again shows "Unlock your vault". | ☐ Pass ☐ Fail | |
| C-MAC-05 | Clipboard tools and Universal Clipboard | 1. Run B-CB-16 with any clipboard history tool you have.<br>2. If Handoff is on and another Apple device signed in to the same Apple Account is nearby, copy a password and try to paste it on that device. | Step 1: as in B-CB-16. Step 2: Universal Clipboard may offer the password to the other device; this is a documented limitation ([Clipboard behaviour, macOS](../../clipboard.md#macos)). Record what happened; it is not a NovaVault failure. | ☐ Pass ☐ Fail | |
| C-MAC-06 | Lock events | For each event, unlock first and copy a password:<br>1. Ctrl+Cmd+Q (Lock Screen).<br>2. Apple menu › **Sleep**.<br>3. On a laptop, close the lid.<br>4. Start the screen saver with "Require password after screen saver begins" set to **Immediately** (System Settings › Lock Screen).<br>5. Fast user switching (B-AL-14). | After each event, NovaVault shows "Unlock your vault" and the clipboard is empty. Step 4: the vault locks when the screen locks, which may be when the screen saver starts or when you return to it; record which. | ☐ Pass ☐ Fail | |
| C-MAC-07 | Appearance and Retina | 1. Switch System Settings › Appearance between Light, Dark and **Auto** with **Match system** selected in the app.<br>2. Use the app on a Retina display and, if available, a non-Retina external display. | Step 1: as C-UI-11; with **Auto**, the app follows the system when it changes. Step 2: text is sharp on both displays. | ☐ Pass ☐ Fail | |

### 4.3 Linux

Find out which session you are in before starting, and record it:

```sh
echo "$XDG_SESSION_TYPE"            # wayland or x11
echo "$WAYLAND_DISPLAY / $DISPLAY"  # wayland-0 / :0 on Wayland (with XWayland); empty / :0 on X11
echo "$XDG_CURRENT_DESKTOP"         # for example GNOME, KDE, sway
```

For the clipboard checks, install `wl-clipboard` (Wayland: `wl-paste`) or `xclip` (X11). Reading the clipboard with these tools does not change it.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| C-LNX-01 | Install the Debian package | Debian, Ubuntu and derivatives.<br>1. Run `sudo apt install ./target/release/bundle/deb/*.deb`.<br>2. Start NovaVault from the desktop's application menu, and then from a terminal by the command the package installs (find it with `dpkg -L <package> \| grep bin/`, the package name being shown by `dpkg -l \| grep -i novavault`). | apt installs the package and its dependencies (including WebKitGTK) without errors. The app has a menu entry with its icon and starts both ways. | ☐ Pass ☐ Fail | |
| C-LNX-02 | Install the RPM package | Fedora, openSUSE and derivatives.<br>1. Run `sudo dnf install ./target/release/bundle/rpm/*.rpm` (or `sudo zypper install` on openSUSE).<br>2. Start it from the application menu. | As C-LNX-01. | ☐ Pass ☐ Fail | |
| C-LNX-03 | Run the AppImage | 1. Run `chmod +x target/release/bundle/appimage/*.AppImage` and start the file.<br>2. Try it on a second distribution if available. | The app starts without installation. If it does not, record the error printed in the terminal (a missing FUSE library is a common cause: the AppImage needs `libfuse2` on many distributions). | ☐ Pass ☐ Fail | |
| C-LNX-04 | Uninstalling keeps the vault | 1. Create or unlock a vault, then quit.<br>2. Remove the package (`sudo apt remove <package>` or `sudo dnf remove <package>`).<br>3. Check `~/.local/share/io.github.acevflow.novavault/`. | The vault and its attachments are still there; reinstalling and starting shows "Unlock your vault". | ☐ Pass ☐ Fail | |
| C-LNX-05 | WebKitGTK version and rendering | 1. Record the WebKitGTK version: `dpkg -l 'libwebkit2gtk-4.1*'` or `rpm -q webkit2gtk4.1`.<br>2. Start the app from a terminal. If the window stays blank or flickers, quit and start it again with `WEBKIT_DISABLE_DMABUF_RENDERER=1` in front of the command. | The window shows the app normally. If the workaround was needed, record the GPU, driver and session: this is a known WebKitGTK issue with some graphics drivers, not a NovaVault failure, but it is worth noting. | ☐ Pass ☐ Fail | |
| C-LNX-06 | Data directory follows XDG_DATA_HOME | 1. Quit the app.<br>2. Start it from a terminal with `XDG_DATA_HOME=~/nv-test/xdg` in front of the command.<br>3. Create a vault, quit, and list `~/nv-test/xdg/io.github.acevflow.novavault/`. | Step 2: "Create your vault" (a different, empty data directory). Step 3: the new `vault.nvault` is there, and the vault in `~/.local/share/` is untouched. Delete `~/nv-test/xdg` afterwards. | ☐ Pass ☐ Fail | |
| C-LNX-07 | Only the CLIPBOARD selection is used | 1. Select the word `primary-test` in a text editor (this sets the PRIMARY selection).<br>2. In NovaVault, copy a password.<br>3. Middle-click in the text editor; then press Ctrl+V.<br>4. Run `wl-paste --primary` (Wayland) or `xclip -o -selection primary` (X11). | Step 3: middle-click pastes `primary-test`; Ctrl+V pastes the password. Step 4: `primary-test`. NovaVault never writes the PRIMARY selection. | ☐ Pass ☐ Fail | |
| C-LNX-08 | Password-manager hint on the clipboard | 1. Copy a password in NovaVault.<br>2. List the offered types: `wl-paste --list-types` (Wayland) or `xclip -o -selection clipboard -t TARGETS` (X11). | The list includes `x-kde-passwordManagerHint` together with the text types, so clipboard managers that honour it (for example Klipper) skip the value. | ☐ Pass ☐ Fail | |
| C-LNX-09 | X11 session: clear and exit | X11 session only.<br>1. Copy a password and run `xclip -o -selection clipboard` at once and again after 35 seconds.<br>2. Copy again and quit NovaVault, then run the command again.<br>3. If a clipboard manager is running (for example Klipper or the one in Xfce), open its history. | Step 1: the password, then an error or nothing (the selection has no owner after the clear). Step 2: nothing: NovaVault clears before releasing the clipboard, so no clipboard manager is handed the value on exit. Step 3: record whether the manager kept the password from step 1. | ☐ Pass ☐ Fail | |
| C-LNX-10 | Wayland session with data-control | Wayland session on KDE Plasma, Sway, Hyprland or another compositor with data-control.<br>1. Copy a password, then focus another app (for example a terminal) and stay there.<br>2. Run `wl-paste` at once and again after 35 seconds, without focusing NovaVault. | The password, then nothing: copying and clearing work while NovaVault does not have focus. | ☐ Pass ☐ Fail | |
| C-LNX-11 | Wayland session without data-control | Wayland session on GNOME, or another compositor without data-control.<br>1. Run B-CB-17.<br>2. Also run `wl-paste` from a terminal at 35 seconds, before focusing NovaVault again. | As B-CB-17: the clear may reach native Wayland apps only once NovaVault is focused again. Record which GNOME version you used and what `wl-paste` returned. | ☐ Pass ☐ Fail | |
| C-LNX-12 | Forced XWayland on a Wayland session | Wayland session only.<br>1. Quit the app and start it from a terminal with `env -u WAYLAND_DISPLAY GDK_BACKEND=x11` in front of the command, so that both the window and the clipboard use XWayland.<br>2. Copy a password and paste it into a native Wayland app (for example GNOME Text Editor or Kate).<br>3. Stay in that app for 35 seconds and paste again; then focus NovaVault, return and paste again. | Step 2: the password is pasted. Step 3: record whether the second paste was empty or still the password; the third paste must be empty. This is the documented XWayland limitation ([Clipboard behaviour, Wayland](../../clipboard.md#wayland)). | ☐ Pass ☐ Fail | |
| C-LNX-13 | Lock events | For each event, unlock first and copy a password:<br>1. `systemctl suspend`.<br>2. `loginctl lock-session`.<br>3. The desktop's lock shortcut (Super+L on GNOME, Meta+L on KDE) or its menu.<br>4. Let the screen saver start and lock the screen (set a short delay first). | After each event, NovaVault shows "Unlock your vault" and the clipboard is empty. Record the desktop, and any event that did not lock; then run B-AL-15 if logind or the screen-saver service is missing. | ☐ Pass ☐ Fail | |
| C-LNX-14 | Fractional scaling | 1. Set the display scale to 100 %, then 125 % or 150 % if the desktop supports fractional scaling.<br>2. Use the app at each scale. | The layout is correct at every scale. Record whether text is sharp or blurry at fractional scales: on some Wayland desktops XWayland or WebKitGTK renders blurry text, which is a platform limitation. | ☐ Pass ☐ Fail | |

## 5. Edge cases

Start: an unlocked test vault (not the marker vault, whose values section 2 relies on), on **Vault**. Take a backup first (guide A, section 2.3): some cases add very large items.

### 5.1 Generating long text

Typing thousands of characters by hand is impractical. These commands put a string of a given length on the clipboard, ready to paste into a field. Each `x` is one byte.

| OS | 10 000 characters | 1 000 000 characters |
| --- | --- | --- |
| Windows (PowerShell) | `Set-Clipboard ('x' * 10000)` | `Set-Clipboard ('x' * 1000000)` |
| macOS | `head -c 10000 /dev/zero \| tr '\0' x \| pbcopy` | `head -c 1000000 /dev/zero \| tr '\0' x \| pbcopy` |
| Linux (Wayland) | `head -c 10000 /dev/zero \| tr '\0' x \| wl-copy` | `head -c 1000000 /dev/zero \| tr '\0' x \| wl-copy` |
| Linux (X11) | `head -c 10000 /dev/zero \| tr '\0' x \| xclip -selection clipboard` | `head -c 1000000 /dev/zero \| tr '\0' x \| xclip -selection clipboard` |

Change the number for other lengths. An item, including its title, all its fields and the names of its attachments, can hold at most about 1 MiB (1 048 576 bytes) once encoded. The limit is in bytes of UTF-8, so a non-Latin character takes 2 to 4 bytes of it.

### 5.2 Empty fields

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| C-ED-01 | Title of spaces only | 1. Select **New secret**, type three spaces in **Title**, and select **Save**. | "Enter a title." Nothing is added. A title is required and spaces alone do not count. | ☐ Pass ☐ Fail | |
| C-ED-02 | Item with only a title | 1. Select **New password**, type the title `Only a title`, leave every other field empty, and save.<br>2. Repeat with **New note** and **New secret**. | "Item added." each time. Every empty field shows "Empty", with no **Reveal** or **Copy** button. | ☐ Pass ☐ Fail | |
| C-ED-03 | Title with surrounding spaces | 1. Create a note titled `  Spaced title  ` (two spaces before and after).<br>2. Search for `spaced title`. | The item is saved and found by the search. Record whether the list shows the spaces; the title is stored as typed. | ☐ Pass ☐ Fail | |
| C-ED-04 | Empty and blank search | 1. Type only spaces in **Search by title**.<br>2. Clear the field. | Both times, every item is listed and the count shows the total. | ☐ Pass ☐ Fail | |
| C-ED-05 | Blank secrets on the unlock screen | 1. Lock. On **Master password**, type three spaces and select **Unlock**.<br>2. On **Recovery code**, type `- - -` and select **Unlock**. | Step 1: after the usual delay, "That master password is not correct.": spaces are a password like any other. Step 2: at once, "That recovery code is not valid. Check it for typos: it has 8 groups of 4 characters." | ☐ Pass ☐ Fail | |
| C-ED-06 | Clearing a field while editing | 1. Edit an item with a username and a website; delete both and save. | "Changes saved." **Username** and **Website** show "Empty" and have no **Copy** button. | ☐ Pass ☐ Fail | |

### 5.3 Very long text

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| C-ED-07 | Very long title | 1. Put 10 000 characters on the clipboard (section 5.1).<br>2. Create a note, paste them into **Title**, type `Long` in **Text**, and save.<br>3. Look at the list, the details and the edit form, at full and at minimum width. | "Item added." In the list the title is on one line, cut off with an ellipsis, and the item's type is still visible. In the details the title wraps inside the window; there is no horizontal scroll bar. The app stays responsive. | ☐ Pass ☐ Fail | |
| C-ED-08 | Long value without spaces | 1. Put 10 000 characters on the clipboard.<br>2. Create a password item titled `Long values`, paste them into **Username**, **Website** and **Password**, and save.<br>3. Reveal **Password**; copy it and paste it into a text editor. | The username and website wrap inside the details; the revealed password wraps too. The pasted password has exactly 10 000 characters. | ☐ Pass ☐ Fail | |
| C-ED-09 | Large note within the limit | 1. Put 1 000 000 characters on the clipboard.<br>2. Create a note titled `Large note`, paste them into **Text**, and save.<br>3. Reveal **Text**, then lock and unlock and open the item. | Step 2: "Item added.", perhaps after a short delay. Step 3: the text is shown in full and the app stays usable; scrolling may be slow. Record how long saving and revealing took. | ☐ Pass ☐ Fail | |
| C-ED-10 | Item over the size limit | 1. Put 1 100 000 characters on the clipboard.<br>2. Create a note titled `Too large`, paste them into **Text**, and save. | "The item could not be saved. It may be too large." The form stays open with what you typed; nothing is added. Cancelling returns to the vault, which still works normally. | ☐ Pass ☐ Fail | |
| C-ED-11 | Limit counts bytes, not characters | 1. Put 400 000 copies of a 3-byte character on the clipboard. Windows: `Set-Clipboard (([string][char]0x65E5) * 400000)`; macOS and Linux: `printf '日%.0s' $(seq 400000) \| pbcopy` (or `wl-copy`, or `xclip -selection clipboard`).<br>2. Create a note, paste into **Text**, and save. | "The item could not be saved. It may be too large.": 400 000 characters take about 1.2 MB in UTF-8. With 300 000 copies (about 900 KB) the note is saved. | ☐ Pass ☐ Fail | |
| C-ED-12 | Editing an item past the limit | 1. Edit `Large note` from C-ED-09, select **Reveal current text**, add 100 000 more characters at the end, and save. | "The item could not be saved. It may be too large." After cancelling, `Large note` still has its original text. | ☐ Pass ☐ Fail | |
| C-ED-13 | Long attachment name | 1. In `nv-test`, create a file whose name has 200 characters, for example 196 `a` and `.txt`. Windows: `New-Item -ItemType File ("$HOME\nv-test\" + ('a' * 196) + '.txt')`; macOS and Linux: `touch ~/nv-test/$(printf 'a%.0s' $(seq 196)).txt`.<br>2. Attach it to an item, then export it. | "Attached “aaaa…a.txt”." The name wraps inside the attachment list without pushing **Export** and **Remove** out of view. The export dialog suggests the full name. | ☐ Pass ☐ Fail | |
| C-ED-14 | Attachment name over 255 bytes | Windows only (most Linux and macOS file systems refuse such names).<br>1. Create a file whose name has 130 copies of a 3-byte character: `New-Item -ItemType File ("$HOME\nv-test\" + (([string][char]0x65E5) * 130) + '.txt')`. Its name is 134 characters but 394 bytes in UTF-8.<br>2. Attach it to an item. | "The item could not be saved. It may be too large." Nothing is attached, the item is unchanged, and no new `.nvatt` file remains in `vault.nvault.attachments`. | ☐ Pass ☐ Fail | |

### 5.4 Unicode and emoji

Use these sample strings. Copy them from this document rather than typing them, so that they are exact.

| Name | Text |
| --- | --- |
| Mixed scripts | `Ünïcødé 日本語 Привет Ελληνικά` |
| Emoji | `🔐 Vault 👍🏽 👨‍👩‍👧 🇪🇸` (includes a skin tone, a family joined with zero-width joiners, and a flag) |
| Right to left | `שלום مرحبا 123` |
| Combining accent | `Cafe` followed by U+0301 COMBINING ACUTE ACCENT (looks like `Café`). Windows: `Set-Clipboard ("Cafe" + [char]0x0301)`; macOS and Linux: `printf 'Cafe\xcc\x81' \| pbcopy` (or `wl-copy`, or `xclip -selection clipboard`). |

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| C-ED-15 | Unicode in every field | 1. Create a password item titled with the mixed-scripts text, with the emoji text as **Username**, the right-to-left text as **Password**, and all three on separate lines in **Notes**.<br>2. Reveal every field; copy each one and paste it into a text editor. | Every value is shown exactly as entered, including the family emoji as a single symbol and the flag. Pasted values are identical to the originals. The right-to-left text reads right to left; record how it is aligned. | ☐ Pass ☐ Fail | |
| C-ED-16 | Unicode survives a restart | 1. Quit and restart the app, unlock, and open the item from C-ED-15. | Every field reveals the same text as before. | ☐ Pass ☐ Fail | |
| C-ED-17 | Search with Unicode | 1. Search for `日本`, then `привет`, then `unicød`, then `unicode`.<br>2. Create a note titled with the combining-accent text, then search for `cafe` and for `café` (typed with a precomposed `é`). | Step 1: `日本`, `привет` (ignoring case) and `unicød` (ignoring the accents on `Ü` and `ï`) find the item from C-ED-15. `unicode` does not: `ø` is a letter of its own, not an `o` with an accent. Step 2: both searches find the note. | ☐ Pass ☐ Fail | |
| C-ED-18 | Emoji and accents in the master password | Run after a backup, on a fresh install (guide A, section 2.2).<br>1. Create a vault with the master password `Pässwörd-日本-🔐-orbit-lantern-violet`.<br>2. Lock and unlock with it, typed (or pasted) the same way.<br>3. Change the master password to `correct-horse-battery-staple-7` and back. | Every step succeeds. A password with non-Latin characters and emoji works like any other. Restore the backup afterwards. | ☐ Pass ☐ Fail | |
| C-ED-19 | Unicode attachment name | 1. In `nv-test`, create a file named `📄 informe – ñandú.txt` (copy the name from this document).<br>2. Attach it, export it, and compare checksums (guide B, section 2.2). | "Attached “📄 informe – ñandú.txt”." The export dialog suggests the same name and the checksums match. | ☐ Pass ☐ Fail | |
| C-ED-20 | Multi-line paste into a single-line field | 1. Copy two lines of text from a text editor.<br>2. Paste them into **Title** and into **Username** of a new password item and save. | The line break is removed or replaced by the webview (single-line fields cannot hold one); record which. The item is saved and displayed on one line. | ☐ Pass ☐ Fail | |

### 5.5 Rapid repeated actions

These cases check that a quick second action is ignored, or handled once, while the first one is still running. Buttons show what they are doing ("Saving…", "Unlocking…") and are disabled until it ends.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| C-ED-21 | Double-click Save | 1. Create a new note titled `Double save` and double-click **Save** as fast as you can.<br>2. Edit it, change the title to `Double save 2` and double-click **Save**. | Exactly one `Double save` item is created; the edit is saved once. No error message. | ☐ Pass ☐ Fail | |
| C-ED-22 | Repeated unlock | 1. Lock. Type the master password and press Enter several times quickly, or double-click **Unlock**. | The button reads "Unlocking…" and is disabled; the vault unlocks once, with no error. | ☐ Pass ☐ Fail | |
| C-ED-23 | Repeated lock | 1. Double-click **Lock**.<br>2. Unlock, and immediately select **Lock** again. | The vault locks once and shows "Unlock your vault", with no error and no reload loop. | ☐ Pass ☐ Fail | |
| C-ED-24 | Repeated copy | 1. Select **Copy** on a password ten times quickly, then paste.<br>2. Wait 35 seconds after the last click and paste again. | Step 1: the password, and "Copied. The clipboard clears in 30 seconds." Step 2: the clipboard is empty: the last copy sets the timer (as B-CB-09). | ☐ Pass ☐ Fail | |
| C-ED-25 | Reveal and hide quickly | 1. Click **Reveal** and **Hide** on a field repeatedly and quickly, ending on **Hide**. | The field ends masked. Nothing else is revealed, and no error appears. | ☐ Pass ☐ Fail | |
| C-ED-26 | Switching items quickly | 1. Click through several items in the list as fast as you can, or hold the Tab and Enter keys over the list.<br>2. Stop on one item. | The details always end up showing the last item selected: its title, fields and attachments, never those of an earlier one. | ☐ Pass ☐ Fail | |
| C-ED-27 | Double-click Delete in the confirmation | 1. Delete an item and double-click the confirming **Delete**. | "Item deleted." once; no "That item no longer exists." error. | ☐ Pass ☐ Fail | |
| C-ED-28 | Double-click Attach file | 1. Double-click **Attach file**. | Only one "Attach a file" dialog opens. While it is open, **Attach file**, **Export** and **Remove** are disabled. | ☐ Pass ☐ Fail | |
| C-ED-29 | Repeated settings changes | 1. In **Settings**, switch the theme quickly between **Light**, **Dark** and **Match system** about ten times.<br>2. Change **Lock after inactivity**, double-click **Save**, and repeat with the clipboard setting. | Step 1: the app ends in the theme last chosen, and keeps it after a restart. Step 2: each setting is saved once, with its confirmation message, and `settings.json` holds the last values. Set them back to their defaults. | ☐ Pass ☐ Fail | |
| C-ED-30 | Many items quickly | 1. Create 20 notes as fast as you can (**New note**, a title, Enter, repeat).<br>2. Lock, unlock and count the items. | Every note is created once; the count is right after unlocking. | ☐ Pass ☐ Fail | |
