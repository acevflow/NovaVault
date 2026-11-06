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
