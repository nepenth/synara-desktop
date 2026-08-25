# Hermes Matrix rendering contract

This document records the producer-to-client contract audited against the
Hermes Agent Matrix adapter. It is a compatibility boundary, not a parallel
message format owned by Synara.

## Producer contract

Hermes sends normal agent text as `m.room.message` with `msgtype: m.text`.
The `body` is the Markdown source. When Markdown produces distinct rich
output, Hermes also sends `format: org.matrix.custom.html` and a sanitized
`formatted_body`.

| Hermes behavior | Matrix wire shape | Synara behavior |
| --- | --- | --- |
| Rich text | Markdown `body` plus sanitized `formatted_body` | Shared core projects SDK-sanitized HTML without rewriting it; clients render rich HTML with the plain body as fallback. |
| Edits | `m.replace` relation with the new body under `m.new_content` | Matrix SDK edit aggregation supplies the latest body and edited state. |
| Replies | `m.in_reply_to` relation | Shared core projects the parent preview and clients expose navigation to it. |
| Threads | `m.thread`, root event ID, fallback reply | Shared core projects thread summaries and focused thread navigation. |
| Mentions | `m.mentions` plus `matrix.to` links in rich HTML | SDK notification semantics remain authoritative; links are sanitized and rendered. |
| Long output | Roughly 4,000-character message chunks; every chunk remains in the thread, only the first repeats reply fallback | Each chunk remains a separate Matrix event, preserving order and relations without inventing client-only concatenation. |
| Media | `m.image`, `m.file`, `m.audio`, or `m.video`; caption or filename in `body`; encrypted media uses `file`, otherwise `url` | Shared core projects the media and caption; clients retain authenticated/decrypted download ownership. |
| Voice | `m.audio` plus `org.matrix.msc3245.voice` | Rendered as audio; the extension flag does not replace the standard media fallback. |
| Approval prompt | Ordinary rich `m.text` beginning `⚠️ **Dangerous command requires approval**`, with a fenced command and inline-code actions | The exact event is rendered normally. Approval classification and decisions are separate shared-core policy and never depend on displayed HTML alone. |

Hermes currently emits headings, paragraphs, strong/emphasis, inline and
fenced code, strikethrough, safe links, blockquotes, ordered/unordered lists,
horizontal rules, tables, line breaks, and sanitized `details`/`summary`
metadata supplied by progress messages. Desktop and iOS tests cover this
vocabulary, including the exact approval payload emitted by Hermes.

On iOS, ordinary rich text is imported as sanitized HTML into a typed run
model. It is never converted to Markdown and parsed a second time. This is a
correctness boundary: literal `**`, `~~`, or backticks in an HTML text node
remain literal, while actual `<strong>`, `<del>`, and `<code>` elements retain
their semantics. Bold, emphasis, strike, underline, code, heading hierarchy,
superscript/subscript, strict Matrix foreground/background colors, and
revalidated safe-link attributes cross into SwiftUI. Images become their
accessible `alt`/`title` fallback without resource loading. Code blocks,
blockquotes, tables, spoilers, and `details` blocks retain purpose-built
presentation; table cells use the same typed rich-text runs so inline code and
links are not flattened. Spoilers are concealed from both display and
accessibility text until an explicit accessible reveal action. `details`
starts collapsed and recursively retains nested lists, quotes, code, spoilers,
tables, and details in source order.

## Compatibility rules

- Prefer SDK-sanitized `formatted_body` when present and non-empty; preserve
  `body` as the accessible and failure fallback.
- Never execute, classify, or authorize from arbitrary HTML. Approval actions
  re-read the exact Matrix event in shared core, enforce the five-minute
  window, validate the prompt contract, and submit only an allowlisted
  reaction.
- Do not concatenate Hermes chunks in the client. Separate Matrix events carry
  independent timestamps, relations, edits, reactions, and delivery state.
- Preserve edits as edits rather than displaying the `* ` compatibility body
  as a second message.
- Sanitize links and HTML again at the client presentation boundary. Rich
  content must not introduce scripts, unsafe URL schemes, or event handlers.
- Enforce the Matrix v1.19 HTML boundary identically on desktop and iOS:
  discard `mx-reply` with its contents, reject relative and non-allowlisted
  hyperlink schemes, never load inline-image resources and retain only their
  accessible fallback text, preserve numeric `ol[start]`, accept color attributes only as
  exact `#RRGGBB`, retain only `language-*` code classes, and emit no tags
  deeper than 100 levels. Quoted and unquoted safe absolute link attributes
  follow the same validation. Unsupported or empty rich presentation falls
  back to `body`.
- Preserve semantic block boundaries deterministically: one visible line
  between list items, one blank line between paragraphs, and no duplicate
  break for Hermes' pretty-printed `<br>\n` form.

## Upstream dependency found during the audit

Hermes stores an approval prompt's `requester_user_id` only when its caller
provides that metadata. The current gateway call path supplies thread metadata
but does not supply the requester. In a room with multiple allowlisted users,
Hermes therefore validates the reactor against the allowlist but cannot also
require the original requester. Synara cannot safely repair that producer-side
identity gap from a Matrix notification; it requires a separate Hermes fix and
multi-user regression test before multi-user approval rooms should be treated
as production-proven.

The Hermes HTML producer also differs from the current Matrix v1.19 contract
in three places: it currently accepts relative and `matrix:` hyperlinks, does
not enforce the 100-level tag nesting limit, and does not preserve an ordered
list's `start` value. Synara safely removes the non-conforming link targets and
bounds nesting, preserving readable link labels and body fallback. A lost
ordered-list start value cannot be reconstructed by any client. Exact
producer-to-client parity therefore requires a separate Hermes producer change
and golden wire-fixture test before release can be described as fully proven.
