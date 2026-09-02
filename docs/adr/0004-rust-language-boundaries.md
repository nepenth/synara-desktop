# ADR 0004: Rust and Platform Ownership Boundaries

Originally accepted: 2026-08-17.

Last reviewed: 2026-09-01.

Status: accepted and clarified. This revision separates durable boundaries from
implementation-era sequencing and current technology preferences.

Companion to [ADR 0003](0003-shared-native-rust-core.md), which establishes one
shared Rust application Core. This ADR decides which behavior belongs in that
Core and which behavior remains platform-owned.

## Context

Synara has two UI shells over one Matrix/application engine:

- macOS and Linux use Tauri 2 plus a React/TypeScript presenter;
- iOS uses SwiftUI through project-owned UniFFI bindings;
- `crates/synara-core` owns shared Matrix and product authority through
  matrix-rust-sdk.

Sharing every line of implementation is not the goal. Correctness and parity
come from sharing authoritative decisions while each platform retains the
observations, rendering, accessibility, and operating-system behavior that only
it can implement well.

## Decision framework

Classify a behavior before choosing a language or API:

| Kind        | Default owner | Examples                                                                                                                        |
| ----------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Authority   | Core          | Protocol truth, state machines, eligibility, shared schemas, resource bounds, ordering, persistence policy, and Matrix writes   |
| Observation | Platform      | Viewport/focus, keystrokes, app lifecycle, notification delivery state, local files, permissions, and locale UI context         |
| Rendering   | Platform      | React/SwiftUI widgets, layout, typography, animation, selection, accessibility, Dynamic Type, syntax highlighting, and gestures |

A platform observation may be a typed input to Core authority without Core
owning the observation. A platform projection of a Core model is not duplicated
authority.

### Put behavior in Core when

Most of these are true:

- desktop and iOS require the same semantic result;
- the behavior is protocol-, security-, trust-, or correctness-sensitive;
- it needs one concurrency, persistence, or retry owner;
- it operates over large Matrix state more efficiently before FFI/IPC;
- it can be proven with Rust unit/property tests and Synapse integration tests;
- leaving it in presenters would create competing product policy.

Examples include Matrix lifecycle, crypto and verification states, timeline
event relationships, receipt eligibility/writes, account-data schemas,
notification eligibility, and agent-approval authorization policy.

### Keep behavior platform-owned when

Any of these dominate:

- it is UI composition, rendering, accessibility, interaction, or viewport
  behavior;
- it is an OS API or lifecycle integration such as Keychain, APNs/NSE, tray,
  windows, file dialogs, permissions, or haptics;
- it is an observation that Core cannot independently know, such as whether a
  message is genuinely visible;
- a mature platform library owns editor or renderer state and crossing the
  boundary would duplicate that state;
- IPC/FFI schema churn and serialization cost exceed the parity benefit.

Examples include Slate and Swift editor state, React/SwiftUI message cells,
scroll virtualization, text selection, notification buttons, and native file
handoff.

## Hard invariants

These restrictions protect correctness, security, or an accepted platform
architecture:

1. **No second Matrix engine.** Swift and TypeScript must not independently own
   session, sync, crypto, room/timeline state, account data, or Matrix writes.
2. **No UI framework in Core.** Core does not prescribe platform widgets,
   layout, gestures, typography, accessibility trees, or viewport geometry.
3. **No generic-envelope secret or byte transport.** Passwords, recovery
   material, OAuth client secrets, local filesystem paths, and media/attachment
   bytes must not use the generic `Core::command` JSON envelope. Narrow typed
   platform or dedicated byte APIs are allowed when separately designed and
   bounded.
4. **No universal output sanitizer claim.** Core may validate protocol fields,
   URLs, identifiers, sizes, and semantic invariants. React DOM and Swift
   attributed-text renderers still sanitize/escape for their own output
   contexts.
5. **No permanent dual owner.** A migration must name the old authority being
   removed and prove the cutover. A feature flag cannot become an indefinite
   second implementation.
6. **NSE remains narrow.** The iOS notification service extension must not boot
   the full sync engine or become an independent Matrix client.

[ADR 0005](0005-native-media-handle-channel.md) applies invariant 3 to media
through opaque handles and dedicated native byte channels.

## Current layer map

