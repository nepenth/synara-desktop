# Shared message-format corpus

This corpus is the cross-client semantic and security contract for Matrix
`body` plus optional `formatted_body`. It is deliberately not a rendering
snapshot and does not define typography, Prism tokens, Dynamic Type, table
chrome, or any other platform-native presentation detail.

`corpus.json` is consumed by three independent harnesses:

- Rust/Core proves the formatted field remains untrusted protocol content and
  enforces the shared outbound formatted-body byte cap.
- Desktop runs the sanitized DOM through the pure node-decision projection
  consumed by the React presenter, including inert `mxc://` inline-image
  fallback, and proves readable semantics while removing executable or
  navigable unsafe content.
- iOS proves the Swift parser produces the same readable/security outcomes
  before SwiftUI presentation, including the rendered ordinals, nesting, and
  bullets of list fixtures rather than merely retaining their source tags.

Each case supplies literal HTML or a bounded generator. Expectations compare
only plaintext visibility, accepted link schemes and exact mention targets,
preserved structural semantics, exact inline/preformatted code content,
ordered-list starts, bounded spoiler reasons, fallback behavior, and forbidden
executable/resource fragments. The top-level `coverage` register makes every
required presentation/security area point to at least one real fixture, and
all three harnesses reject stale fixture identifiers.

Redaction, late-decryption replacement, pagination overlap, and relation
ordering are timeline-sequencing semantics, not formatted-body sanitization.
They are proved separately by the pinned-SDK/Core sequencing suite in
`crates/synara-core/tests/p4_s37_timeline_sequencing.rs`; they are intentionally
not reimplemented in this corpus or either presenter. Live homeserver and
cross-client interoperability evidence remains a distinct release gate.
A new security-relevant semantic difference must first be represented here;
it does not by itself authorize a shared renderer or presentation AST.
