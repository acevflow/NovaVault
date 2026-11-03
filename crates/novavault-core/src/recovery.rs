//! Emergency recovery codes.
//!
//! A recovery code is a second, NovaVault-generated secret that opens the
//! vault through its own key slot when the master password is lost. It is
//! shown to the user once, so it must be easy to write down and type back:
//!
//! - **Alphabet:** the 32 symbols `23456789ABCDEFGHJKLMNPQRSTUVWXYZ`. The
//!   digits `0` and `1` and the letters `O` and `I` are left out because
//!   they are easily confused with each other.
//! - **Entropy:** 30 random symbols of 5 bits each, 150 bits in total.
//! - **Checksum:** 2 extra symbols that catch typos before the slow key
//!   derivation runs (see [`checksum`]).
//! - **Display:** the 32 symbols in 8 groups of 4 separated by dashes, for
//!   example `ABCD-EFGH-JKLM-NPQR-STUV-WXYZ-2345-6789`.
//!
//! Parsing is case-insensitive and ignores dashes and whitespace, so
//! formatting differences never stop a correct code from working. The
//! canonical form (32 upper-case symbols, no separators) is what the key
//! derivation uses.

use secrecy::{ExposeSecret, SecretSlice, SecretString};
use zeroize::Zeroizing;

use crate::error::{CryptoError, Result};

/// Symbols used by recovery codes, in value order (symbol `i` encodes `i`).
pub const ALPHABET: &[u8; 32] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

/// Number of random symbols in a recovery code (150 bits of entropy).
pub const DATA_LEN: usize = 30;

/// Number of checksum symbols appended to the random symbols.
pub const CHECKSUM_LEN: usize = 2;

/// Total number of symbols in a recovery code.
pub const CODE_LEN: usize = DATA_LEN + CHECKSUM_LEN;

/// Number of symbols per dash-separated group in the display form.
pub const GROUP_LEN: usize = 4;

/// Modulus of the checksum. A prime larger than every weight and every
/// difference between two symbol values, which is what makes the error
/// detection guarantees of [`checksum`] hold. It fits in two symbols
/// (1021 < 32²).
const CHECKSUM_MODULUS: u32 = 1021;

/// Length of the display form: symbols plus one dash between groups.
const DISPLAY_LEN: usize = CODE_LEN + CODE_LEN / GROUP_LEN - 1;

/// An emergency recovery code.
///
/// The code is held in zeroizing storage and is never printed by `Debug`.
/// There is deliberately no `Display` implementation: the only way to get
/// the text is [`RecoveryCode::to_display`], which is meant for the single
/// moment the code is shown to the user.
pub struct RecoveryCode {
    /// The canonical form: [`CODE_LEN`] ASCII symbols from [`ALPHABET`].
    canonical: SecretSlice<u8>,
}

impl RecoveryCode {
    /// Generates a new random recovery code from the OS CSPRNG.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::Random`] if the OS random number generator fails.
    pub fn generate() -> Result<Self> {
        let mut random = Zeroizing::new([0u8; DATA_LEN]);
        getrandom::fill(random.as_mut_slice()).map_err(|_| CryptoError::Random)?;
        let mut values = Zeroizing::new([0u8; DATA_LEN]);
        for (value, byte) in values.iter_mut().zip(random.iter()) {
            // 256 is a multiple of 32, so keeping the low 5 bits of a uniform
            // byte yields a uniform symbol with no modulo bias.
            *value = byte & 0x1F;
        }
        Ok(Self::from_data(&values))
    }

    /// Parses a recovery code typed by the user.
    ///
    /// Letters may be in either case; dashes and whitespace are ignored.
    ///
    /// # Errors
    ///
    /// - [`CryptoError::RecoveryCodeCharacter`] if the input contains a
    ///   character outside [`ALPHABET`] (after upper-casing).
    /// - [`CryptoError::RecoveryCodeLength`] if it does not contain exactly
    ///   [`CODE_LEN`] symbols.
    /// - [`CryptoError::RecoveryCodeChecksum`] if the checksum does not match.
    pub fn parse(input: &str) -> Result<Self> {
        let mut values = Zeroizing::new([0u8; CODE_LEN]);
        let mut count = 0;
        for ch in input.chars() {
            if ch == '-' || ch.is_whitespace() {
                continue;
            }
            let value = symbol_value(ch).ok_or(CryptoError::RecoveryCodeCharacter)?;
            let slot = values
                .get_mut(count)
                .ok_or(CryptoError::RecoveryCodeLength)?;
            *slot = value;
            count += 1;
        }
        if count != CODE_LEN {
            return Err(CryptoError::RecoveryCodeLength);
        }

        let (data, check) = values.split_at(DATA_LEN);
        let expected = checksum(data);
        let actual = u32::from(check[0]) * 32 + u32::from(check[1]);
        // Constant-time comparison is not needed: the checksum is derived
        // from the user's own input and protects against typos, not attacks.
        if actual != expected {
            return Err(CryptoError::RecoveryCodeChecksum);
        }

        let mut data_values = Zeroizing::new([0u8; DATA_LEN]);
        data_values.copy_from_slice(data);
        Ok(Self::from_data(&data_values))
    }

