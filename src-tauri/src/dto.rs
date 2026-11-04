//! Data exchanged with the UI over Tauri IPC.
//!
//! Inputs deserialize secret fields straight into [`SecretString`], so they
//! are wiped when dropped and never printed. Outputs carry only what a
//! screen needs: item listings have no secret fields at all, and the
//! recovery code is serialized only in the two responses that must show it
//! once. No type here derives `Debug`, so contents cannot end up in logs.

use novavault_core::item::ITEM_ID_LEN;
use novavault_core::{
    Item, ItemContent, ItemId, ItemKind, ItemSummary, NoteItem, PasswordItem, RecoveryCode,
    SecretItem,
};
use secrecy::{ExposeSecret, SecretString};
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{CommandError, CommandResult};

/// The content of an item to create or update, tagged by `kind`.
///
/// JSON example: `{ "kind": "note", "title": "Diary", "body": "…" }`.
#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ItemInput {
    /// A login.
    Password {
        /// Display name.
        title: String,
        /// User name or e-mail address.
        username: String,
        /// The password.
        password: SecretString,
        /// Address of the site or service.
        url: String,
        /// Free-form notes.
        notes: SecretString,
    },
    /// A free-form note.
    Note {
        /// Display name.
        title: String,
        /// The note's text.
        body: SecretString,
    },
    /// A secret value such as an API key.
    Secret {
        /// Display name.
        title: String,
        /// The secret value.
        value: SecretString,
        /// Free-form notes.
        notes: SecretString,
    },
}

impl From<ItemInput> for ItemContent {
    /// Moves the fields into the core item types (which wipe them on drop)
    /// without copying them.
    fn from(input: ItemInput) -> Self {
        match input {
            ItemInput::Password {
                title,
                username,
                password,
                url,
                notes,
            } => Self::Password(PasswordItem {
                title,
                username,
                password,
                url,
                notes,
            }),
            ItemInput::Note { title, body } => Self::Note(NoteItem { title, body }),
            ItemInput::Secret {
                title,
                value,
                notes,
            } => Self::Secret(SecretItem {
                title,
                value,
                notes,
            }),
        }
    }
}

/// New content for an existing item, tagged by `kind`.
///
/// Plain fields are always replaced. A secret field set to `null` (or left
/// out) keeps its stored value, so the edit form never needs a secret the
/// user did not reveal. The kind must match the stored item's kind.
///
/// JSON example: `{ "kind": "note", "title": "Diary", "body": null }`.
#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ItemUpdate {
    /// A login.
    Password {
        /// Display name.
        title: String,
        /// User name or e-mail address.
        username: String,
        /// The new password, or `None` to keep it.
        #[serde(default)]
        password: Option<SecretString>,
        /// Address of the site or service.
        url: String,
        /// The new notes, or `None` to keep them.
        #[serde(default)]
        notes: Option<SecretString>,
    },
    /// A free-form note.
    Note {
        /// Display name.
        title: String,
        /// The new text, or `None` to keep it.
        #[serde(default)]
        body: Option<SecretString>,
    },
    /// A secret value such as an API key.
    Secret {
        /// Display name.
        title: String,
        /// The new value, or `None` to keep it.
        #[serde(default)]
        value: Option<SecretString>,
        /// The new notes, or `None` to keep them.
        #[serde(default)]
        notes: Option<SecretString>,
    },
}

impl ItemUpdate {
    /// Applies the update to `content`, the item's stored content, in place.
    ///
    /// Replaced plain fields are wiped first; replaced secret fields wipe
    /// themselves when dropped.
    ///
    /// # Errors
    ///
    /// [`CommandError::InvalidInput`] if the update's kind is not the stored
    /// item's kind: changing the type would silently drop fields.
    pub fn apply_to(self, mut content: ItemContent) -> CommandResult<ItemContent> {
        match (self, &mut content) {
            (
                Self::Password {
                    title,
                    username,
                    password,
                    url,
                    notes,
                },
                ItemContent::Password(item),
            ) => {
                replace_plain(&mut item.title, title);
                replace_plain(&mut item.username, username);
                replace_plain(&mut item.url, url);
                replace_secret(&mut item.password, password);
                replace_secret(&mut item.notes, notes);
            }
            (Self::Note { title, body }, ItemContent::Note(item)) => {
                replace_plain(&mut item.title, title);
                replace_secret(&mut item.body, body);
            }
            (
                Self::Secret {
                    title,
                    value,
                    notes,
                },
                ItemContent::Secret(item),
            ) => {
                replace_plain(&mut item.title, title);
                replace_secret(&mut item.value, value);
                replace_secret(&mut item.notes, notes);
            }
            _ => return Err(CommandError::InvalidInput),
        }
        Ok(content)
    }
}

