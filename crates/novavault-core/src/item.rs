//! The vault's items: passwords, notes and secrets.
//!
//! Every item has a random [`ItemId`], a title, creation and update
//! timestamps and type-specific fields. Only the identifier and the
//! [`ItemKind`] are stored in plaintext; the title, timestamps and fields
//! form the item's payload, which is encrypted as a whole by the vault.
//!
//! # Payload encoding
//!
//! Integers are little-endian; a `str` is a `u32` byte length followed by
//! that many UTF-8 bytes.
//!
//! ```text
//! created_at_ms u64 ‖ updated_at_ms u64 ‖ title str ‖ fields… ‖ zero padding
//!
//! password: username str ‖ password str ‖ url str ‖ notes str
//! note:     body str
//! secret:   value str ‖ notes str
//! ```
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

/// Payloads are padded to a multiple of this many bytes.
pub const PAD_BLOCK: usize = 64;

/// Largest accepted payload, padding included (1 MiB).
///
/// Bounds the memory a crafted file can make the reader allocate per item,
/// while leaving ample room for long notes.
pub const MAX_PAYLOAD_LEN: usize = 1024 * 1024;

/// Length of the two timestamps at the start of a payload.
const TIMESTAMPS_LEN: usize = 2 * 8;

/// Random identifier of an item, unique within its vault. Not secret.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId([u8; ITEM_ID_LEN]);

impl ItemId {
    /// Generates a new random identifier from the OS CSPRNG.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::Random`] if the OS random number generator fails.
    pub fn generate() -> Result<Self> {
        let mut bytes = [0u8; ITEM_ID_LEN];
        getrandom::fill(&mut bytes).map_err(|_| CryptoError::Random)?;
        Ok(Self(bytes))
    }

    /// Builds an identifier from its bytes.
    pub const fn from_bytes(bytes: [u8; ITEM_ID_LEN]) -> Self {
        Self(bytes)
    }

    /// The identifier's bytes.
    pub const fn as_bytes(&self) -> &[u8; ITEM_ID_LEN] {
        &self.0
    }
}

