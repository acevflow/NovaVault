# [`crates/novavault-core/src/recovery.rs`](../../../../../crates/novavault-core/src/recovery.rs#L211) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `recovery.rs` checks the recovery code format: the alphabet, the grouped display form, generate/display/parse round trips, a hand-computed checksum vector, tolerance of formatting differences, and exhaustive typo detection (every single substitution and every adjacent swap). It reads the private canonical form directly to compare codes.

## Public API

| Name                                           | Kind   | Signature                                               | Description                                                                 |
| ---------------------------------------------- | ------ | ------------------------------------------------------- | --------------------------------------------------------------------------- |
| `display`                                      | helper | `fn display(code: &RecoveryCode) -> String`             | Returns `to_display()` as a plain `String`.                                 |
| `canonical`                                    | helper | `fn canonical(code: &RecoveryCode) -> String`           | Returns the private 32-symbol canonical form as a `String`.                 |
| `alphabet_has_no_look_alikes_or_duplicates`    | test   | `fn alphabet_has_no_look_alikes_or_duplicates()`        | `0`, `1`, `O`, `I` are absent and the 32 symbols are distinct.              |
| `display_form_is_grouped`                      | test   | `fn display_form_is_grouped()`                          | Display form is 8 dash-separated groups of 4 alphabet symbols.              |
| `generate_display_parse_round_trip`            | test   | `fn generate_display_parse_round_trip()`                | 50 generated codes parse back from their display form unchanged.           |
| `known_code_checksum`                          | test   | `fn known_code_checksum()`                              | Two hand-computed checksum vectors (`22` and `GK`).                         |
| `formatting_differences_are_ignored`           | test   | `fn formatting_differences_are_ignored()`               | Lower case, no dashes, spaces and padding all parse to the same code.       |
| `every_single_substitution_is_rejected`        | test   | `fn every_single_substitution_is_rejected()`            | Any one-symbol change, in any of the 32 positions, fails the checksum.      |
| `every_adjacent_swap_in_random_part_is_rejected` | test | `fn every_adjacent_swap_in_random_part_is_rejected()`  | Swapping any two adjacent different random symbols fails the checksum.      |
| `wrong_length_is_rejected`                     | test   | `fn wrong_length_is_rejected()`                         | 31, 33 and 0 symbols return `RecoveryCodeLength`.                           |
| `invalid_characters_are_rejected`              | test   | `fn invalid_characters_are_rejected()`                  | Look-alikes, punctuation, non-ASCII and NUL return `RecoveryCodeCharacter`. |
| `generated_codes_are_distinct`                 | test   | `fn generated_codes_are_distinct()`                     | Two generated codes differ.                                                 |
| `debug_redacts_code`                           | test   | `fn debug_redacts_code()`                               | `Debug` prints `RecoveryCode([REDACTED])`.                                  |

## Details

### `alphabet_has_no_look_alikes_or_duplicates`

Asserts none of the bytes `0`, `1`, `O`, `I` is in `ALPHABET`, then sorts and deduplicates a copy and asserts 32 remain, so every symbol is unique and the 5-bit mapping is one to one.

### `display_form_is_grouped`

For one generated code, the display text has length `DISPLAY_LEN` (39: 32 symbols plus 7 dashes), splits on `-` into `CODE_LEN / GROUP_LEN` (8) groups, and every group has 4 bytes, all from `ALPHABET`.

### `generate_display_parse_round_trip`

Repeats 50 times: generate a code, parse its display form, and assert the canonical forms are equal. Covers the checksum computed by `from_data` agreeing with the one checked by `parse` on random data.

### `known_code_checksum`

Hand-computed vectors for the checksum `Σ (i + 1) · data[i] mod 1021`:

| Input                         | Data values  | Checksum                             | Checksum symbols                  |
| ----------------------------- | ------------ | ------------------------------------ | --------------------------------- |
| `2` × 32                      | all 0        | 0 = 0·32 + 0                         | `ALPHABET[0] ALPHABET[0]` = `22`  |
| `3` × 30 followed by `GK`     | all 1        | Σ 1..=30 = 465 = 14·32 + 17          | `ALPHABET[14] ALPHABET[17]` = `GK` |

The first parses and its canonical form is `2` × 32. The second parses and its display form is `3333-3333-3333-3333-3333-3333-3333-33GK`. These are fake codes, never valid for any real vault.

### `formatting_differences_are_ignored`

For one generated code with display text `t`, each of these parses to the same canonical form:

- `t` in lower case;
- `t` with dashes removed;
- `t` with dashes replaced by spaces;
- `t` with dashes replaced by ` - `, two leading spaces and a trailing newline.

### `every_single_substitution_is_rejected`

For one generated code, for each of the 32 positions (random and checksum symbols) and each of the 31 other alphabet symbols, replaces that one symbol and asserts `parse` returns `RecoveryCodeChecksum`. That is 992 typos, all detected, as the checksum design in [`recovery.rs.md`](recovery.rs.md) claims.

### `every_adjacent_swap_in_random_part_is_rejected`

For one generated code, for each index `0..DATA_LEN - 1` (29 pairs within the random part), skips equal neighbours and otherwise swaps them and asserts `RecoveryCodeChecksum`. Swaps that involve checksum symbols are not covered, matching the guarantee stated for `checksum`.

### `wrong_length_is_rejected`

The first 31 symbols of a code, the code with one extra `2` (33 symbols) and the empty string each return `RecoveryCodeLength`. The 33-symbol case fails as soon as the 33rd symbol has no slot, before the checksum.

### `invalid_characters_are_rejected`

Replaces the first symbol of a generated code with each of `0`, `1`, `O`, `o`, `I`, `i`, `_`, `é` and `U+0000`, and asserts `RecoveryCodeCharacter`. The lower-case look-alikes are rejected because they are upper-cased before lookup and `O`/`I` are not in the alphabet; `é` (U+00E9) converts to the byte `0xE9`, which is not in the alphabet.

### `generated_codes_are_distinct`

Two generated codes have different canonical forms (probabilistic, 150 bits of entropy).

### `debug_redacts_code`

`format!("{code:?}")` is exactly `RecoveryCode([REDACTED])`.

## Dependencies

- **Internal**: [`recovery`](recovery.rs.md) (`super::*`): `RecoveryCode`, the private `canonical` field, `ALPHABET` and the length constants including the private `DISPLAY_LEN`; [`error`](error.rs.md): `CryptoError`.
- **External**: `secrecy::ExposeSecret` (via `super::*`): reads the display string and canonical bytes.

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`crates/novavault-core/src/recovery.rs`](../../../../../crates/novavault-core/src/recovery.rs), documented in [`recovery.rs.md`](recovery.rs.md).

How to run:

```sh
cargo test -p novavault-core recovery::tests
cargo test -p novavault-core recovery::tests::every_single_substitution_is_rejected
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- The substitution and swap tests use one random code per run, so they are exhaustive over typos of that code, not over all codes. The guarantee for every code follows from the checksum argument (1021 prime, larger than every weight and difference), not from the tests.
- `kdf_input` is not tested here; it is exercised through the key slot tests in [`slots.rs.tests.md`](slots.rs.tests.md).
