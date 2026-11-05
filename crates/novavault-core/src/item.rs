//! The vault's items: passwords, notes and secrets.
//!
//! Every item has a random [`ItemId`], a title, creation and update
//! timestamps, type-specific fields and a list of file [`Attachment`]s.
//! Only the identifier and the [`ItemKind`] are stored in plaintext; the
//! title, timestamps, fields and attachment metadata form the item's
//! payload, which is encrypted as a whole by the [`crate::vault`]. The
//! attachments' contents are stored separately (see [`crate::attachment`]).
//!
//! # Payload encoding
//!
//! Integers are little-endian; a `str` is a `u32` byte length followed by
//! that many UTF-8 bytes.
//!
//! ```text
//! created_at_ms u64 ‖ updated_at_ms u64 ‖ title str ‖ fields…
//!     ‖ attachment_count u32 ‖ attachment… ‖ zero padding
//!
//! password:   username str ‖ password str ‖ url str ‖ notes str
//! note:       body str
//! secret:     value str ‖ notes str
//! attachment: attachment_id (16) ‖ size u64 ‖ added_at_ms u64 ‖ name str
//! ```
//!
//! Format version 0 payloads have no attachment count or attachments; they
//! are decoded only to migrate a version 0 vault.
//!
//! The payload is padded with zero bytes to a multiple of [`PAD_BLOCK`]
//! bytes, so the ciphertext length reveals only a coarse size bucket, not
//! the exact length of a password or note. Decoding is strict: every field
//! must be present, valid UTF-8 and in bounds, and the padding must be
//! shorter than one block and all zero.
//!
//! # Memory hygiene
//!
//! Fields that usually hold secrets (passwords, secret values, note bodies
//! and notes) are [`SecretString`]s. The other strings are wiped when the
//! item is dropped. `Debug` output never shows any field content.

use secrecy::{ExposeSecret, SecretString};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::error::{CryptoError, Result, VaultError, VaultResult};

/// Length in bytes of an [`ItemId`].
pub const ITEM_ID_LEN: usize = 16;

/// Length in bytes of an [`AttachmentId`].
pub const ATTACHMENT_ID_LEN: usize = 16;

/// Largest number of attachments on one item.
pub const MAX_ATTACHMENTS_PER_ITEM: usize = 1000;

/// Longest attachment file name, in bytes of UTF-8.
pub const MAX_ATTACHMENT_NAME_LEN: usize = 255;

/// Payloads are padded to a multiple of this many bytes.
pub const PAD_BLOCK: usize = 64;

/// Largest accepted payload, padding included (1 MiB).
///
/// Bounds the memory a crafted file can make the reader allocate per item,
/// while leaving ample room for long notes.
pub const MAX_PAYLOAD_LEN: usize = 1024 * 1024;

/// Length of the two timestamps at the start of a payload.
const TIMESTAMPS_LEN: usize = 2 * 8;

/// Defines a random 16-byte identifier type: generation from the OS
/// CSPRNG, byte access, strict hexadecimal parsing and lower-case
/// hexadecimal `Display`.
macro_rules! random_id {
    ($(#[$doc:meta])* $name:ident, $len:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; $len]);

        impl $name {
            /// Generates a new random identifier from the OS CSPRNG.
            ///
            /// # Errors
            ///
            /// Returns [`CryptoError::Random`] if the OS random number
            /// generator fails.
            pub fn generate() -> Result<Self> {
                let mut bytes = [0u8; $len];
                getrandom::fill(&mut bytes).map_err(|_| CryptoError::Random)?;
                Ok(Self(bytes))
            }

            /// Builds an identifier from its bytes.
            pub const fn from_bytes(bytes: [u8; $len]) -> Self {
                Self(bytes)
            }

            /// The identifier's bytes.
            pub const fn as_bytes(&self) -> &[u8; $len] {
                &self.0
            }

            /// Parses the identifier from exactly 32 hexadecimal digits, in
            /// either case. Returns `None` for anything else, including
            /// signs and whitespace.
            pub fn from_hex(text: &str) -> Option<Self> {
                let digits = text.as_bytes();
                if digits.len() != 2 * $len {
                    return None;
                }
                let mut bytes = [0u8; $len];
                for (byte, pair) in bytes.iter_mut().zip(digits.chunks_exact(2)) {
                    *byte = hex_digit(pair[0])? << 4 | hex_digit(pair[1])?;
                }
                Some(Self(bytes))
            }
        }

        impl core::fmt::Display for $name {
            /// Formats the identifier as 32 lower-case hexadecimal digits.
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                for byte in self.0 {
                    write!(f, "{byte:02x}")?;
                }
                Ok(())
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, concat!(stringify!($name), "({})"), self)
            }
        }
    };
}

