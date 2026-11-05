//! Key slots: the vault key wrapped under the user's secrets.
//!
//! The 256-bit vault key that encrypts all vault data is never stored in
//! the clear. Each [`KeySlot`] stores it encrypted ("wrapped") under a key
//! derived with Argon2id from one secret: the master password
//! ([`SlotType::Password`]) or the emergency recovery code
//! ([`SlotType::Recovery`]). Either slot alone recovers the same vault key.
//!
//! A slot serializes to exactly [`SLOT_LEN`] bytes (integers little-endian):
//!
//! ```text
//! slot_type (1) ‖ kdf_salt (16) ‖ memory_kib (4) ‖ iterations (4) ‖ parallelism (4)
//!   ‖ nonce (24) ‖ wrapped vault key (32) ‖ tag (16)
//! ```
//!
//! The first [`SLOT_METADATA_LEN`] bytes are the slot metadata. The wrapping
//! AEAD authenticates `header ‖ slot_metadata`, so changing the slot type,
//! salt or KDF parameters, or moving a slot to another vault, makes
//! unwrapping fail. `header` is supplied by the caller: the vault passes
//! [`Header::slot_ad`](crate::header::Header::slot_ad), the header with its
//! version field zeroed, so a format migration does not need to rewrite
//! slots it has no secret for.

use secrecy::{ExposeSecret, SecretSlice};

use crate::aead::{self, OVERHEAD};
use crate::error::{CryptoError, Result};
use crate::kdf::{KdfParams, derive_key};
use crate::keys::{KEY_LEN, SALT_LEN, Salt, SecretKey};
use crate::recovery::RecoveryCode;

/// Length in bytes of the slot metadata: type, salt and KDF parameters.
pub const SLOT_METADATA_LEN: usize = 1 + SALT_LEN + 3 * 4;

/// Length in bytes of the sealed vault key: nonce, wrapped key and tag.
pub const SEALED_KEY_LEN: usize = KEY_LEN + OVERHEAD;

/// Length in bytes of a serialized key slot.
pub const SLOT_LEN: usize = SLOT_METADATA_LEN + SEALED_KEY_LEN;

/// Which secret a key slot is wrapped under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotType {
    /// Wrapped under a key derived from the master password.
    Password,
    /// Wrapped under a key derived from the emergency recovery code.
    Recovery,
}

impl SlotType {
    /// The on-disk identifier of the slot type.
    pub const fn id(self) -> u8 {
        match self {
            Self::Password => 1,
            Self::Recovery => 2,
        }
    }

    /// Parses an on-disk slot type identifier.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::MalformedKeySlot`] for an unknown identifier.
    pub const fn from_id(id: u8) -> Result<Self> {
        match id {
            1 => Ok(Self::Password),
            2 => Ok(Self::Recovery),
            _ => Err(CryptoError::MalformedKeySlot {
                reason: "unknown slot type",
            }),
        }
    }
}

/// One key slot: the vault key wrapped under a key derived from one secret.
///
/// A slot contains no secret material (only a salt, public parameters and
/// ciphertext), so it can be freely copied and written to disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySlot {
    slot_type: SlotType,
    salt: Salt,
    params: KdfParams,
    sealed_key: [u8; SEALED_KEY_LEN],
}

impl KeySlot {
    /// Wraps `vault_key` under the master `password` (its UTF-8 bytes, used
    /// as given).
    ///
    /// A fresh salt and nonce are drawn for every call, so wrapping the same
    /// key twice never produces the same slot.
    ///
    /// # Errors
    ///
    /// Any error of [`derive_key`] or [`aead::encrypt`], and
    /// [`CryptoError::Random`] if the OS random number generator fails.
    pub fn wrap_password(
        password: &SecretSlice<u8>,
        vault_key: &SecretKey,
        params: KdfParams,
        header: &[u8],
    ) -> Result<Self> {
        Self::wrap(SlotType::Password, password, vault_key, params, header)
    }

    /// Wraps `vault_key` under the recovery code `code`.
    ///
    /// # Errors
    ///
    /// Same as [`KeySlot::wrap_password`].
    pub fn wrap_recovery_code(
        code: &RecoveryCode,
        vault_key: &SecretKey,
        params: KdfParams,
        header: &[u8],
    ) -> Result<Self> {
        Self::wrap(
            SlotType::Recovery,
            code.kdf_input(),
            vault_key,
            params,
            header,
        )
    }