/// Wipes `field` and stores `value` in it.
fn replace_plain(field: &mut String, value: String) {
    field.zeroize();
    *field = value;
}

/// Stores `value` in `field` if given; the old secret wipes itself on drop.
fn replace_secret(field: &mut SecretString, value: Option<SecretString>) {
    if let Some(value) = value {
        *field = value;
    }
}

/// A field of an item, for revealing or copying it.
///
/// JSON: `"username"`, `"password"`, `"url"`, `"notes"`, `"body"` or
/// `"value"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemField {
    /// A password item's user name (not secret).
    Username,
    /// A password item's password.
    Password,
    /// A password item's address (not secret).
    Url,
    /// The notes of a password or secret item.
    Notes,
    /// A note's text.
    Body,
    /// A secret item's value.
    Value,
}

impl ItemField {
    /// Whether the field is secret, that is, hidden until revealed.
    pub const fn is_secret(self) -> bool {
        !matches!(self, Self::Username | Self::Url)
    }

    /// The text of this field in `content`.
    ///
    /// # Errors
    ///
    /// [`CommandError::InvalidInput`] if items of this kind have no such
    /// field.
    pub fn value_in(self, content: &ItemContent) -> CommandResult<&str> {
        match (content, self) {
            (ItemContent::Password(item), Self::Username) => Ok(&item.username),
            (ItemContent::Password(item), Self::Password) => Ok(item.password.expose_secret()),
            (ItemContent::Password(item), Self::Url) => Ok(&item.url),
            (ItemContent::Password(item), Self::Notes) => Ok(item.notes.expose_secret()),
            (ItemContent::Note(item), Self::Body) => Ok(item.body.expose_secret()),
            (ItemContent::Secret(item), Self::Value) => Ok(item.value.expose_secret()),
            (ItemContent::Secret(item), Self::Notes) => Ok(item.notes.expose_secret()),
            _ => Err(CommandError::InvalidInput),
        }
    }
}

/// One item as shown on its details screen: every plain field, and for each
/// secret field only whether it is non-empty. Never contains a secret
/// value; those are sent one at a time by [`RevealedValueDto`].
///
/// JSON example: `{ "id": "…", "kind": "password", "title": "Bank",
/// "username": "alice", "url": "…", "hasPassword": true, "hasNotes": false,
/// "createdAtMs": 0, "updatedAtMs": 0 }`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDetailsDto {
    /// Item identifier, 32 lower-case hexadecimal digits.
    pub id: String,
    /// Display name.
    pub title: String,
    /// Creation time, milliseconds since the Unix epoch (UTC).
    pub created_at_ms: u64,
    /// Last update time, milliseconds since the Unix epoch (UTC).
    pub updated_at_ms: u64,
    /// The kind-specific fields, with the `kind` tag.
    #[serde(flatten)]
    pub fields: ItemDetailFields,
}

/// The kind-specific part of [`ItemDetailsDto`].
#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ItemDetailFields {
    /// A login.
    Password {
        /// User name or e-mail address.
        username: String,
        /// Address of the site or service.
        url: String,
        /// Whether the password is non-empty.
        has_password: bool,
        /// Whether the notes are non-empty.
        has_notes: bool,
    },
    /// A free-form note.
    Note {
        /// Whether the text is non-empty.
        has_body: bool,
    },
    /// A secret value.
    Secret {
        /// Whether the value is non-empty.
        has_value: bool,
        /// Whether the notes are non-empty.
        has_notes: bool,
    },
}

impl From<&Item> for ItemDetailsDto {
    fn from(item: &Item) -> Self {
        let fields = match item.content() {
            ItemContent::Password(login) => ItemDetailFields::Password {
                username: login.username.clone(),
                url: login.url.clone(),
                has_password: !login.password.expose_secret().is_empty(),
                has_notes: !login.notes.expose_secret().is_empty(),
            },
            ItemContent::Note(note) => ItemDetailFields::Note {
                has_body: !note.body.expose_secret().is_empty(),
            },
            ItemContent::Secret(secret) => ItemDetailFields::Secret {
                has_value: !secret.value.expose_secret().is_empty(),
                has_notes: !secret.notes.expose_secret().is_empty(),
            },
        };
        Self {
            id: item.id().to_string(),
            title: item.content().title().to_owned(),
            created_at_ms: item.created_at_ms(),
            updated_at_ms: item.updated_at_ms(),
            fields,
        }
    }
}

