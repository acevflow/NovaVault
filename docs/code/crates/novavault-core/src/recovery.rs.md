# [`crates/novavault-core/src/recovery.rs`](../../../../../crates/novavault-core/src/recovery.rs)

## Purpose

Generates, parses and displays emergency recovery codes. A recovery code is the second, NovaVault-generated secret that opens the vault through its own key slot (see [`slots`](slots.rs.md)) when the master password is lost. The code is designed to be written down and typed back: a 32-symbol alphabet with no look-alikes, a typo-detecting checksum, and dash-separated groups. It is held in zeroizing storage throughout.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `ALPHABET` | constant | `pub const ALPHABET: &[u8; 32] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";` | Symbols in value order. |
| `DATA_LEN` | constant | `pub const DATA_LEN: usize = 30;` | Random symbols (150 bits). |
| `CHECKSUM_LEN` | constant | `pub const CHECKSUM_LEN: usize = 2;` | Checksum symbols. |
| `CODE_LEN` | constant | `pub const CODE_LEN: usize = DATA_LEN + CHECKSUM_LEN;` | Total symbols (32). |
| `GROUP_LEN` | constant | `pub const GROUP_LEN: usize = 4;` | Symbols per display group. |
| `RecoveryCode` | struct | `pub struct RecoveryCode { canonical: SecretSlice<u8> }` | A recovery code in canonical form, zeroized on drop, redacted in `Debug`. |
| `RecoveryCode::generate` | function | `pub fn generate() -> Result<Self>` | New random code. |
| `RecoveryCode::parse` | function | `pub fn parse(input: &str) -> Result<Self>` | Parses user input. |
| `RecoveryCode::to_display` | function | `pub fn to_display(&self) -> SecretString` | `XXXX-XXXX-…` form for showing once. |
| `RecoveryCode::kdf_input` | function (crate) | `pub(crate) fn kdf_input(&self) -> &SecretSlice<u8>` | Canonical bytes for Argon2id. |

`RecoveryCode` is re-exported from the crate root; the constants are reachable as `novavault_core::recovery::*`.

## Details

### `ALPHABET`

`23456789ABCDEFGHJKLMNPQRSTUVWXYZ`. Symbol `i` encodes value `i`. The digits `0` and `1` and the letters `O` and `I` are excluded because they are easily confused with each other.

### `DATA_LEN`, `CHECKSUM_LEN`, `CODE_LEN`, `GROUP_LEN`

A code is 30 random symbols of 5 bits each (150 bits of entropy) followed by 2 checksum symbols, 32 in total. It is displayed as 8 groups of 4, for 39 characters with the dashes (`DISPLAY_LEN`, private).

### `RecoveryCode`

Holds the canonical form: `CODE_LEN` upper-case ASCII bytes from `ALPHABET`, with no separators.

- **Security**: the bytes are in a `SecretSlice<u8>`, wiped on drop. `Debug` prints `RecoveryCode([REDACTED])`. There is deliberately no `Display`, `Clone` or `PartialEq`. The only way to get the text out of the crate is `to_display`.

### `RecoveryCode::generate`

1. Fills a `Zeroizing<[u8; 30]>` from `getrandom`.
2. Keeps the low 5 bits of each byte as a symbol value. Since 256 is a multiple of 32, every value is equally likely (no modulo bias).
3. Calls the private `from_data`, which appends the checksum and maps values to symbols.

- **Returns**: a new code.
- **Errors**:
  - `CryptoError::Random`: the OS random number generator failed.
- **Security**: the random bytes and the symbol values both live in zeroizing buffers.

### `RecoveryCode::parse`

1. Walks `input` character by character and skips `-` and any Unicode whitespace.
2. Upper-cases each other character (ASCII only) and looks it up in `ALPHABET`. A character that is not found fails at once with `RecoveryCodeCharacter`.
3. Stores each value in a `Zeroizing<[u8; 32]>`. A 33rd symbol fails at once with `RecoveryCodeLength`.
4. After the loop, fewer than 32 symbols fails with `RecoveryCodeLength`.
5. Recomputes the checksum of the first 30 values and compares it with `check[0] * 32 + check[1]`. A mismatch fails with `RecoveryCodeChecksum`.
6. Rebuilds the code from the 30 data values with `from_data`.

- **Parameters**: `input`, the text typed by the user.
- **Returns**: the code in canonical form.
- **Errors**:
  - `CryptoError::RecoveryCodeCharacter`: a character outside `ALPHABET` after upper-casing.
  - `CryptoError::RecoveryCodeLength`: other than exactly 32 symbols.
  - `CryptoError::RecoveryCodeChecksum`: checksum mismatch.
- **Security**:
  - Every check is cheap and runs before any key derivation, so a typo is reported at once instead of after Argon2id.
  - The checksum comparison is not constant-time on purpose: it is computed from the user's own input and guards against typos, not attacks.
  - The parsed values are kept in zeroizing buffers. `input` itself belongs to the caller; the shell passes the inner `&str` of a `SecretString`.
- **Edge cases and invariants**: lower case, missing dashes, extra spaces and newlines are all accepted. A non-ASCII character is always rejected, because upper-casing is ASCII-only and the alphabet is ASCII.

### `RecoveryCode::to_display`

Builds a `String` with exactly `DISPLAY_LEN` capacity, inserting `-` before every fourth symbol except the first, and returns it as a `SecretString`.

