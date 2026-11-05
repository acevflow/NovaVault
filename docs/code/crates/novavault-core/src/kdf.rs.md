# [`crates/novavault-core/src/kdf.rs`](../../../../../crates/novavault-core/src/kdf.rs)

## Purpose

Derives a 256-bit wrapping key from a low-entropy secret (the master password or the recovery code) with Argon2id v0x13 (RFC 9106). The parameters are stored per key slot in the vault file, so when a file is read they are untrusted input. `KdfParams` therefore enforces a floor, so a tampered file cannot downgrade protection, and a ceiling, so a crafted file cannot exhaust memory or time.

## Public API

| Name | Kind | Signature | Description |
| ---- | ---- | --------- | ----------- |
| `KdfParams` | struct | `pub struct KdfParams { memory_kib: u32, iterations: u32, parallelism: u32 }` | Validated Argon2id cost parameters. `Debug, Clone, Copy, PartialEq, Eq`. |
| `KdfParams::DEFAULT_MEMORY_KIB` | constant | `pub const DEFAULT_MEMORY_KIB: u32 = 64 * 1024;` | Default memory cost, 64 MiB. |
| `KdfParams::DEFAULT_ITERATIONS` | constant | `pub const DEFAULT_ITERATIONS: u32 = 3;` | Default time cost. |
| `KdfParams::DEFAULT_PARALLELISM` | constant | `pub const DEFAULT_PARALLELISM: u32 = 4;` | Default lanes. |
| `KdfParams::MIN_MEMORY_KIB` | constant | `pub const MIN_MEMORY_KIB: u32 = 19 * 1024;` | Memory floor, 19 MiB (OWASP minimum). |
| `KdfParams::MAX_MEMORY_KIB` | constant | `pub const MAX_MEMORY_KIB: u32 = 4 * 1024 * 1024;` | Memory ceiling, 4 GiB. |
| `KdfParams::MIN_ITERATIONS` | constant | `pub const MIN_ITERATIONS: u32 = 2;` | Time cost floor. |
| `KdfParams::MAX_ITERATIONS` | constant | `pub const MAX_ITERATIONS: u32 = 64;` | Time cost ceiling. |
| `KdfParams::MIN_PARALLELISM` | constant | `pub const MIN_PARALLELISM: u32 = 1;` | Parallelism floor. |
| `KdfParams::MAX_PARALLELISM` | constant | `pub const MAX_PARALLELISM: u32 = 16;` | Parallelism ceiling. |
| `KdfParams::new` | function | `pub const fn new(memory_kib: u32, iterations: u32, parallelism: u32) -> Result<Self>` | Validated constructor. |
| `KdfParams::validate` | function | `pub const fn validate(&self) -> Result<()>` | Checks the bounds. |
| `KdfParams::memory_kib` | function | `pub const fn memory_kib(&self) -> u32` | Memory cost in KiB. |
| `KdfParams::iterations` | function | `pub const fn iterations(&self) -> u32` | Number of passes. |
| `KdfParams::parallelism` | function | `pub const fn parallelism(&self) -> u32` | Number of lanes. |
| `impl Default for KdfParams` | trait impl | `fn default() -> Self` | The recommended parameters for new slots (64 MiB, 3, 4). |
| `derive_key` | function | `pub fn derive_key(password: &SecretSlice<u8>, salt: &Salt, params: &KdfParams) -> Result<SecretKey>` | Argon2id into zeroizing storage. |

`KdfParams` and `derive_key` are re-exported from the crate root.

## Details

### `KdfParams`

The fields are private, so a value can only come from `new` (validated) or `Default` (valid by construction). Inside the crate's own tests a helper builds unchecked values to run the RFC vectors.

- **Edge cases and invariants**: every `KdfParams` built outside this module is within the bounds. `derive_key` checks them again anyway.

### Constants

| Parameter | Default | Minimum | Maximum |
| --------- | ------- | ------- | ------- |
| Memory (KiB) | 65 536 (64 MiB) | 19 456 (19 MiB) | 4 194 304 (4 GiB) |
| Iterations | 3 | 2 | 64 |
| Parallelism | 4 | 1 | 16 |

The defaults are the second recommended option of RFC 9106 §4. The floor is the OWASP minimum for Argon2id (19 MiB, 2 passes). The ceilings bound allocation and unlock time when reading an untrusted file.

### `KdfParams::new`

Builds the struct and returns it if `validate` passes.

- **Parameters**: memory cost in KiB, time cost (passes) and parallelism (lanes).
- **Returns**: the validated parameters.
- **Errors**:
  - `CryptoError::InvalidKdfParams`: a value is outside its bound (see `validate`).
- **Security**: this is the function [`slots`](slots.rs.md) calls on every value read from a vault file. It is what stops a tampered file from weakening or inflating key derivation.

### `KdfParams::validate`

Checks the bounds in a fixed order and reports the first one violated:

1. memory below minimum, then memory above maximum;
2. time cost below minimum, then time cost above maximum;
3. parallelism below minimum, then parallelism above maximum.

