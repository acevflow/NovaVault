# [`crates/novavault-core/src/vault.rs`](../../../../../crates/novavault-core/src/vault.rs)

## Purpose

Defines `Vault`, the unlocked vault and the core crate's main entry point. It ties the format pieces together: it creates and opens vault files following the reader rules of the specification, keeps items encrypted in memory and decrypts them on demand, saves atomically, manages attachment blobs, migrates older format versions and rotates credentials. The vault key lives inside the `Vault` and is wiped when it is dropped. Dropping the `Vault` is how a vault is locked.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `ATTACHMENTS_DIR_SUFFIX` | constant | `pub const ATTACHMENTS_DIR_SUFFIX: &str = ".attachments";` | Suffix of the blob directory name. |
| `Vault` | struct | `pub struct Vault { path: PathBuf, header: Header, slots: KeySlots, key: SecretKey, records: Vec<RecordEnvelope>, pending_deletions: Vec<AttachmentId> }` | An unlocked vault. |
| `Vault::create` | function | `pub fn create(path: impl AsRef<Path>, password: &SecretSlice<u8>, params: KdfParams) -> VaultResult<(Self, RecoveryCode)>` | Creates and saves a new vault. |
| `Vault::open_with_password` | function | `pub fn open_with_password(path: impl AsRef<Path>, password: &SecretSlice<u8>) -> VaultResult<Self>` | Opens with the master password. |
| `Vault::open_with_recovery_code` | function | `pub fn open_with_recovery_code(path: impl AsRef<Path>, code: &RecoveryCode) -> VaultResult<Self>` | Opens with the recovery code. |
| `Vault::path` | function | `pub fn path(&self) -> &Path` | Where the vault is saved. |
| `Vault::attachments_dir` | function | `pub fn attachments_dir(&self) -> PathBuf` | The blob directory. |
| `Vault::len` | function | `pub fn len(&self) -> usize` | Number of items. |
| `Vault::is_empty` | function | `pub fn is_empty(&self) -> bool` | Whether there are no items. |
| `Vault::list` | function | `pub fn list(&self) -> VaultResult<Vec<ItemSummary>>` | Summaries of every item. |
| `Vault::get` | function | `pub fn get(&self, id: ItemId) -> VaultResult<Item>` | Decrypts one item. |
| `Vault::insert` | function | `pub fn insert(&mut self, content: ItemContent) -> VaultResult<ItemId>` | Adds an item. |
| `Vault::update` | function | `pub fn update(&mut self, id: ItemId, content: ItemContent) -> VaultResult<()>` | Replaces an item's content. |
| `Vault::remove` | function | `pub fn remove(&mut self, id: ItemId) -> VaultResult<()>` | Removes an item. |
| `Vault::attachment` | function | `pub fn attachment(&self, item: ItemId, attachment: AttachmentId) -> VaultResult<Attachment>` | Metadata of one attachment. |
| `Vault::add_attachment` | function | `pub fn add_attachment(&mut self, item: ItemId, name: &str, source: &mut impl Read, progress: impl FnMut(u64) -> ControlFlow<()>) -> VaultResult<Attachment>` | Encrypts a stream as a new attachment. |
| `Vault::export_attachment` | function | `pub fn export_attachment(&self, item: ItemId, attachment: AttachmentId, destination: &Path, progress: impl FnMut(u64) -> ControlFlow<()>) -> VaultResult<()>` | Decrypts an attachment to a file. |
| `Vault::remove_attachment` | function | `pub fn remove_attachment(&mut self, item: ItemId, attachment: AttachmentId) -> VaultResult<()>` | Removes an attachment. |
| `Vault::save` | function | `pub fn save(&mut self) -> VaultResult<()>` | Writes the file atomically, then deletes removed blobs. |
| `Vault::change_password` | function | `pub fn change_password(&mut self, new_password: &SecretSlice<u8>, params: KdfParams) -> VaultResult<()>` | Rewrites the password slot and saves. |
| `Vault::regenerate_recovery_code` | function | `pub fn regenerate_recovery_code(&mut self, params: KdfParams) -> VaultResult<RecoveryCode>` | Rewrites the recovery slot and saves. |
| `Vault::verify_password` | function | `pub fn verify_password(&self, password: &SecretSlice<u8>) -> VaultResult<()>` | Re-authenticates with the password. |
| `Vault::verify_recovery_code` | function | `pub fn verify_recovery_code(&self, code: &RecoveryCode) -> VaultResult<()>` | Re-authenticates with the recovery code. |
| `impl Debug for Vault` | trait impl | `fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result` | Prints the path and the item count only. |