random_id!(
    /// Random identifier of an item, unique within its vault. Not secret.
    ItemId,
    ITEM_ID_LEN
);

random_id!(
    /// Random identifier of an attachment, unique within its vault. Not
    /// secret: it names the attachment's encrypted file on disk.
    AttachmentId,
    ATTACHMENT_ID_LEN
);

/// Value of one ASCII hexadecimal digit, in either case.
const fn hex_digit(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

/// The type of an item, stored in plaintext as the record type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// A login: username, password, URL and notes.
    Password,
    /// A free-form note.
    Note,
    /// A secret value such as an API key or token, with notes.
    Secret,
}

impl ItemKind {
    /// The on-disk record type identifier.
    pub const fn id(self) -> u8 {
        match self {
            Self::Password => 1,
            Self::Note => 2,
            Self::Secret => 3,
        }
    }

    /// Parses an on-disk record type identifier.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::Corrupted`] for an unknown identifier.
    pub const fn from_id(id: u8) -> VaultResult<Self> {
        match id {
            1 => Ok(Self::Password),
            2 => Ok(Self::Note),
            3 => Ok(Self::Secret),
            _ => Err(VaultError::Corrupted {
                reason: "unknown record type",
            }),
        }
    }
}

/// Fields of a [`ItemKind::Password`] item.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct PasswordItem {
    /// Display name of the item.
    pub title: String,
    /// User name or e-mail address for the login.
    pub username: String,
    /// The password.
    #[zeroize(skip)] // `SecretString` wipes itself on drop.
    pub password: SecretString,
    /// Address of the site or service.
    pub url: String,
    /// Free-form notes.
    #[zeroize(skip)]
    pub notes: SecretString,
}

/// Fields of a [`ItemKind::Note`] item.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct NoteItem {
    /// Display name of the item.
    pub title: String,
    /// The note's text.
    #[zeroize(skip)]
    pub body: SecretString,
}

/// Fields of a [`ItemKind::Secret`] item.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecretItem {
    /// Display name of the item.
    pub title: String,
    /// The secret value, for example an API key.
    #[zeroize(skip)]
    pub value: SecretString,
    /// Free-form notes.
    #[zeroize(skip)]
    pub notes: SecretString,
}

/// The user-editable content of an item.
pub enum ItemContent {
    /// A login.
    Password(PasswordItem),
    /// A note.
    Note(NoteItem),
    /// A secret value.
    Secret(SecretItem),
}

impl ItemContent {
    /// The item's type.
    pub const fn kind(&self) -> ItemKind {
        match self {
            Self::Password(_) => ItemKind::Password,
            Self::Note(_) => ItemKind::Note,
            Self::Secret(_) => ItemKind::Secret,
        }
    }

    /// The item's title.
    pub fn title(&self) -> &str {
        match self {
            Self::Password(item) => &item.title,
            Self::Note(item) => &item.title,
            Self::Secret(item) => &item.title,
        }
    }
}

/// Metadata of one file attached to an item.
///
/// The file's contents are stored encrypted outside the vault file (see
/// [`crate::attachment`]); this metadata lives in the item's encrypted
/// payload, so names and sizes are never stored in plaintext.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct Attachment {
    #[zeroize(skip)]
    id: AttachmentId,
    name: String,
    size: u64,
    added_at_ms: u64,
}

impl Attachment {
    /// Assembles attachment metadata. Crate-internal: the vault assigns
    /// identifiers and measures sizes while encrypting.
    pub(crate) const fn new(id: AttachmentId, name: String, size: u64, added_at_ms: u64) -> Self {
        Self {
            id,
            name,
            size,
            added_at_ms,
        }
    }

    /// The attachment's identifier.
    pub const fn id(&self) -> AttachmentId {
        self.id
    }

    /// The original file name (no directory part).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The file's size in bytes.
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// When the file was attached, in milliseconds since the Unix epoch
    /// (UTC).
    pub const fn added_at_ms(&self) -> u64 {
        self.added_at_ms
    }
}

