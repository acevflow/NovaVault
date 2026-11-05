# [`crates/novavault-core/Cargo.toml`](../../../../crates/novavault-core/Cargo.toml)

## Purpose

The Cargo manifest of `novavault-core`, the UI-independent library that holds all cryptography, the vault format and the key slots. It declares the crate's six direct dependencies and inherits its package metadata and lint levels from the workspace manifest at the repository root ([`Cargo.toml`](../../../../Cargo.toml)). It is the only configuration file in the crate.

## Public API

### `[package]`

| Key            | Type   | Value                                                                    | Purpose                                                          |
| -------------- | ------ | ------------------------------------------------------------------------ | ---------------------------------------------------------------- |
| `name`         | string | `novavault-core`                                                         | Crate name; imported as `novavault_core`.                        |
| `description`  | string | `Cryptography, vault format and key slots for NovaVault (UI-independent).` | One-line summary.                                              |
| `version`      | inherited | `0.1.0` (`version.workspace = true`)                                  | Shared workspace version.                                        |
| `edition`      | inherited | `2024`                                                                 | Rust edition.                                                    |
| `rust-version` | inherited | `1.85`                                                                 | Minimum supported Rust version (the first with edition 2024).    |
| `license`      | inherited | `GPL-3.0-or-later`                                                     | Project license.                                                 |
| `repository`   | inherited | `https://github.com/acevflow/NovaVault`                                | Source location.                                                 |
| `publish`      | inherited | `false`                                                                | Never published to crates.io.                                    |

### `[dependencies]`

| Crate              | Requirement | Features                                   | Purpose                                                                 |
| ------------------ | ----------- | ------------------------------------------ | ----------------------------------------------------------------------- |
| `argon2`           | `0.6.0`     | `default-features = false`; `alloc`, `zeroize` | Argon2id key derivation ([`kdf`](src/kdf.rs.md)).                    |
| `chacha20poly1305` | `0.11.0`    | `default-features = false`; `zeroize`      | XChaCha20-Poly1305 ([`aead`](src/aead.rs.md)).                          |
| `getrandom`        | `0.4.3`     | defaults                                   | OS CSPRNG for keys, salts, nonces, recovery codes and identifiers.     |
| `secrecy`          | `0.10.3`    | defaults                                   | `SecretBox`, `SecretSlice`, `SecretString` wrappers.                    |
| `thiserror`        | `2.0.21`    | defaults                                   | Derives `CryptoError` and `VaultError` ([`error`](src/error.rs.md)).    |
| `zeroize`          | `1.9.1`     | `derive`                                   | `Zeroizing`, `Zeroize`, `ZeroizeOnDrop` for buffers and item types.    |

There are no `[dev-dependencies]`, `[build-dependencies]`, features, targets or benches: tests use only the standard library and the crate's own [`test_util`](src/test_util.rs.md).

### `[lints]`

| Key         | Value  | Effect                                                                                  |
| ----------- | ------ | --------------------------------------------------------------------------------------- |
| `workspace` | `true` | Applies the root `[workspace.lints]` table, listed below.                               |

| Inherited lint         | Level  |
| ---------------------- | ------ |
| `rust::unsafe_code`    | `deny` |
| `rust::missing_docs`   | `warn` |
| `clippy::unwrap_used`  | `deny` |
| `clippy::expect_used`  | `deny` |

## Details

### Dependency versions

Requirements are written as plain versions, which Cargo treats as caret requirements (`^0.6.0` allows `0.6.x`, `^2.0.21` allows any `2.x` at or above it). The exact versions used are pinned by the workspace `Cargo.lock`, and CI builds with `--locked`. At the time of writing `Cargo.lock` resolves each of the six to exactly the version written here.

### Security-relevant settings

- **`default-features = false` on `argon2` and `chacha20poly1305`**: keeps the RustCrypto crates minimal. In particular neither pulls in a random number generator, so `getrandom` is the crate's only randomness source. `argon2` needs `alloc` back for its memory blocks.
- **`zeroize` features on `argon2` and `chacha20poly1305`**: make Argon2 wipe its working memory and the cipher wipe its internal key copy when dropped. Without them key material would outlive the call in freed memory.
- **`zeroize` with `derive`**: the item types use `#[derive(Zeroize, ZeroizeOnDrop)]`, so their fields are wiped on drop.
- **`unsafe_code = "deny"`**: the workspace allows a crate-level exception if justified; this crate goes further with `#![forbid(unsafe_code)]` in [`lib.rs`](src/lib.rs.md), which cannot be overridden.
- **`unwrap_used` and `expect_used` denied**: enforces the project rule of no panics on secret-handling paths. The root `clippy.toml` sets `allow-unwrap-in-tests` and `allow-expect-in-tests`, which is why the test modules (see the `*.rs.tests.md` pages next to each source page) may use them.
- **`missing_docs = "warn"`**: every public item must be documented. CI runs clippy with `-D warnings`, so a missing doc comment fails CI.

## Dependencies

- **Internal**: the workspace manifest [`Cargo.toml`](../../../../Cargo.toml) (package fields and lints) and [`clippy.toml`](../../../../clippy.toml) (test exemptions).
- **External**: the six crates above. Each one, and every transitive cryptographic crate they pull in, is justified with its audit status in [`docs/dependencies.md`](../../../dependencies.md), as CLAUDE.md requires.

## Used by

- The workspace manifest lists `crates/novavault-core` in `members`.
- [`src-tauri/Cargo.toml`](../../../../src-tauri/Cargo.toml) depends on it with `novavault-core = { path = "../crates/novavault-core" }`.

## Tests

_Not applicable: a manifest has no tests. It is exercised whenever the crate builds; `cargo test -p novavault-core` builds and runs the unit tests: 129 on Unix, 128 on Windows, where the Unix-only `atomic::tests::file_is_private_to_its_owner` is not compiled._

## Notes

- The workspace comment on `unsafe_code` says exceptions must be justified at the crate level; this crate uses `forbid`, so it can never take one.
- `docs/dependencies.md` also lists `zeroize_derive` 1.5.0. It is not declared here; it comes in through the `derive` feature of `zeroize`.
- `getrandom` and `thiserror` also appear in `Cargo.lock` at older major versions (0.3.4 and 1.0.69), pulled in by other workspace dependencies, not by this crate.