impl core::fmt::Display for ItemId {
    /// Formats the identifier as 32 lower-case hexadecimal digits.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl core::fmt::Debug for ItemId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ItemId({self})")
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

/// A decrypted item: identifier, timestamps and content.
pub struct Item {
    id: ItemId,
    created_at_ms: u64,
    updated_at_ms: u64,
    content: ItemContent,
}

impl Item {
    /// Assembles an item. Crate-internal: the vault assigns identifiers and
    /// timestamps.
    pub(crate) const fn new(
        id: ItemId,
        created_at_ms: u64,
        updated_at_ms: u64,
        content: ItemContent,
    ) -> Self {
        Self {
            id,
            created_at_ms,
            updated_at_ms,
            content,
        }
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

/// Encodes an item's payload (timestamps and fields), padded to
/// [`PAD_BLOCK`].
///
/// # Errors
///
/// Returns [`VaultError::InvalidItem`] if the padded payload would exceed
/// [`MAX_PAYLOAD_LEN`].
pub(crate) fn encode_payload(
    created_at_ms: u64,
    updated_at_ms: u64,
    content: &ItemContent,
) -> VaultResult<Zeroizing<Vec<u8>>> {
    const TOO_LARGE: VaultError = VaultError::InvalidItem {
        reason: "item is too large",
    };
    let fields = fields(content);
    let mut len = TIMESTAMPS_LEN;
    for field in &fields {
        if u32::try_from(field.len()).is_err() {
            return Err(TOO_LARGE);
        }
        len = len.checked_add(4 + field.len()).ok_or(TOO_LARGE)?;
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
    payload.resize(padded_len, 0);
    Ok(payload)
}

/// Decodes a payload produced by [`encode_payload`] for an item of `kind`.
///
/// Returns `(created_at_ms, updated_at_ms, content)`.
///
/// # Errors
///
/// Returns [`VaultError::Corrupted`] if the payload is truncated, has a
/// field out of bounds or not valid UTF-8, or has invalid padding.
pub(crate) fn decode_payload(
    kind: ItemKind,
    payload: &[u8],
) -> VaultResult<(u64, u64, ItemContent)> {
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
    reader.padding(payload.len())?;
    Ok((created_at_ms, updated_at_ms, content))
}

/// Bounds-checked cursor over a payload.
struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    const MALFORMED: VaultError = VaultError::Corrupted {
        reason: "malformed record payload",
    };

    fn take(&mut self, len: usize) -> VaultResult<&[u8]> {
        if len > self.0.len() {
            return Err(Self::MALFORMED);
        }
        let (head, rest) = self.0.split_at(len);
        self.0 = rest;
        Ok(head)
    }

    fn u64(&mut self) -> VaultResult<u64> {
        let bytes = self.take(8)?;
        let bytes: [u8; 8] = bytes.try_into().map_err(|_| Self::MALFORMED)?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn str(&mut self) -> VaultResult<&str> {
        let len = self.take(4)?;
        let len: [u8; 4] = len.try_into().map_err(|_| Self::MALFORMED)?;
        let len = usize::try_from(u32::from_le_bytes(len)).map_err(|_| Self::MALFORMED)?;
        // Validating in place, then copying once into an exactly sized
        // allocation, leaves no stray plaintext copy behind on error.
        core::str::from_utf8(self.take(len)?).map_err(|_| Self::MALFORMED)
    }

    fn string(&mut self) -> VaultResult<String> {
        self.str().map(str::to_owned)
    }

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

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// Payload of a note titled "t" with body "b", created at 1 ms and
    /// updated at 2 ms, as published in docs/vault-format.md.
    #[test]
    fn known_note_encoding() {
        let payload = encode_payload(1, 2, &note_item()).unwrap();
        let expected = concat!(
            "0100000000000000", // created_at_ms = 1
            "0200000000000000", // updated_at_ms = 2
            "0100000074",       // title "t"
            "0100000062",       // body "b"
        );
        let mut expected = expected.to_owned();
        expected.push_str(&"00".repeat(PAD_BLOCK - 26));
        assert_eq!(hex(&payload), expected);
    }

    fn assert_same(a: &ItemContent, b: &ItemContent) {
        assert_eq!(a.kind(), b.kind());
        assert_eq!(fields(a), fields(b));
    }

    #[test]
    fn round_trips_every_kind() {
        for content in [password_item(), note_item(), secret_item()] {
            let payload = encode_payload(1_700_000_000_000, u64::MAX, &content).unwrap();
            assert_eq!(payload.len() % PAD_BLOCK, 0);
            let (created, updated, decoded) = decode_payload(content.kind(), &payload).unwrap();
            assert_eq!(created, 1_700_000_000_000);
            assert_eq!(updated, u64::MAX);
            assert_same(&decoded, &content);
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
        assert_eq!(
            encode_payload(0, 0, &short).unwrap().len(),
            encode_payload(0, 0, &longer).unwrap().len()
        );
    }

    #[test]
    fn exact_block_needs_no_padding() {
        // 16 (timestamps) + 2 × 4 (lengths) + 40 = 64 bytes exactly.
        let content = ItemContent::Note(NoteItem {
            title: String::new(),
            body: s(&"x".repeat(40)),
        });
        let payload = encode_payload(0, 0, &content).unwrap();
        assert_eq!(payload.len(), PAD_BLOCK);
        assert!(decode_payload(ItemKind::Note, &payload).is_ok());
    }

    #[test]
    fn rejects_oversized_items() {
        let content = ItemContent::Note(NoteItem {
            title: String::new(),
            body: s(&"x".repeat(MAX_PAYLOAD_LEN)),
        });
        assert!(matches!(
            encode_payload(0, 0, &content),
            Err(VaultError::InvalidItem { .. })
        ));
    }

    fn assert_malformed(kind: ItemKind, payload: &[u8]) {
        assert!(matches!(
            decode_payload(kind, payload),
            Err(VaultError::Corrupted { .. })
        ));
    }

    #[test]
    fn rejects_malformed_payloads() {
        let payload = encode_payload(1, 2, &note_item()).unwrap();

        // Truncated anywhere.
        for len in 0..payload.len() {
            assert_malformed(ItemKind::Note, &payload[..len]);
        }
        // One extra padding block, or non-zero padding.
        let mut extra = payload.to_vec();
        extra.extend_from_slice(&[0u8; PAD_BLOCK]);
        assert_malformed(ItemKind::Note, &extra);
        let mut dirty = payload.to_vec();
        dirty[PAD_BLOCK - 1] = 1;
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
        let id = ItemId::from_bytes([0xAB; ITEM_ID_LEN]);
        assert_eq!(id.to_string(), "ab".repeat(ITEM_ID_LEN));
    }

    #[test]
    fn debug_shows_no_field_content() {
        let item = Item::new(ItemId::from_bytes([0; ITEM_ID_LEN]), 0, 0, password_item());
        let printed = format!("{item:?} {:?}", item.content());
        for secret in ["Bank", "alice", "correct horse", "bank.example", "PIN"] {
            assert!(!printed.contains(secret), "{printed}");
        }
        let summary = ItemSummary::from(item);
        assert_eq!(summary.title, "Bank");
        assert!(!format!("{summary:?}").contains("Bank"));
    }
}
