# Future Project: Rust Ownership Expansion

Status: research backlog; not approved for implementation.

## Intent

Explore whether additional cross-client product semantics should be owned by
the existing `crates/synara-core` Rust engine instead of being duplicated or
decided independently by the React/TypeScript desktop presenter and the
SwiftUI iOS presenter.

The intended architectural shape is a **Rust application brain with
platform-owned interfaces**:

```text
matrix-rust-sdk
       |
       v
crates/synara-core
       |
       +-- typed Tauri DTOs/events --> React presenter on macOS/Linux
       |
       `-- typed UniFFI models -----> SwiftUI presenter on iOS
```

This project does not presume that every candidate should move. Much of the
Matrix lifecycle is already Rust-owned, and
[ADR 0004](../../adr/0004-rust-language-boundaries.md) deliberately keeps UI,
composer, viewport, HTML rendering, OS integrations, secrets, paths, and large
media bytes on platform-specific boundaries. Research must identify the
actual residual behavior and preserve those decisions unless a replacement
ADR is explicitly approved.

## Candidate workstreams

| ID     | Candidate                                           | Brief                                                            |
| ------ | --------------------------------------------------- | ---------------------------------------------------------------- |
| ROE-01 | Matrix SDK orchestration, sync, and encryption      | [Workstream](workstreams/01-matrix-sdk-orchestration.md)         |
| ROE-02 | Device verification and trust state machines        | [Workstream](workstreams/02-device-verification-trust.md)        |
| ROE-03 | Timeline normalization and event relationships      | [Workstream](workstreams/03-timeline-normalization.md)           |
| ROE-04 | Semantic message-format presentation models         | [Workstream](workstreams/04-semantic-message-presentation.md)    |
| ROE-05 | Read-marker and unread calculations                 | [Workstream](workstreams/05-read-marker-unread.md)               |
| ROE-06 | Room sorting and filtering rules                    | [Workstream](workstreams/06-room-sorting-filtering.md)           |
| ROE-07 | Notification and critical-approval policy           | [Workstream](workstreams/07-notification-critical-policy.md)     |
| ROE-08 | Agent approval detection and action resolution      | [Workstream](workstreams/08-agent-approval-resolution.md)        |
| ROE-09 | Notes and account-data synchronization              | [Workstream](workstreams/09-notes-account-data.md)               |
| ROE-10 | Draft serialization and reply metadata              | [Workstream](workstreams/10-drafts-replies.md)                   |
| ROE-11 | Media metadata and cache policy                     | [Workstream](workstreams/11-media-metadata-cache.md)             |
| ROE-12 | Shared validation, sanitization, and security rules | [Workstream](workstreams/12-validation-sanitization-security.md) |

## Typed presentation-model hypothesis

ROE-04 should evaluate—not assume—the value of Rust returning a typed,
versioned, UI-neutral message tree. Candidate nodes include:

- paragraph and text spans;
- strong and emphasis;
- inline code and code blocks with an optional language;
- tables with header/body structure;
- replies and quoted context;
- spoilers with an optional reason;
- links, lists, mentions, and line breaks required by observed Matrix events.

The model must not prescribe colors, typography, spacing, accessibility
labels, gestures, selection behavior, or platform widgets. React and SwiftUI
must remain responsible for native rendering. Research must compare this
approach with the accepted ADR 0004 rule that Markdown/HTML rendering stays in
TypeScript and determine whether a narrower protocol-semantic model can add
parity without importing presenter behavior into Core.

## Definition of a planning-ready workstream

A workstream is ready for an implementation decision only when it has:

1. a source-linked current-state and duplication census;
2. explicit in-scope and out-of-scope behavior;
3. functional, security, performance, accessibility, and compatibility
   requirements appropriate to the domain;
4. at least two alternatives, including leaving ownership unchanged;
5. a proposed typed Core API, DTO/event evolution, and versioning story when
   migration is recommended;
6. desktop and iOS adoption paths with no dual-owner interval left ambiguous;
7. unit, property, contract, integration, Synapse, simulator, desktop, and
   manual validation coverage as applicable;
8. observability, rollback, data migration, and failure-mode plans;
9. dependencies and sequencing relative to the other workstreams;
10. adversarial review findings and a clear go/no-go recommendation.

Use the [agent guide](AGENT-GUIDE.md) to investigate a workstream and write the
result using the [plan template](PLAN-TEMPLATE.md).

## Portfolio-level questions

- Which candidate behaviors are already correctly Rust-owned?
- Which apparent duplicates are intentional presenter projections rather than
  competing business logic?
- Where would FFI/IPC latency, model churn, or serialization cost exceed the
  parity benefit?
- Which APIs can be delivered as end-to-end vertical slices rather than a
  large coordinated rewrite?
- Which workstreams share primitives and therefore require an explicit order?
- What evidence would justify superseding any part of ADR 0004?

## Non-goals

- Replacing React, SwiftUI, Slate, WebKit, or platform accessibility systems.
- Introducing Dioxus, Slint, egui, Leptos, or another Rust UI framework.
- Moving passwords, recovery material, local paths, or large media bytes over
  the generic Core command envelope.
- Rewriting Node build and governance tooling in Rust.
- Creating speculative Core routes solely to reduce TypeScript line count.