`Vault` and `ATTACHMENTS_DIR_SUFFIX` are re-exported from the crate root.

## Details

### `ATTACHMENTS_DIR_SUFFIX`

`vault.nvault` keeps its blobs in `vault.nvault.attachments/`, next to it. Both must be backed up together.

### `Vault`

- **Security**:
  - **Key material**: `key` is the vault key, a `SecretKey` that is wiped on drop and never exposed. No method returns it.
  - **Encryption at rest in memory**: `records` hold ciphertext. An item is decrypted only inside the method that needs it, and the decrypted `Item` is returned to (or dropped by) that caller.
  - Dropping the `Vault` locks it and loses unsaved edits.
  - `Debug` prints the path and the item count.
- **Edge cases and invariants**:
  - `records` is always sorted by identifier with no duplicates, because inserts use a binary search.
  - `pending_deletions` lists blobs that the saved file may still refer to, and that are deleted after the next successful save.

### `Vault::create`

1. Refuses the path if `fs::symlink_metadata` finds anything there, which also catches dangling symbolic links.
2. Generates a header, then `KeySlots::create` with `header.slot_ad()`. This yields the slots, the vault key and the recovery code.
3. Builds an empty vault and saves it.

- **Parameters**:
  - `path`: where to create the vault.
  - `password`: the master password.
  - `params`: Argon2id cost for both slots.
- **Returns**: the unlocked vault and the recovery code, which must be shown once and is not stored.
- **Errors**:
  - `VaultError::AlreadyExists`: something exists at `path`. An existing vault is never overwritten.
  - `VaultError::Io`: the file cannot be written.
  - `VaultError::Crypto`: key derivation or randomness failed.
- **Edge cases and invariants**: the existence check and the later write are not one atomic step. A file created at `path` between the two is replaced by the rename (see Notes).

### `Vault::open_with_password` and `Vault::open_with_recovery_code`

Both run the private `open` sequence with an unlock closure for the matching slot:

1. `read_bounded`: reads at most `MAX_FILE_LEN + 1` bytes.
2. `format::parse`: checks the structure, so a foreign, newer or damaged file is reported before any Argon2id work.
3. Unwraps the vault key with `header.slot_ad()`, mapping `CryptoError::Decryption` to `InvalidCredentials`.
4. `format::verify_seal`: until the seal verifies, deleted, duplicated or replayed records go undetected, so nothing is decrypted before this step.
5. If the header version is below `FORMAT_VERSION`, runs `migrate`.
6. Decrypts every record once to collect attachment identifiers, rejecting duplicates.
7. Prunes blobs and temporary files that no item refers to.

- **Returns**: the unlocked vault.
- **Errors**:
  - `VaultError::Io`: reading the file (or, while migrating, writing) failed.
  - `VaultError::NotAVault`, `UnsupportedVersion`, `UnsupportedAlgorithm` or `Corrupted`: the structure is invalid. These are reported before key derivation.
  - `VaultError::InvalidCredentials`: wrong secret or damaged slot.
  - `VaultError::Corrupted`: the seal fails, a record fails to decrypt or decode, or two attachments share an identifier.
  - `VaultError::Crypto`: any other unwrap failure (for example `KeyDerivation`).
- **Security**:
  - **Format versioning**: the slots do not authenticate the version, but the seal and every record do, so a rewritten version field fails at step 4 or 6.
  - A wrong secret never triggers a migration, because migration runs only after a successful unwrap and seal check.

### `Vault::path`, `Vault::len`, `Vault::is_empty`

Simple getters.

### `Vault::attachments_dir`

The vault path with `ATTACHMENTS_DIR_SUFFIX` appended to its file name. The directory is created (mode `0700` on Unix) when the first file is attached.

### `Vault::list`

Decrypts every record and converts it to an `ItemSummary`. Secret fields are dropped, and wiped, at once.

- **Errors**:
  - `VaultError::Corrupted`: a record fails to decrypt or decode.

### `Vault::get`

Binary-searches the record and decrypts it.