    /// Returns the code in its display form, 8 dash-separated groups of 4
    /// symbols, for showing to the user once.
    pub fn to_display(&self) -> SecretString {
        let mut text = String::with_capacity(DISPLAY_LEN);
        for (index, &symbol) in self.canonical.expose_secret().iter().enumerate() {
            if index > 0 && index % GROUP_LEN == 0 {
                text.push('-');
            }
            text.push(char::from(symbol));
        }
        // `text` was allocated with its exact final length, so converting it
        // to a boxed `str` does not reallocate and leave a copy behind.
        SecretString::from(text)
    }

    /// Builds a code from its random symbol values (each below 32), appending
    /// the checksum symbols.
    fn from_data(data: &[u8; DATA_LEN]) -> Self {
        let check = checksum(data);
        // `checksum` is below 1021 < 32², so both quotient and remainder are
        // valid symbol values.
        let check_values = [(check / 32) as usize, (check % 32) as usize];

        let mut canonical = Vec::with_capacity(CODE_LEN);
        canonical.extend(
            data.iter()
                .map(|&value| ALPHABET[usize::from(value & 0x1F)]),
        );
        canonical.extend(check_values.iter().map(|&value| ALPHABET[value & 0x1F]));
        // Exact capacity: `SecretSlice::from` will not reallocate.
        Self {
            canonical: SecretSlice::from(canonical),
        }
    }
}

impl core::fmt::Debug for RecoveryCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("RecoveryCode([REDACTED])")
    }
}

/// Computes the checksum of the random symbol values `data`.
///
/// The checksum is `Σ (i + 1) · data[i] mod 1021`. Because 1021 is a prime
/// larger than every weight (at most 30) and every difference between two
/// symbol values (at most 31), it detects:
///
/// - every single-symbol substitution in the random part, since the sum
///   changes by `weight × difference`, which is never a multiple of 1021;
/// - every swap of two adjacent different symbols in the random part, since
///   the sum changes by exactly their difference;
/// - every single-symbol substitution in the checksum itself, since the two
///   symbols encode the checksum value one to one.
///
/// It is an error-detecting code, not a cryptographic one: it gives an
/// attacker nothing, as it is computed from the code itself.
fn checksum(data: &[u8]) -> u32 {
    data.iter()
        .zip(1u32..)
        .map(|(&value, weight)| weight * u32::from(value))
        .sum::<u32>()
        % CHECKSUM_MODULUS
}

