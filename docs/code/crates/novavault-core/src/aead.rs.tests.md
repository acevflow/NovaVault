# [`crates/novavault-core/src/aead.rs`](../../../../../crates/novavault-core/src/aead.rs#L140) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `aead.rs` checks the XChaCha20-Poly1305 wrapper: a published known-answer vector, round trips, nonce freshness, and rejection of every kind of tampering (bit flips, wrong associated data, wrong key, truncation, appended bytes). It is the only place the private `encrypt_with_nonce` is called with a fixed nonce.

## Public API

| Name                              | Kind         | Signature                                   | Description                                                                 |
| --------------------------------- | ------------ | ------------------------------------------- | --------------------------------------------------------------------------- |
| `unhex`                           | helper       | `fn unhex(s: &str) -> Vec<u8>`              | Decodes a hex string (two characters per byte) for the vectors.             |
| `test_key`                        | helper       | `fn test_key() -> SecretKey`                | Returns the fixed test key `[0x42; 32]`.                                    |
| `xchacha_draft_test_vector`       | test         | `fn xchacha_draft_test_vector()`            | Known-answer vector from draft-irtf-cfrg-xchacha-03 Appendix A.3.1.         |
| `round_trip`                      | test         | `fn round_trip()`                           | Empty, 1-byte and 1000-byte plaintexts round-trip with the expected length. |
| `nonces_are_fresh_per_encryption` | test         | `fn nonces_are_fresh_per_encryption()`      | Two encryptions of the same message differ in nonce and output.             |
| `any_flipped_bit_is_detected`     | test         | `fn any_flipped_bit_is_detected()`          | Flipping the low bit of any byte yields `Decryption`.                       |
| `wrong_aad_is_rejected`           | test         | `fn wrong_aad_is_rejected()`                | Different or empty associated data yields `Decryption`.                     |
| `wrong_key_is_rejected`           | test         | `fn wrong_key_is_rejected()`                | Another key yields `Decryption`.                                            |
| `truncated_input_is_rejected`     | test         | `fn truncated_input_is_rejected()`          | Short inputs yield `MalformedCiphertext`, longer truncations `Decryption`.  |
| `appended_bytes_are_rejected`     | test         | `fn appended_bytes_are_rejected()`          | One extra trailing byte yields `Decryption`.                                |

## Details

### `unhex` and `test_key`

`unhex` parses two hex digits at a time with `u8::from_str_radix` and `unwrap`s, so a malformed vector panics the test. `test_key` builds `SecretKey::from_bytes([0x42; 32])`; `wrong_key_is_rejected` uses `[0x43; 32]` as the other key. Both are obviously fake test keys.

### `xchacha_draft_test_vector`

Test vector from draft-irtf-cfrg-xchacha-03, Appendix A.3.1:

| Input            | Value                                                                                                                           |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Plaintext        | `Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.` (ASCII)    |
| AAD              | `50515253c0c1c2c3c4c5c6c7`                                                                                                      |
| Key              | bytes `0x80, 0x81, …, 0x9f` (`0x80 + i` for `i` in `0..32`)                                                                     |
| Nonce            | bytes `0x40, 0x41, …, 0x57` (`0x40 + i` for `i` in `0..24`)                                                                     |
| Ciphertext       | `bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb731c7f1b0b4aa6440bf3a82f4eda7e39ae64c6708c54c216cb96b72e1213b4522f8c9ba40db5d945b11b69b982c1bb9e3f3fac2bc369488f76b2383565d3fff921f9664c97637da9768812f615c68b13b52e` |
| Tag              | `c0875924c1c7987947deafd8780acf49`                                                                                              |

Steps:

1. Seals the plaintext with `encrypt_with_nonce(&key, &nonce, plaintext, &aad)`.
2. Asserts the first `NONCE_LEN` bytes equal the nonce, the middle equals the expected ciphertext and the last `TAG_LEN` bytes equal the expected tag.
3. Asserts `decrypt` returns the original plaintext.

This pins both the `nonce ‖ ciphertext ‖ tag` layout and the cipher wiring to an external reference.

### `round_trip`

For plaintexts `b""`, `b"x"` and `[0xA5; 1000]` with associated data `b"ctx"`: `encrypt` output length is `plaintext.len() + OVERHEAD`, and `decrypt` returns the plaintext. Covers the empty-message edge case.

### `nonces_are_fresh_per_encryption`

Encrypts `b"same message"` twice under the same key with empty associated data and asserts the nonce prefixes differ and the full outputs differ. A probabilistic check that `encrypt` draws a new random nonce each call.

### `any_flipped_bit_is_detected`

Seals `b"attack at dawn"` with associated data `b"header"`, then for every byte index XORs that byte with `0x01` and asserts `decrypt` returns `Err(CryptoError::Decryption)`. Covers the nonce, ciphertext and tag regions; the failure message names the index.

### `wrong_aad_is_rejected`

Seals with `b"record-1"`; decrypting with `b"record-2"` and with `b""` both return `Decryption`. Associated data is not stored, so a mismatch must fail authentication.

### `wrong_key_is_rejected`

Seals with `test_key()` and decrypts with `[0x43; 32]`: `Decryption`.

### `truncated_input_is_rejected`

Seals `b"secret"` (46 bytes sealed). Every prefix shorter than `OVERHEAD` (0 to 39 bytes) returns `MalformedCiphertext`; every prefix from `OVERHEAD` up to one byte short returns `Decryption`. This fixes the boundary between structural and authentication errors.

### `appended_bytes_are_rejected`

Appends one `0` byte to a sealed message; `decrypt` returns `Decryption` because the last 16 bytes are no longer the tag.

## Dependencies

- **Internal**: [`aead`](aead.rs.md) (`super::*`): `encrypt`, `decrypt`, `encrypt_with_nonce`, the length constants and `CryptoError`; [`keys`](keys.rs.md): `SecretKey::from_bytes`.
- **External**: `secrecy::ExposeSecret`: reads the decrypted `SecretSlice` to compare it.

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`crates/novavault-core/src/aead.rs`](../../../../../crates/novavault-core/src/aead.rs), documented in [`aead.rs.md`](aead.rs.md).

How to run:

```sh
cargo test -p novavault-core aead::tests
cargo test -p novavault-core aead::tests::xchacha_draft_test_vector
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- `any_flipped_bit_is_detected` flips only the lowest bit of each byte, not every bit. The **Tests** section of [`aead.rs.md`](aead.rs.md) says "flipping any bit in any byte", which overstates it slightly.
- `nonces_are_fresh_per_encryption` cannot fail deterministically on a broken RNG that happens to differ twice; it only catches a fixed or reused nonce.