    /// Recovers the vault key from a [`SlotType::Password`] slot.
    ///
    /// # Errors
    ///
    /// - [`CryptoError::Decryption`] if the password is wrong, the slot was
    ///   modified or `header` differs from the one used to wrap. These cases
    ///   are intentionally indistinguishable.
    /// - [`CryptoError::MalformedKeySlot`] if this is not a password slot.
    /// - Any error of [`derive_key`].
    pub fn unwrap_password(&self, password: &SecretSlice<u8>, header: &[u8]) -> Result<SecretKey> {
        self.unwrap(SlotType::Password, password, header)
    }

    /// Recovers the vault key from a [`SlotType::Recovery`] slot.
    ///
    /// # Errors
    ///
    /// Same as [`KeySlot::unwrap_password`], for a recovery slot.
    pub fn unwrap_recovery_code(&self, code: &RecoveryCode, header: &[u8]) -> Result<SecretKey> {
        self.unwrap(SlotType::Recovery, code.kdf_input(), header)
    }

    /// The secret this slot is wrapped under.
    pub const fn slot_type(&self) -> SlotType {
        self.slot_type
    }

    /// The Argon2id parameters used to derive this slot's wrapping key.
    pub const fn kdf_params(&self) -> KdfParams {
        self.params
    }

    /// Serializes the slot to its fixed [`SLOT_LEN`]-byte encoding.
    pub fn to_bytes(&self) -> [u8; SLOT_LEN] {
        let mut bytes = [0u8; SLOT_LEN];
        let (metadata, sealed) = bytes.split_at_mut(SLOT_METADATA_LEN);
        metadata.copy_from_slice(&self.metadata());
        sealed.copy_from_slice(&self.sealed_key);
        bytes
    }

    /// Parses a slot from its [`SLOT_LEN`]-byte encoding.
    ///
    /// Parsing is strict: the length must be exact, the slot type known and
    /// the KDF parameters within NovaVault's bounds. Authenticity is only
    /// established later, when the slot is unwrapped.
    ///
    /// # Errors
    ///
    /// - [`CryptoError::MalformedKeySlot`] for a wrong length or an unknown
    ///   slot type.
    /// - [`CryptoError::InvalidKdfParams`] for out-of-bounds parameters.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        const MALFORMED: CryptoError = CryptoError::MalformedKeySlot {
            reason: "wrong length",
        };
        if bytes.len() != SLOT_LEN {
            return Err(MALFORMED);
        }
        let (&type_id, rest) = bytes.split_first().ok_or(MALFORMED)?;
        let (salt, rest) = rest.split_first_chunk::<SALT_LEN>().ok_or(MALFORMED)?;
        let (memory_kib, rest) = rest.split_first_chunk::<4>().ok_or(MALFORMED)?;
        let (iterations, rest) = rest.split_first_chunk::<4>().ok_or(MALFORMED)?;
        let (parallelism, rest) = rest.split_first_chunk::<4>().ok_or(MALFORMED)?;
        let sealed_key: [u8; SEALED_KEY_LEN] = rest.try_into().map_err(|_| MALFORMED)?;

        Ok(Self {
            slot_type: SlotType::from_id(type_id)?,
            salt: Salt::from_bytes(*salt),
            params: KdfParams::new(
                u32::from_le_bytes(*memory_kib),
                u32::from_le_bytes(*iterations),
                u32::from_le_bytes(*parallelism),
            )?,
            sealed_key,
        })
    }

    /// Derives a wrapping key from `secret` with a new salt and seals
    /// `vault_key` under it, authenticating `header ‖ slot_metadata`.
    fn wrap(
        slot_type: SlotType,
        secret: &SecretSlice<u8>,
        vault_key: &SecretKey,
        params: KdfParams,
        header: &[u8],
    ) -> Result<Self> {
        // A new salt on every wrap means a rewritten slot shares nothing with
        // the previous one, so an old secret cannot open a new slot.
        let salt = Salt::generate()?;
        let wrapping_key = derive_key(secret, &salt, &params)?;
        let mut slot = Self {
            slot_type,
            salt,
            params,
            sealed_key: [0u8; SEALED_KEY_LEN],
        };
        let sealed = aead::encrypt(
            &wrapping_key,
            vault_key.expose(),
            &associated_data(header, &slot.metadata()),
        )?;
        slot.sealed_key = sealed.try_into().map_err(|_| CryptoError::Encryption)?;
        Ok(slot)
    }

    /// Re-derives the wrapping key from `secret` and opens the sealed vault
    /// key, checking first that this is a slot of type `expected`.
    fn unwrap(
        &self,
        expected: SlotType,
        secret: &SecretSlice<u8>,
        header: &[u8],
    ) -> Result<SecretKey> {
        // Checked before the slow derivation: trying a password against the
        // recovery slot (or the reverse) is a caller bug, not a wrong secret.
        if self.slot_type != expected {
            return Err(CryptoError::MalformedKeySlot {
                reason: "unexpected slot type",
            });
        }
        let wrapping_key = derive_key(secret, &self.salt, &self.params)?;
        let vault_key = aead::decrypt(
            &wrapping_key,
            &self.sealed_key,
            &associated_data(header, &self.metadata()),
        )?;
        SecretKey::try_from_slice(vault_key.expose_secret()).ok_or(CryptoError::MalformedKeySlot {
            reason: "wrapped key has the wrong length",
        })
    }

    /// The first [`SLOT_METADATA_LEN`] bytes of the slot encoding (type,
    /// salt, KDF parameters), which are also the slot's part of the
    /// wrapping associated data.
    fn metadata(&self) -> [u8; SLOT_METADATA_LEN] {
        let mut metadata = [0u8; SLOT_METADATA_LEN];
        let fields = [self.slot_type.id()]
            .into_iter()
            .chain(*self.salt.as_bytes())
            .chain(self.params.memory_kib().to_le_bytes())
            .chain(self.params.iterations().to_le_bytes())
            .chain(self.params.parallelism().to_le_bytes());
        for (byte, value) in metadata.iter_mut().zip(fields) {
            *byte = value;
        }
        metadata
    }
}

