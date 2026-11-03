# NovaVault — Project rules

NovaVault is an open-source, local-first desktop app for managing passwords, notes, secrets and files in an encrypted vault.
Maintainer contact: acevflow@gmail.com

## Product

- Desktop only, 100% local. No server, no telemetry, no network access for core features.
- Stack: Tauri 2 (shell), React + TypeScript (UI), Rust (core).
- Platforms: Windows, macOS, Linux. CI builds and tests on all three.
- License: GPL-3.0-or-later. Solo maintainer: keep scope tight and code simple.
- MVP in scope: create/unlock/lock vault with master password; passwords, notes, secrets (CRUD); encrypted file attachments; emergency recovery code and master password change; auto-lock on inactivity, suspend and screen lock; clipboard auto-clear.
- Out of scope: sync, browser extension/autofill, mobile, multi-user sharing.

## Security architecture (non-negotiable)

- All cryptography and data access live in Rust. The React UI never receives the vault key or raw key material; expose only minimal Tauri commands.
- Never implement custom cryptographic primitives. Use audited crates (RustCrypto, or `age`).
- Key derivation: Argon2id. Authenticated encryption: XChaCha20-Poly1305 or AES-GCM.
- Key slots: a random 256-bit vault key encrypts all data and is stored wrapped twice: under a key derived from the master password and under a key derived from the emergency recovery code. Either slot opens the vault. Changing the master password only re-wraps the vault key.
- The recovery code is shown once at creation in a human-readable format, the user must confirm they saved it, and it can be regenerated (invalidating the previous one).
- Use `zeroize` and `secrecy` for secrets in memory. Never log secrets. Strict CSP and a minimal Tauri capability set.
- The vault format is versioned and documented as a public specification.

## Repository structure

```text
crates/novavault-core/   crypto, vault format, key slots; UI-independent library
src-tauri/               thin Tauri layer
ui/                      React + TypeScript
docs/
assets/
.github/
README.md
SECURITY.md
CODE_OF_CONDUCT.md
CONTRIBUTING.md
CHANGELOG.md
LICENSE
```

## Git workflow

- Commit, branch and push incrementally while working, not only at the end.
- Never commit directly to `main` after the initial commit. One short-lived branch per unit of work, pushed to `origin`.
- Branch names: `feat/<topic>`, `fix/<topic>`, `docs/<topic>`, `chore/<topic>`, `refactor/<topic>`, `test/<topic>`, `ci/<topic>`.
- Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`, `ci:`), imperative subject of 72 chars or fewer, body explains the why when not obvious.
- Small atomic commits; each one leaves the project buildable. Push after every meaningful commit.
- When a branch is complete, open a PR into `main` with `gh` and merge it (`gh pr merge --merge --delete-branch`); if `gh` is unavailable, merge locally with `--no-ff` and push `main`.
- Update `CHANGELOG.md` (Keep a Changelog, Unreleased section) with every feature or fix.
- Never commit secrets, vault files, build artifacts, `node_modules/` or `target/`.
- If auth or permissions block a push, stop and report exactly what is needed. Do not work around it.

## Code quality

- Clean, idiomatic, modular code with descriptive names.
- Document every public item (Rust `///` and `//!`, TSDoc for exported TypeScript). Inline comments explain WHY, especially around security decisions. No noise comments.
- Rust: `cargo fmt`, `cargo clippy -- -D warnings`, no `unwrap()`/`expect()` in non-test code, typed errors (`thiserror`), unit tests with known vectors where applicable.
- TypeScript: strict mode, ESLint + Prettier, no `any`, accessible components.
- A single command must run the full local check (format, lint, test).
- Keep dependencies minimal; justify every cryptography-related dependency in `docs/`.

## Working style

- Do only the task requested in the current prompt, then stop with a short summary: done, pending, decisions needed.
- Keep README status, CHANGELOG and docs in sync with the work done.
