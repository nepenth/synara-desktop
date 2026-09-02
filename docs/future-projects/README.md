# Future Projects

Status: exploratory. Nothing in this directory authorizes product changes.

This directory holds ideas that deserve structured investigation but are not
yet accepted architecture, scheduled implementation, or release commitments.
It exists so an agent can receive one stable entry point, examine current
source and decisions, challenge the premise, and produce an implementation-
ready proposal without quietly turning a brainstorm into a migration.

## Projects

| Project                                                        | Question                                                                                                                | Status           |
| -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ---------------- |
| [Rust ownership expansion](rust-ownership-expansion/README.md) | Which remaining cross-client product semantics should move from platform presenters into the existing shared Rust core? | Research backlog; start with [triage](rust-ownership-expansion/TRIAGE.md) |

## Rules for work in this directory

1. Current source, tests, and accepted ADRs take precedence over proposals.
2. Research must begin with a current-state census. Do not assume the named
   behavior is still implemented in TypeScript or duplicated across clients.
3. A conclusion of **stay platform-side**, **already correctly owned**, or
   **delete rather than migrate** is a valid and valuable outcome.
4. Do not create a second core, parallel Matrix engine, or Rust UI rewrite.
5. Do not implement product code from a research brief. First produce the
   required plan, obtain review, and record any boundary change in an ADR.
6. Plans must cover desktop macOS, desktop Linux, and iOS consequences, even
   when a workstream changes only one shared component.
7. Never place credentials, private homeserver details, private room content,
   crash reports, or personal filesystem paths in these documents.

The accepted language boundary remains
[ADR 0004](../adr/0004-rust-language-boundaries.md). The existing implementation
route remains the [shared-core implementer playbook](../shared-native-core/11-implementer-playbook.md).
Future-projects research does not override playbook §5, the language-boundary
goal graph, or an accepted ADR.