/// The vault's key slot table: one password slot and one recovery slot,
/// both wrapping the same vault key.
///
/// Every operation that rewrites a slot builds the new slot completely
/// before replacing the old one, so a failure leaves the table unchanged.
/// Persisting the updated table (atomically) is the caller's job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySlots {
    password: KeySlot,
    recovery: KeySlot,
}

impl KeySlots {
    /// Sets up the slots of a new vault.
    ///
    /// Generates a random 256-bit vault key and a new recovery code, and
    /// wraps the key under both the master `password` and the code. Returns
    /// the table, the vault key (to encrypt the vault's data) and the
    /// recovery code (to show to the user once).
    ///
    /// # Errors
    ///
    /// Any error of [`SecretKey::generate`], [`RecoveryCode::generate`],
    /// [`KeySlot::wrap_password`] or [`KeySlot::wrap_recovery_code`].
    pub fn create(
        password: &SecretSlice<u8>,
        params: KdfParams,
        header: &[u8],
    ) -> Result<(Self, SecretKey, RecoveryCode)> {
        let vault_key = SecretKey::generate()?;
        let code = RecoveryCode::generate()?;
        let slots = Self {
            password: KeySlot::wrap_password(password, &vault_key, params, header)?,
            recovery: KeySlot::wrap_recovery_code(&code, &vault_key, params, header)?,
        };
        Ok((slots, vault_key, code))
    }

    /// Rebuilds a table from slots read from a vault file.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::MalformedKeySlot`] if `password` is not a
    /// password slot or `recovery` is not a recovery slot.
    pub fn from_slots(password: KeySlot, recovery: KeySlot) -> Result<Self> {
        if password.slot_type() != SlotType::Password || recovery.slot_type() != SlotType::Recovery
        {
            return Err(CryptoError::MalformedKeySlot {
                reason: "unexpected slot type",
            });
        }
        Ok(Self { password, recovery })
    }

    /// The password slot.
    pub const fn password_slot(&self) -> &KeySlot {
        &self.password
    }

    /// The recovery slot.
    pub const fn recovery_slot(&self) -> &KeySlot {
        &self.recovery
    }