- **Errors**:
  - `VaultError::ItemNotFound`: no item has that identifier.
  - `VaultError::Corrupted`: the record fails to decrypt or decode.

### `Vault::insert`

Rejects the insert if the vault already holds `MAX_RECORDS` items. Otherwise it draws a random `ItemId`, retrying on the practically impossible collision, sets both timestamps to now, encrypts with no attachments, and inserts the record at its sorted position.

- **Errors**:
  - `VaultError::InvalidItem`: the vault has too many items, or the item is too large.
  - `VaultError::Crypto`: randomness or encryption failed.
- **Security**: every write of a record uses a fresh nonce (through `aead::encrypt`).

### `Vault::update`

Decrypts the existing item, keeps its identifier, creation time and attachments, sets the update time to `max(now, previous update time)` so it never moves backwards, and re-encrypts with the new content. The content may be of another `ItemKind`.

- **Errors**:
  - `VaultError::ItemNotFound`.
  - `VaultError::Corrupted`: the existing record cannot be read.
  - `VaultError::InvalidItem`: the item is too large.

### `Vault::remove`

Decrypts the item to collect its attachment identifiers, adds them to `pending_deletions`, and removes the record. The blobs are deleted by the next successful `save`.

- **Errors**:
  - `VaultError::ItemNotFound`.
  - `VaultError::Corrupted`: the record cannot be read. An unreadable item therefore cannot be removed.

### `Vault::attachment`

Decrypts the item and returns a clone of the matching `Attachment`.

- **Errors**:
  - `VaultError::ItemNotFound`, `VaultError::AttachmentNotFound` or `VaultError::Corrupted`.

### `Vault::add_attachment`

1. Validates `name` (`validate_attachment_name`) and finds the item.
2. Rejects the item if it already has `MAX_ATTACHMENTS_PER_ITEM` attachments.
3. Creates the attachments directory (owner-only on Unix).
4. Draws an `AttachmentId` whose blob path does not exist yet, so a blob is never replaced.
5. Streams `source` through `attachment::encrypt_stream` into an `AtomicFile` at the blob path, then commits.
6. Appends an `Attachment` (name, measured size, `added_at_ms` = now) and re-encrypts the item with the update time set to `max(now, previous)`.
7. If re-encryption fails, deletes the new blob.

- **Parameters**:
  - `item`: the owning item.
  - `name`: the file name shown to the user.
  - `source`: the plaintext.
  - `progress`: called after each chunk with the bytes encrypted so far.
- **Returns**: the new attachment's metadata.
- **Errors**:
  - `VaultError::ItemNotFound`.
  - `VaultError::InvalidItem`: invalid name, or too many attachments.
  - `VaultError::Io`: reading `source` or writing the blob failed.
  - `VaultError::Cancelled`: `progress` returned `Break`.
  - `VaultError::Crypto`.
  - `VaultError::Corrupted`: the item's record cannot be read.
- **Security**:
  - The blob is complete on disk before the item refers to it.
  - The item change stays in memory until the next `save`. A blob that is never saved is pruned on the next open.
  - On error or cancellation, the `AtomicFile` is dropped and its temporary file deleted.

### `Vault::export_attachment`

1. Looks up the attachment metadata.
2. Refuses a destination that `is_vault_storage` reports as the vault file or a path inside the attachments directory.
3. Opens the blob. A missing blob is `Corrupted { reason: "attachment data is missing" }`.
4. Streams it through `attachment::decrypt_stream` into an `AtomicFile` at `destination`, checking the recorded size, and commits only on success.

- **Errors**:
  - `VaultError::ItemNotFound` or `AttachmentNotFound`.
  - `VaultError::InvalidItem { reason: "cannot export over the vault's own files" }`.
  - `VaultError::Corrupted`: the blob is missing, damaged or does not match the record.
  - `VaultError::Io`: the blob cannot be opened for a reason other than "not found", or the destination cannot be written.
  - `VaultError::Cancelled`.
- **Security**:
  - The plaintext goes to a temporary file next to `destination`, which is renamed into place only after every chunk has authenticated and the size matches.
  - On any error, including cancellation, the temporary file is deleted, so a damaged blob never leaves partial plaintext at the destination path.
  - An existing file at `destination` is replaced.

### `Vault::remove_attachment`

Decrypts the item, removes the attachment from its list, re-encrypts with the update time set to `max(now, previous)`, and queues the blob in `pending_deletions`.

