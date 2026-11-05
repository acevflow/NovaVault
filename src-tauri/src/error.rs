//! Errors returned by Tauri commands to the UI.
//!
//! [`CommandError`] is what crosses the trust boundary when a command fails.
//! It is serialized as `{ "code": "<snake_case>", "message": "<text>" }`:
//! the UI branches on `code` and may show `message`.
//!
//! Every message is a fixed string. Details carried by core errors are
//! dropped on purpose: an I/O error's text can contain file paths, and
//! nothing a command was given (passwords, recovery codes, item fields) is
//! ever stored in an error, so it cannot be echoed back or logged.

use novavault_core::{CryptoError, VaultError};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use thiserror::Error;

/// A failed command, as reported to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CommandError {
    /// The command needs an unlocked vault and the vault is locked.
    #[error("the vault is locked")]
    Locked,

    /// There is no vault file to unlock yet.
    #[error("no vault exists yet")]
    VaultNotFound,

    /// A vault already exists, so a new one cannot be created over it.
    #[error("a vault already exists")]
    VaultExists,

    /// The master password or recovery code is wrong, or the key slot is
    /// damaged. These cases are intentionally indistinguishable.
    #[error("wrong password or recovery code")]
    InvalidCredentials,

    /// The recovery code is malformed (wrong length, invalid character or
    /// failed checksum), so it was rejected before any key derivation.
    #[error("the recovery code is not valid; check it for typos")]
    InvalidRecoveryCode,

    /// A command argument is malformed, for example an item identifier.
    #[error("invalid input")]
    InvalidInput,

    /// No item has the given identifier.
    #[error("item not found")]
    ItemNotFound,

    /// The item has no attachment with the given identifier.
    #[error("attachment not found")]
    AttachmentNotFound,

    /// A file the user picked could not be read, or the export destination
    /// could not be written.
    #[error("the file could not be read or written")]
    FileAccess,

    /// An attachment transfer was stopped because the vault was locked.
    #[error("the operation was cancelled")]
    Cancelled,

    /// The item cannot be stored, for example because it is too large.
    #[error("the item cannot be stored")]
    InvalidItem,

    /// The file is not a vault, or uses a version or algorithm this build
    /// does not support.
    #[error("the vault file is not supported by this version of NovaVault")]
    UnsupportedVault,

    /// The vault file failed a structural or integrity check.
    #[error("the vault file is corrupted")]
    Corrupted,

    /// Reading or writing the vault file failed.
    #[error("the vault file could not be read or written")]
    Storage,

    /// The system clipboard could not be opened or written.
    #[error("the clipboard is not available")]
    ClipboardUnavailable,

    /// An unexpected internal failure (for example, the system random
    /// number generator failed).
    #[error("internal error")]
    Internal,
}

impl CommandError {
    /// Stable machine-readable code sent to the UI.
    ///
    /// These strings are a contract with the UI (`ui/src/api`), which
    /// branches on them: renaming one is a breaking change on both sides.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Locked => "locked",
            Self::VaultNotFound => "vault_not_found",
            Self::VaultExists => "vault_exists",
            Self::InvalidCredentials => "invalid_credentials",
            Self::InvalidRecoveryCode => "invalid_recovery_code",
            Self::InvalidInput => "invalid_input",
            Self::ItemNotFound => "item_not_found",
            Self::AttachmentNotFound => "attachment_not_found",
            Self::FileAccess => "file_access",
            Self::Cancelled => "cancelled",
            Self::InvalidItem => "invalid_item",
            Self::UnsupportedVault => "unsupported_vault",
            Self::Corrupted => "corrupted",
            Self::Storage => "storage",
            Self::ClipboardUnavailable => "clipboard_unavailable",
            Self::Internal => "internal",
        }
    }
}

// Written by hand: a derived `Serialize` would emit only the variant name,
// while the UI expects `{ code, message }` with the fixed message text.
impl Serialize for CommandError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut error = serializer.serialize_struct("CommandError", 2)?;
        error.serialize_field("code", self.code())?;
        error.serialize_field("message", &self.to_string())?;
        error.end()
    }
}

