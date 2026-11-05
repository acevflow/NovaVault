# NovaVault documentation

Design and contributor documentation for NovaVault. Start with the architecture, then the threat model; the vault format specification is the reference for anything stored on disk.

> [!NOTE]
> NovaVault is pre-alpha. These documents describe the implemented code and are updated together with it.

| Document | What it covers |
| -------- | -------------- |
| [Architecture](architecture.md) | Components, layers, trust boundary, data flows and why all cryptography lives in Rust. |
| [Threat model](threat-model.md) | Assets, adversaries, implemented mitigations, out-of-scope threats, and known limitations and gaps. |
| [Security design](security-design.md) | Key hierarchy, recovery code flow, memory handling, auto-lock and clipboard behaviour, with per-OS notes. |
| [Cryptography dependencies](dependencies.md) | Every cryptography-related dependency with its locked version and the reason it was chosen. |
| [Vault format specification](vault-format.md) | Versioned on-disk format: header, key slots, encrypted records, integrity seal, encrypted attachment blobs, migration, reader rules and test vectors. Format version 1, pre-release. |
| [API reference](api-reference.md) | Every Tauri command the UI can call, with its arguments, return value and error codes. |
| [Development guide](development.md) | Setup on Windows, macOS and Linux, commands and project layout. |
| [Clipboard behaviour](clipboard.md) | How copy and auto-clear work, and what happens on Windows, macOS, X11 and Wayland. |
| [Rust test report](testing/reports/rust.md) | Latest local run of `cargo fmt`, `clippy` and `cargo test`: environment, results per crate, failures and untested areas. |

Algorithms and parameters are covered in [Architecture: Cryptography and dependencies](architecture.md#cryptography-and-dependencies); the justification for every cryptography-related crate is in [dependencies.md](dependencies.md).

## Code reference

The [code reference](code/README.md) has one page per hand-written source file and significant configuration file, mirroring the source tree. Each page covers the file's purpose, public API with exact signatures, security notes, dependencies, callers and tests. The index groups the pages by layer (core, Tauri shell, UI, configuration); [TEMPLATE.md](code/TEMPLATE.md) defines the page format and what is covered.

## Related project files

- [README](../README.md): overview, features and roadmap.
- [CONTRIBUTING](../CONTRIBUTING.md): branches, commits, code style and review.
- [SECURITY](../SECURITY.md): how to report a vulnerability.
- [CHANGELOG](../CHANGELOG.md): notable changes.
