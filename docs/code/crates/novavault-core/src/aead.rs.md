# [`crates/novavault-core/src/aead.rs`](../../../../../crates/novavault-core/src/aead.rs)

## Purpose

This module is the crate's only authenticated-encryption primitive: XChaCha20-Poly1305 with a fresh random 192-bit nonce for every message. Everything encrypted in a vault goes through `encrypt` and `decrypt`: key slots, records, the integrity seal and attachment chunks. Each output is a self-contained sealed message, `nonce ‖ ciphertext ‖ tag`.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `NONCE_LEN` | constant | `pub const NONCE_LEN: usize = 24;` | Length of the random nonce that prefixes every sealed message. |
| `TAG_LEN` | constant | `pub const TAG_LEN: usize = 16;` | Length of the Poly1305 tag. |
| `OVERHEAD` | constant | `pub const OVERHEAD: usize = NONCE_LEN + TAG_LEN;` | Bytes a sealed message adds to the plaintext (40). |
| `encrypt` | function | `pub fn encrypt(key: &SecretKey, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>>` | Seals `plaintext` under `key`, bound to `aad`, with a fresh random nonce. |
| `decrypt` | function | `pub fn decrypt(key: &SecretKey, sealed: &[u8], aad: &[u8]) -> Result<SecretSlice<u8>>` | Verifies and opens a sealed message. |

All five are re-exported from the crate root. The private helpers `encrypt_with_nonce` and `cipher` are described below because they are where the nonce and key handling happens.

## Details

### `NONCE_LEN`, `TAG_LEN`, `OVERHEAD`

The fixed layout of a sealed message:

```text
nonce (24 bytes) ‖ ciphertext (len(plaintext) bytes) ‖ tag (16 bytes)
```

A sealed message is always `plaintext.len() + 40` bytes. [`format`](format.rs.md) uses `OVERHEAD` as the seal length and as the lower bound of `sealed_len`. [`slots`](slots.rs.md) and [`attachment`](attachment.rs.md) use it to size sealed keys and chunks.

### `encrypt`

1. Fills a 24-byte stack array from `getrandom::fill`.
2. Calls `encrypt_with_nonce` with that nonce.

- **Parameters**:
  - `key`: the encryption key, borrowed.
  - `plaintext`: the message, borrowed.
  - `aad`: associated data. It is authenticated but neither encrypted nor stored; `decrypt` must be given exactly the same bytes.
- **Returns**: `nonce ‖ ciphertext ‖ tag` as a plain `Vec<u8>`, of length `plaintext.len() + OVERHEAD`.
- **Errors**:
  - `CryptoError::Random`: the OS random number generator failed.
  - `CryptoError::Encryption`: the cipher rejected the message (too long).
- **Security**:
  - **Nonces**: a new random 192-bit nonce is drawn on every call, and callers cannot supply one. With 192 bits, a nonce repeating under one key is negligibly likely even after far more messages than a vault produces, so no counter or nonce state is kept anywhere in the crate.
  - **Key material**: the key is only borrowed. The returned bytes are ciphertext and need no wiping. `plaintext` stays the caller's to wipe.
- **Edge cases and invariants**: an empty plaintext is valid and yields a 40-byte message. The integrity seal of [`format`](format.rs.md) is exactly such a message.

### `decrypt`

1. Rejects `sealed` shorter than `OVERHEAD` before doing anything else.
2. Splits it into nonce (first 24 bytes), ciphertext and tag (last 16 bytes).
3. Copies the ciphertext into a `Vec` allocated with exactly its length.
4. Decrypts in place with `decrypt_inout_detached`, verifying the tag against `aad`.
5. On failure, zeroizes the buffer and returns `Decryption`. On success, converts the buffer into a `SecretSlice<u8>`.

- **Parameters**:
  - `key`: the key, borrowed.
  - `sealed`: the full `nonce ‖ ciphertext ‖ tag`.
  - `aad`: must equal the associated data used for encryption.
- **Returns**: the plaintext in a `SecretSlice<u8>`, which is zeroized on drop.
- **Errors**:
  - `CryptoError::MalformedCiphertext`: `sealed.len() < OVERHEAD`. The two `try_from` conversions return the same error, but they cannot fail once the length check has passed.
  - `CryptoError::Decryption`: authentication failed (wrong key, any changed byte in the nonce, ciphertext or tag, or different `aad`).
