# Manual test guide B: system behavior

Manual test procedure for the parts of NovaVault that depend on the operating system and the file system: clipboard auto-clear, auto-lock, encrypted file attachments, and how the vault file behaves when it is closed, copied, damaged or written by an unknown format version. It covers only features that exist in the code today.

This guide builds on [Manual test guide A: setup and core flows](01-setup-and-core-flows.md). Read guide A first: it explains how to build and start the app (section 1), where the vault is stored (section 1.4), and how to reset, back up and restore a test vault (sections 2.2 and 2.3). Those procedures are not repeated here.

Automated results are in the [Rust test report](../reports/rust.md) and the [UI test report](../reports/ui.md). The design behind the cases is documented in [Clipboard behaviour](../../clipboard.md), the [vault format specification](../../vault-format.md) and the [threat model](../../threat-model.md).

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

Never use real passwords, real files or personal data while testing. Use made-up values, the generated test files from section 2.3, and a vault that holds nothing of value.

## 1. Preconditions

Before every run:

- The preconditions of guide A, section 2.1, are met: the app builds and starts, you use a test account or machine, no other copy of NovaVault is running, and you record the test master password and recovery code.
- A test vault exists with at least one item of each kind (password, note, secret). Guide A, sections 3.1 and 3.4, create one. The cases below use the master password `correct-horse-battery-staple-7`.
- You have taken a backup of the data directory (guide A, section 2.3) so that you can return to a known state after the destructive cases in section 6.
- A text editor is open, to paste into when checking the clipboard.

Unless a case says otherwise, settings are at their defaults: **Lock after inactivity** 5 minutes and **Clear the clipboard after** 30 seconds. The settings live in `settings.json` in the data directory and are read only when the app starts.

Text in quotes is the exact wording the app should show. Mark each case by ticking one box (`☒ Pass` or `☒ Fail`), and write what you saw in **Notes**.

## 2. Helper commands

The cases refer to the commands in this section. All of them work on files you name explicitly; none of them touches the vault unless you point it at the vault.

### 2.1 Shells and working folder

- **Windows:** Windows PowerShell 5.1 or PowerShell 7. Paste the functions from section 2.4 into the session before using them; they last until the window is closed.
- **macOS and Linux:** `bash` (on macOS, start it with `bash` if your default shell is `zsh`). Paste the functions from section 2.4 into the session the same way.

Create a working folder for test files, outside the data directory:

| OS | Command |
| --- | --- |
| Windows (PowerShell) | `New-Item -ItemType Directory -Force "$HOME\nv-test"; Set-Location "$HOME\nv-test"` |
| macOS and Linux | `mkdir -p ~/nv-test && cd ~/nv-test` |

The PowerShell functions use .NET file APIs, which resolve relative paths against the process directory rather than the current PowerShell location. Always pass them full paths, such as `"$HOME\nv-test\small.bin"`.

The vault file in the commands below is written as `$VAULT`. Set it once per session:

| OS | Command |
| --- | --- |
| Windows (PowerShell) | `$VAULT = "$env:APPDATA\io.github.acevflow.novavault\vault.nvault"` |
| macOS | `VAULT=~/Library/Application\ Support/io.github.acevflow.novavault/vault.nvault` |
| Linux | `VAULT="${XDG_DATA_HOME:-$HOME/.local/share}/io.github.acevflow.novavault/vault.nvault"` |

On macOS and Linux, quote it when you use it: `"$VAULT"`.

### 2.2 Checksums

A SHA-256 checksum identifies a file's exact contents. Two files with the same checksum are identical.

| OS | Command |
| --- | --- |
| Windows (PowerShell) | `Get-FileHash -Algorithm SHA256 "<file>"` |
| Windows (Command Prompt) | `certutil -hashfile "<file>" SHA256` |
| macOS | `shasum -a 256 "<file>"` |
| Linux | `sha256sum "<file>"` |

To compare an original with its exported copy, run the command on both and check that the two hashes are equal. On Windows, PowerShell can compare them for you:

```powershell
(Get-FileHash "$HOME\nv-test\large.bin").Hash -eq (Get-FileHash "$HOME\nv-export\large.bin").Hash
```

**Expected:** `True`.

### 2.3 Test files

Attachments are encrypted in chunks of 65 536 bytes (64 KB), so the sizes below cover an empty file, the smallest file, exactly one chunk, one chunk plus one byte, a medium file and a large one. Random content makes a wrong export impossible to miss.

| Name | Size in bytes | Purpose |
| --- | --- | --- |
| `empty.bin` | 0 | Empty file: a single empty chunk. |
| `one.bin` | 1 | Smallest non-empty file. |
| `chunk.bin` | 65 536 | Exactly one full chunk. |
| `chunk-plus-one.bin` | 65 537 | One full chunk and a one-byte chunk. |
| `medium.bin` | 10 485 760 (10 MB) | Many chunks, quick transfer. |
| `large.bin` | 1 073 741 824 (1 GB) | Long enough transfer to watch the progress bar and to interrupt it. Needs 1 GB of free disk space, plus 1 GB for the encrypted copy and 1 GB for each export. |
| `Café résumé.txt` | Any | A name with accents and a space. Create it with any text editor. |

Create them with `New-TestFile` (Windows) or `head` (macOS and Linux):

