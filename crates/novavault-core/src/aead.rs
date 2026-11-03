//! Authenticated encryption with XChaCha20-Poly1305.
//!
//! Every call to [`encrypt`] draws a fresh 192-bit nonce from the OS CSPRNG
//! and returns a self-contained sealed message:
//!
//! ```text
//! nonce (24 bytes) ‖ ciphertext (len(plaintext) bytes) ‖ tag (16 bytes)
//! ```
//!
//! The 192-bit nonce is what makes random nonces safe here: the probability
//! of a repeat stays negligible even after far more messages than a vault
//! will ever encrypt under one key, so callers never manage nonces or
//! counters. Associated data (`aad`) is authenticated but not encrypted;
//! callers use it to bind a ciphertext to its context (for example the
//! vault header and record identifier) so it cannot be moved elsewhere.

use chacha20poly1305::aead::inout::InOutBuf;
use chacha20poly1305::{AeadInOut, Key, KeyInit, Tag, XChaCha20Poly1305, XNonce};
use secrecy::SecretSlice;
use zeroize::Zeroize;

use crate::error::{CryptoError, Result};
use crate::keys::SecretKey;

/// Length in bytes of the random nonce prefixed to every sealed message.
pub const NONCE_LEN: usize = 24;

/// Length in bytes of the Poly1305 authentication tag.
pub const TAG_LEN: usize = 16;

/// Bytes a sealed message adds on top of the plaintext length.
pub const OVERHEAD: usize = NONCE_LEN + TAG_LEN;

/// Encrypts and authenticates `plaintext`, binding it to `aad`.
///
/// Returns `nonce ‖ ciphertext ‖ tag`, of length `plaintext.len() + OVERHEAD`.
///
/// # Errors
///
/// - [`CryptoError::Random`] if the OS random number generator fails.
/// - [`CryptoError::Encryption`] if the message is too long for the cipher.
pub fn encrypt(key: &SecretKey, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce).map_err(|_| CryptoError::Random)?;
    encrypt_with_nonce(key, &nonce, plaintext, aad)
}

/// Verifies and decrypts a message produced by [`encrypt`].
///
/// The plaintext is returned in a [`SecretSlice`] so it is zeroized when
/// dropped.
///
/// # Errors
///
/// - [`CryptoError::MalformedCiphertext`] if `sealed` is shorter than
///   [`OVERHEAD`].
/// - [`CryptoError::Decryption`] if authentication fails: wrong key, any
///   modified byte (nonce, ciphertext or tag) or different `aad`.
pub fn decrypt(key: &SecretKey, sealed: &[u8], aad: &[u8]) -> Result<SecretSlice<u8>> {
    if sealed.len() < OVERHEAD {
        return Err(CryptoError::MalformedCiphertext);
    }
    let (nonce, rest) = sealed.split_at(NONCE_LEN);
    let (ciphertext, tag) = rest.split_at(rest.len() - TAG_LEN);
    let nonce = XNonce::try_from(nonce).map_err(|_| CryptoError::MalformedCiphertext)?;
    let tag = Tag::try_from(tag).map_err(|_| CryptoError::MalformedCiphertext)?;

    // `to_vec` allocates exactly `len` bytes, so converting to a boxed slice
    // below does not reallocate and leave an unwiped plaintext copy behind.
    let mut buffer = ciphertext.to_vec();
    let result = cipher(key).decrypt_inout_detached(
        &nonce,
        aad,
        InOutBuf::from(buffer.as_mut_slice()),
        &tag,
    );
    if result.is_err() {
        buffer.zeroize();
        return Err(CryptoError::Decryption);
    }
    Ok(SecretSlice::from(buffer))
}