- **Security**:
  - Nothing is returned unless the whole message authenticates. On failure the working buffer is wiped first, so no unauthenticated plaintext survives.
  - `ciphertext.to_vec()` allocates exactly the needed length. Converting it to a boxed slice for `SecretSlice` therefore does not reallocate, and no unwiped copy of the plaintext is left in freed memory.
  - The length check runs before any cryptographic work, so `MalformedCiphertext` reveals nothing about the key.
- **Edge cases and invariants**: a message cut by a few bytes but still at least 40 bytes long gets past the length check and fails authentication (`Decryption`). The same happens to a message with bytes appended.

### `encrypt_with_nonce` (private)

Builds the output in one `Vec` with capacity `plaintext.len() + OVERHEAD`: first the nonce, then the plaintext copied after it. The plaintext is encrypted in place (`encrypt_inout_detached`) and the tag appended. If encryption fails, the buffer, which still holds plaintext, is zeroized before the error is returned.

- **Security**: private on purpose. Reusing a nonce under the same key breaks confidentiality and authenticity. Only `encrypt`, which always draws a fresh nonce, is public. The function exists separately so the unit tests can check the draft's known-answer vector.

### `cipher` (private)

Builds an `XChaCha20Poly1305` instance from `key.expose()`. The `zeroize` feature of `chacha20poly1305` wipes the cipher's internal copy of the key when the instance is dropped. A new instance is built for every call and dropped at the end of it.

## Dependencies

- **Internal**:
  - [`error`](error.rs.md): `CryptoError` and `Result`.
  - [`keys`](keys.rs.md): `SecretKey` (`expose`).
- **External**:
  - `chacha20poly1305` (feature `zeroize`): the XChaCha20-Poly1305 implementation (`XChaCha20Poly1305`, `AeadInOut`, `InOutBuf`, `XNonce`, `Tag`).
  - `secrecy`: `SecretSlice` for the decrypted plaintext.
  - `zeroize`: wiping working buffers on failure.
  - `getrandom`: nonces.

## Used by

- [`slots`](slots.rs.md): wraps and unwraps the vault key.
- [`format`](format.rs.md): computes and verifies the integrity seal; `OVERHEAD` bounds record lengths.
- [`attachment`](attachment.rs.md): encrypts and decrypts each chunk.
- [`vault`](vault.rs.md): encrypts and decrypts records, including during migration.
- [`lib.rs`](lib.rs.md): re-exports `NONCE_LEN`, `OVERHEAD`, `TAG_LEN`, `encrypt`, `decrypt`.

## Tests

- [`xchacha_draft_test_vector`](../../../../../crates/novavault-core/src/aead.rs#L157): draft-irtf-cfrg-xchacha-03 Appendix A.3.1. Checks the exact ciphertext and tag through `encrypt_with_nonce`, then a round trip through `decrypt`.
- [`round_trip`](../../../../../crates/novavault-core/src/aead.rs#L188): empty, 1-byte and 1000-byte plaintexts round-trip; output length is `len + OVERHEAD`.
- [`nonces_are_fresh_per_encryption`](../../../../../crates/novavault-core/src/aead.rs#L201): two encryptions of the same message have different nonces and outputs.
- [`any_flipped_bit_is_detected`](../../../../../crates/novavault-core/src/aead.rs#L210): flipping any bit in any byte (nonce, ciphertext or tag) yields `Decryption`.
- [`wrong_aad_is_rejected`](../../../../../crates/novavault-core/src/aead.rs#L226): different or empty associated data yields `Decryption`.
- [`wrong_key_is_rejected`](../../../../../crates/novavault-core/src/aead.rs#L240): another key yields `Decryption`.
- [`truncated_input_is_rejected`](../../../../../crates/novavault-core/src/aead.rs#L250): lengths below 40 yield `MalformedCiphertext`; longer truncations yield `Decryption`.
- [`appended_bytes_are_rejected`](../../../../../crates/novavault-core/src/aead.rs#L270): one extra trailing byte yields `Decryption`.

## Notes

- The vault key encrypts records, the seal and attachment chunks directly, with no per-purpose subkeys. Domain separation comes from a context byte in each caller's associated data (`0x01`, `0x02`, `0x03`; see [`format`](format.rs.md) and [`attachment`](attachment.rs.md)), not from this module.
- The nonce is drawn into a plain stack array. That is fine: a nonce is public and is written into the output anyway.