impl From<VaultError> for CommandError {
    /// Maps a core error to its UI code, dropping every detail it carries
    /// (I/O text, corruption reasons, version numbers).
    fn from(error: VaultError) -> Self {
        match error {
            VaultError::Io(_) => Self::Storage,
            VaultError::NotAVault
            | VaultError::UnsupportedVersion { .. }
            | VaultError::UnsupportedAlgorithm { .. } => Self::UnsupportedVault,
            VaultError::Corrupted { .. } => Self::Corrupted,
            VaultError::InvalidCredentials => Self::InvalidCredentials,
            VaultError::ItemNotFound => Self::ItemNotFound,
            VaultError::AlreadyExists => Self::VaultExists,
            VaultError::InvalidItem { .. } => Self::InvalidItem,
            VaultError::AttachmentNotFound => Self::AttachmentNotFound,
            VaultError::Cancelled => Self::Cancelled,
            VaultError::Crypto(error) => error.into(),
            // `VaultError` is non-exhaustive; a new variant must not leak
            // its details before it gets a dedicated code.
            _ => Self::Internal,
        }
    }
}

impl From<CryptoError> for CommandError {
    /// Maps a core cryptography error to its UI code. A failed decryption
    /// is reported as wrong credentials, never as a distinct crypto error.
    fn from(error: CryptoError) -> Self {
        match error {
            CryptoError::RecoveryCodeLength
            | CryptoError::RecoveryCodeCharacter
            | CryptoError::RecoveryCodeChecksum => Self::InvalidRecoveryCode,
            CryptoError::Decryption => Self::InvalidCredentials,
            CryptoError::MalformedKeySlot { .. } | CryptoError::MalformedCiphertext => {
                Self::Corrupted
            }
            // `CryptoError` is non-exhaustive too: same reasoning as above.
            _ => Self::Internal,
        }
    }
}

/// Result type of the command logic.
pub type CommandResult<T> = Result<T, CommandError>;

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [CommandError; 16] = [
        CommandError::Locked,
        CommandError::VaultNotFound,
        CommandError::VaultExists,
        CommandError::InvalidCredentials,
        CommandError::InvalidRecoveryCode,
        CommandError::InvalidInput,
        CommandError::ItemNotFound,
        CommandError::AttachmentNotFound,
        CommandError::FileAccess,
        CommandError::Cancelled,
        CommandError::InvalidItem,
        CommandError::UnsupportedVault,
        CommandError::Corrupted,
        CommandError::Storage,
        CommandError::ClipboardUnavailable,
        CommandError::Internal,
    ];

    #[test]
    fn serializes_as_code_and_message() {
        let json = serde_json::to_value(CommandError::Locked).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "locked", "message": "the vault is locked" })
        );
        let mut codes: Vec<&str> = ALL.iter().map(|error| error.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), ALL.len(), "codes must be unique");
    }

    #[test]
    fn io_details_are_dropped() {
        let io = std::io::Error::other("C:\\Users\\alice\\secret-path\\vault.nvault");
        let error = CommandError::from(VaultError::Io(io));
        assert_eq!(error, CommandError::Storage);
        let json = serde_json::to_string(&error).unwrap();
        assert!(
            !json.contains("alice") && !json.contains("secret-path"),
            "{json}"
        );
    }

    #[test]
    fn core_errors_map_to_codes() {
        let cases = [
            (
                VaultError::InvalidCredentials,
                CommandError::InvalidCredentials,
            ),
            (VaultError::NotAVault, CommandError::UnsupportedVault),
            (
                VaultError::UnsupportedVersion { found: 9 },
                CommandError::UnsupportedVault,
            ),
            (
                VaultError::Corrupted { reason: "x" },
                CommandError::Corrupted,
            ),
            (VaultError::AlreadyExists, CommandError::VaultExists),
            (VaultError::ItemNotFound, CommandError::ItemNotFound),
            (
                VaultError::AttachmentNotFound,
                CommandError::AttachmentNotFound,
            ),
            (VaultError::Cancelled, CommandError::Cancelled),
            (
                VaultError::Crypto(CryptoError::Random),
                CommandError::Internal,
            ),
        ];
        for (core, expected) in cases {
            assert_eq!(CommandError::from(core), expected);
        }
        for typo in [
            CryptoError::RecoveryCodeLength,
            CryptoError::RecoveryCodeCharacter,
            CryptoError::RecoveryCodeChecksum,
        ] {
            assert_eq!(CommandError::from(typo), CommandError::InvalidRecoveryCode);
        }
    }

    #[test]
    fn messages_are_fixed_strings() {
        // Every message is a constant: none can embed caller-supplied data.
        for error in ALL {
            let json = serde_json::to_value(error).unwrap();
            assert_eq!(json["code"], error.code());
            assert_eq!(json["message"], error.to_string());
            assert_eq!(json.as_object().map(serde_json::Map::len), Some(2));
        }
    }
}