/// Encrypts with a caller-supplied nonce. Crate-internal: reusing a nonce
/// under the same key breaks confidentiality and authenticity, so only
/// [`encrypt`] (fresh random nonce) is public. Exposed to tests for known
/// answer vectors.
fn encrypt_with_nonce(
    key: &SecretKey,
    nonce: &[u8; NONCE_LEN],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>> {
    let mut sealed = Vec::with_capacity(plaintext.len() + OVERHEAD);
    sealed.extend_from_slice(nonce);
    sealed.extend_from_slice(plaintext);
    let body = &mut sealed[NONCE_LEN..];
    match cipher(key).encrypt_inout_detached(&XNonce::from(*nonce), aad, InOutBuf::from(body)) {
        Ok(tag) => {
            sealed.extend_from_slice(&tag);
            Ok(sealed)
        }
        Err(_) => {
            // The buffer still holds plaintext; wipe it before discarding.
            sealed.zeroize();
            Err(CryptoError::Encryption)
        }
    }
}

/// Builds the cipher from a borrowed key. The `zeroize` feature of
/// `chacha20poly1305` wipes the cipher's internal key copy on drop.
fn cipher(key: &SecretKey) -> XChaCha20Poly1305 {
    let key: &Key = key.expose().into();
    XChaCha20Poly1305::new(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    fn test_key() -> SecretKey {
        SecretKey::from_bytes([0x42; 32])
    }

    #[test]
    fn xchacha_draft_test_vector() {
        // draft-irtf-cfrg-xchacha-03, Appendix A.3.1.
        let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you \
only one tip for the future, sunscreen would be it.";
        let aad = unhex("50515253c0c1c2c3c4c5c6c7");
        let key: [u8; 32] = core::array::from_fn(|i| 0x80 + i as u8);
        let nonce: [u8; NONCE_LEN] = core::array::from_fn(|i| 0x40 + i as u8);
        let expected_ciphertext = unhex(
            "bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb\
             731c7f1b0b4aa6440bf3a82f4eda7e39ae64c6708c54c216cb96b72e1213b452\
             2f8c9ba40db5d945b11b69b982c1bb9e3f3fac2bc369488f76b2383565d3fff9\
             21f9664c97637da9768812f615c68b13b52e",
        );
        let expected_tag = unhex("c0875924c1c7987947deafd8780acf49");

        let key = SecretKey::from_bytes(key);
        let sealed = encrypt_with_nonce(&key, &nonce, plaintext, &aad).unwrap();

        assert_eq!(&sealed[..NONCE_LEN], &nonce);
        assert_eq!(
            &sealed[NONCE_LEN..sealed.len() - TAG_LEN],
            expected_ciphertext
        );
        assert_eq!(&sealed[sealed.len() - TAG_LEN..], expected_tag);
        assert_eq!(
            decrypt(&key, &sealed, &aad).unwrap().expose_secret(),
            plaintext
        );
    }

    #[test]
    fn round_trip() {
        let key = test_key();
        for plaintext in [&b""[..], b"x", &[0xA5; 1000]] {
            let sealed = encrypt(&key, plaintext, b"ctx").unwrap();
            assert_eq!(sealed.len(), plaintext.len() + OVERHEAD);
            assert_eq!(
                decrypt(&key, &sealed, b"ctx").unwrap().expose_secret(),
                plaintext
            );
        }
    }

    #[test]
    fn nonces_are_fresh_per_encryption() {
        let key = test_key();
        let a = encrypt(&key, b"same message", b"").unwrap();
        let b = encrypt(&key, b"same message", b"").unwrap();
        assert_ne!(a[..NONCE_LEN], b[..NONCE_LEN]);
        assert_ne!(a, b);
    }

    #[test]
    fn any_flipped_bit_is_detected() {
        let key = test_key();
        let sealed = encrypt(&key, b"attack at dawn", b"header").unwrap();
        // Covers every byte of the nonce, ciphertext and tag regions.
        for index in 0..sealed.len() {
            let mut tampered = sealed.clone();
            tampered[index] ^= 0x01;
            assert_eq!(
                decrypt(&key, &tampered, b"header").map(|_| ()),
                Err(CryptoError::Decryption),
                "tampering at byte {index} went undetected"
            );
        }
    }

    #[test]
    fn wrong_aad_is_rejected() {
        let key = test_key();
        let sealed = encrypt(&key, b"secret", b"record-1").unwrap();
        assert_eq!(
            decrypt(&key, &sealed, b"record-2").map(|_| ()),
            Err(CryptoError::Decryption)
        );
        assert_eq!(
            decrypt(&key, &sealed, b"").map(|_| ()),
            Err(CryptoError::Decryption)
        );
    }

    #[test]
    fn wrong_key_is_rejected() {
        let sealed = encrypt(&test_key(), b"secret", b"").unwrap();
        let other = SecretKey::from_bytes([0x43; 32]);
        assert_eq!(
            decrypt(&other, &sealed, b"").map(|_| ()),
            Err(CryptoError::Decryption)
        );
    }

    #[test]
    fn truncated_input_is_rejected() {
        let key = test_key();
        let sealed = encrypt(&key, b"secret", b"").unwrap();
        // Shorter than nonce + tag: structurally invalid.
        for len in 0..OVERHEAD {
            assert_eq!(
                decrypt(&key, &sealed[..len], b"").map(|_| ()),
                Err(CryptoError::MalformedCiphertext)
            );
        }
        // Long enough to parse but missing bytes: fails authentication.
        for len in OVERHEAD..sealed.len() {
            assert_eq!(
                decrypt(&key, &sealed[..len], b"").map(|_| ()),
                Err(CryptoError::Decryption)
            );
        }
    }

    #[test]
    fn appended_bytes_are_rejected() {
        let key = test_key();
        let mut sealed = encrypt(&key, b"secret", b"").unwrap();
        sealed.push(0);
        assert_eq!(
            decrypt(&key, &sealed, b"").map(|_| ()),
            Err(CryptoError::Decryption)
        );
    }
}