    /// Unlocks the vault key with the master password.
    ///
    /// # Errors
    ///
    /// [`CryptoError::Decryption`] for a wrong password or a corrupted slot;
    /// see [`KeySlot::unwrap_password`].
    pub fn unlock_with_password(
        &self,
        password: &SecretSlice<u8>,
        header: &[u8],
    ) -> Result<SecretKey> {
        self.password.unwrap_password(password, header)
    }

    /// Unlocks the vault key with the recovery code.
    ///
    /// # Errors
    ///
    /// [`CryptoError::Decryption`] for a wrong code or a corrupted slot;
    /// see [`KeySlot::unwrap_recovery_code`].
    pub fn unlock_with_recovery_code(
        &self,
        code: &RecoveryCode,
        header: &[u8],
    ) -> Result<SecretKey> {
        self.recovery.unwrap_recovery_code(code, header)
    }

    /// Changes the master password by re-wrapping the vault key under
    /// `new_password`.
    ///
    /// Only the password slot changes: the vault key, and therefore all
    /// encrypted data, stays the same. `vault_key` is the key obtained from
    /// an unlock, which may have used the recovery code (the
    /// forgotten-password flow).
    ///
    /// # Errors
    ///
    /// Any error of [`KeySlot::wrap_password`]; the table is then unchanged.
    pub fn change_password(
        &mut self,
        vault_key: &SecretKey,
        new_password: &SecretSlice<u8>,
        params: KdfParams,
        header: &[u8],
    ) -> Result<()> {
        self.password = KeySlot::wrap_password(new_password, vault_key, params, header)?;
        Ok(())
    }

    /// Replaces the recovery code with a newly generated one and returns it.
    ///
    /// The recovery slot is rewritten with a fresh salt, so the previous
    /// code no longer opens the vault once the updated table is saved.
    ///
    /// # Errors
    ///
    /// Any error of [`RecoveryCode::generate`] or
    /// [`KeySlot::wrap_recovery_code`]; the table is then unchanged.
    pub fn regenerate_recovery_code(
        &mut self,
        vault_key: &SecretKey,
        params: KdfParams,
        header: &[u8],
    ) -> Result<RecoveryCode> {
        let code = RecoveryCode::generate()?;
        self.recovery = KeySlot::wrap_recovery_code(&code, vault_key, params, header)?;
        Ok(code)
    }
}

