# [`src-tauri/src/dto.rs`](../../../../src-tauri/src/dto.rs#L543) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `dto.rs` checks the IPC wire format: that UI inputs deserialize into the right core types (including secrets into `SecretString`), that malformed inputs are rejected, that outputs serialize with the expected camelCase keys, and that item and attachment identifiers parse strictly.

## Public API

| Name                                                  | Kind | Signature                                                    | Description                                                                  |
| ----------------------------------------------------- | ---- | ------------------------------------------------------------ | ---------------------------------------------------------------------------- |
| `item_input_deserializes_every_kind`                  | test | `fn item_input_deserializes_every_kind()`                    | `password`, `note` and `secret` inputs become the matching `ItemContent`.    |
| `item_input_rejects_missing_fields_and_unknown_kinds` | test | `fn item_input_rejects_missing_fields_and_unknown_kinds()`   | A note without `body` and an unknown `kind` are rejected.                    |
| `item_update_treats_null_or_missing_secrets_as_unchanged` | test | `fn item_update_treats_null_or_missing_secrets_as_unchanged()` | `null` or absent secret fields deserialize to `None`.                  |
| `item_fields_deserialize_from_camel_case`             | test | `fn item_fields_deserialize_from_camel_case()`               | `ItemField` names, the secret flag, and rejection of `title`.                |
| `credential_deserializes_both_methods`                | test | `fn credential_deserializes_both_methods()`                  | `password` and `recoveryCode` credentials.                                   |
| `recovery_code_is_serialized_in_display_form`         | test | `fn recovery_code_is_serialized_in_display_form()`           | `RecoveryCodeDto` carries the dashed display form under `recoveryCode`.      |
| `item_ids_round_trip_through_hex`                     | test | `fn item_ids_round_trip_through_hex()`                       | `parse_item_id` and `parse_attachment_id` accept 32 hex digits only.         |
| `transfer_progress_serializes_both_counts`            | test | `fn transfer_progress_serializes_both_counts()`              | `TransferProgressDto` is `{ "done", "total" }`.                              |

## Details

### [`item_input_deserializes_every_kind`](../../../../src-tauri/src/dto.rs#L548)

1. A `password` input (`title` `Bank`, `username` `alice`, `password` `hunter2`, `url` `https://bank.example`, empty `notes`) converts with `ItemContent::from` into `ItemContent::Password` whose username is `alice` and whose exposed password is `hunter2`.
2. A `note` input (`title`, `body`) converts into an item of kind `ItemKind::Note`.
3. A `secret` input (`title`, `value`, `notes`) converts into an item of kind `ItemKind::Secret`.

### [`item_input_rejects_missing_fields_and_unknown_kinds`](../../../../src-tauri/src/dto.rs#L571)

`{"kind":"note","title":"t"}` (no `body`) and `{"kind":"file","title":"t"}` (no such kind) both fail to deserialize, so the UI cannot create a half-filled item or an item kind the core does not know.

### [`item_update_treats_null_or_missing_secrets_as_unchanged`](../../../../src-tauri/src/dto.rs#L577)

- A password update with `"password": null` and no `notes` gives `ItemUpdate::Password { password: None, notes: None, .. }`.
- A secret update with `"value": "v"` and no `notes` gives `ItemUpdate::Secret { value: Some(v), notes: None, .. }` with `v` exposing `v`.

`None` means "keep the stored value", which lets the UI edit an item without first revealing its secrets.

### [`item_fields_deserialize_from_camel_case`](../../../../src-tauri/src/dto.rs#L599)

`"username"` gives `ItemField::Username`, which is not secret; `"value"` gives a secret field; `"title"` is rejected because the title is not a field the UI can reveal or copy.

### [`credential_deserializes_both_methods`](../../../../src-tauri/src/dto.rs#L609)

`{"method":"password","secret":"pw"}` gives `Credential::Password` and `{"method":"recoveryCode","secret":"abcd"}` gives `Credential::RecoveryCode`, each exposing the given secret. The recovery code is not parsed here; that happens in the session.

### [`recovery_code_is_serialized_in_display_form`](../../../../src-tauri/src/dto.rs#L619)

Generates a real code with `RecoveryCode::generate`, serializes `RecoveryCodeDto::from(&code)`, and checks that `json["recoveryCode"]` equals `code.to_display()`.

### [`item_ids_round_trip_through_hex`](../../../../src-tauri/src/dto.rs#L629)

1. A generated `ItemId` round-trips through `to_string` and `parse_item_id`, in lowercase and uppercase.
2. These are rejected with `CommandError::InvalidInput`: the empty string, `abc`, 32 × `g`, 33 × `a`, and `+f` repeated 16 times (32 bytes with sign characters).
3. `é` repeated 16 times (32 bytes of UTF-8, 16 characters) is rejected too.
4. A generated `AttachmentId` round-trips through `parse_attachment_id`, and `x` is rejected.

### [`transfer_progress_serializes_both_counts`](../../../../src-tauri/src/dto.rs#L656)

`TransferProgressDto { done: 1, total: 2 }` serializes to exactly `{ "done": 1, "total": 2 }`.

## Dependencies

- **Internal**: [`dto`](dto.rs.md) (`super::*`): `ItemInput`, `ItemUpdate`, `ItemField`, `Credential`, `RecoveryCodeDto`, `TransferProgressDto`, `parse_item_id`, `parse_attachment_id`; [`error`](error.rs.md): `CommandError`.
- **External**: `serde_json` (test inputs and outputs); `novavault-core` (`ItemContent`, `ItemKind`, `ItemId`, `AttachmentId`, `RecoveryCode`); `secrecy::ExposeSecret`.

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: [`src-tauri/src/dto.rs`](../../../../src-tauri/src/dto.rs), documented in [`dto.rs.md`](dto.rs.md).

How to run:

```sh
cargo test -p novavault dto::tests
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run these too.

## Notes

- That output DTOs never carry secret fields is tested in [`session.rs.tests.md`](session.rs.tests.md) (`listing_never_serializes_secret_fields`, `item_details_have_plain_fields_and_flags_only`), not here.
- The `ItemUpdate::Note` variant and the `ItemUpdate` rejection of unknown fields or kinds are not tested in this module.