- **Errors**:
  - `CryptoError::InvalidKdfParams { reason }`, where `reason` is one of `"memory cost below minimum"`, `"memory cost above maximum"`, `"time cost below minimum"`, `"time cost above maximum"`, `"parallelism below minimum"` or `"parallelism above maximum"`.

### `KdfParams::memory_kib`, `iterations`, `parallelism`

`const` getters. [`slots`](slots.rs.md) uses them to serialize the parameters into the slot metadata.

### `Default for KdfParams`

Returns `DEFAULT_MEMORY_KIB`, `DEFAULT_ITERATIONS` and `DEFAULT_PARALLELISM`. The shell passes `KdfParams::default()` for new vaults and for rewritten slots (see [`src-tauri/src/main.rs`](../../../../../src-tauri/src/main.rs)).

### `derive_key`

1. Calls `params.validate()`, so unchecked values can never reach Argon2.
2. Allocates a zeroed `SecretBox<[u8; 32]>`.
3. Runs Argon2id v0x13 over `password` and `salt` with an empty Argon2 secret (`K`) and empty associated data (`X`), writing the 32-byte output directly into the box.
4. Wraps the box in a `SecretKey` with `SecretKey::from_secret_box`.

- **Parameters**:
  - `password`: the secret bytes, used byte for byte.
  - `salt`: the slot's salt.
  - `params`: the cost parameters.
- **Returns**: the derived key. The same inputs always give the same key.
- **Errors**:
  - `CryptoError::InvalidKdfParams`: `params` fail `validate`.
  - `CryptoError::KeyDerivation`: Argon2 fails, for example because it cannot allocate the memory, or rejects the parameters itself.
- **Security**:
  - The output never exists outside zeroizing storage. Argon2's working memory is wiped too, through the `zeroize` feature of `argon2`.
  - `password` is not normalized or trimmed. The UI must pass exactly the bytes the user will type again. [`vault-format.md` §6.3](../../../../vault-format.md#63-wrapping-and-unwrapping) explains the consequences.
  - The call is deliberately slow (well under a second with the defaults). The shell runs it on a blocking worker thread.
- **Edge cases and invariants**: an empty password is accepted here. Refusing it is an application check in the shell.

### `argon2id` (private)

Wires `KdfParams` into `argon2::ParamsBuilder` (`m_cost`, `t_cost`, `p_cost`, `output_len`). It adds associated data only when `ad` is non-empty, and uses `Argon2::new_with_secret` only when `secret` is non-empty. Production always passes both empty. The test vectors pass both non-empty, so the RFC 9106 vector runs through exactly the same wiring. Every `argon2` error maps to `CryptoError::KeyDerivation`.

## Dependencies

- **Internal**:
  - [`error`](error.rs.md): `CryptoError` and `Result`.
  - [`keys`](keys.rs.md): `KEY_LEN`, `Salt`, `SecretKey`.
- **External**:
  - `argon2` (features `alloc`, `zeroize`; defaults off): the Argon2id implementation, called through `hash_password_into` for raw output.
  - `secrecy`: `SecretSlice` (input) and `SecretBox` (output).

## Used by

- [`slots`](slots.rs.md): `derive_key` for every wrap and unwrap; `KdfParams::new` when parsing a slot.
- [`vault`](vault.rs.md): takes `KdfParams` for `create`, `change_password` and `regenerate_recovery_code`.
- [`lib.rs`](lib.rs.md): re-exports `KdfParams` and `derive_key`.
- [`src-tauri/src/main.rs`](../../../../../src-tauri/src/main.rs) and [`src-tauri/src/session.rs`](../../../../../src-tauri/src/session.rs): pass `KdfParams` to the session and the vault.

## Tests

- [`rfc9106_argon2id_test_vector`](../../../../../crates/novavault-core/src/kdf.rs#L226): RFC 9106 §5.3 (32 KiB, 3 passes, 4 lanes, with secret and associated data) through the private wiring.
- [`reference_implementation_test_vector`](../../../../../crates/novavault-core/src/kdf.rs#L245): phc-winner-argon2 `password`/`somesalt` vector (t=2, m=2^16 KiB, p=1).
- [`derive_key_is_deterministic_and_input_sensitive`](../../../../../crates/novavault-core/src/kdf.rs#L264): same inputs give the same key; a different salt or a one-character password change gives a different key.
- [`default_params_are_valid_rfc9106_option`](../../../../../crates/novavault-core/src/kdf.rs#L281): `Default` is valid and equals `(65 536, 3, 4)`.
- [`out_of_bounds_params_are_rejected`](../../../../../crates/novavault-core/src/kdf.rs#L295): one step outside each of the six bounds is rejected by `new`.
- [`derive_key_rechecks_params`](../../../../../crates/novavault-core/src/kdf.rs#L316): `derive_key` rejects unchecked parameters below the floor.

## Notes

- `MAX_MEMORY_KIB` (4 GiB) bounds what a crafted file can request, but a machine with less free memory still tries the allocation. Argon2 then fails with `KeyDerivation`, or the OS may kill the process before Argon2 can return an error. This is not handled specially.
- The parameter name `password` is also used for the recovery code's canonical bytes; [`slots`](slots.rs.md) passes either.