/// Associated data for wrapping: `header ‖ slot_metadata`. The metadata has
/// a fixed length, so the concatenation is unambiguous.
fn associated_data(header: &[u8], metadata: &[u8; SLOT_METADATA_LEN]) -> Vec<u8> {
    let mut ad = Vec::with_capacity(header.len() + SLOT_METADATA_LEN);
    ad.extend_from_slice(header);
    ad.extend_from_slice(metadata);
    ad
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lowest accepted cost, so debug-build tests stay fast.
    fn fast_params() -> KdfParams {
        KdfParams::new(KdfParams::MIN_MEMORY_KIB, KdfParams::MIN_ITERATIONS, 1).unwrap()
    }

    fn secret(text: &str) -> SecretSlice<u8> {
        SecretSlice::from(text.as_bytes().to_vec())
    }

    const HEADER: &[u8] = b"test vault header";

    #[test]
    fn layout_sizes() {
        assert_eq!(SLOT_METADATA_LEN, 29);
        assert_eq!(SEALED_KEY_LEN, 72);
        assert_eq!(SLOT_LEN, 101);
    }

    #[test]
    fn password_and_recovery_slots_open_the_same_key() {
        let vault_key = SecretKey::generate().unwrap();
        let code = RecoveryCode::generate().unwrap();
        let password = secret("correct horse battery staple");

        let by_password =
            KeySlot::wrap_password(&password, &vault_key, fast_params(), HEADER).unwrap();
        let by_code =
            KeySlot::wrap_recovery_code(&code, &vault_key, fast_params(), HEADER).unwrap();

        let a = by_password.unwrap_password(&password, HEADER).unwrap();
        let b = by_code.unwrap_recovery_code(&code, HEADER).unwrap();
        assert_eq!(a.expose(), vault_key.expose());
        assert_eq!(b.expose(), vault_key.expose());
    }

    #[test]
    fn wrong_secrets_fail() {
        let vault_key = SecretKey::generate().unwrap();
        let code = RecoveryCode::generate().unwrap();
        let by_password =
            KeySlot::wrap_password(&secret("right"), &vault_key, fast_params(), HEADER).unwrap();
        let by_code =
            KeySlot::wrap_recovery_code(&code, &vault_key, fast_params(), HEADER).unwrap();

        assert_eq!(
            by_password
                .unwrap_password(&secret("wrong"), HEADER)
                .map(|_| ()),
            Err(CryptoError::Decryption)
        );
        let other_code = RecoveryCode::generate().unwrap();
        assert_eq!(
            by_code
                .unwrap_recovery_code(&other_code, HEADER)
                .map(|_| ()),
            Err(CryptoError::Decryption)
        );
    }

    #[test]
    fn wrong_header_fails() {
        let vault_key = SecretKey::generate().unwrap();
        let password = secret("pw");
        let slot = KeySlot::wrap_password(&password, &vault_key, fast_params(), HEADER).unwrap();
        assert_eq!(
            slot.unwrap_password(&password, b"another vault")
                .map(|_| ()),
            Err(CryptoError::Decryption)
        );
    }

    #[test]
    fn slot_type_is_enforced() {
        let vault_key = SecretKey::generate().unwrap();
        let code = RecoveryCode::generate().unwrap();
        let slot = KeySlot::wrap_recovery_code(&code, &vault_key, fast_params(), HEADER).unwrap();
        assert!(matches!(
            slot.unwrap_password(&secret(code.to_display().expose_secret()), HEADER),
            Err(CryptoError::MalformedKeySlot { .. })
        ));
    }

    #[test]
    fn wrapping_is_randomized() {
        let vault_key = SecretKey::generate().unwrap();
        let password = secret("pw");
        let a = KeySlot::wrap_password(&password, &vault_key, fast_params(), HEADER).unwrap();
        let b = KeySlot::wrap_password(&password, &vault_key, fast_params(), HEADER).unwrap();
        assert_ne!(a.to_bytes(), b.to_bytes());
    }

    #[test]
    fn encoding_round_trips() {
        let vault_key = SecretKey::generate().unwrap();
        let password = secret("pw");
        let slot = KeySlot::wrap_password(&password, &vault_key, fast_params(), HEADER).unwrap();
        let bytes = slot.to_bytes();

        assert_eq!(bytes[0], SlotType::Password.id());
        assert_eq!(&bytes[1..17], slot.salt.as_bytes());
        assert_eq!(&bytes[17..21], &KdfParams::MIN_MEMORY_KIB.to_le_bytes());

        let parsed = KeySlot::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, slot);
        let key = parsed.unwrap_password(&password, HEADER).unwrap();
        assert_eq!(key.expose(), vault_key.expose());
    }

    #[test]
    fn every_tampered_byte_is_detected() {
        let vault_key = SecretKey::generate().unwrap();
        let password = secret("pw");
        let bytes = KeySlot::wrap_password(&password, &vault_key, fast_params(), HEADER)
            .unwrap()
            .to_bytes();
        for index in 0..SLOT_LEN {
            let mut tampered = bytes;
            tampered[index] ^= 0x01;
            // Depending on the byte, the change is caught by strict parsing,
            // the slot type check or AEAD authentication; it is never accepted.
            let result = KeySlot::from_bytes(&tampered)
                .and_then(|slot| slot.unwrap_password(&password, HEADER));
            assert!(result.is_err(), "tampering at byte {index} went undetected");
        }
    }

    #[test]
    fn strict_parsing() {
        let vault_key = SecretKey::generate().unwrap();
        let bytes = KeySlot::wrap_password(&secret("pw"), &vault_key, fast_params(), HEADER)
            .unwrap()
            .to_bytes();

        for len in [0, SLOT_LEN - 1, SLOT_LEN + 1] {
            let mut input = bytes.to_vec();
            input.resize(len, 0);
            assert!(matches!(
                KeySlot::from_bytes(&input),
                Err(CryptoError::MalformedKeySlot { .. })
            ));
        }

        for unknown in [0u8, 3, 0xFF] {
            let mut input = bytes;
            input[0] = unknown;
            assert!(matches!(
                KeySlot::from_bytes(&input),
                Err(CryptoError::MalformedKeySlot { .. })
            ));
        }

        let mut weak = bytes;
        weak[17..21].copy_from_slice(&1024u32.to_le_bytes());
        assert!(matches!(
            KeySlot::from_bytes(&weak),
            Err(CryptoError::InvalidKdfParams { .. })
        ));
    }

    fn new_vault(password: &str) -> (KeySlots, SecretKey, RecoveryCode) {
        KeySlots::create(&secret(password), fast_params(), HEADER).unwrap()
    }

    #[test]
    fn create_wraps_one_key_in_both_slots() {
        let (slots, vault_key, code) = new_vault("pw");
        assert_eq!(slots.password_slot().slot_type(), SlotType::Password);
        assert_eq!(slots.recovery_slot().slot_type(), SlotType::Recovery);
        let a = slots.unlock_with_password(&secret("pw"), HEADER).unwrap();
        let b = slots.unlock_with_recovery_code(&code, HEADER).unwrap();
        assert_eq!(a.expose(), vault_key.expose());
        assert_eq!(b.expose(), vault_key.expose());
    }

    #[test]
    fn unlock_with_wrong_secrets_fails() {
        let (slots, _, _) = new_vault("pw");
        assert_eq!(
            slots
                .unlock_with_password(&secret("PW"), HEADER)
                .map(|_| ()),
            Err(CryptoError::Decryption)
        );
        let wrong_code = RecoveryCode::generate().unwrap();
        assert_eq!(
            slots
                .unlock_with_recovery_code(&wrong_code, HEADER)
                .map(|_| ()),
            Err(CryptoError::Decryption)
        );
    }

    #[test]
    fn regenerated_recovery_code_replaces_the_old_one() {
        let (mut slots, _, old_code) = new_vault("pw");
        let vault_key = slots.unlock_with_password(&secret("pw"), HEADER).unwrap();
        let password_slot = slots.password_slot().clone();

        let new_code = slots
            .regenerate_recovery_code(&vault_key, fast_params(), HEADER)
            .unwrap();

        assert_eq!(
            slots
                .unlock_with_recovery_code(&old_code, HEADER)
                .map(|_| ()),
            Err(CryptoError::Decryption)
        );
        let unlocked = slots.unlock_with_recovery_code(&new_code, HEADER).unwrap();
        assert_eq!(unlocked.expose(), vault_key.expose());
        assert_eq!(slots.password_slot(), &password_slot);
    }

    #[test]
    fn password_change_keeps_data_accessible() {
        let (mut slots, vault_key, code) = new_vault("old password");
        let record = aead::encrypt(&vault_key, b"my secret note", b"record-1").unwrap();
        let recovery_slot = slots.recovery_slot().clone();

        slots
            .change_password(&vault_key, &secret("new password"), fast_params(), HEADER)
            .unwrap();

        assert_eq!(
            slots
                .unlock_with_password(&secret("old password"), HEADER)
                .map(|_| ()),
            Err(CryptoError::Decryption)
        );
        let unlocked = slots
            .unlock_with_password(&secret("new password"), HEADER)
            .unwrap();
        assert_eq!(unlocked.expose(), vault_key.expose());
        assert_eq!(
            aead::decrypt(&unlocked, &record, b"record-1")
                .unwrap()
                .expose_secret(),
            b"my secret note"
        );
        // Re-wrap only: the recovery slot is untouched and still works.
        assert_eq!(slots.recovery_slot(), &recovery_slot);
        assert!(slots.unlock_with_recovery_code(&code, HEADER).is_ok());
    }

    #[test]
    fn forgotten_password_flow() {
        let (mut slots, _, code) = new_vault("forgotten");
        let vault_key = slots.unlock_with_recovery_code(&code, HEADER).unwrap();
        slots
            .change_password(&vault_key, &secret("fresh"), fast_params(), HEADER)
            .unwrap();
        let unlocked = slots
            .unlock_with_password(&secret("fresh"), HEADER)
            .unwrap();
        assert_eq!(unlocked.expose(), vault_key.expose());
    }

    #[test]
    fn from_slots_checks_slot_types() {
        let (slots, _, _) = new_vault("pw");
        let password = slots.password_slot().clone();
        let recovery = slots.recovery_slot().clone();
        assert_eq!(
            KeySlots::from_slots(password.clone(), recovery.clone()).unwrap(),
            slots
        );
        assert!(matches!(
            KeySlots::from_slots(recovery, password),
            Err(CryptoError::MalformedKeySlot { .. })
        ));
    }
}
