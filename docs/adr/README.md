# Architecture Decision Records

Architecture Decision Records (ADRs) preserve consequential repository
decisions, their reasons, and their boundaries. Current source and accepted
later ADRs take precedence over dated implementation inventories inside an ADR.

## Status vocabulary

- **Accepted:** the decision governs new work until another ADR amends or
  supersedes it.
- **Accepted as amended:** the durable part remains, but a later ADR replaced a
  named portion.
- **Superseded:** retained for history; no longer governs the replaced decision.
- **Proposed:** not binding until explicitly accepted.
- **Archived proposal:** never became an accepted ADR and must not occupy the
  canonical accepted sequence.

An ADR is not immutable. A change that crosses an accepted boundary should cite
current evidence, compare alternatives, describe migration and rollback, and
land an explicit amendment or replacement ADR before implementation.

## Accepted sequence

| ID                                          | Decision                               | Current status                                   | Last reviewed |
| ------------------------------------------- | -------------------------------------- | ------------------------------------------------ | ------------- |
| [0001](0001-ios-repository-layout.md)       | iOS repository layout                  | Accepted and implemented                         | 2026-09-01    |
| [0002](0002-ios-architecture.md)            | Native SwiftUI iOS application         | Accepted as amended by ADR 0003                  | 2026-09-01    |
| [0003](0003-shared-native-rust-core.md)     | Shared native Rust core                | Accepted; architectural source shape implemented | 2026-09-01    |
| [0004](0004-rust-language-boundaries.md)    | Rust and platform ownership boundaries | Accepted and clarified                           | 2026-09-01    |
| [0005](0005-native-media-handle-channel.md) | Native media handle and byte channels  | Accepted and implemented                         | 2026-09-01    |

## Archived proposals

- [2026-07-24 Matrix Rust SDK migration UX proposal](archive/2026-07-24-matrix-rust-sdk-migration-ux-proposal.md)
  was once labeled “ADR 0003” while still pending review. It was never accepted
  into the numbered sequence. Its detailed migration policy remains historical
  evidence under `docs/matrix-rust-sdk/`.