/// Maps a user-typed character to its symbol value, accepting lower case.
fn symbol_value(ch: char) -> Option<u8> {
    let upper = u8::try_from(ch.to_ascii_uppercase()).ok()?;
    ALPHABET
        .iter()
        .position(|&symbol| symbol == upper)
        .and_then(|index| u8::try_from(index).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(code: &RecoveryCode) -> String {
        code.to_display().expose_secret().to_owned()
    }

    fn canonical(code: &RecoveryCode) -> String {
        String::from_utf8(code.canonical.expose_secret().to_vec()).unwrap()
    }

    #[test]
    fn alphabet_has_no_look_alikes_or_duplicates() {
        for excluded in *b"01OI" {
            assert!(!ALPHABET.contains(&excluded));
        }
        let mut sorted = ALPHABET.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 32);
    }

    #[test]
    fn display_form_is_grouped() {
        let text = display(&RecoveryCode::generate().unwrap());
        assert_eq!(text.len(), DISPLAY_LEN);
        let groups: Vec<&str> = text.split('-').collect();
        assert_eq!(groups.len(), CODE_LEN / GROUP_LEN);
        for group in groups {
            assert_eq!(group.len(), GROUP_LEN);
            assert!(group.bytes().all(|b| ALPHABET.contains(&b)));
        }
    }

    #[test]
    fn generate_display_parse_round_trip() {
        for _ in 0..50 {
            let code = RecoveryCode::generate().unwrap();
            let parsed = RecoveryCode::parse(&display(&code)).unwrap();
            assert_eq!(canonical(&parsed), canonical(&code));
        }
    }

    #[test]
    fn known_code_checksum() {
        // Data symbols all "2" (value 0) give checksum 0, encoded "22".
        let code = RecoveryCode::parse(&"2".repeat(CODE_LEN)).unwrap();
        assert_eq!(canonical(&code), "2".repeat(CODE_LEN));
        // Data symbols all "3" (value 1): Σ i for i in 1..=30 = 465 = 14·32 + 17,
        // encoded as ALPHABET[14] ALPHABET[17] = "GK".
        let code = RecoveryCode::parse(&format!("{}GK", "3".repeat(DATA_LEN))).unwrap();
        assert_eq!(display(&code), "3333-3333-3333-3333-3333-3333-3333-33GK");
    }

    #[test]
    fn formatting_differences_are_ignored() {
        let code = RecoveryCode::generate().unwrap();
        let text = display(&code);
        let variants = [
            text.to_lowercase(),
            text.replace('-', ""),
            text.replace('-', " "),
            format!("  {}\n", text.replace('-', " - ")),
        ];
        for variant in variants {
            let parsed = RecoveryCode::parse(&variant).unwrap();
            assert_eq!(canonical(&parsed), canonical(&code), "rejected {variant:?}");
        }
    }

    #[test]
    fn every_single_substitution_is_rejected() {
        let code = canonical(&RecoveryCode::generate().unwrap());
        for index in 0..CODE_LEN {
            for &symbol in ALPHABET {
                let mut typo = code.clone().into_bytes();
                if typo[index] == symbol {
                    continue;
                }
                typo[index] = symbol;
                let typo = String::from_utf8(typo).unwrap();
                assert_eq!(
                    RecoveryCode::parse(&typo).map(|_| ()),
                    Err(CryptoError::RecoveryCodeChecksum),
                    "substitution at {index} accepted"
                );
            }
        }
    }

    #[test]
    fn every_adjacent_swap_in_random_part_is_rejected() {
        let code = canonical(&RecoveryCode::generate().unwrap());
        for index in 0..DATA_LEN - 1 {
            let mut swapped = code.clone().into_bytes();
            if swapped[index] == swapped[index + 1] {
                continue;
            }
            swapped.swap(index, index + 1);
            let swapped = String::from_utf8(swapped).unwrap();
            assert_eq!(
                RecoveryCode::parse(&swapped).map(|_| ()),
                Err(CryptoError::RecoveryCodeChecksum),
                "swap at {index} accepted"
            );
        }
    }

    #[test]
    fn wrong_length_is_rejected() {
        let code = canonical(&RecoveryCode::generate().unwrap());
        assert_eq!(
            RecoveryCode::parse(&code[..CODE_LEN - 1]).map(|_| ()),
            Err(CryptoError::RecoveryCodeLength)
        );
        assert_eq!(
            RecoveryCode::parse(&format!("{code}2")).map(|_| ()),
            Err(CryptoError::RecoveryCodeLength)
        );
        assert_eq!(
            RecoveryCode::parse("").map(|_| ()),
            Err(CryptoError::RecoveryCodeLength)
        );
    }

    #[test]
    fn invalid_characters_are_rejected() {
        let code = canonical(&RecoveryCode::generate().unwrap());
        for bad in ['0', '1', 'O', 'o', 'I', 'i', '_', 'é', '\u{0}'] {
            let mut input = code.clone();
            input.replace_range(0..1, &bad.to_string());
            assert_eq!(
                RecoveryCode::parse(&input).map(|_| ()),
                Err(CryptoError::RecoveryCodeCharacter),
                "accepted {bad:?}"
            );
        }
    }

    #[test]
    fn generated_codes_are_distinct() {
        let a = RecoveryCode::generate().unwrap();
        let b = RecoveryCode::generate().unwrap();
        assert_ne!(canonical(&a), canonical(&b));
    }

    #[test]
    fn debug_redacts_code() {
        let code = RecoveryCode::generate().unwrap();
        assert_eq!(format!("{code:?}"), "RecoveryCode([REDACTED])");
    }
}
