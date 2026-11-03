# NovaVault documentation

Design and contributor documentation for NovaVault. Start with the architecture, then the threat model; the vault format specification is the reference for anything stored on disk.

> [!NOTE]
> NovaVault is pre-alpha. These documents describe the intended design and are updated together with the code.

| Document | What it covers |
| -------- | -------------- |
| [Architecture](architecture.md) | Components, layers, trust boundary, data flows and why all cryptography lives in Rust. |
| [Threat model](threat-model.md) | Assets, adversaries, in-scope and out-of-scope threats, mitigations and residual risks. |
| [Vault format specification](vault-format.md) | Versioned on-disk format: header, key slots, encrypted records, attachments. Draft skeleton. |
| [Development guide](development.md) | Setup on Windows, macOS and Linux, commands and project layout. |

Planned:

- **Cryptography and dependencies:** algorithms, parameters and the justification for every cryptography-related crate.

Related project files:

- [README](../README.md): overview, features and roadmap.
- [CONTRIBUTING](../CONTRIBUTING.md): branches, commits, code style and review.
- [SECURITY](../SECURITY.md): how to report a vulnerability.
- [CHANGELOG](../CHANGELOG.md): notable changes.