/// Checks that `name` is usable as an attachment's file name: 1 to
/// [`MAX_ATTACHMENT_NAME_LEN`] bytes, no path separators, no control
/// characters, and not `.` or `..`.
///
/// The name is suggested again when the file is exported, so it must never
/// be able to point outside the directory the user picks.
///
/// # Errors
///
/// [`VaultError::InvalidItem`] if the name is not acceptable.
pub fn validate_attachment_name(name: &str) -> VaultResult<()> {
    let valid = !name.is_empty()
        && name.len() <= MAX_ATTACHMENT_NAME_LEN
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|c| c == '/' || c == '\\' || c.is_control());
    if valid {
        Ok(())
    } else {
        Err(VaultError::InvalidItem {
            reason: "invalid attachment name",
        })
    }
}

/// A decrypted item: identifier, timestamps, content and attachments.
pub struct Item {
    id: ItemId,
    created_at_ms: u64,
    updated_at_ms: u64,
    content: ItemContent,
    attachments: Vec<Attachment>,
}

impl Item {
    /// Assembles an item. Crate-internal: the vault assigns identifiers and
    /// timestamps.
    pub(crate) const fn new(
        id: ItemId,
        created_at_ms: u64,
        updated_at_ms: u64,
        content: ItemContent,
        attachments: Vec<Attachment>,
    ) -> Self {
        Self {
            id,
            created_at_ms,
            updated_at_ms,
            content,
            attachments,
        }
    }

    /// The files attached to the item, in the order they were added.
    pub fn attachments(&self) -> &[Attachment] {
        &self.attachments
    }

    /// Splits the item into its content and attachments. Crate-internal:
    /// used to re-encrypt an item with changed attachments.
    pub(crate) fn into_parts(self) -> (ItemContent, Vec<Attachment>) {
        (self.content, self.attachments)
    }

    /// The item's identifier.
    pub const fn id(&self) -> ItemId {
        self.id
    }

    /// When the item was created, in milliseconds since the Unix epoch (UTC).
    pub const fn created_at_ms(&self) -> u64 {
        self.created_at_ms
    }

    /// When the item was last updated, in milliseconds since the Unix epoch
    /// (UTC).
    pub const fn updated_at_ms(&self) -> u64 {
        self.updated_at_ms
    }

    /// The item's content.
    pub const fn content(&self) -> &ItemContent {
        &self.content
    }

    /// Takes the item's content, for example to edit and store it again.
    pub fn into_content(self) -> ItemContent {
        self.content
    }
}

/// The non-secret-field view of an item, for listing the vault.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct ItemSummary {
    /// The item's identifier.
    #[zeroize(skip)]
    pub id: ItemId,
    /// The item's type.
    #[zeroize(skip)]
    pub kind: ItemKind,
    /// The item's title.
    pub title: String,
    /// Creation time, in milliseconds since the Unix epoch (UTC).
    pub created_at_ms: u64,
    /// Last update time, in milliseconds since the Unix epoch (UTC).
    pub updated_at_ms: u64,
}

impl From<Item> for ItemSummary {
    fn from(item: Item) -> Self {
        Self {
            id: item.id,
            kind: item.content.kind(),
            title: item.content.title().to_owned(),
            created_at_ms: item.created_at_ms,
            updated_at_ms: item.updated_at_ms,
        }
    }
}

// Field contents are never printed: titles, user names and URLs can be as
// revealing as the secrets themselves.
macro_rules! redacted_debug {
    ($($type:ty),*) => {$(
        impl core::fmt::Debug for $type {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct(stringify!($type)).finish_non_exhaustive()
            }
        }
    )*};
}
redacted_debug!(PasswordItem, NoteItem, SecretItem);

impl core::fmt::Debug for Attachment {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Attachment")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl core::fmt::Debug for ItemContent {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Password(item) => item.fmt(f),
            Self::Note(item) => item.fmt(f),
            Self::Secret(item) => item.fmt(f),
        }
    }
}

impl core::fmt::Debug for Item {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Item")
            .field("id", &self.id)
            .field("kind", &self.content.kind())
            .field("attachments", &self.attachments.len())
            .finish_non_exhaustive()
    }
}

