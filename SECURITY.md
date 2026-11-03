# Security Policy

NovaVault protects passwords, notes, secrets and files. Security reports are taken seriously and handled with priority. Thank you for helping keep NovaVault and its users safe.

## Supported versions

NovaVault has not had a stable release yet. Until version 1.0, only the latest release (and the current `main` branch) receives security fixes.

| Version                     | Supported          |
| --------------------------- | ------------------ |
| Latest release / `main`     | :white_check_mark: |
| Any older pre-1.0 release   | :x:                |

This table will be updated once stable releases exist.

## Reporting a vulnerability

> [!IMPORTANT]
> **Never report vulnerabilities through public GitHub issues, pull requests or discussions.** Public disclosure before a fix is available puts users at risk.

### Preferred: GitHub Private Vulnerability Reporting

1. Go to the [Security tab](https://github.com/acevflow/NovaVault/security) of the repository.
2. Click **"Report a vulnerability"**.
3. Fill in the advisory form.

This creates a private advisory visible only to you and the maintainer, where the issue can be discussed, fixed and disclosed in a coordinated way.

### Fallback: email

If you cannot use GitHub's reporting flow, email **acevflow@gmail.com** with the subject line `[NovaVault security]`. Do not include real secrets or personal vault files in your report.

### What to include

To help triage quickly, please provide as much of the following as you can:

- A description of the issue and its potential impact.
- The affected component (for example the core crate, the vault format, the Tauri command layer or the UI).
- The NovaVault version or commit hash, and your operating system.
- Step-by-step reproduction instructions or a proof of concept.
- Any suggested fix or mitigation.

## Response expectations

NovaVault is maintained by a single volunteer, so the timelines below are best-effort targets rather than guarantees:

- **Acknowledgement** of your report within **7 days**.
- **Initial assessment** (confirmed, needs more information, or not a vulnerability) within **14 days**.
- **Regular updates** on progress until the issue is resolved.
- **Coordinated disclosure**: a fix is released before details are made public. The target is to disclose within **90 days** of the report, or sooner once a fix ships. Timelines can be adjusted together for complex issues.
- **Credit** in the security advisory and changelog, if you wish to be named.

Please give the maintainer a reasonable opportunity to fix the issue before any public disclosure.

## Scope

### In scope

- The Rust core (`crates/novavault-core`): cryptography usage, key derivation, key slots, the vault format and parsing.
- The Tauri layer (`src-tauri`): exposed commands, capabilities, Content Security Policy and any path that could leak key material to the UI.
- Handling of secrets in memory (zeroization, unintended copies, logging).
- Auto-lock, clipboard auto-clear and recovery code flows.
- Issues that let an attacker read, modify or forge vault contents without the master password or recovery code.
- Insecure defaults in official builds or documentation.

### Out of scope

- Attacks that require a fully compromised host (malware, keyloggers, administrator or root access, a malicious OS).
- Physical access to a machine while the vault is unlocked.
- Weak or reused master passwords chosen by the user.
- Vulnerabilities in third-party dependencies that do not affect NovaVault in practice; please report those upstream (a heads-up is still appreciated).
- Social engineering, phishing or attacks against the maintainer's accounts.
- Denial of service against the local application by a user who already controls the machine.
- Reports produced only by automated scanners without a demonstrated impact.

## Safe harbor

Good-faith security research that follows this policy is welcome. Please test only against your own installations and vaults, avoid accessing data that is not yours, and do not degrade other users' systems.
