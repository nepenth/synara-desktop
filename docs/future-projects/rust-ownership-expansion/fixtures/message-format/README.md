# Shared message-format corpus

This corpus is the cross-client semantic and security contract for Matrix
`body` plus optional `formatted_body`. It is deliberately not a rendering
snapshot and does not define typography, Prism tokens, Dynamic Type, table
chrome, or any other platform-native presentation detail.

`corpus.json` is consumed by three independent harnesses:

- Rust/Core proves the formatted field remains untrusted protocol content and
  enforces the shared outbound formatted-body byte cap.
- Desktop proves the DOM/React presentation sanitizer preserves readable
  semantics while removing executable or navigable unsafe content.
- iOS proves the Swift parser produces the same readable/security outcomes
  before SwiftUI presentation.

Each case supplies literal HTML or a bounded generator. Expectations compare
only plaintext visibility, accepted link schemes, spoiler recognition,
fallback behavior, and forbidden executable/resource fragments. A new
security-relevant semantic difference must first be represented here; it does
not by itself authorize a shared renderer or presentation AST.
