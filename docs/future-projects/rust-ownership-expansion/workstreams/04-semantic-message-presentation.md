# ROE-04: Semantic Message-Format Presentation

Prior: **stay platform-side; shared fixtures before shared types**.

Matrix rich messages arrive as formatted HTML with plaintext fallback. ADR 0004
keeps HTML/Markdown rendering in platform presenters, and `TimelineViewRow`
already supplies the event-level semantic model. A full Core presentation AST
is not the default next layer.

## Bounded research sequence

1. Assemble one golden and adversarial Matrix/Hermes corpus covering replies,
   mentions, spoilers, lists, tables, links, inline/code blocks, edits,
   plaintext fallback, malformed HTML, and unknown constructs.
2. Run the desktop and iOS renderers against that corpus and identify material
   semantic—not merely visual—drift.
3. If drift is security- or protocol-relevant, evaluate small structured
   `TimelineViewRow` fields such as validated links, mentions, spoiler reason,
   or a reply-fallback flag.
4. Consider a full paragraph/text/strong/code/table/reply/spoiler AST only if
   bounded fields cannot solve the proven problem.

## Decision boundary

A full AST changes ADR 0004 and requires a replacement ADR before API, DTO, or
UniFFI design. Its memo must account for schema/version churn, serialization
and 1 MiB envelope costs, unknown HTML fallback, and the fact that native text
selection, accessibility, Dynamic Type, Prism, widgets, and output-context
sanitization still remain in React and SwiftUI.