| Layer                                                                      | Authoritative owner                                                       | Platform responsibility                                                                   |
| -------------------------------------------------------------------------- | ------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Matrix session, sync, crypto, verification, room/timeline state and writes | `crates/synara-core`                                                      | Credentials and lifecycle observations through typed adapters                             |
| Desktop native shell                                                       | `src-tauri/`                                                              | Windows, tray, notifications, Keychain/Secret Service, files, updater and bridge adapters |
| Desktop presentation                                                       | `synara/` React/TypeScript                                                | Navigation, settings, Slate composer, rendering, selection and virtualization             |
| iOS presentation and services                                              | `synara-ios/` Swift                                                       | SwiftUI, Keychain, APNs/NSE, Apple permissions, files/media UI and App Store lifecycle    |
| Timeline semantics                                                         | Core `TimelineViewRow` and relationship owners                            | Native grouping, cells, scroll behavior and visual treatment                              |
| Message formatting                                                         | Core may expose protocol fields, validation and bounded semantic metadata | Platform parses/sanitizes for its output context and performs native rendering            |
| Notifications                                                              | Core push rules and shared eligibility/privacy/deduplication policy       | APNs/NSE/tray delivery, banners, actions, badges and haptics                              |
| Agent/Hermes workflows                                                     | Core recognition, eligibility, expiry and action resolution where shared  | Cards, sheets, composer and notification UI                                               |
| Notes/account data                                                         | Core schema, normalization and Matrix synchronization                     | Editors, drag/reorder affordances and presentation                                        |
| Media                                                                      | Core metadata/policy and opaque handles                                   | Dedicated native byte transfer, filesystem paths, caching integration and display         |
| Build/release governance                                                   | Current Node/shell/Rust/CI tooling                                        | Repository automation; not product runtime                                                |

## Message-format boundary

Matrix rich-message input is formatted HTML with plaintext fallback. Core
currently projects it on typed timeline rows; that field is untrusted protocol
content, not universally sanitized rendering markup.

Shared golden/adversarial fixtures and small protocol-semantic row fields comply
with this ADR. A complete paragraph/code/table/reply/spoiler presentation AST
would materially change the existing boundary because it moves broad parsing
and versioned presentation semantics into Core. It requires an explicit ADR
amendment or replacement, evidence that bounded fields cannot solve the
problem, and accounting for schema/serialization cost. Even then, rendering,
selection, accessibility, syntax highlighting, and output-context sanitization
remain platform-owned.

## Current technology choices, not hard invariants

React, SwiftUI, Slate, pdf.js, Prism, and Node governance scripts remain the
preferred implementations because they fit their current jobs. Rewriting them
in Rust solely to reduce TypeScript/Swift/Node line count is not justified.

These choices may be reconsidered through a product and architecture decision
if requirements or economics materially change. They are intentionally not
treated as security invariants equivalent to one Matrix owner or safe byte and
secret boundaries.

Likewise, Windows, Android, a standalone web client, a Rust desktop UI toolkit,
or Tauri iOS are not current product paths. Adding or replacing a platform
requires a separate ADR; it must not be used to bypass this ownership model.

## Change process

A proposal that crosses this boundary must:

1. identify an observable problem and the earliest duplicated authority;
2. compare leaving ownership unchanged, a bounded extraction, and the broader
   move;
3. account for secrets/bytes, latency, schema versions, lifecycle, failure,
   migration, removal, and rollback;
4. explain how desktop and iOS retain native rendering and accessibility;
5. land an explicit amendment or replacement ADR before implementation.

Current research guidance lives in the
[Rust ownership residual census](../future-projects/rust-ownership-expansion/README.md).
That portfolio does not override this ADR or authorize product work.

## Historical implementation context

The original ADR included P4/P5 sequencing, command counts, leftover routes,
and migration tasks. Those records explain the shared-Core cutover but are not
timeless language rules. Current program status and stop gates live under
[`docs/shared-native-core/`](../shared-native-core/README.md). The architecture
decision must not be used to infer that a historical phase or release gate is
complete.

## Consequences

- Shared correctness policy converges in one testable Core without turning Core
  into a UI framework or OS abstraction layer.
- Platform clients may differ visually and behaviorally where native
  interaction requires it, while consuming the same authoritative state.
- New work must distinguish authority from observation/rendering before adding
  Rust, TypeScript, or Swift owners.
- Dedicated typed channels are legitimate; the generic envelope is not a
  shortcut for secret-, path-, or byte-sensitive work.

## Related decisions

- [ADR 0002 — native iOS architecture](0002-ios-architecture.md)
- [ADR 0003 — shared native Rust core](0003-shared-native-rust-core.md)
- [ADR 0005 — native media handle channel](0005-native-media-handle-channel.md)