| OS | Command (repeat for each name and size) |
| --- | --- |
| Windows (PowerShell) | `New-TestFile "$HOME\nv-test\chunk.bin" 65536` |
| macOS and Linux | `head -c 65536 /dev/urandom > chunk.bin` |

### 2.4 Byte-level helpers

Section 6 changes single bytes of the vault file, cuts it short, and checks its first bytes. Only ever do this:

1. with NovaVault closed, since the app may rewrite the vault on any save;
2. on a test vault you have backed up (guide A, section 2.3), so that you can restore it afterwards.

NovaVault always opens `vault.nvault` in the data directory, so the damaged file must be placed there. The backup is your copy.

**Windows (PowerShell).** Paste these functions:

```powershell
# Creates a file of $Size random bytes.
function New-TestFile([string]$Path, [long]$Size) {
  $rng = [Security.Cryptography.RandomNumberGenerator]::Create()
  $out = [IO.File]::Create($Path)
  try {
    $buffer = New-Object byte[] 1048576
    $left = $Size
    while ($left -gt 0) {
      $n = [int][Math]::Min($left, $buffer.Length)
      $rng.GetBytes($buffer)
      $out.Write($buffer, 0, $n)
      $left -= $n
    }
  } finally { $out.Dispose(); $rng.Dispose() }
}

# Inverts every bit of the byte at $Offset. A negative offset counts from the end (-1 = last byte).
function Switch-TestByte([string]$Path, [long]$Offset) {
  $bytes = [IO.File]::ReadAllBytes($Path)
  if ($Offset -lt 0) { $Offset = $bytes.Length + $Offset }
  $bytes[$Offset] = $bytes[$Offset] -bxor 0xFF
  [IO.File]::WriteAllBytes($Path, $bytes)
}

# Overwrites bytes starting at $Offset, for example: Set-TestBytes $VAULT 8 ([byte[]](0x02, 0x00))
function Set-TestBytes([string]$Path, [long]$Offset, [byte[]]$Values) {
  $bytes = [IO.File]::ReadAllBytes($Path)
  [Array]::Copy($Values, 0, $bytes, $Offset, $Values.Length)
  [IO.File]::WriteAllBytes($Path, $bytes)
}

# Cuts the file to $Length bytes.
function Set-TestLength([string]$Path, [long]$Length) {
  $stream = [IO.File]::Open($Path, 'Open')
  try { $stream.SetLength($Length) } finally { $stream.Dispose() }
}

# Prints the first $Count bytes in hexadecimal.
function Show-TestBytes([string]$Path, [int]$Count = 16) {
  ([IO.File]::ReadAllBytes($Path)[0..($Count - 1)] | ForEach-Object { $_.ToString('x2') }) -join ' '
}
```

The file size is `(Get-Item $VAULT).Length`.

**macOS and Linux (bash).** Paste these functions:

```sh
# Inverts every bit of the byte at offset $2 of file $1.
flip_byte() {
  b=$(od -A n -t u1 -j "$2" -N 1 "$1" | tr -d ' ')
  printf "$(printf '\\%03o' $((b ^ 255)))" | dd of="$1" bs=1 seek="$2" conv=notrunc 2>/dev/null
}

# Sets the byte at offset $2 of file $1 to the decimal value $3 (0-255).
set_byte() {
  printf "$(printf '\\%03o' "$3")" | dd of="$1" bs=1 seek="$2" conv=notrunc 2>/dev/null
}

# Cuts file $1 to $2 bytes.
cut_to() {
  head -c "$2" "$1" > "$1.cut" && mv "$1.cut" "$1"
}
```

The file size is `wc -c < "$VAULT"`, so the offset of the last byte is `$(( $(wc -c < "$VAULT") - 1 ))`. Show the first 16 bytes with `od -A d -t x1 -N 16 "$VAULT"`.

