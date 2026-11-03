# Contributing to NovaVault

Thank you for your interest in NovaVault. This guide explains how to set up a development environment, the conventions the project follows and how changes get reviewed and merged.

NovaVault is maintained by a single person, so the project deliberately keeps its scope tight and its code simple. Please read the [README](README.md) for the product scope before proposing new features, and follow the [Code of Conduct](CODE_OF_CONDUCT.md) in all project spaces.

> [!IMPORTANT]
> **Security vulnerabilities must not be reported through issues or pull requests.** Follow [SECURITY.md](SECURITY.md) instead.

## Ways to contribute

- **Report bugs** by opening an issue with steps to reproduce, expected versus actual behavior, your OS and the NovaVault version or commit.
- **Suggest improvements** by opening an issue first. Features outside the MVP scope (sync, browser extensions, mobile, multi-user sharing) will be declined.
- **Improve documentation**, including the vault format specification once it exists.
- **Submit code** for an open issue. For anything non-trivial, comment on the issue first so the approach can be agreed before you invest time.

## Development setup

> [!NOTE]
> **Coming soon.** The application scaffolding has not been committed yet. The prerequisites below are final; the exact commands will be added with the first code drop.

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) stable toolchain, with the `rustfmt` and `clippy` components.
- [Node.js](https://nodejs.org/) LTS.
- The [Tauri 2 system prerequisites](https://tauri.app/start/prerequisites/) for your platform (WebView2 on Windows, Xcode Command Line Tools on macOS, WebKitGTK and related packages on Linux).

### Getting the code

```sh
git clone https://github.com/acevflow/NovaVault.git
cd NovaVault
```

### Running the full local check (coming soon)

A single command will run formatting checks, linters and all tests for both the Rust and TypeScript code. Run it before every push; CI runs the same checks on Windows, macOS and Linux.

### Repository layout

```text
crates/novavault-core/   crypto, vault format, key slots; UI-independent library
src-tauri/               thin Tauri layer
ui/                      React + TypeScript
docs/                    specifications and design documents
assets/                  logo and other static assets
```

## Branches

Never commit directly to `main`. Create one short-lived branch per unit of work, named with one of these prefixes:

| Prefix        | Use for                                |
| ------------- | -------------------------------------- |
| `feat/`       | New features                           |
| `fix/`        | Bug fixes                              |
| `docs/`       | Documentation only                     |
| `refactor/`   | Code changes that do not alter behavior |
| `test/`       | Adding or improving tests              |
| `chore/`      | Tooling, dependencies, housekeeping    |
| `ci/`         | Continuous integration                 |

Example: `feat/clipboard-auto-clear`.

## Commits

NovaVault uses [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/):

```text
<type>: <imperative summary, 72 characters or fewer>

<optional body explaining why the change is needed>
```

- Allowed types: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `ci`.
- Write the subject in the imperative mood ("add", not "added" or "adds").
- Keep commits small and atomic. Each commit must leave the project buildable and passing its checks.
- Explain the *why* in the body whenever it is not obvious from the diff.
- Never commit secrets, vault files, build artifacts, `node_modules/` or `target/`.

## Code style

### General

- Write clean, idiomatic, modular code with descriptive names.
- Document every public item. Inline comments should explain *why*, especially around security decisions; avoid comments that merely restate the code.
- Keep dependencies to a minimum and justify each new one in the pull request.

### Rust

- Format with `cargo fmt`.
- Lint with `cargo clippy -- -D warnings` (warnings are errors).
- No `unwrap()` or `expect()` outside of tests. Propagate errors with typed error enums built on `thiserror`.
- Document public items with `///` and modules or crates with `//!`.

### TypeScript and React

- TypeScript `strict` mode; never use `any`.
- Lint with ESLint and format with Prettier.
- Document exported items with TSDoc.
- Build accessible components: semantic HTML, labelled controls, full keyboard navigation and visible focus states.

## Testing

- Every feature and bug fix comes with tests. A bug fix should include a test that fails without the fix.
- Rust code has unit tests next to the code it covers. Cryptographic and format code must be tested against **known test vectors** wherever they exist, plus round-trip and tamper-detection tests.
- UI logic has unit tests; user-facing flows should be testable without real secrets.
- Tests must never use real personal data or real vault files.

## Pull request process

1. Make sure your branch is up to date with `main` and the full local check passes.
2. Update [CHANGELOG.md](CHANGELOG.md) under **Unreleased** for every user-visible feature or fix.
3. Update the README and `docs/` when behavior, scope or the vault format changes.
4. Open a pull request into `main` with a clear title (Conventional Commits style) and a description of what changed and why. Link the related issue.
5. Keep pull requests focused on a single concern; split unrelated changes.
6. CI must pass on all three platforms before a pull request is merged.
7. Address review feedback with additional commits; the maintainer merges once the change is approved.

By submitting a contribution, you agree that it is licensed under the project's license, the [GNU General Public License v3.0 or later](LICENSE).

## Security-sensitive changes

Changes to cryptography, key handling, the vault format, the Tauri command surface or memory handling receive extra scrutiny. Expect slower, more detailed review, and please follow these rules:

- **No custom cryptographic primitives.** Use audited crates only (RustCrypto, or `age`). The approved algorithms are Argon2id for key derivation and XChaCha20-Poly1305 or AES-GCM for authenticated encryption.
- **Justify cryptography-related dependencies** in `docs/`, covering why the crate is needed, its audit status and maintenance health.
- **Keep secrets in Rust.** The UI must never receive the vault key or raw key material. New Tauri commands must be minimal and return only what the UI strictly needs.
- **Keep the capability set minimal.** Do not widen Tauri capabilities or relax the Content Security Policy without a documented reason.
- **Handle memory carefully.** Wrap secrets in `secrecy` types, wipe them with `zeroize` and avoid unnecessary copies.
- **Never log secrets**, including in debug output, error messages and panics.
- **Version the vault format.** Any change to the on-disk format requires a format version bump, a migration path for existing vaults and an update to the public specification.
- **Explain the threat model impact** in the pull request description: what an attacker could or could no longer do after the change.
- **Add tests**, including known vectors and negative tests (wrong password, corrupted or tampered data).

If you are unsure whether a change is security-sensitive, assume it is and say so in the pull request.

## Questions

Open an issue for general questions. For anything security-related, use the private channels described in [SECURITY.md](SECURITY.md).
