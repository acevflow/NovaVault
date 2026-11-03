<!--
Security vulnerabilities must not be disclosed in pull requests. See SECURITY.md.
Use a Conventional Commits title, for example "feat: add clipboard auto-clear".
-->

## Summary

<!-- What does this change do, and why? -->

## Type of change

- [ ] `feat`: new feature
- [ ] `fix`: bug fix
- [ ] `docs`: documentation only
- [ ] `refactor`: no behavior change
- [ ] `test`: tests only
- [ ] `ci` / `chore`: build, CI or tooling

## Related issue

<!-- For example "Closes #123". -->

## Security impact

<!--
Does this touch cryptography, key handling, the vault format, Tauri commands or capabilities, CSP, or anything else listed as security-sensitive in CONTRIBUTING.md?
If yes, explain the threat model impact: what an attacker could or could no longer do after this change.
If no, write "None".
-->

## Checklist

- [ ] The full local check (format, lint, test) passes.
- [ ] Tests cover the change, including negative cases where relevant.
- [ ] Public items are documented (Rust `///` / `//!`, TSDoc).
- [ ] `CHANGELOG.md` is updated under **Unreleased** (for features and fixes).
- [ ] Related docs and README status are updated.
- [ ] No secrets, vault files, build artifacts, `node_modules/` or `target/` are committed.