impl core::fmt::Debug for ItemSummary {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ItemSummary")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

/// The string fields of `content`, in payload order.
fn fields(content: &ItemContent) -> Vec<&str> {
    match content {
        ItemContent::Password(item) => vec![
            &item.title,
            &item.username,
            item.password.expose_secret(),
            &item.url,
            item.notes.expose_secret(),
        ],
        ItemContent::Note(item) => vec![&item.title, item.body.expose_secret()],
        ItemContent::Secret(item) => vec![
            &item.title,
            item.value.expose_secret(),
            item.notes.expose_secret(),
        ],
    }
}

/// A decoded payload.
pub(crate) struct Payload {
    /// Creation time, milliseconds since the Unix epoch.
    pub(crate) created_at_ms: u64,
    /// Last update time, milliseconds since the Unix epoch.
    pub(crate) updated_at_ms: u64,
    /// The item's fields.
    pub(crate) content: ItemContent,
    /// The item's attachments (always empty in format version 0).
    pub(crate) attachments: Vec<Attachment>,
}

/// Fixed-size part of an encoded attachment: identifier, size and time.
const ATTACHMENT_FIXED_LEN: usize = ATTACHMENT_ID_LEN + 8 + 8;

/// Encodes an item's payload (timestamps, fields and, from format version
/// 1, attachments) for format `version`, padded to [`PAD_BLOCK`].
///
/// `version` is the vault's format version; it decides whether the
/// attachment section is written. The result is plaintext and is returned
/// in a [`Zeroizing`] buffer that the caller encrypts and then drops.
///
/// # Errors
///
/// Returns [`VaultError::InvalidItem`] if the padded payload would exceed
/// [`MAX_PAYLOAD_LEN`], if an attachment name is invalid, if there are more
/// than [`MAX_ATTACHMENTS_PER_ITEM`] attachments, or if attachments are
/// given for format version 0, which cannot store them.
pub(crate) fn encode_payload(
    version: u16,
    created_at_ms: u64,
    updated_at_ms: u64,
    content: &ItemContent,
    attachments: &[Attachment],
) -> VaultResult<Zeroizing<Vec<u8>>> {
    const TOO_LARGE: VaultError = VaultError::InvalidItem {
        reason: "item is too large",
    };
    let with_attachments = version >= 1;
    if attachments.len() > MAX_ATTACHMENTS_PER_ITEM
        || (!with_attachments && !attachments.is_empty())
    {
        return Err(VaultError::InvalidItem {
            reason: "too many attachments",
        });
    }
    let fields = fields(content);
    let mut len = TIMESTAMPS_LEN;
    for field in &fields {
        if u32::try_from(field.len()).is_err() {
            return Err(TOO_LARGE);
        }
        len = len.checked_add(4 + field.len()).ok_or(TOO_LARGE)?;
    }
    if with_attachments {
        len = len.checked_add(4).ok_or(TOO_LARGE)?;
        for attachment in attachments {
            validate_attachment_name(&attachment.name)?;
            len = len
                .checked_add(ATTACHMENT_FIXED_LEN + 4 + attachment.name.len())
                .ok_or(TOO_LARGE)?;
        }
    }
    let padded_len = len.div_ceil(PAD_BLOCK) * PAD_BLOCK;
    if padded_len > MAX_PAYLOAD_LEN {
        return Err(TOO_LARGE);
    }

    // Allocate the exact final size up front: a growing Vec would leave
    // unwiped copies of the plaintext behind in freed memory.
    let mut payload = Zeroizing::new(Vec::with_capacity(padded_len));
    payload.extend_from_slice(&created_at_ms.to_le_bytes());
    payload.extend_from_slice(&updated_at_ms.to_le_bytes());
    for field in &fields {
        // Lengths were checked to fit in a u32 above.
        let field_len = u32::try_from(field.len()).map_err(|_| TOO_LARGE)?;
        payload.extend_from_slice(&field_len.to_le_bytes());
        payload.extend_from_slice(field.as_bytes());
    }
    if with_attachments {
        // Both bounds were checked above.
        let count = u32::try_from(attachments.len()).map_err(|_| TOO_LARGE)?;
        payload.extend_from_slice(&count.to_le_bytes());
        for attachment in attachments {
            let name_len = u32::try_from(attachment.name.len()).map_err(|_| TOO_LARGE)?;
            payload.extend_from_slice(attachment.id.as_bytes());
            payload.extend_from_slice(&attachment.size.to_le_bytes());
            payload.extend_from_slice(&attachment.added_at_ms.to_le_bytes());
            payload.extend_from_slice(&name_len.to_le_bytes());
            payload.extend_from_slice(attachment.name.as_bytes());
        }
    }
    payload.resize(padded_len, 0);
    Ok(payload)
}

/// Decodes a payload produced by [`encode_payload`] for an item of `kind`
/// in format `version`.
///
/// `kind` comes from the record's plaintext type byte, which is safe to
/// trust here because it is part of the record's authenticated data.
///
/// # Errors
///
/// Returns [`VaultError::Corrupted`] if the payload is truncated, has a
/// field out of bounds or not valid UTF-8, has too many attachments, an
/// invalid attachment name or a repeated attachment identifier, or has
/// invalid padding.
pub(crate) fn decode_payload(version: u16, kind: ItemKind, payload: &[u8]) -> VaultResult<Payload> {
    let mut reader = Reader(payload);
    let created_at_ms = reader.u64()?;
    let updated_at_ms = reader.u64()?;
    let title = reader.string()?;
    let content = match kind {
        ItemKind::Password => ItemContent::Password(PasswordItem {
            title,
            username: reader.string()?,
            password: reader.secret()?,
            url: reader.string()?,
            notes: reader.secret()?,
        }),
        ItemKind::Note => ItemContent::Note(NoteItem {
            title,
            body: reader.secret()?,
        }),
        ItemKind::Secret => ItemContent::Secret(SecretItem {
            title,
            value: reader.secret()?,
            notes: reader.secret()?,
        }),
    };
    let attachments = if version >= 1 {
        reader.attachments()?
    } else {
        Vec::new()
    };
    reader.padding(payload.len())?;
    Ok(Payload {
        created_at_ms,
        updated_at_ms,
        content,
        attachments,
    })
}

/// Bounds-checked cursor over a payload; holds the bytes not read yet.
struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    /// The single error for any payload problem. The payload has already
    /// been authenticated, so a decoding failure means a writer bug, not an
    /// attack, and one reason is enough.
    const MALFORMED: VaultError = VaultError::Corrupted {
        reason: "malformed record payload",
    };

