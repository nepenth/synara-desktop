# Security

Synara is a public AGPL-3.0-only Matrix client. Report vulnerabilities through
[GitHub Security Advisories](https://github.com/nepenth/synara-desktop/security/advisories/new).
Do not open a public issue for a vulnerability, and do not include passwords,
access tokens, recovery keys, session exports, or message contents.

## In scope

- The shared Rust core in `crates/synara-core`
- The Tauri desktop shell in `src-tauri`
- The desktop UI in `synara` when it can reach native commands
- The iOS app and notification service extension in `synara-ios`

## Out of scope

- Historical plans under `docs/matrix-rust-sdk/` and `docs/shared-native-core/`
- Third-party homeservers
- Vulnerabilities in `matrix-rust-sdk` itself, which should also be reported upstream

## What a report should include

- The affected version or commit
- The platform (macOS, Linux, or iOS)
- What an attacker can do, and what they already need (for example, a local process or a logged-in account)
- Whether message contents, recovery keys, or session tokens are exposed

## Handling of secrets in this repository

Session tokens and store keys are kept in the operating-system credential
store. A generated recovery key is shown once in the desktop UI and is saved
only when you choose a file. It is not written to Downloads automatically.

`ITSAppUsesNonExemptEncryption` remains false. Matrix encryption in this client
is the standard mass-market cryptography used for messaging. Confirm that
export declaration with counsel before an App Store submission.
