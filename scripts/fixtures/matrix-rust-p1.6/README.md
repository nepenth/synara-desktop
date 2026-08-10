# P1.6 prohibited fixtures

Representative files that **must fail** `scripts/matrix-rust-p1.6-guardrails.mjs`
when scanned as if they lived at the given repository-relative paths.

These fixtures are **not** part of the product tree. Unit tests mount them into
temporary directory trees and assert non-zero exit / findings.

| Fixture tree | Rule |
|--------------|------|
| `prohibited/js-sdk-in-matrix-ipc/` | Hard-ban matrix-js-sdk in matrix-ipc |
| `prohibited/js-sdk-new-file/` | New production importer outside allowlist |
| `prohibited/raw-matrix-http/` | Raw `/_matrix/` outside exceptions |
| `prohibited/unversioned-ipc/` | Envelope without protocolVersion |
| `prohibited/sdk-types-in-dto/` | matrix_sdk in DTO wire module |
| `prohibited/sdk-types-in-ipc/` | ruma in IPC wire module |