- **Returns**: the display form, for example `ABCD-EFGH-JKLM-NPQR-STUV-WXYZ-2345-6789`.
- **Security**: the exact capacity means the conversion to `SecretString` does not reallocate and leave a copy behind. The returned value is wiped on drop, but any copy the caller makes (for example to send it to the UI) is not. The code is meant to be shown once and then dropped.

### `RecoveryCode::kdf_input`

Crate-internal. Returns the canonical bytes, which [`slots`](slots.rs.md) passes to `derive_key`. Keeping it `pub(crate)` means the code's text leaves the crate only through `to_display`.

### `from_data` (private)

Computes the checksum, splits it into two symbol values (`check / 32`, `check % 32`; both are below 32 because the checksum is below 1021 < 32²), and maps all 32 values through `ALPHABET` into a `Vec` with exact capacity, which becomes the `SecretSlice`. The `& 0x1F` masks are defensive: they keep every index in range.

### `checksum` (private)

`Σ (i + 1) · data[i] mod 1021`, over the 30 data values. 1021 is a prime larger than every weight (at most 30) and every difference between two symbol values (at most 31). As a result it detects:

- every single-symbol substitution in the random part, since the sum changes by `weight × difference`, which is never a multiple of 1021;
- every swap of two adjacent, different symbols in the random part, since the sum changes by exactly their difference;
- every single-symbol substitution in the two checksum symbols, since they encode the checksum value one to one.

It is an error-detecting code, not a cryptographic one, and gives an attacker nothing they do not already have.

### `symbol_value` (private)

Upper-cases an ASCII character, converts it to a byte (characters above U+00FF fail), and returns its index in `ALPHABET`.

## Dependencies

- **Internal**: [`error`](error.rs.md): `CryptoError` and `Result`.
- **External**:
  - `secrecy`: `SecretSlice` (canonical form) and `SecretString` (display form).
  - `zeroize`: `Zeroizing` for intermediate buffers.
  - `getrandom`: random symbols.

## Used by

- [`slots`](slots.rs.md): `KeySlots::create` and `regenerate_recovery_code` generate codes; `wrap_recovery_code` and `unwrap_recovery_code` use `kdf_input`.
- [`vault`](vault.rs.md): returns new codes from `create` and `regenerate_recovery_code`, and accepts them in `open_with_recovery_code` and `verify_recovery_code`.
- [`lib.rs`](lib.rs.md): re-exports `RecoveryCode`.
- [`src-tauri/src/session.rs`](../../../../../src-tauri/src/session.rs): parses codes typed by the user. [`src-tauri/src/dto.rs`](../../../../../src-tauri/src/dto.rs): sends `to_display` to the UI once.

## Tests

- [`alphabet_has_no_look_alikes_or_duplicates`](../../../../../crates/novavault-core/src/recovery.rs#L224): `0`, `1`, `O` and `I` are absent, and all 32 symbols are distinct.
- [`display_form_is_grouped`](../../../../../crates/novavault-core/src/recovery.rs#L235): 39 characters, 8 groups of 4 alphabet symbols.
- [`generate_display_parse_round_trip`](../../../../../crates/novavault-core/src/recovery.rs#L247): 50 random codes parse back from their display form.
- [`known_code_checksum`](../../../../../crates/novavault-core/src/recovery.rs#L256): all-`2` data gives checksum `22`; all-`3` data gives `GK`, displayed as `3333-3333-3333-3333-3333-3333-3333-33GK`.
- [`formatting_differences_are_ignored`](../../../../../crates/novavault-core/src/recovery.rs#L267): lower case, no dashes, spaces and surrounding whitespace parse to the same code.
- [`every_single_substitution_is_rejected`](../../../../../crates/novavault-core/src/recovery.rs#L283): every substitution at every one of the 32 positions fails the checksum.
- [`every_adjacent_swap_in_random_part_is_rejected`](../../../../../crates/novavault-core/src/recovery.rs#L303): every adjacent swap of different symbols in the random part fails the checksum.
- [`wrong_length_is_rejected`](../../../../../crates/novavault-core/src/recovery.rs#L321): 31, 33 and 0 symbols give `RecoveryCodeLength`.
- [`invalid_characters_are_rejected`](../../../../../crates/novavault-core/src/recovery.rs#L338): `0`, `1`, `O`, `o`, `I`, `i`, `_`, `é` and NUL give `RecoveryCodeCharacter`.
- [`generated_codes_are_distinct`](../../../../../crates/novavault-core/src/recovery.rs#L352): two codes differ.
- [`debug_redacts_code`](../../../../../crates/novavault-core/src/recovery.rs#L359): `Debug` prints `RecoveryCode([REDACTED])`.

## Notes

- **Error order differs from the specification.** [`vault-format.md` §10](../../../../vault-format.md#10-recovery-code) says NovaVault checks "a character outside the alphabet, … not exactly 32 symbols, or fails the checksum … in that order". The code checks characters and the 33-symbol limit in a single pass, in input order. Input with more than 32 valid symbols followed by an invalid character therefore yields `RecoveryCodeLength`, not `RecoveryCodeCharacter`. Both are rejected before Argon2id, so security is unaffected; only the reported error differs.
- The detection guarantees do not cover a swap between the last random symbol and the first checksum symbol. Such a swap is usually caught, but nothing guarantees it, and neither the code nor the specification claims otherwise. A swap of the two checksum symbols is always caught, because it changes the encoded value whenever the symbols differ.
- The two checksum symbols can encode values 1021 to 1023, which no valid code ever has. Input containing them fails the checksum, as intended.
