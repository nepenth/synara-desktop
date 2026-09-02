# Future Project: Rust Ownership Residual Census

Status: optional, docs-only research portfolio; not approved for implementation.

## Charter

Synara already ships the intended architecture: one Rust application engine in
`crates/synara-core`, a React/TypeScript desktop presenter, and a SwiftUI iOS
presenter. This directory is therefore a **post-cutover residual ownership
census**, not a Rust migration program or a twelve-item implementation queue.

Research here asks whether a specific, evidenced remainder still has competing
owners. The default answer is **leave behavior platform-side** unless harmful
duplication of product or protocol policy is demonstrated. Thin adapters,
renderers, viewport observations, and OS integrations are not duplicate
engines.

## Governing decisions and stop gate

Read these sources in order and apply the most specific accepted decision:

1. The [ADR index and lifecycle rules](../../adr/README.md) explain status,
   amendments, and supersession.
2. [ADR 0001](../../adr/0001-ios-repository-layout.md) keeps desktop, iOS,
   shared contracts, Core, CI, and release policy in one monorepo.
3. [ADR 0002](../../adr/0002-ios-architecture.md) preserves the native SwiftUI
   and Apple-platform boundary as amended by ADR 0003.
4. [ADR 0003](../../adr/0003-shared-native-rust-core.md) establishes one shared
   Matrix/application Core.
5. [ADR 0004](../../adr/0004-rust-language-boundaries.md) classifies Core
   authority, platform observation/rendering, hard invariants, and revisable
   technology preferences.
6. [ADR 0005](../../adr/0005-native-media-handle-channel.md) is the specific
   decision for media handles, paths, and byte channels.
7. The [shared-core implementer playbook](../../shared-native-core/11-implementer-playbook.md)
   and current [language-boundary goal graph](../../shared-native-core/13-language-boundary-goal-graph.md)—not
   an ADR or this portfolio—define current sequencing, stop conditions, and
   release proof.

These future-project documents cannot override an ADR, the current goal graph,
or release evidence. They must not create or advance shared-Core phase,
scoreboard, or release state. A human must explicitly charter docs-only
research; implementation requires a separate acceptance gate.

## Ownership taxonomy

Classify every behavior before proposing a move:

| Kind        | Owner    | Examples                                                                                                   |
| ----------- | -------- | ---------------------------------------------------------------------------------------------------------- |
| Authority   | Core     | Protocol truth, state machines, eligibility, resource bounds, shared schemas, and Matrix writes            |
| Observation | Platform | Viewport/focus, banners, keystrokes, app lifecycle, OS delivery state, and locale UI context               |
| Rendering   | Platform | React/SwiftUI widgets, text selection, accessibility, Prism, Dynamic Type, layout, gestures, and animation |

A platform observation may be an input to Core authority without Core owning
the observation. A presenter projection of a Core model is not a second source
of truth.

Also classify every constraint as one of:

- a **hard invariant** protecting correctness, security, or the accepted
  platform architecture;
- an **accepted platform boundary** describing observation, rendering, or OS
  ownership; or
- a **current technology preference** that may change through an evidence-based
  product and architecture decision.

Do not present a preference for React, SwiftUI, Slate, Prism, pdf.js, or Node as
an irreversible security boundary. Do not weaken a hard invariant merely
because a technology preference can be revisited.

## Unordered research clusters

The IDs are retained for traceability, not priority. Investigate at most one
deep cluster at a time.

| Cluster                        | Workstreams                                                                                                                                                                 | Default outcome                                                                                                                 |
| ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Residual engine census         | [ROE-01](workstreams/01-matrix-sdk-orchestration.md), [ROE-02](workstreams/02-device-verification-trust.md), parts of [ROE-03](workstreams/03-timeline-normalization.md)    | Census and close behavior already owned by Core; record only a concrete remainder                                               |
| Message format and safety      | [ROE-03](workstreams/03-timeline-normalization.md), [ROE-04](workstreams/04-semantic-message-presentation.md), [ROE-12](workstreams/12-validation-sanitization-security.md) | Build shared fixtures first; keep rendering and output-context sanitization platform-side                                       |
| Read and list semantics        | [ROE-05](workstreams/05-read-marker-unread.md), [ROE-06](workstreams/06-room-sorting-filtering.md)                                                                          | Formalize visibility inputs and shared policy if needed; keep viewport math, navigation sections, and locale presentation local |
| Notifications and agent policy | [ROE-07](workstreams/07-notification-critical-policy.md), [ROE-08](workstreams/08-agent-approval-resolution.md)                                                             | Consolidate shared policy only; keep APNs/NSE/tray delivery and cards platform-owned                                            |
| Account data and drafts        | [ROE-09](workstreams/09-notes-account-data.md), [ROE-10](workstreams/10-drafts-replies.md)                                                                                  | Close notes as owned; keep composer state local; examine only typed reply/draft metadata                                        |
| Media metadata                 | [ROE-11](workstreams/11-media-metadata-cache.md)                                                                                                                            | Remain subordinate to ADR 0005; never reopen the generic byte/path envelope                                                     |

## Portfolio priors

These are evidence-based defaults, not implementation commitments. A research
memo may overturn one only with current source and test evidence.

