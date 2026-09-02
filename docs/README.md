# Documentation Guide

This directory contains current operating documentation and dated engineering
records. Use this guide to distinguish normative product behavior from
historical plans and snapshots.

## Current Entry Points

| Area                            | Current documentation                                                                                                                                                  |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Product and local setup         | [`../README.md`](../README.md)                                                                                                                                         |
| Source-oriented architecture    | [`../CODEBASE_KNOWLEDGE_BASE.md`](../CODEBASE_KNOWLEDGE_BASE.md)                                                                                                       |
| Repository ownership and layout | [`repository-layout.md`](repository-layout.md)                                                                                                                         |
| Build, validation, and release  | [`build-and-release.md`](build-and-release.md), [`production-smoke-checklist.md`](production-smoke-checklist.md)                                                       |
| Desktop platform behavior       | [`desktop-modernization.md`](desktop-modernization.md), [`linux.md`](linux.md), [`macos-local-signing.md`](macos-local-signing.md)                                     |
| Shared Rust core                | [`adr/0003-shared-native-rust-core.md`](adr/0003-shared-native-rust-core.md), [`adr/0004-rust-language-boundaries.md`](adr/0004-rust-language-boundaries.md)           |
| Future project explorations     | [`future-projects/README.md`](future-projects/README.md), [`future-projects/rust-ownership-expansion/TRIAGE.md`](future-projects/rust-ownership-expansion/TRIAGE.md)   |
| Timeline reliability            | [`timeline-room-state-reliability-contract.md`](timeline-room-state-reliability-contract.md), [`timeline-room-state-acceptance.md`](timeline-room-state-acceptance.md) |
| iOS application                 | [`../synara-ios/README.md`](../synara-ios/README.md), [`../synara-ios/docs/ios-validation-status.md`](../synara-ios/docs/ios-validation-status.md)                     |
| Shared product contracts        | [`../synara/docs/synara-contracts.md`](../synara/docs/synara-contracts.md), [`../synara/docs/contracts/README.md`](../synara/docs/contracts/README.md)                 |

The source tree and executable validation scripts take precedence if a current
document and implementation disagree. Update the document in the same change
that modifies the behavior.

Documents under [`future-projects/`](future-projects/README.md) are proposals
and research workspaces, not accepted architecture or authorization to change
product code. An accepted ADR and an approved implementation plan are required
before a future-project proposal may override a current boundary.

## Architecture Decisions

Files under [`adr/`](adr/) record decisions and their consequences. An ADR
remains authoritative until another ADR explicitly supersedes it. Dated source
censuses and implementation counts inside an ADR are evidence from that point
in time, not live inventory.

## Historical Records

The following categories are intentionally retained for provenance:

- migration plans and spikes;
- dated audits and review reports;
- progress logs, scoreboards, handoffs, and validation snapshots;
- task packets and acceptance reports under `matrix-rust-sdk/`;
- completed phase plans and release-readiness goals.

These records may mention old branches, versions, dependencies, test counts, or
implementation paths. A historical statement is not a current product claim.
Documents with especially easy-to-misread pre-cutover architecture include an
explicit supersession notice at their beginning.

## Public Repository Hygiene

Documentation must contain only public information and non-secret placeholders.
Do not include:

- passwords, access or refresh tokens, recovery material, private keys, signing
  certificates, provisioning profiles, or session exports;
- private homeserver names, internal dashboards, runner hostnames, local account
  names, personal filesystem paths, device identifiers, or private room/event
  identifiers;
- screenshots, logs, or crash reports containing non-disposable Matrix data.

Use `example.org`, `example.com`, `example.invalid`, zero UUIDs, and descriptive
angle-bracket placeholders in commands. Store real local values only in ignored
environment files or external permission-restricted storage.

Run the documentation guard before committing documentation changes:

```sh
npm run check:docs
```

The check scans tracked documentation for prohibited private patterns and
verifies repository-local Markdown links.
