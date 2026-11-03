# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- README with project overview, features, security model summary, tech stack, build instructions and roadmap.
- Security policy (`SECURITY.md`) with private vulnerability reporting instructions, response expectations and scope.
- Code of Conduct based on the Contributor Covenant 2.1.
- Contributing guidelines covering development setup, branch and commit conventions, code style, testing, the pull request process and rules for security-sensitive changes.
- This changelog.
- Project documentation in `docs/`, linked from the README:
  - Documentation index (`docs/README.md`).
  - Architecture overview: components, layers, trust boundary, data flows and why all cryptography lives in Rust.
  - Threat model: assets, adversaries, in-scope and out-of-scope threats, mitigations and residual risks.
  - Vault format specification skeleton (format version 0, draft): header, key slots, encrypted records, attachments and recovery code.
  - Development guide: per-OS setup, commands and project layout.
- GitHub issue forms for bug reports and feature requests; blank issues are disabled and vulnerability reports are directed to private security advisories and `SECURITY.md`.
- Pull request template with a security impact section and a contribution checklist.
- CI workflow running Rust format, clippy and tests and UI lint, typecheck and tests on Windows, macOS and Linux, with caching; steps are skipped until the corresponding component exists. Actions are pinned to commit SHAs.
- Dependabot configuration for Cargo, npm and GitHub Actions updates.
- CI status badge in the README.
- Cargo workspace with shared lints (unsafe code denied; `unwrap()`/`expect()` denied outside tests) and an empty, documented `novavault-core` library crate.
- Minimal Tauri 2 desktop shell (`src-tauri`) with one window, a strict production Content Security Policy, frozen prototypes, no global Tauri API and a default capability that grants no permissions.
- App icons generated from a new square emblem (`assets/icon.svg`).
- React + TypeScript UI scaffold in `ui/` built with Vite: strict TypeScript, ESLint (type-checked strict rules, React hooks, jsx-a11y), Prettier and Vitest with Testing Library.
- Single full local check, `npm run check`, covering formatting, linting, typechecking, tests and builds for the UI and the Rust workspace; documented in `docs/development.md`.
- Core cryptographic primitives in `novavault-core`:
  - Argon2id key derivation (`derive_key`) with validated `KdfParams`: RFC 9106 defaults (64 MiB, 3 iterations, 4 lanes) and enforced minimum and maximum bounds.
  - XChaCha20-Poly1305 `encrypt`/`decrypt` with a fresh random 192-bit nonce per message and a single generic error for every authentication failure.
  - Zeroizing `SecretKey` (via `secrecy`) and `Salt` types; random bytes come from the OS CSPRNG.
  - Typed `CryptoError` (via `thiserror`) with no secret data in any variant.
  - Unit tests with RFC 9106, Argon2 reference and XChaCha20-Poly1305 draft test vectors, plus tamper-detection tests.
- Cryptography and dependencies section in `docs/architecture.md`: algorithms, parameters, the XChaCha20-Poly1305 choice and the justification for every cryptography-related crate.

### Changed

- Vault format specification: AEAD fixed to XChaCha20-Poly1305; Argon2id bounds and salt, nonce and wrapped-key sizes filled in.
- CI builds the UI before the Rust checks (the Tauri shell embeds the built UI) and checks UI formatting; Dependabot also tracks the root npm package (Tauri CLI).

[Unreleased]: https://github.com/acevflow/NovaVault/commits/main
