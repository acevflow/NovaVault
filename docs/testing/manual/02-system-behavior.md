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