- **Errors**:
  - `VaultError::ItemNotFound`, `AttachmentNotFound` or `Corrupted`.

### `Vault::save`

1. `format::encode_body` builds everything before the seal.
2. `format::seal` computes the seal over it.
3. `write_atomic` writes the file.
4. Only after a successful write, deletes every blob in `pending_deletions`, ignoring failures; the next open prunes any leftover.

- **Errors**:
  - `VaultError::Io`: the write failed. The previous file is intact and no blob is deleted. One exception: on Unix, a failure to sync the directory after the rename is also reported as `Io`, although the new file is already in place.
  - `VaultError::InvalidItem`: format limits exceeded.
  - `VaultError::Crypto`: sealing failed.
- **Security**: the seal gets a fresh nonce on every save. Because a blob is deleted only after a save that no longer refers to it, a removal can never leave the saved file pointing at a missing blob.

### `Vault::change_password`

Clones the slot table, calls `KeySlots::change_password` with the in-memory vault key and `header.slot_ad()`, then installs the new table and saves through `replace_slots_and_save`. Only the password slot changes, with a fresh salt and nonce and the given `params`. The vault key, every record and the recovery code stay the same. The vault may have been unlocked with the recovery code; that is the forgotten-password flow. Any unsaved item edits are written too, because the whole file is saved.

- **Errors**:
  - `VaultError::Crypto`: key derivation or randomness failed.
  - Any error of `save`. On error, the previous slot table is restored in memory, so it still matches the file on disk.

### `Vault::regenerate_recovery_code`

Like `change_password`, for the recovery slot.

