# Rich Composer Modernization

Synara's rich composer should author a focused, reliable subset of Matrix rich text, render the
safe Matrix HTML profile broadly, and degrade gracefully when editing or pasting structures the
composer cannot fully model.

## Scope

The modernization work is split into phases so each change can be accepted independently.

### Phase 1: Core correctness

Requirements:

- Ordered lists render without clipping two-digit markers.
- Ordered lists preserve `ol start` when parsing existing formatted messages.
- Ordered markdown supports multi-digit starts such as `10. item`.
- Ordered-list plain-text fallbacks use the correct visible number.
- Spoiler plain-text fallbacks do not reveal spoiler text.
- Link elements serialize with their visible text rather than object placeholders.
- Composer action controls do not permanently reserve horizontal space in multiline drafts.

Acceptance criteria:

- A composed ordered list displays `8, 9, 10, 11, 12` correctly in the editor and rendered message.
- Editing or quoting a message with `<ol start="10">` keeps the list starting at `10`.
- Sending `10. alpha\n11. beta` in markdown mode produces an ordered list starting at `10`.
- The `body` fallback for a spoiler message does not contain the hidden spoiler content.
- Rich links emit valid `<a href="...">label</a>` HTML and readable plain fallback text.
- A long multiline composer draft can wrap under the top-right action buttons.

### Phase 2: Matrix HTML policy

Status: Core profile implemented.

Requirements:

- Define a single documented Matrix HTML profile for inbound sanitization, outbound emission, and
  rendering.
- Keep inbound legacy tolerance where needed, but emit current-spec HTML for new messages.
- Separate Synara-only edit hints such as `data-md` from Matrix-facing output wherever possible.

Acceptance criteria:

- Sanitizer, renderer, and serializer supported tags/attributes are documented in one place.
- Outbound formatted messages only include supported Matrix-safe tags and attributes.
- Existing historical messages using legacy tags still render safely.

Implementation notes:

- The source of truth lives in `src/app/utils/matrixHtmlProfile.ts`.
- `emittedTags` and `emittedAttributes` describe new Synara-authored Matrix HTML.
- `allowedTags` and `inboundAttributes` are intentionally wider so historical or third-party
  messages using legacy tags remain safe and readable.
- Synara edit hints such as `data-md` are tolerated inbound, but stripped from outbound composer
  HTML before sending.

### Phase 3: List and block editing UX

Status: Core list behavior implemented.

Requirements:

- Implement predictable Enter, Shift+Enter, Backspace, Tab, and Shift+Tab behavior for lists,
  quotes, and code blocks.
- Support nested ordered and unordered lists.
- Preserve list item structure across paste, edit, send, and render.

Acceptance criteria:

- Empty list item + Enter exits the list.
- Shift+Enter inserts a soft line break without leaving the current block.
- Tab indents and Shift+Tab outdents list items without corrupting surrounding content.
- Nested lists round-trip through send and edit.

Implementation notes:

- List items now support a stable nested-list Slate shape by promoting mixed inline/nested content
  into paragraph-plus-list children.
- Ordered and unordered nested lists serialize to Matrix HTML and readable plain-text fallbacks.
- Tab and Shift+Tab are wired to explicit list-item indent/outdent transforms with golden coverage.

### Phase 4: Paste and edit round-trip

Requirements:

- Preserve safe links, lists, quotes, code blocks, mentions, and spoilers when editing rich messages.
- Degrade unsupported structures such as tables and details/summary into readable editable content
  without losing semantic text.

Acceptance criteria:

- Pasting from browser/Google Docs/plain text produces valid Slate content.
- Editing a supported rich message and saving without changes does not rewrite it unnecessarily.
- Unsupported structures remain readable and safe after edit.

### Phase 5: Composer UI and accessibility

Requirements:

- Use a dedicated room composer shell instead of overloading the generic editor layout.
- Toolbar buttons have accessible labels, pressed state, and predictable keyboard focus order.
- Mobile and narrow layouts avoid overlap between text and floating actions.

Acceptance criteria:

- Keyboard-only users can reach every composer action and understand its current state.
- Narrow windows and long drafts preserve usable text width and visible caret behavior.
- Attachment, emoji, GIF, poll, formatting, and send actions remain reachable in all composer states.

### Phase 6: Golden tests

Requirements:

- Add tests for Slate to Matrix content, Matrix HTML to Slate, markdown interop, and edit round-trip.

Acceptance criteria:

- Golden cases cover lists, `ol start`, nested lists, links, spoilers, mentions, code blocks, quotes,
  headings, and markdown mode.
- Tests fail on malformed `[object Object]` output, revealed spoiler fallbacks, or lost list starts.