impl Drop for ItemDetailsDto {
    /// Wipes the plain fields: titles, user names and addresses can be
    /// revealing too.
    fn drop(&mut self) {
        self.title.zeroize();
        if let ItemDetailFields::Password { username, url, .. } = &mut self.fields {
            username.zeroize();
            url.zeroize();
        }
    }
}

/// One secret field's value, sent only when the user asks to reveal it.
///
/// JSON: `{ "value": "…" }`.
pub struct RevealedValueDto(SecretString);

impl RevealedValueDto {
    /// Wraps a revealed value.
    pub const fn new(value: SecretString) -> Self {
        Self(value)
    }
}

impl Serialize for RevealedValueDto {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Exposed on purpose: the user asked to see this one field.
        let mut dto = serializer.serialize_struct("RevealedValueDto", 1)?;
        dto.serialize_field("value", self.0.expose_secret())?;
        dto.end()
    }
}

/// The outcome of copying a field to the clipboard. The value itself never
/// comes back to the UI.
///
/// JSON: `{ "clearAfterSeconds": 30 }`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyResultDto {
    /// Seconds until the clipboard is cleared.
    pub clear_after_seconds: u32,
}

/// The secret used to unlock the vault.
///
/// JSON: `{ "method": "password" | "recoveryCode", "secret": "…" }`.
#[derive(Deserialize)]
#[serde(tag = "method", content = "secret", rename_all = "camelCase")]
pub enum Credential {
    /// The master password.
    Password(SecretString),
    /// The emergency recovery code, in any case, with or without dashes.
    RecoveryCode(SecretString),
}

/// One entry of the item list: identifier, type, title and timestamps.
/// Never contains secret fields.
#[derive(Serialize, Zeroize, ZeroizeOnDrop)]
#[serde(rename_all = "camelCase")]
pub struct ItemSummaryDto {
    /// Item identifier, 32 lower-case hexadecimal digits.
    #[zeroize(skip)]
    pub id: String,
    /// `"password"`, `"note"` or `"secret"`.
    #[zeroize(skip)]
    pub kind: &'static str,
    /// Display name (wiped on drop: titles can be revealing).
    pub title: String,
    /// Creation time, milliseconds since the Unix epoch (UTC).
    pub created_at_ms: u64,
    /// Last update time, milliseconds since the Unix epoch (UTC).
    pub updated_at_ms: u64,
}

impl From<ItemSummary> for ItemSummaryDto {
    fn from(mut summary: ItemSummary) -> Self {
        Self {
            id: summary.id.to_string(),
            kind: kind_name(summary.kind),
            // Taken rather than cloned so only one copy exists; the
            // summary wipes what is left when it drops.
            title: core::mem::take(&mut summary.title),
            created_at_ms: summary.created_at_ms,
            updated_at_ms: summary.updated_at_ms,
        }
    }
}

/// Whether a vault exists and whether it is unlocked.
///
/// JSON: `{ "exists": true, "unlocked": false }`.
#[derive(Serialize)]
pub struct VaultStatusDto {
    /// A vault file exists at the app's vault location.
    pub exists: bool,
    /// The vault is currently unlocked.
    pub unlocked: bool,
}

/// A newly created or regenerated recovery code, to show to the user once.
///
/// JSON: `{ "recoveryCode": "XXXX-XXXX-…" }`.
pub struct RecoveryCodeDto(SecretString);

impl From<&RecoveryCode> for RecoveryCodeDto {
    fn from(code: &RecoveryCode) -> Self {
        Self(code.to_display())
    }
}

impl Serialize for RecoveryCodeDto {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // The one place the shell deliberately exposes a secret to the UI:
        // the user must see the code to write it down.
        let mut dto = serializer.serialize_struct("RecoveryCodeDto", 1)?;
        dto.serialize_field("recoveryCode", self.0.expose_secret())?;
        dto.end()
    }
}

/// Wire name of an item kind.
const fn kind_name(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Password => "password",
        ItemKind::Note => "note",
        ItemKind::Secret => "secret",
    }
}

/// Parses an item identifier sent by the UI (32 hexadecimal digits).
///
/// # Errors
///
/// [`CommandError::InvalidInput`] if `text` is not exactly 32 hexadecimal
/// digits.
pub fn parse_item_id(text: &str) -> CommandResult<ItemId> {
    let digits = text.as_bytes();
    if digits.len() != 2 * ITEM_ID_LEN {
        return Err(CommandError::InvalidInput);
    }
    let mut bytes = [0u8; ITEM_ID_LEN];
    for (byte, pair) in bytes.iter_mut().zip(digits.chunks_exact(2)) {
        // Strict digits only: `u8::from_str_radix` would accept a sign.
        let (Some(high), Some(low)) = (hex_digit(pair[0]), hex_digit(pair[1])) else {
            return Err(CommandError::InvalidInput);
        };
        *byte = high << 4 | low;
    }
    Ok(ItemId::from_bytes(bytes))
}