- **Returns**: the new code, which must be shown once.
- **Errors**: the same as `change_password`. On error, the previous code still opens the vault.
- **Security**: the previous code stops opening the current file as soon as this returns. An older copy of the file still opens with it (see [threat-model.md](../../../../threat-model.md#cryptography-and-key-management)).

### `Vault::verify_password` and `Vault::verify_recovery_code`

Unwrap the matching in-memory slot, which costs one Argon2id derivation, and drop the result at once. A successful unwrap is the proof; the key itself is already held. The shell uses these to re-authenticate the user before sensitive changes, so an unlocked but unattended vault cannot be taken over. Nothing is changed or saved.

- **Errors**:
  - `VaultError::InvalidCredentials`: wrong secret.
  - `VaultError::Crypto`: key derivation failed.
- **Security**: the unwrapped copy of the vault key is a `SecretKey`, wiped as soon as it is dropped.

### `migrate` (private)

1. Decrypts every record with the old header and decodes its old payload.
2. Re-encodes it for `FORMAT_VERSION` and re-encrypts it under the new header, with a fresh nonce.
3. Writes a copy of the original file as `<file name>.v<old version>.bak` with `write_atomic`.
4. Installs the new header and records, and saves. If the save fails, it deletes the backup (the original file is intact) and returns the error.
5. Reads the new file back and checks its structure, its seal, and that its header and records are identical to what was written. If anything differs, it keeps the backup and returns `Corrupted { reason: "migrated vault did not read back" }`, or the parse or seal error.
6. Deletes the backup (best effort).

- **Security**:
  - **Format versioning**: the key slots are untouched, because their associated data leaves out the version. Both secrets therefore keep working, even though a migration holds only one of them.
  - The `Zeroizing` payload buffers are wiped after each record.

### `backup_path`, `referenced_attachments`, `prune_unreferenced_blobs`, `blob_path`, `is_vault_storage`, `search`, `position`, `encrypt`, `decrypt`, `decrypt_payload` (private)

- `prune_unreferenced_blobs` deletes regular files in the attachments directory that are either:
  - named like blobs (`<32 hex>.nvatt`, hex in either case) but not referenced by any item, or
  - named like interrupted blob writes (`.<blob name>.<16 hex>.tmp`).

  It skips names that are not valid UTF-8 and leaves every other file alone. Failures are ignored.
- `is_vault_storage` resolves the destination's directory with `fs::canonicalize`, then compares the result with the resolved vault path and checks whether it lies inside the canonical attachments directory.
- `encrypt` and `decrypt` encode or decode a payload for the vault's current header version and use `format::record_ad`. A decryption failure in memory is `Corrupted { reason: "record failed authentication" }`.

### Free functions (private)

- `read_bounded`: reads at most `MAX_FILE_LEN + 1` bytes.
- `create_private_dir`: recursive `create_dir`, mode `0700` on Unix.
- `parse_blob_name` and `is_blob_temp_name`: the name patterns above.
- `resolve`: canonical directory plus file name, or `None` if the directory does not exist.
- `now_ms`: milliseconds since the Unix epoch, or 0 if the clock is set before the epoch.

## Dependencies

- **Internal**:
  - [`aead`](aead.rs.md): record encryption.
  - [`atomic`](atomic.rs.md): `AtomicFile`, `write_atomic`.
  - [`attachment`](attachment.rs.md): blob streaming, `BlobHeader`, `BLOB_EXTENSION`.
  - [`error`](error.rs.md): errors.
  - [`format`](format.rs.md): parse, seal, `RecordEnvelope`, limits.
  - [`header`](header.rs.md): `Header`, `FORMAT_VERSION`.
  - [`item`](item.rs.md): the item model and payload encoding.
  - [`kdf`](kdf.rs.md): `KdfParams`.
  - [`keys`](keys.rs.md): `SecretKey`.
  - [`recovery`](recovery.rs.md): `RecoveryCode`.
  - [`slots`](slots.rs.md): `KeySlots`.
- **External**:
  - `secrecy`: `SecretSlice` for passwords, `ExposeSecret` for decrypted payloads.
  - Standard library for files and time.

## Used by

- [`lib.rs`](lib.rs.md): re-exports `Vault` and `ATTACHMENTS_DIR_SUFFIX`.
- [`src-tauri/src/session.rs`](../../../../../src-tauri/src/session.rs): the shell's `Session` owns the unlocked `Vault` and calls every public method on behalf of the Tauri commands.

## Tests

- [`round_trip_with_password_and_recovery_code`](../../../../../crates/novavault-core/src/vault.rs#L944): three items of each kind survive save and reopen with either secret.
- [`edits_persist_after_save`](../../../../../crates/novavault-core/src/vault.rs#L995): updates and removals reach disk only after `save`; the creation time is kept.
- [`unknown_items_are_reported`](../../../../../crates/novavault-core/src/vault.rs#L1021): `get`, `update` and `remove` of a missing identifier give `ItemNotFound`.
- [`create_never_overwrites`](../../../../../crates/novavault-core/src/vault.rs#L1037): an existing file gives `AlreadyExists` and is left unchanged.
- [`wrong_secrets_are_rejected_generically`](../../../../../crates/novavault-core/src/vault.rs#L1049): a wrong password or code gives `InvalidCredentials`.
- [`missing_file_is_an_io_error`](../../../../../crates/novavault-core/src/vault.rs#L1064): opening a missing file gives `Io`.
- [`unsupported_versions_and_foreign_files_are_rejected`](../../../../../crates/novavault-core/src/vault.rs#L1080): newer versions, foreign files, another algorithm and non-zero flags each give their own error.
- [`corruption_is_always_detected`](../../../../../crates/novavault-core/src/vault.rs#L1114): truncations and a flipped byte in every field of every section give `Corrupted` or `InvalidCredentials`.
- [`replaying_an_old_record_is_detected`](../../../../../crates/novavault-core/src/vault.rs#L1177): splicing an older valid record into a newer file gives `Corrupted`.
- [`verify_password_accepts_only_the_current_password`](../../../../../crates/novavault-core/src/vault.rs#L1200): `verify_password` follows password changes.
- [`verify_recovery_code_accepts_only_the_current_code`](../../../../../crates/novavault-core/src/vault.rs#L1219): `verify_recovery_code` follows regeneration.
- [`changing_the_password_keeps_data_and_recovery_code`](../../../../../crates/novavault-core/src/vault.rs#L1232): the old password fails, the new one and the code work, and unsaved edits are saved.
- [`regenerating_the_recovery_code_invalidates_the_old_one`](../../../../../crates/novavault-core/src/vault.rs#L1251): the old code fails, the new code and the password work.
- [`failed_save_keeps_previous_slots`](../../../../../crates/novavault-core/src/vault.rs#L1267): a failed save restores the previous slots, and the original secrets still work.
- [`debug_hides_contents`](../../../../../crates/novavault-core/src/vault.rs#L1286): `Debug` shows the item count and no content.
- [`version_zero_vaults_are_migrated_on_open`](../../../../../crates/novavault-core/src/vault.rs#L1341): a version 0 file is rewritten as version 1 with no backup left behind, items and timestamps survive, both secrets work, and attachments can be added.
- [`version_zero_vaults_open_with_the_recovery_code_too`](../../../../../crates/novavault-core/src/vault.rs#L1391): migration also works when unlocking with the code.
- [`a_wrong_secret_never_migrates`](../../../../../crates/novavault-core/src/vault.rs#L1403): a wrong password leaves the version 0 file byte-for-byte unchanged.
- [`rewriting_the_version_field_is_detected`](../../../../../crates/novavault-core/src/vault.rs#L1417): downgrading the version byte of a version 1 file gives `Corrupted`.
- [`attachments_round_trip_through_save_and_reopen`](../../../../../crates/novavault-core/src/vault.rs#L1464): empty and multi-chunk attachments round-trip, progress is reported per chunk, and blobs contain no plaintext.
- [`editing_an_item_keeps_its_attachments`](../../../../../crates/novavault-core/src/vault.rs#L1506): `update` keeps attachments.
- [`removed_attachments_are_deleted_only_after_a_save`](../../../../../crates/novavault-core/src/vault.rs#L1521): the blob survives until the save that drops its reference.
- [`removing_an_item_deletes_its_blobs_after_a_save`](../../../../../crates/novavault-core/src/vault.rs#L1546): removing an item deletes only its own blobs, after save.
- [`unsaved_and_interrupted_blobs_are_pruned_on_open`](../../../../../crates/novavault-core/src/vault.rs#L1561): unsaved blobs and temporary files are deleted on open, unrelated files are kept.
- [`cancelled_transfers_leave_nothing_behind`](../../../../../crates/novavault-core/src/vault.rs#L1582): a cancelled add leaves no blob; a cancelled export leaves no destination and no temporary file.
- [`damaged_blobs_never_produce_an_export`](../../../../../crates/novavault-core/src/vault.rs#L1612): flipped, truncated, swapped and missing blobs give `Corrupted` and leave the previous destination file unchanged.
- [`exports_never_overwrite_the_vault`](../../../../../crates/novavault-core/src/vault.rs#L1664): exporting onto the vault file, another spelling of its path, or a blob gives `InvalidItem`.
- [`attachment_inputs_are_validated`](../../../../../crates/novavault-core/src/vault.rs#L1691): invalid names, a missing item and a missing attachment are rejected, and nothing is written.

## Notes

- **Possible bug: the export guard can be bypassed by letter case.** `is_vault_storage` compares paths with `==` and `starts_with` after canonicalizing only the destination's *directory*. On case-insensitive file systems (the Windows and macOS defaults), a destination such as `TEST.NVAULT` next to `test.nvault` compares unequal, but `AtomicFile::commit` renames over the real vault file. A file name that differs only by case inside the attachments directory has the same problem. The doc comment claims that "another spelling of the same path is caught". The test covers only `./test.nvault`. Whether this can be reached depends on the shell's save dialog.
- **Migration failure leaves the file migrated.** If the read-back check in `migrate` fails, the new version 1 file is already in place and `open` returns an error. The backup is kept, as the specification requires, but the next open reads the migrated file, not the backup.
- **Race between check and create.** `create` checks for an existing file with `symlink_metadata`, then `save` renames a temporary file over the path. A file created between the two steps would be replaced. It is a narrow race on a path the shell chooses.
- **Concurrent instances can delete each other's blobs.** `prune_unreferenced_blobs` deletes any blob not referenced by the file just opened. A second process opening the same vault while another holds unsaved attachments would delete those blobs. Nothing in the core or the shell prevents two instances, for example through a single-instance plugin or a lock file. This is an open question.
- **Doc comment grammar.** The module documentation says blobs of removed attachments are deleted "only after the vault file no longer refers to them has been saved". The intended meaning is "after the vault file that no longer refers to them has been saved".
- **Empty passwords are accepted.** The core accepts an empty master password. Refusing it is a shell check, as [vault-format.md §6.3](../../../../vault-format.md#63-wrapping-and-unwrapping) states.
- **Unprotected neighbour files.** `export_attachment` refuses only the vault file and the attachments directory. It does not refuse the migration backup or other files next to the vault, such as the shell's `settings.json`.