| ID     | Prior                                        | Bounded residual question                                                                                                                                                                         |
| ------ | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ROE-01 | Already owned                                | Is any shipped desktop or iOS behavior still a second Matrix lifecycle/crypto engine rather than a thin adapter?                                                                                  |
| ROE-02 | Already owned                                | Does iOS still lack a device-key continuity or lifecycle input required by the existing Core verification state machine?                                                                          |
| ROE-03 | Extend existing rows only                    | Is a protocol-semantic relationship missing from `TimelineViewRow`? Never add a parallel normalization layer or move viewport behavior.                                                           |
| ROE-04 | Stay platform-side by default                | Can shared Matrix/Hermes fixtures remove observed renderer drift? Consider small row fields only if fixtures prove a security-relevant semantic gap.                                              |
| ROE-05 | Bounded remainder                            | Does the platform-to-Core visibility contract need formalization? Core owns counts and receipt writes; platforms own visibility measurement.                                                      |
| ROE-06 | Split ownership; census existing Core policy | Are existing Core predicates/sort helpers the shared semantic owner, and do both clients consume them? Keep navigation sections and locale/collation presentation local.                          |
| ROE-07 | Policy yes, delivery no                      | Is shared eligibility/privacy/deduplication policy missing? APNs, NSE, tray, banners, and actions remain platform integrations.                                                                   |
| ROE-08 | Highest-value real residual                  | Which desktop/iOS approval classifiers or planners still compete with `app/agent_approvals.rs`, and when does the current goal graph permit their removal?                                        |
| ROE-09 | Already owned                                | Do both clients consume the existing Core schema and CRUD path without a second notes engine?                                                                                                     |
| ROE-10 | Split ownership                              | Core may own reply metadata and a wire-neutral durable draft schema; Slate/Swift editor state and ordinary local composer bodies stay platform-side absent a cross-device rich-draft requirement. |
| ROE-11 | Metadata only                                | Is shared cache eligibility or integrity metadata missing from the native handle owner? Paths and bytes remain on ADR 0005 channels.                                                              |
| ROE-12 | Shared rules and fixtures, not one sanitizer | Which protocol bounds are truly identical? DOM/React and Swift attributed-text output sanitization remains platform-specific.                                                                     |

The 2026-09-01 census completed the initial ownership research. Its promotion
review separated closed ownership conclusions from unresolved defects and
proof. The current risk order and acceptance evidence are maintained in
[program/ACTIONS.md](program/ACTIONS.md); that register is not a new phase
queue or implementation authorization.

## Memo-first workflow

1. Read the governing decisions, this charter, and the assigned briefs.
2. Perform a current source/test census across Rust, desktop, and iOS.
3. Write a short memo from [RESEARCH-MEMO-TEMPLATE.md](RESEARCH-MEMO-TEMPLATE.md)
   under [`memos/`](memos/README.md).
4. Recommend one of: already correctly owned; stay platform-side; extract a
   bounded subset; proceed with Core ownership; or requires a product/ADR
   decision.
5. Stop. Do not design APIs or delivery slices for a recommendation of
   already owned or stay platform-side.
6. A full [implementation plan](PLAN-TEMPLATE.md) under [`plans/`](plans/README.md)
   is allowed only after a proceed recommendation is accepted by a human and
   any required ADR amendment or replacement is accepted.

Research PRs are docs-only under `docs/future-projects/**`. They must not add
Core commands, DTOs, UniFFI APIs, feature flags, or product code.

## Completed research run

The docs-only census ran on `feature/rust-ownership-residual-census` on
2026-09-01. Its durable outcome, review provenance, and unresolved action
register are recorded under [`program/`](program/README.md). The time-bound
operating protocol is historical and does not authorize new work. The branch,
these documents, and accepted memo verdicts do not override ADRs or the goal
graph and must not be treated as authorization to change product code.

## Message-format decision ladder

`TimelineViewRow` is already the semantic model at the event/row layer.
Shared fixtures should precede shared types:

1. Create a shared golden and adversarial corpus of legitimate Matrix and
   Hermes formatted bodies and run both platform renderers against it.
2. If the corpus proves material semantic drift, consider small structured row
   fields such as validated links, mentions, spoiler reason, or a reply-fallback
   flag.
3. Consider a full Core message AST only if bounded fields cannot resolve the
   proven drift. That would change part of ADR 0004 and requires an accepted ADR
   amendment or replacement before API, DTO, or UniFFI design.

Matrix inbound rich text is HTML, not Markdown. A Core AST would also incur
schema churn, serialization and 1 MiB envelope pressure while leaving
selection, accessibility, syntax highlighting, and output-context safety in
the presenters. It is therefore a deliberately high bar, not the default
architecture.

## Closed boundaries

- No second Core, Matrix engine, or timeline-normalization layer.
- No UI layout, widgets, gestures, colors, typography, animation, viewport
  math, platform selection, or locale/collation presentation in Core.
- No Slate or Swift editor state in Rust absent an accepted product requirement
  and boundary decision.
- No APNs, NSE, tray, banner, or notification-card delivery in Core.
- No DOM or attributed-string sanitizer masquerading as a universal sanitizer.
- No passwords, recovery material, local paths, or media bytes on the generic
  Core envelope. ADR 0005's dedicated media channel remains authoritative.
- No shared-Core phase, scoreboard, release-gate, or acceptance-state change may
  originate from this directory; follow the current goal graph.

## Non-goals

- Reducing TypeScript or Swift line count for its own sake.
- Making the clients render identically instead of natively and accessibly.
- Replacing React, SwiftUI, Slate, WebKit, Prism, or OS integration APIs as a
  shortcut for reducing non-Rust code. Those current technology choices may be
  reconsidered only through a separately chartered product/architecture case.
- Rewriting Node build/governance tooling or adopting a Rust UI framework as
  part of this residual-ownership portfolio.