    /// Consumes and returns the next `len` bytes.
    fn take(&mut self, len: usize) -> VaultResult<&[u8]> {
        if len > self.0.len() {
            return Err(Self::MALFORMED);
        }
        let (head, rest) = self.0.split_at(len);
        self.0 = rest;
        Ok(head)
    }

    /// Consumes the next `N` bytes as an array.
    fn array<const N: usize>(&mut self) -> VaultResult<[u8; N]> {
        self.take(N)?.try_into().map_err(|_| Self::MALFORMED)
    }

    /// Consumes a little-endian `u64`.
    fn u64(&mut self) -> VaultResult<u64> {
        self.array().map(u64::from_le_bytes)
    }

    /// Consumes a little-endian `u32`.
    fn u32(&mut self) -> VaultResult<u32> {
        self.array().map(u32::from_le_bytes)
    }

    /// Consumes the attachment section: a count, then that many entries,
    /// each with a valid name and an identifier not seen before.
    fn attachments(&mut self) -> VaultResult<Vec<Attachment>> {
        let count = usize::try_from(self.u32()?).map_err(|_| Self::MALFORMED)?;
        if count > MAX_ATTACHMENTS_PER_ITEM {
            return Err(Self::MALFORMED);
        }
        // `count` is bounded above, so pre-allocating is safe.
        let mut attachments: Vec<Attachment> = Vec::with_capacity(count);
        for _ in 0..count {
            let id = AttachmentId::from_bytes(self.array()?);
            let size = self.u64()?;
            let added_at_ms = self.u64()?;
            let name = self.string()?;
            if validate_attachment_name(&name).is_err()
                || attachments.iter().any(|existing| existing.id == id)
            {
                return Err(Self::MALFORMED);
            }
            attachments.push(Attachment::new(id, name, size, added_at_ms));
        }
        Ok(attachments)
    }

    /// Consumes a length-prefixed string, borrowed from the payload.
    fn str(&mut self) -> VaultResult<&str> {
        let len = self.take(4)?;
        let len: [u8; 4] = len.try_into().map_err(|_| Self::MALFORMED)?;
        let len = usize::try_from(u32::from_le_bytes(len)).map_err(|_| Self::MALFORMED)?;
        // Validated in place and borrowed: `string` and `secret` then copy
        // once into an exactly sized allocation, so no stray plaintext copy
        // is left behind, even on error.
        core::str::from_utf8(self.take(len)?).map_err(|_| Self::MALFORMED)
    }

    /// Consumes a plain string field; it is wiped on drop once it is part
    /// of an item or attachment.
    fn string(&mut self) -> VaultResult<String> {
        self.str().map(str::to_owned)
    }

    /// Consumes a string field that usually holds a secret.
    fn secret(&mut self) -> VaultResult<SecretString> {
        self.str().map(SecretString::from)
    }

