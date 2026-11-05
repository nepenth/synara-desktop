# Composer formatting model

The desktop room composer and the message editor share one Slate editor
(`synara/src/app/components/editor`). This document is the contract for how its
formatting behaves. `components/editor/__tests__/formatting.test.ts` follows it
case by case and `synara/e2e/composer-formatting.spec.ts` drives the main paths
with real key presses; change them together.

## Principles

1. **One schema, enforced by normalization.** Every edit path (typing, paste,
   drafts, toolbar, undo) ends in a valid tree because `withFormatting`
   normalizes it, not because each command is careful.
2. **Behaviour lives in editor methods, not keydown handlers.** Slate routes
   Enter, Shift+Enter and Backspace through `insertBreak`, `insertSoftBreak` and
   `deleteBackward` for every input path (keyboard, IME, autocorrect,
   `beforeinput`). Keydown handles only shortcuts with no input event: Tab,
   Shift+Tab, Escape and the formatting hotkeys.
3. **Commands act on every block in the selection**, the same way for every
   block type.
4. **One rule per key, the same in every block type.** A line break never
   leaves a block unless the line is empty; Backspace at the start of a block
   removes that block's formatting before it joins lines.

## Schema

Top level holds blocks only:

| Block | Children |
| --- | --- |
| `paragraph` | inline content |
| `heading` (`level` 1–3) | inline content |
| `block-quote` | `quote-line`+ (inline content) |
| `code-block` | `code-line`+ (one plain text leaf: no marks, links, mentions or emoji) |
| `ordered-list` (`start` ≥ 2 optional) / `unordered-list` | `list-item`+ |
| `list-item` | `paragraph`, then zero or more lists (its sub-items) |

"Text blocks" are the blocks that hold inline content: paragraphs, headings,
quote lines, code lines and the paragraph inside each list item. Commands and
keys work on text blocks.

Normalization also:

- joins a list, quote or code block with a directly following one of the same
  type (a numbered list keeps the first one's `start`);
- removes containers left without children, and keeps at least one paragraph in
  the document;
- converts anything in the wrong place to the nearest valid shape (stray inline
  content is wrapped in a paragraph, a stray line or item becomes a paragraph, a
  second paragraph inside a list item becomes the next item).

## Keys

"Line break" is Shift+Enter, and Enter when Enter does not send.

| Where | Line break | Backspace at the start |
| --- | --- | --- |
| Paragraph | New paragraph. In Markdown mode, a line starting with `- `, `* `, `+ ` or `N. ` continues the list on the next line (`N+1. `); on a line holding only the marker it clears the marker instead. | Joins onto the end of the block above (Slate default). |
| Heading | Splits the heading. A new line that would be empty is a paragraph, so a heading never continues by accident. | Becomes a paragraph. |
| List item, non-empty | Next item at the same level; the text after the caret moves into it, with any sub-items. | Nested item: moves out one level. Top-level item: becomes a paragraph, splitting the list; the items below keep their numbers. |
| List item, empty | Nested: moves out one level. Top level: leaves the list as a paragraph in its place, splitting the list; the items below keep their numbers. | (as above) |
| Quote line | New quote line. On an empty last line: leaves the quote as a paragraph after it. | First line: becomes a paragraph before the quote. Other lines: joins onto the line above. |
| Code line | New code line. On an empty last line: leaves the code block as a paragraph after it. | First line: becomes a paragraph before the block. Other lines: joins onto the line above. |

With a range selected, a line break first deletes the selection. Deleting
everything leaves a single empty paragraph.

| Key | Action |
| --- | --- |
| Tab / Shift+Tab | In a list: indent under the previous item / outdent one level. Outdenting keeps reading order: the items that followed become the outdented item's sub-items. Outside a list Tab keeps its focus-moving role. |
| Escape, Mod+E | Removes active inline marks; with none active, turns the current blocks back into paragraphs. Escape does this only when the composer did not use it (closing a suggestion list, cancelling a reply, closing the message editor). |
| Mod+B / I / U / S / [ / H | Toggle bold, italic, underline, strikethrough, inline code, spoiler. Disabled in code blocks. |
| Mod+7 / 8 / ' / ; | Toggle numbered list, bulleted list, quote, code block. |
| Mod+1 / 2 / 3 | Toggle heading level. |

## Block commands

`toggleBlock(format)` applies to every text block the selection touches:

- If every one already has the format (for lists: is an item of that list type),
  they become paragraphs, lifted out of their containers. Containers split
  around them; a numbered list below a split keeps counting.
- Otherwise every one becomes that format. Blocks are first lifted out of other
  containers, then wrapped together: several selected lines make one list, one
  quote or one code block. Code lines keep their text and drop marks and
  inline elements.
- A numbered list that ends up directly under another numbered list joins it.
  A new top-level numbered list continues from the nearest numbered list above
  it, across paragraphs, quotes and code blocks; a heading or a bulleted list in
  between restarts it at 1.

Toolbar buttons keep the editor focused (they act on mouse down without taking
focus), so the selection they act on is the one the user sees.

## Empty composer and drafts

A document whose only content is empty text blocks, with no mention, emoji or
command chip, is empty: it is not saved as a draft, and stored drafts like it
are dropped when the composer opens.

## Serialization

- Matrix HTML: `paragraph` → `text<br/>`, `heading` → `<hN>`, quote →
  `<blockquote>line<br/>…</blockquote>`, code → `<pre><code>`, lists →
  `<ol start>`/`<ul>` with `<li><p>text</p>sub-lists</li>`.
- Plain-text `body`: quote lines as `> line`, list items as `- ` or `N. ` with
  two-space indentation per level, spoilers as `[spoiler]`.
- HTML read back for editing and paste: `<br>` inside a paragraph starts a new
  paragraph; each line of a list item (`<br>` or several `<p>`) becomes its own
  item, since every line in a list carries a marker; lists, headings and code
  inside a quote become quote lines (list lines keep a `- ` or `N. ` prefix);
  unsupported tags keep their text.
