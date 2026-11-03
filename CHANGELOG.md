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

[Unreleased]: https://github.com/acevflow/NovaVault/commits/main