    /// Checks that only valid padding remains for a payload of `total` bytes.
    fn padding(&self, total: usize) -> VaultResult<()> {
        let valid = total % PAD_BLOCK == 0
            && self.0.len() < PAD_BLOCK
            && self.0.iter().all(|&byte| byte == 0);
        if valid { Ok(()) } else { Err(Self::MALFORMED) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::FORMAT_VERSION;

    fn s(text: &str) -> SecretString {
        SecretString::from(text)
    }

    fn password_item() -> ItemContent {
        ItemContent::Password(PasswordItem {
            title: "Bank".into(),
            username: "alice@example.com".into(),
            password: s("correct horse battery staple"),
            url: "https://bank.example".into(),
            notes: s("PIN is elsewhere"),
        })
    }

    fn note_item() -> ItemContent {
        ItemContent::Note(NoteItem {
            title: "t".into(),
            body: s("b"),
        })
    }

    fn secret_item() -> ItemContent {
        ItemContent::Secret(SecretItem {
            title: "API key ✓".into(),
            value: s("sk-test-123"),
            notes: s(""),
        })
    }

    fn attachment(byte: u8, name: &str) -> Attachment {
        Attachment::new(
            AttachmentId::from_bytes([byte; ATTACHMENT_ID_LEN]),
            name.into(),
            u64::from(byte) * 1000,
            u64::from(byte),
        )
    }

    fn encode(content: &ItemContent, attachments: &[Attachment]) -> Zeroizing<Vec<u8>> {
        encode_payload(FORMAT_VERSION, 1, 2, content, attachments).unwrap()
    }

    fn decode(kind: ItemKind, payload: &[u8]) -> VaultResult<Payload> {
        decode_payload(FORMAT_VERSION, kind, payload)
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn padded(hex: &str) -> String {
        let mut padded = hex.to_owned();
        let len = hex.len() / 2;
        padded.push_str(&"00".repeat(len.div_ceil(PAD_BLOCK) * PAD_BLOCK - len));
        padded
    }

    /// Format version 0 payload of a note titled "t" with body "b", created
    /// at 1 ms and updated at 2 ms, as published in docs/vault-format.md.
    #[test]
    fn known_note_encoding_v0() {
        let payload = encode_payload(0, 1, 2, &note_item(), &[]).unwrap();
        let expected = concat!(
            "0100000000000000", // created_at_ms = 1
            "0200000000000000", // updated_at_ms = 2
            "0100000074",       // title "t"
            "0100000062",       // body "b"
        );
        assert_eq!(hex(&payload), padded(expected));
    }

    /// The same note in format version 1, with one attachment, as
    /// published in docs/vault-format.md.
    #[test]
    fn known_note_encoding_v1() {
        let file = Attachment::new(
            AttachmentId::from_bytes([0xAA; ATTACHMENT_ID_LEN]),
            "a.txt".into(),
            3,
            4,
        );
        let payload = encode(&note_item(), &[file]);
        let expected = concat!(
            "0100000000000000",                 // created_at_ms = 1
            "0200000000000000",                 // updated_at_ms = 2
            "0100000074",                       // title "t"
            "0100000062",                       // body "b"
            "01000000",                         // attachment_count = 1
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", // attachment_id
            "0300000000000000",                 // size = 3
            "0400000000000000",                 // added_at_ms = 4
            "05000000612e747874",               // name "a.txt"
        );
        assert_eq!(payload.len(), 2 * PAD_BLOCK);
        assert_eq!(hex(&payload), padded(expected));
    }

    fn assert_same(a: &ItemContent, b: &ItemContent) {
        assert_eq!(a.kind(), b.kind());
        assert_eq!(fields(a), fields(b));
    }

    #[test]
    fn round_trips_every_kind() {
        for version in [0, FORMAT_VERSION] {
            for content in [password_item(), note_item(), secret_item()] {
                let payload =
                    encode_payload(version, 1_700_000_000_000, u64::MAX, &content, &[]).unwrap();
                assert_eq!(payload.len() % PAD_BLOCK, 0);
                let decoded = decode_payload(version, content.kind(), &payload).unwrap();
                assert_eq!(decoded.created_at_ms, 1_700_000_000_000);
                assert_eq!(decoded.updated_at_ms, u64::MAX);
                assert_same(&decoded.content, &content);
                assert!(decoded.attachments.is_empty());
            }
        }
    }

    #[test]
    fn attachments_round_trip_in_order() {
        let files = [attachment(3, "scan.pdf"), attachment(1, "photo ✓.jpg")];
        let payload = encode(&password_item(), &files);
        let decoded = decode(ItemKind::Password, &payload).unwrap();
        assert_same(&decoded.content, &password_item());
        let got: Vec<(AttachmentId, &str, u64, u64)> = decoded
            .attachments
            .iter()
            .map(|a| (a.id(), a.name(), a.size(), a.added_at_ms()))
            .collect();
        let want: Vec<(AttachmentId, &str, u64, u64)> = files
            .iter()
            .map(|a| (a.id(), a.name(), a.size(), a.added_at_ms()))
            .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn version_zero_cannot_hold_attachments() {
        assert!(matches!(
            encode_payload(0, 0, 0, &note_item(), &[attachment(1, "a")]),
            Err(VaultError::InvalidItem { .. })
        ));
    }

    #[test]
    fn attachment_limits_are_enforced() {
        let too_many: Vec<Attachment> = (0..=MAX_ATTACHMENTS_PER_ITEM)
            .map(|n| {
                let mut id = [0u8; ATTACHMENT_ID_LEN];
                id[..8].copy_from_slice(&(n as u64).to_le_bytes());
                Attachment::new(AttachmentId::from_bytes(id), "f".into(), 0, 0)
            })
            .collect();
        assert!(matches!(
            encode_payload(FORMAT_VERSION, 0, 0, &note_item(), &too_many),
            Err(VaultError::InvalidItem { .. })
        ));
        assert!(
            encode_payload(
                FORMAT_VERSION,
                0,
                0,
                &note_item(),
                &too_many[..MAX_ATTACHMENTS_PER_ITEM]
            )
            .is_ok()
        );
        assert!(matches!(
            encode_payload(FORMAT_VERSION, 0, 0, &note_item(), &[attachment(1, "a/b")]),
            Err(VaultError::InvalidItem { .. })
        ));
    }

    #[test]
    fn attachment_names_are_validated() {
        for good in [
            "a",
            "report 2026.pdf",
            "é✓.txt",
            ".hidden",
            &"x".repeat(255),
        ] {
            assert!(validate_attachment_name(good).is_ok(), "{good}");
        }
        for bad in [
            "",
            ".",
            "..",
            "a/b",
            "a\\b",
            "a\0b",
            "line\nbreak",
            &"x".repeat(256),
        ] {
            assert!(validate_attachment_name(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn rejects_malformed_attachment_sections() {
        let payload = encode(&note_item(), &[attachment(1, "a"), attachment(2, "b")]);
        // Layout: 16 timestamps, 5 title, 5 body, then the count.
        let count_at = 26;
        let first = count_at + 4;
        let second = first + ATTACHMENT_FIXED_LEN + 4 + 1;

        let mut too_many = payload.to_vec();
        too_many[count_at..first]
            .copy_from_slice(&(MAX_ATTACHMENTS_PER_ITEM as u32 + 1).to_le_bytes());
        let mut duplicate = payload.to_vec();
        duplicate[second..second + ATTACHMENT_ID_LEN].fill(1);
        let mut bad_name = payload.to_vec();
        bad_name[first + ATTACHMENT_FIXED_LEN + 4] = b'/';
        let mut empty_name = payload.to_vec();
        empty_name[first + ATTACHMENT_FIXED_LEN..first + ATTACHMENT_FIXED_LEN + 4]
            .copy_from_slice(&0u32.to_le_bytes());
        for (name, input) in [
            ("count", too_many),
            ("duplicate id", duplicate),
            ("bad name", bad_name),
            ("empty name", empty_name),
        ] {
            assert!(
                matches!(
                    decode(ItemKind::Note, &input),
                    Err(VaultError::Corrupted { .. })
                ),
                "{name}"
            );
        }
    }

    #[test]
    fn padding_hides_exact_length() {
        let short = ItemContent::Note(NoteItem {
            title: String::new(),
            body: s("a"),
        });
        let longer = ItemContent::Note(NoteItem {
            title: String::new(),
            body: s("a much longer note body"),
        });
        assert_eq!(encode(&short, &[]).len(), encode(&longer, &[]).len());
    }

    #[test]
    fn exact_block_needs_no_padding() {
        // 16 (timestamps) + 2 × 4 (lengths) + 36 + 4 (count) = 64 bytes.
        let content = ItemContent::Note(NoteItem {
            title: String::new(),
            body: s(&"x".repeat(36)),
        });
        let payload = encode(&content, &[]);
        assert_eq!(payload.len(), PAD_BLOCK);
        assert!(decode(ItemKind::Note, &payload).is_ok());
    }

    #[test]
    fn rejects_oversized_items() {
        let content = ItemContent::Note(NoteItem {
            title: String::new(),
            body: s(&"x".repeat(MAX_PAYLOAD_LEN)),
        });
        assert!(matches!(
            encode_payload(FORMAT_VERSION, 0, 0, &content, &[]),
            Err(VaultError::InvalidItem { .. })
        ));
    }

    fn assert_malformed(kind: ItemKind, payload: &[u8]) {
        assert!(matches!(
            decode(kind, payload),
            Err(VaultError::Corrupted { .. })
        ));
    }

    #[test]
    fn rejects_malformed_payloads() {
        let payload = encode(&note_item(), &[attachment(1, "a")]);

        // Truncated anywhere.
        for len in 0..payload.len() {
            assert_malformed(ItemKind::Note, &payload[..len]);
        }
        // One extra padding block, or non-zero padding.
        let mut extra = payload.to_vec();
        extra.extend_from_slice(&[0u8; PAD_BLOCK]);
        assert_malformed(ItemKind::Note, &extra);
        let mut dirty = payload.to_vec();
        let last = dirty.len() - 1;
        dirty[last] = 1;
        assert_malformed(ItemKind::Note, &dirty);
        // A length pointing past the end.
        let mut long = payload.to_vec();
        long[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_malformed(ItemKind::Note, &long);
        // Invalid UTF-8 in the title.
        let mut invalid = payload.to_vec();
        invalid[20] = 0xFF;
        assert_malformed(ItemKind::Note, &invalid);
        // Decoding as a different kind is not checked here: zero padding
        // reads as empty strings, so it may succeed. The record type is
        // authenticated as associated data, so that mismatch cannot occur.
    }

    #[test]
    fn kind_ids_are_strict() {
        for kind in [ItemKind::Password, ItemKind::Note, ItemKind::Secret] {
            assert_eq!(ItemKind::from_id(kind.id()).unwrap(), kind);
        }
        for id in [0u8, 4, 0xFF] {
            assert!(matches!(
                ItemKind::from_id(id),
                Err(VaultError::Corrupted { .. })
            ));
        }
    }

    #[test]
    fn ids_are_random_and_hex_formatted() {
        assert_ne!(ItemId::generate().unwrap(), ItemId::generate().unwrap());
        assert_ne!(
            AttachmentId::generate().unwrap(),
            AttachmentId::generate().unwrap()
        );
        let id = ItemId::from_bytes([0xAB; ITEM_ID_LEN]);
        assert_eq!(id.to_string(), "ab".repeat(ITEM_ID_LEN));
        assert_eq!(format!("{id:?}"), format!("ItemId({id})"));
        let attachment = AttachmentId::from_bytes([0x01; ATTACHMENT_ID_LEN]);
        assert_eq!(
            format!("{attachment:?}"),
            format!("AttachmentId({attachment})")
        );
    }

    #[test]
    fn ids_parse_from_strict_hex() {
        let id = ItemId::generate().unwrap();
        assert_eq!(ItemId::from_hex(&id.to_string()), Some(id));
        assert_eq!(ItemId::from_hex(&id.to_string().to_uppercase()), Some(id));
        let attachment = AttachmentId::generate().unwrap();
        assert_eq!(
            AttachmentId::from_hex(&attachment.to_string()),
            Some(attachment)
        );
        for bad in [
            "",
            "abc",
            &"g".repeat(32),
            &"a".repeat(33),
            &"+f".repeat(16),
            &" a".repeat(16),
            // Multi-byte UTF-8 of the right byte length.
            &"é".repeat(16),
        ] {
            assert_eq!(ItemId::from_hex(bad), None, "{bad}");
            assert_eq!(AttachmentId::from_hex(bad), None, "{bad}");
        }
    }

    #[test]
    fn debug_shows_no_field_content() {
        let item = Item::new(
            ItemId::from_bytes([0; ITEM_ID_LEN]),
            0,
            0,
            password_item(),
            vec![attachment(1, "tax-return.pdf")],
        );
        let printed = format!("{item:?} {:?} {:?}", item.content(), item.attachments());
        for secret in [
            "Bank",
            "alice",
            "correct horse",
            "bank.example",
            "PIN",
            "tax-return",
        ] {
            assert!(!printed.contains(secret), "{printed}");
        }
        assert!(printed.contains("attachments: 1"), "{printed}");
        let summary = ItemSummary::from(item);
        assert_eq!(summary.title, "Bank");
        assert!(!format!("{summary:?}").contains("Bank"));
    }
}