A healthy vault starts with `4e 4f 56 41 56 4c 54 00` (the text `NOVAVLT` and a zero byte), then the format version `01 00`, then `01` (the encryption algorithm). The layout is in the [vault format specification](../../vault-format.md#4-file-layout).

## 3. Clipboard

Start: unlocked test vault with a password item `Mail account` (username `tester@example.com`, password `S3cret-pass!`), on **Vault**, clipboard delay at its default. A stopwatch or the clock in the taskbar is enough for timing: allow a second or two either way.

"The clipboard is empty" means that pasting into the text editor inserts nothing. Before each case, copy a harmless word such as `before` from the text editor, so you can tell an emptied clipboard from one that was never written.

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| B-CB-01 | Copy shows the clear delay | 1. Select `Mail account`.<br>2. Select **Copy** next to **Password**.<br>3. Paste into the text editor. | Next to the field: "Copied. The clipboard clears in 30 seconds." The pasted text is `S3cret-pass!`. The password stays masked in the app. | ☐ Pass ☐ Fail | |
| B-CB-02 | Clipboard clears after the default delay | 1. Copy the password as in B-CB-01 and start timing.<br>2. Paste at about 25 seconds.<br>3. Paste again at about 35 seconds. | Step 2 pastes the password. Step 3 pastes nothing: the clipboard is empty, not back to `before`. | ☐ Pass ☐ Fail | |
| B-CB-03 | Plain fields are cleared too | 1. Select **Copy** next to **Username**.<br>2. Paste at once, then again after 35 seconds. | The first paste is `tester@example.com`; after 35 seconds the clipboard is empty. Every copied field is cleared, not only secret ones. | ☐ Pass ☐ Fail | |
| B-CB-04 | Clipboard setting choices | 1. Open **Settings** and find **Clipboard**.<br>2. Open **Clear the clipboard after**. | The text reads "Copied passwords and other fields are removed from the clipboard after this delay, and when the vault locks, unless you have copied something else since." The choices are "10 seconds", "30 seconds", "1 minute" and "2 minutes", with "30 seconds" selected. **Save** is disabled while the saved value is selected. | ☐ Pass ☐ Fail | |
| B-CB-05 | Change the delay | 1. Choose "10 seconds" and select **Save**.<br>2. On **Vault**, copy the password and time it. | Step 1: "Copied fields are now cleared after 10 seconds." and **Save** is disabled again. Step 2: "Copied. The clipboard clears in 10 seconds."; the clipboard is empty after about 10 seconds. | ☐ Pass ☐ Fail | |
| B-CB-06 | Longest offered delay | 1. Set the delay to "2 minutes".<br>2. Copy the password; paste at about 1 minute 50 seconds and again at about 2 minutes 10 seconds. | "Copied. The clipboard clears in 2 minutes." The first paste is the password; the second pastes nothing. | ☐ Pass ☐ Fail | |
| B-CB-07 | The delay is kept across restarts | 1. Set the delay to "1 minute".<br>2. Quit and restart the app, unlock, and open **Settings**.<br>3. Open `settings.json` in the data directory. | "1 minute" is selected. `settings.json` contains `"clipboardClearSeconds": 60`. Copying now shows "The clipboard clears in 1 minute." Set the delay back to "30 seconds" afterwards. | ☐ Pass ☐ Fail | |
| B-CB-08 | Something copied in between is left alone | 1. Copy the password.<br>2. Within a few seconds, copy the word `shopping` in the text editor.<br>3. Wait 40 seconds and paste. | The paste is `shopping`. NovaVault clears the clipboard only if it still holds the value NovaVault put there. | ☐ Pass ☐ Fail | |
| B-CB-09 | A second copy replaces the first and restarts the timer | 1. Copy the password and start timing.<br>2. At about 20 seconds, copy the username.<br>3. Paste at about 40 seconds (20 seconds after the second copy).<br>4. Paste again at about 55 seconds. | Step 3 pastes `tester@example.com`: the first timer no longer applies. Step 4 pastes nothing: the clipboard was cleared about 30 seconds after the second copy. | ☐ Pass ☐ Fail | |
| B-CB-10 | A pending clear keeps its original delay | 1. Copy the password ("clears in 30 seconds").<br>2. At once, set the delay to "2 minutes" in **Settings**.<br>3. Paste at about 35 seconds. | The clipboard is empty: a copy keeps the delay that applied when it was made. Set the delay back to "30 seconds". | ☐ Pass ☐ Fail | |
| B-CB-11 | Locking clears the clipboard at once | 1. Copy the password.<br>2. Within 5 seconds, select **Lock**.<br>3. Paste at once. | The clipboard is empty right after locking, long before the delay ends. | ☐ Pass ☐ Fail | |
| B-CB-12 | Locking keeps what you copied since | 1. Unlock, copy the password, then copy `shopping` in the text editor.<br>2. Select **Lock** and paste. | The paste is `shopping`. | ☐ Pass ☐ Fail | |
| B-CB-13 | Quitting the app clears the clipboard | 1. Unlock and copy the password.<br>2. Close the NovaVault window at once (or stop `npm run dev` with `Ctrl+C`).<br>3. Paste. | The clipboard is empty. | ☐ Pass ☐ Fail | |
| B-CB-14 | A killed app cannot clear the clipboard | 1. Start, unlock and copy the password.<br>2. Kill the process: Task Manager › **End task** on Windows, **Force Quit** (Option+Cmd+Esc) on macOS, `pkill -9 novavault` on Linux.<br>3. Paste at once and again after 40 seconds. | Windows and macOS: both pastes are the password, because nothing is left to clear it. This is a documented limitation (see [Clipboard behaviour](../../clipboard.md#what-novavault-cannot-protect-against)); copy something else to remove it. Linux: the value usually disappears with the process. Record what you saw. | ☐ Pass ☐ Fail | |
| B-CB-15 | Windows only: kept out of clipboard history | 1. Turn on clipboard history (Settings › System › Clipboard).<br>2. Copy `history-test` in the text editor, then copy the password in NovaVault.<br>3. Press Win+V. | The history shows `history-test` but not the password. | ☐ Pass ☐ Fail | |
| B-CB-16 | macOS and Linux: clipboard managers | Only if a clipboard history tool is installed (for example Maccy, Raycast or Alfred on macOS; Klipper on KDE).<br>1. Copy the password.<br>2. Open the tool's history. | A tool that honours the password markers does not list the value. Some tools ignore them: record the tool, its version and what it showed. A tool known to ignore the markers is not a NovaVault failure. | ☐ Pass ☐ Fail | |
| B-CB-17 | Linux Wayland: clearing reaches other apps | Only on a Wayland session.<br>1. Copy the password.<br>2. Switch to a native Wayland app (for example GNOME Text Editor) and stay there for 40 seconds.<br>3. Paste there, then focus NovaVault, return to the Wayland app and paste again. | With a compositor that supports data-control (KDE Plasma, Sway, Hyprland) the first paste is empty. Without it (GNOME in many releases), the clear may only reach Wayland apps once NovaVault is focused again, so the first paste can still be the password and the second is empty. This is a documented limitation ([Clipboard behaviour, Wayland](../../clipboard.md#wayland)). Record which happened. | ☐ Pass ☐ Fail | |
| B-CB-18 | Hand-edited delay outside the choices | 1. Quit the app.<br>2. In `settings.json`, set `"clipboardClearSeconds": 45` and save.<br>3. Start, unlock and open **Settings**. | "45 seconds" is selected and listed among the choices. Copying shows "The clipboard clears in 45 seconds." | ☐ Pass ☐ Fail | |
| B-CB-19 | Invalid settings fall back to defaults | 1. Quit the app.<br>2. In `settings.json`, set `"clipboardClearSeconds": 1`, start the app, unlock and open **Settings**.<br>3. Quit, replace the whole content of `settings.json` with `not json`, and repeat. | Both times the app starts normally and shows the defaults: "30 seconds" for the clipboard and "5 minutes" for auto-lock. An unusable settings file never keeps the app from starting. Saving a setting writes a valid file again. | ☐ Pass ☐ Fail | |

## 4. Auto-lock

Start: unlocked test vault, on **Vault**, auto-lock at its default, nothing on the clipboard.

How auto-lock works, so the expected results make sense:

- **Inactivity** is measured from the last request the app made to its Rust core: opening an item, revealing or copying a field, saving, changing a setting. Mouse movement, scrolling, typing in a form or in **Search by title**, and changing the **Type** filter happen only in the window and do **not** count as activity.
- The timer is checked every 5 seconds, so the vault locks between the timeout and about 5 seconds after it.
- Every automatic lock wipes the vault key, clears a copied field from the clipboard (unless you copied something else since) and reloads the window, which discards anything shown or typed and returns to "Unlock your vault".

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| B-AL-01 | Auto-lock setting choices | 1. Open **Settings** and find **Auto-lock**.<br>2. Open **Lock after inactivity**. | The text reads "The vault also locks when the computer sleeps or the screen locks." The choices are "1 minute", "5 minutes", "15 minutes", "30 minutes" and "1 hour", with "5 minutes" selected. **Save** is disabled while the saved value is selected. | ☐ Pass ☐ Fail | |
| B-AL-02 | Change the timeout | 1. Choose "1 minute" and select **Save**. | "The vault now locks after 1 minute without activity." **Save** is disabled again. | ☐ Pass ☐ Fail | |
| B-AL-03 | Lock after inactivity | 1. With a 1-minute timeout, select an item and start timing.<br>2. Do not touch the app. | Between 60 and about 65 seconds later the window shows "Unlock your vault" without any action. **Vault**, **Settings** and **Lock** are gone from the header. | ☐ Pass ☐ Fail | |
| B-AL-04 | Activity restarts the timer | 1. Unlock and start timing.<br>2. At about 45 seconds, select another item in the list.<br>3. Keep watching. | The vault is still unlocked at 70 seconds and locks about 60 to 65 seconds after step 2. | ☐ Pass ☐ Fail | |
| B-AL-05 | Typing in the window is not activity | 1. Unlock and start timing.<br>2. Keep typing in **Search by title** and moving the mouse over the window, without selecting anything. | The vault still locks 60 to 65 seconds after unlocking. This is the current design: only requests to the core count as activity. | ☐ Pass ☐ Fail | |
| B-AL-06 | Unsaved changes are discarded | 1. Unlock, select `Mail account` and select **Edit**.<br>2. Change the title to `Draft title` without saving, and wait for the auto-lock.<br>3. Unlock and select the item. | The title is still `Mail account`: the edit form was discarded and nothing was saved. | ☐ Pass ☐ Fail | |
| B-AL-07 | Revealed values are hidden by the lock | 1. Unlock, select `Mail account` and select **Reveal** on Password.<br>2. Wait for the auto-lock, then unlock and select the item again. | After the lock no password is visible anywhere; after unlocking, the password is masked again. | ☐ Pass ☐ Fail | |
| B-AL-08 | Auto-lock clears the clipboard | 1. Set the clipboard delay to "2 minutes".<br>2. Unlock, copy the password and wait for the 1-minute auto-lock.<br>3. Paste right after the lock. | The clipboard is empty, although the 2-minute delay has not ended. Set the clipboard delay back to "30 seconds". | ☐ Pass ☐ Fail | |
| B-AL-09 | The timeout is kept across restarts | 1. Quit and restart the app, unlock, and open **Settings**.<br>2. Open `settings.json` in the data directory. | "1 minute" is selected and `settings.json` contains `"autoLockMinutes": 1`. The 1-minute auto-lock applies straight after the restart. | ☐ Pass ☐ Fail | |
| B-AL-10 | Longer timeout | 1. Set the timeout to "5 minutes".<br>2. Unlock and leave the app alone for 4 minutes 30 seconds, then for 5 minutes 10 seconds in total. | Unlocked at 4 minutes 30 seconds; locked at 5 minutes 10 seconds. | ☐ Pass ☐ Fail | |
| B-AL-11 | Hand-edited timeout outside the choices | 1. Quit the app.<br>2. In `settings.json`, set `"autoLockMinutes": 2`.<br>3. Start, unlock and open **Settings**.<br>4. Repeat with `"autoLockMinutes": 0` and then `999`. | Step 3: "2 minutes" is selected and listed among the choices, and the vault locks after 2 minutes. Step 4: both values are rejected and the defaults apply ("5 minutes", and "30 seconds" for the clipboard). Set the timeout back to "1 minute" afterwards. | ☐ Pass ☐ Fail | |
| B-AL-12 | Lock on system suspend | 1. Unlock and copy the password.<br>2. Put the computer to sleep: Start › Power › **Sleep** (Windows), Apple menu › **Sleep** (macOS), `systemctl suspend` (Linux).<br>3. Wake it, sign in to the OS, and paste. | NovaVault shows "Unlock your vault" and the clipboard is empty. The lock happened before sleeping, not after the inactivity timeout. | ☐ Pass ☐ Fail | |
| B-AL-13 | Lock on screen lock | 1. Set the timeout to "1 hour", unlock and copy the password.<br>2. Lock the screen: Win+L (Windows), Ctrl+Cmd+Q (macOS), `loginctl lock-session` or the desktop's lock shortcut (Linux).<br>3. Unlock the screen at once and paste. | NovaVault shows "Unlock your vault" and the clipboard is empty, although the 1-hour timeout is far from over. | ☐ Pass ☐ Fail | |
| B-AL-14 | macOS: lock on user switch | macOS only, with a second user account.<br>1. Unlock NovaVault.<br>2. Switch to the other user with fast user switching, then switch back. | NovaVault shows "Unlock your vault". | ☐ Pass ☐ Fail | |
| B-AL-15 | Linux: OS events without logind | Optional, Linux only, on a system without logind or without a session D-Bus (for example a minimal window manager).<br>1. Start the app from a terminal.<br>2. Unlock and wait for the inactivity timeout. | The terminal may show "NovaVault: logind is unavailable; no lock on suspend or session lock" or "NovaVault: the session bus is unavailable; no lock on screen saver". The app still works and the inactivity auto-lock and **Lock** still lock the vault. Record the messages and which events still lock. | ☐ Pass ☐ Fail | |
| B-AL-16 | A locked vault stays locked | 1. Lock the vault and leave the app on "Unlock your vault" for longer than the timeout.<br>2. Lock and unlock the screen. | Nothing changes: no error, no reload loop. The master password still unlocks the vault. | ☐ Pass ☐ Fail | |
| B-AL-17 | Auto-lock while a file dialog is open | 1. Set the timeout to "1 minute" and unlock.<br>2. Select an item and select **Attach file**.<br>3. Leave the "Attach a file" dialog open for 70 seconds.<br>4. Pick `one.bin` in the dialog.<br>5. Unlock and select the item. | The vault locks while the dialog is open (the window behind it shows "Unlock your vault" once the dialog closes). The picked file is not attached: the item shows no new attachment. | ☐ Pass ☐ Fail | |
| B-AL-18 | Lock stops a running transfer | 1. Set the timeout to "1 hour". Select an item, select **Attach file** and pick `large.bin`.<br>2. While the progress bar moves, lock the screen (or select **Lock**).<br>3. Unlock the screen and the vault, and select the item.<br>4. Run this step after B-AT-05 has attached `large.bin`: repeat with **Export** of it, saving to a new `nv-export` folder. | The vault locks within a moment instead of waiting for the transfer to finish. Step 3: `large.bin` is not listed (or is listed only if the transfer had finished before the lock). Step 4: no file, complete or partial, is left at the export destination. | ☐ Pass ☐ Fail | |

Set the timeout back to "5 minutes" before the next section.

## 5. Attachments

Start: unlocked test vault with a note `Attachment test` that has no attachments, the test files from section 2.3 in `nv-test`, and an empty `nv-export` folder next to it. Attachments are shown in the item's details, under **Attachments**.

Encrypted attachment contents are stored in the folder `vault.nvault.attachments` next to the vault file, one file per attachment, named with 32 hexadecimal characters and the extension `.nvatt`. List them with:

| OS | Command |
| --- | --- |
| Windows (PowerShell) | `Get-ChildItem "$VAULT.attachments" \| Select-Object Name, Length` |
| macOS and Linux | `ls -l "$VAULT.attachments"` |

Each encrypted file is larger than the original by 48 bytes plus 40 bytes per 64 KB chunk (at least one chunk):

| Test file | Original size | Encrypted size | Size shown in the app |
| --- | --- | --- | --- |
| `empty.bin` | 0 | 88 | "0 bytes" |
| `one.bin` | 1 | 89 | "1 byte" |
| `chunk.bin` | 65 536 | 65 624 | "64 KB" |
| `chunk-plus-one.bin` | 65 537 | 65 665 | "64 KB" |
| `medium.bin` | 10 485 760 | 10 492 208 | "10 MB" |
| `large.bin` | 1 073 741 824 | 1 074 397 232 | "1.0 GB" |

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| B-AT-01 | No attachments yet | 1. Select `Attachment test`. | Under **Attachments**: "No files attached." and an **Attach file** button. The `vault.nvault.attachments` folder may not exist yet. | ☐ Pass ☐ Fail | |
| B-AT-02 | Close the file dialog | 1. Select **Attach file**.<br>2. Close the dialog without choosing a file. | While the dialog is open the app shows "Attaching file" with "Choose a file in the dialog…". The dialog title is "Attach a file". After closing it, no message is shown and nothing is attached. | ☐ Pass ☐ Fail | |
| B-AT-03 | Attach small files | 1. Attach `empty.bin`, then `one.bin`, `chunk.bin` and `chunk-plus-one.bin`, one at a time. | After each: "Attached “&lt;name&gt;”." The list shows the files in the order added, each with its name, the size from the table above and "added &lt;date and time&gt;", and **Export** and **Remove** buttons. | ☐ Pass ☐ Fail | |
| B-AT-04 | Attach a file with accents in its name | 1. Attach `Café résumé.txt`. | "Attached “Café résumé.txt”." The name is shown exactly, accents and space included. | ☐ Pass ☐ Fail | |
| B-AT-05 | Attach medium and large files | 1. Attach `medium.bin`.<br>2. Attach `large.bin` and watch the app until it finishes. | A progress bar appears with "&lt;percent&gt;% · &lt;done&gt; of &lt;total&gt;" (for example "37% · 379 MB of 1.0 GB") and advances to 100%. **Attach file**, **Export** and **Remove** are disabled until the transfer ends. Both files are then listed with "10 MB" and "1.0 GB". The app stays responsive and does not use memory in proportion to the file size (optional: watch it in Task Manager, Activity Monitor or `top`). | ☐ Pass ☐ Fail | |
| B-AT-06 | Encrypted files on disk | 1. List the attachments folder (command above).<br>2. Open one `.nvatt` file of a text attachment, such as `Café résumé.txt`, in a text editor or with `Show-TestBytes`/`od`. | One `.nvatt` file per attachment (7 so far), with the encrypted sizes from the table above. File names are 32 hexadecimal characters and do not reveal the original names. Each file starts with `4e 56 41 54 54 43 48 00` (`NVATTCH`) and the original text is not readable anywhere in it. | ☐ Pass ☐ Fail | |
| B-AT-07 | Export and compare checksums | 1. Select **Export** next to each attachment and save it to `nv-export`.<br>2. Compute the SHA-256 of every original in `nv-test` and its export in `nv-export` (section 2.2). | The dialog is titled "Export attachment" and suggests the original file name. While it is open the app shows "Exporting “&lt;name&gt;”" with "Choose where to save it in the dialog…", then a progress bar for large files, then "Exported “&lt;name&gt;”.". Every exported file has the same size and the same SHA-256 as its original, including `empty.bin` (0 bytes). | ☐ Pass ☐ Fail | |
| B-AT-08 | Close the export dialog | 1. Select **Export** and close the dialog without saving. | No message is shown and no file is written. | ☐ Pass ☐ Fail | |
| B-AT-09 | Replace an existing export | 1. Export `one.bin` to `nv-export` again, choosing the same name. | The OS asks whether to replace the existing file. Confirming replaces it; the checksum still matches the original. Declining writes nothing. | ☐ Pass ☐ Fail | |
| B-AT-10 | Export onto the vault file is refused | 1. Compute the SHA-256 of `vault.nvault`.<br>2. Export any attachment, navigate to the data directory in the dialog (macOS: Cmd+Shift+G and type the path) and choose `vault.nvault` as the name. Confirm the replacement if asked.<br>3. Compute the SHA-256 again, then lock and unlock. | "The file could not be read or saved there. Check the file or choose another place." The vault file's checksum is unchanged and the vault unlocks normally. | ☐ Pass ☐ Fail | |
| B-AT-11 | Attachments persist | 1. Quit and restart the app, unlock and select `Attachment test`.<br>2. Export `medium.bin` again and compare checksums. | All attachments are listed with the same names, sizes and dates, in the same order. The export matches the original. | ☐ Pass ☐ Fail | |
| B-AT-12 | Remove asks for confirmation | 1. Select **Remove** next to `one.bin`.<br>2. Select **Cancel**.<br>3. Select **Remove** again and press Escape. | "Remove this attachment?" and "“one.bin” will be removed from this item and its encrypted copy deleted. This cannot be undone." with **Remove** and **Cancel**; focus starts on **Cancel**. Both times the question closes and `one.bin` is still listed. | ☐ Pass ☐ Fail | |
| B-AT-13 | Remove an attachment | 1. Note the number of `.nvatt` files.<br>2. Select **Remove** next to `one.bin` and confirm with **Remove**.<br>3. List the attachments folder again.<br>4. Lock and unlock, then select the item. | "Removing…", then "Removed “one.bin”.", and it is gone from the list. The folder holds one `.nvatt` file fewer. It is still gone after unlocking. | ☐ Pass ☐ Fail | |
| B-AT-14 | Deleting an item deletes its attachments | 1. Create a note `Throwaway`, attach `chunk.bin` and `medium.bin`, and note the number of `.nvatt` files.<br>2. Delete the note (**Delete**, then **Delete**).<br>3. List the attachments folder. | The note is deleted and the folder holds two `.nvatt` files fewer. The attachments of `Attachment test` still export correctly. | ☐ Pass ☐ Fail | |
| B-AT-15 | A damaged attachment is detected | 1. Quit the app and back up the data directory (guide A, section 2.3).<br>2. Find the `.nvatt` file of `medium.bin` (the one of 10 492 208 bytes) and invert one byte in its middle: `Switch-TestByte "<path>" 5000000` (Windows) or `flip_byte "<path>" 5000000` (macOS and Linux).<br>3. Start, unlock and export `medium.bin` to `nv-export` under a new name.<br>4. Export `chunk.bin`.<br>5. Quit and restore the backup. | Step 3: "The vault data is damaged and cannot be read." and no file with the new name exists in `nv-export`, not even a partial one. Step 4: works and the checksum matches: one damaged attachment does not affect the others or the items. After step 5, `medium.bin` exports correctly again. | ☐ Pass ☐ Fail | |

## 6. Persistence

Start: the test vault from section 5, with items of each kind and some attachments, unlocked. Write down its master password and recovery code.

### 6.1 Closing, reopening and copying

| ID | Title | Steps | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| B-PS-01 | Everything survives a restart | 1. Note the item titles, the attachments of `Attachment test` and both settings.<br>2. Quit the app and start it again. | The app opens on "Unlock your vault", never unlocked. After unlocking: the same items with the same field values, the same attachments (an export matches its original's checksum), and the same settings. | ☐ Pass ☐ Fail | |
| B-PS-02 | No temporary files are left | 1. Make a few edits (add an item, attach and remove a file), then quit the app.<br>2. List the data directory and the attachments folder, including hidden files (`Get-ChildItem -Force` on Windows, `ls -la` on macOS and Linux). | No file ending in `.tmp` remains in either place: every save writes a temporary file and renames it over the target. Only `vault.nvault`, `vault.nvault.attachments` and `settings.json` are in the data directory. | ☐ Pass ☐ Fail | |
| B-PS-03 | A copy elsewhere is not used | 1. Quit the app and copy `vault.nvault` to `nv-test` as `vault-copy.nvault`. Compute its SHA-256.<br>2. Start the app, unlock, add an item `After copy`, and quit.<br>3. Compute the copy's SHA-256 again. | The copy is unchanged: the app only ever opens `vault.nvault` in the data directory and never asks for a path. | ☐ Pass ☐ Fail | |
| B-PS-04 | Restore a full copy | 1. Quit the app. Copy the whole data directory to `nv-test/data-copy`.<br>2. Delete the data directory (guide A, section 2.2, step 3) and start the app.<br>3. Quit, copy `data-copy` back to the data directory path, and start the app. | Step 2: "Create your vault". Step 3: "Unlock your vault"; the master password and the recovery code both unlock it, every item is there, and every attachment exports with a matching checksum. | ☐ Pass ☐ Fail | |
| B-PS-05 | Open the vault on another account or computer | Optional: needs a second OS account, or a second computer with NovaVault built from the same commit (any of the three OSes).<br>1. Copy the whole data directory to the same location on the other account or computer.<br>2. Start NovaVault there and unlock. | The vault unlocks with the same master password and recovery code. Items and attachments are identical (compare an export's checksum). The format does not depend on the OS or the account. Record the source and target OS. | ☐ Pass ☐ Fail | |
| B-PS-06 | Vault file copied without its attachments folder | 1. Quit the app and back up the data directory.<br>2. Rename `vault.nvault.attachments` to `attachments-moved` (as if only `vault.nvault` had been copied).<br>3. Start, unlock and select `Attachment test`.<br>4. Export `chunk.bin`.<br>5. Quit and restore the backup. | Step 3: the items and the attachment list (names, sizes, dates) are all shown, since they are stored in the vault file. Step 4: "The vault data is damaged and cannot be read." and no file is written. After step 5 the export works again. Backups must include the attachments folder. | ☐ Pass ☐ Fail | |
| B-PS-07 | An older copy is accepted (rollback) | 1. Quit and back up the data directory as `backup-old`.<br>2. Start, unlock, add an item `Newer item`, and quit.<br>3. Restore `backup-old` and start the app. | The restored vault unlocks without any warning and `Newer item` is missing. Replacing the whole vault with an older valid copy is not detected; this is a documented limitation (see the [threat model](../../threat-model.md#cryptography-and-key-management)). | ☐ Pass ☐ Fail | |
| B-PS-08 | Stray files in the attachments folder | 1. Quit the app.<br>2. In the attachments folder, create a file named `0123456789abcdef0123456789abcdef.nvatt` with any content, and a file named `readme.txt`.<br>3. Start the app, unlock, and list the folder. | The `.nvatt` file that no item refers to has been deleted when the vault was unlocked. `readme.txt` is still there: files that do not look like attachments are ignored. All real attachments still export. Delete `readme.txt` afterwards. | ☐ Pass ☐ Fail | |

### 6.2 Damaged files and unknown versions

Each case below damages `vault.nvault` on purpose. Before starting:

1. Quit the app.
2. Back up the data directory (guide A, section 2.3) and keep that backup until the end of the section. Do not use it for anything else.
3. Set `$VAULT` (section 2.1) and paste the helper functions (section 2.4).

For **each** case:

1. Make sure the app is closed and that `vault.nvault` is the original (restore the backup if needed).
2. Apply the change in the **Change** column.
3. Compute the SHA-256 of `vault.nvault` (section 2.2).
4. Start the app and try to unlock with the credential named in the expected result: the master password, and where stated the recovery code too.
5. Quit the app and compute the SHA-256 again. **Expected for every case:** it is unchanged: NovaVault never rewrites or "repairs" a file it cannot open.
6. Restore the backup.

Throughout, "damaged" means the message "The vault data is damaged and cannot be read." and "not supported" means "The vault file is not supported by this version of NovaVault. You may need a newer version." After any failed attempt the vault stays locked, the field is emptied and the app stays on "Unlock your vault"; it offers no way to create a new vault over the existing file.

"At once" means the message appears without the usual one-second key derivation delay: the file is rejected by its structure before any key is derived.

| ID | Title | Change | Expected result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- |
| B-PS-09 | Integrity seal damaged | Invert the last byte.<br>Windows: `Switch-TestByte $VAULT -1`<br>macOS/Linux: `flip_byte "$VAULT" $(( $(wc -c < "$VAULT") - 1 ))` | Master password: "damaged", after the usual delay. Recovery code: the same. | ☐ Pass ☐ Fail | |
| B-PS-10 | Item data damaged | Invert byte 300, inside the first item (the vault must hold at least one item).<br>Windows: `Switch-TestByte $VAULT 300`<br>macOS/Linux: `flip_byte "$VAULT" 300` | Master password: "damaged", after the usual delay. No item is shown. | ☐ Pass ☐ Fail | |
| B-PS-11 | Password key slot damaged | Invert byte 90, inside the master password slot.<br>Windows: `Switch-TestByte $VAULT 90`<br>macOS/Linux: `flip_byte "$VAULT" 90` | Master password: "That master password is not correct." (a damaged slot cannot be told apart from a wrong password). Recovery code: "damaged", because the integrity seal covers both slots. | ☐ Pass ☐ Fail | |
| B-PS-12 | Recovery key slot damaged | Invert byte 190, inside the recovery code slot.<br>Windows: `Switch-TestByte $VAULT 190`<br>macOS/Linux: `flip_byte "$VAULT" 190` | Recovery code: "That recovery code is not correct. It may have been replaced by a newer one." Master password: "damaged". | ☐ Pass ☐ Fail | |
| B-PS-13 | Last byte cut off | Windows: `Set-TestLength $VAULT ((Get-Item $VAULT).Length - 1)`<br>macOS/Linux: `cut_to "$VAULT" $(( $(wc -c < "$VAULT") - 1 ))` | Master password: "damaged", at once. | ☐ Pass ☐ Fail | |
| B-PS-14 | Extra byte appended | Windows: `Set-TestLength $VAULT ((Get-Item $VAULT).Length + 1)` (adds a zero byte)<br>macOS/Linux: `printf '\0' >> "$VAULT"` | Master password: "damaged", at once. | ☐ Pass ☐ Fail | |
| B-PS-15 | Cut to half its size | Windows: `Set-TestLength $VAULT ([long]((Get-Item $VAULT).Length / 2))`<br>macOS/Linux: `cut_to "$VAULT" $(( $(wc -c < "$VAULT") / 2 ))` | Master password: "damaged", at once. | ☐ Pass ☐ Fail | |
| B-PS-16 | Cut inside the header | Keep only the first 20 bytes.<br>Windows: `Set-TestLength $VAULT 20`<br>macOS/Linux: `cut_to "$VAULT" 20` | Master password: "damaged", at once. | ☐ Pass ☐ Fail | |
| B-PS-17 | Empty file | Windows: `Set-TestLength $VAULT 0`<br>macOS/Linux: `cut_to "$VAULT" 0` | The app still opens on "Unlock your vault" (a vault file exists). Master password: "not supported", at once. A file too short to carry the NovaVault signature is reported as not a vault file, which shares this message. | ☐ Pass ☐ Fail | |
| B-PS-18 | Not a vault file | Replace the content with text.<br>Windows: `Set-Content -Encoding ascii $VAULT "hello"`<br>macOS/Linux: `echo hello > "$VAULT"`<br>Repeat with the original file and only byte 0 changed: `Switch-TestByte $VAULT 0` / `flip_byte "$VAULT" 0`. | Both times, master password: "not supported", at once. | ☐ Pass ☐ Fail | |
| B-PS-19 | Newer format version | Set the version (bytes 8 and 9) to 2.<br>Windows: `Set-TestBytes $VAULT 8 ([byte[]](0x02, 0x00))`<br>macOS/Linux: `set_byte "$VAULT" 8 2; set_byte "$VAULT" 9 0`<br>Check with `Show-TestBytes $VAULT` / `od -A d -t x1 -N 16 "$VAULT"`: bytes 8 and 9 read `02 00`.<br>Repeat with `0xFF, 0xFF` / `255` and `255`. | Master password and recovery code: "not supported", at once, for both versions. A file written by a newer NovaVault is reported as such, not as damaged. | ☐ Pass ☐ Fail | |
| B-PS-20 | Older version number on a current file | Set the version to 0.<br>Windows: `Set-TestBytes $VAULT 8 ([byte[]](0x00, 0x00))`<br>macOS/Linux: `set_byte "$VAULT" 8 0; set_byte "$VAULT" 9 0` | Master password: "damaged", after the usual delay. Version 0 is a supported version, so the file is read, but the changed header fails the integrity check. The file is not migrated or modified. | ☐ Pass ☐ Fail | |
| B-PS-21 | Unknown encryption algorithm | Set byte 10 to 2.<br>Windows: `Set-TestBytes $VAULT 10 ([byte[]](0x02))`<br>macOS/Linux: `set_byte "$VAULT" 10 2` | Master password: "not supported", at once. | ☐ Pass ☐ Fail | |
| B-PS-22 | Restoring the backup recovers the vault | After the last case, restore the backup and start the app. | The vault unlocks with the master password and with the recovery code, and all items and attachments are intact. | ☐ Pass ☐ Fail | |