/// Value of one ASCII hexadecimal digit, in either case.
const fn hex_digit(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_input_deserializes_every_kind() {
        let password: ItemInput = serde_json::from_value(serde_json::json!({
            "kind": "password", "title": "Bank", "username": "alice",
            "password": "hunter2", "url": "https://bank.example", "notes": ""
        }))
        .unwrap();
        match ItemContent::from(password) {
            ItemContent::Password(item) => {
                assert_eq!(item.username, "alice");
                assert_eq!(item.password.expose_secret(), "hunter2");
            }
            _ => panic!("expected a password item"),
        }
        let note: ItemInput =
            serde_json::from_str(r#"{"kind":"note","title":"t","body":"b"}"#).unwrap();
        assert_eq!(ItemContent::from(note).kind(), ItemKind::Note);
        let secret: ItemInput =
            serde_json::from_str(r#"{"kind":"secret","title":"t","value":"v","notes":"n"}"#)
                .unwrap();
        assert_eq!(ItemContent::from(secret).kind(), ItemKind::Secret);
    }

    #[test]
    fn item_input_rejects_missing_fields_and_unknown_kinds() {
        assert!(serde_json::from_str::<ItemInput>(r#"{"kind":"note","title":"t"}"#).is_err());
        assert!(serde_json::from_str::<ItemInput>(r#"{"kind":"file","title":"t"}"#).is_err());
    }

    #[test]
    fn item_update_treats_null_or_missing_secrets_as_unchanged() {
        let update: ItemUpdate = serde_json::from_str(
            r#"{"kind":"password","title":"t","username":"u","url":"","password":null}"#,
        )
        .unwrap();
        assert!(matches!(
            update,
            ItemUpdate::Password {
                password: None,
                notes: None,
                ..
            }
        ));
        let update: ItemUpdate =
            serde_json::from_str(r#"{"kind":"secret","title":"t","value":"v"}"#).unwrap();
        assert!(matches!(
            update,
            ItemUpdate::Secret { value: Some(v), notes: None, .. } if v.expose_secret() == "v"
        ));
    }

    #[test]
    fn item_fields_deserialize_from_camel_case() {
        let field: ItemField = serde_json::from_str(r#""username""#).unwrap();
        assert_eq!(field, ItemField::Username);
        assert!(!field.is_secret());
        let field: ItemField = serde_json::from_str(r#""value""#).unwrap();
        assert!(field.is_secret());
        assert!(serde_json::from_str::<ItemField>(r#""title""#).is_err());
    }

    #[test]
    fn credential_deserializes_both_methods() {
        let password: Credential =
            serde_json::from_str(r#"{"method":"password","secret":"pw"}"#).unwrap();
        assert!(matches!(password, Credential::Password(s) if s.expose_secret() == "pw"));
        let code: Credential =
            serde_json::from_str(r#"{"method":"recoveryCode","secret":"abcd"}"#).unwrap();
        assert!(matches!(code, Credential::RecoveryCode(s) if s.expose_secret() == "abcd"));
    }

    #[test]
    fn recovery_code_is_serialized_in_display_form() {
        let code = RecoveryCode::generate().unwrap();
        let json = serde_json::to_value(RecoveryCodeDto::from(&code)).unwrap();
        assert_eq!(
            json["recoveryCode"].as_str().unwrap(),
            code.to_display().expose_secret()
        );
    }

    #[test]
    fn item_ids_round_trip_through_hex() {
        let id = ItemId::generate().unwrap();
        assert_eq!(parse_item_id(&id.to_string()).unwrap(), id);
        assert_eq!(parse_item_id(&id.to_string().to_uppercase()).unwrap(), id);
        for bad in [
            "",
            "abc",
            &"g".repeat(32),
            &"a".repeat(33),
            "+f".repeat(16).as_str(),
        ] {
            assert_eq!(parse_item_id(bad), Err(CommandError::InvalidInput), "{bad}");
        }
        // Multi-byte UTF-8 of the right byte length is rejected too.
        assert_eq!(
            parse_item_id(&"é".repeat(16)),
            Err(CommandError::InvalidInput)
        );
    }
}
