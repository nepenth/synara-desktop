# ROE-04: Semantic Message-Format Presentation Models

Hypothesis: a versioned, UI-neutral semantic tree may improve faithful and
consistent rendering without moving actual rendering into Rust.

This workstream potentially conflicts with ADR 0004's decision to keep
Markdown/HTML rendering in TypeScript. The plan must distinguish Matrix
protocol semantics from HTML sanitization and platform presentation, and must
request an ADR change if that accepted boundary truly needs to move.

Investigate:

- Matrix formatted-body rules, allowed HTML, plaintext fallback, mentions,
  replies, spoilers, lists, tables, links, code, edits, and malformed input;
- nodes such as paragraph, text, strong, emphasis, inline code, code block,
  table, reply, spoiler, link, list, mention, and line break;
- preservation of unknown constructs and safe fallback behavior;
- whether Core should emit semantic nodes, sanitized protocol tokens, or only
  normalized event content;
- compatibility with selection, copying, accessibility, Dynamic Type,
  localization, Prism highlighting, and platform-native widgets.

Minimum proof: corpus/golden tests from legitimate Matrix and Hermes messages,
malformed/adversarial HTML tests, cross-language serialization tests, native
React and SwiftUI renderer conformance, accessibility checks, and visual
regression evidence in light/dark modes.
