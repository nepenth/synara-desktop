# ROE-04 / ROE-12 Research Memo: Message format and output-context safety

Status: boundary accepted; fixture/security proof reopened; docs-only; not approved for implementation.

| Field              | Value                                                                                                                                                                                                                                                                                                                                 |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Workstream/cluster | ROE-04 / ROE-12                                                                                                                                                                                                                                                                                                                       |
| Research owner     | Residual-census researcher (message-format / safety lane)                                                                                                                                                                                                                                                                             |
| Reviewers          | Independent feature-branch review; `ACCEPT_WITH_NITS` on PR `#1089` at `91bf0b14`                                                                                                                                                                                                                                                   |
| Source census      | 2026-09-01 on worktree `roe/memo-04-message-format` at `1b59d12f28e8bb75e7da1462591e7bb6547299ac`. [CENSUS.md](../program/CENSUS.md) is a `main` `011cf39a` snapshot only; every path below was re-read on this commit. Source wins where the snapshot is thinner.                                                                 |
| ADR baseline       | [ADR 0001](../../../adr/0001-ios-repository-layout.md), [0002](../../../adr/0002-ios-architecture.md), [0003](../../../adr/0003-shared-native-rust-core.md), [0004](../../../adr/0004-rust-language-boundaries.md), [0005](../../../adr/0005-native-media-handle-channel.md); last reviewed 2026-09-01; source commit as above. |

## Observable problem

Users read the same Matrix event on desktop and iOS: a required plaintext
`body` plus optional `formatted_body` HTML (replies, mentions, spoilers, lists,
tables, links, code, edits, Hermes approval markup). The portfolio prior is
that each presenter parses and sanitizes that HTML for **its** output context.
The residual questions are:

1. **ROE-04.** Can a shared golden/adversarial Matrix/Hermes corpus remove
   *observed* renderer drift? Should any small `TimelineViewRow` field be added
   only if that corpus proves a security-relevant semantic gap? A
   paragraph/text/strong/code/table AST is not the default and is an
   ADR-gated stop.
2. **ROE-12.** Which bounds are truly identical protocol authority, and which
   are renderer escaping? DOM/React and Swift attributed-text sanitization must
   stay platform-specific. The Core field comment must not be readable as
   “already universally safe.”

This memo does not ask the clients to look the same. Prism, Dynamic Type,
selection, and widgets stay platform rendering. It does not design a Core
message AST or a new byte/path envelope.

## Current ownership census

Re-verified against current source. [CENSUS.md](../program/CENSUS.md) still
correctly names outbound `markdown_to_html()`, inbound `formatted_body` as a
string, desktop `sanitize-html` → React parse + Prism, and iOS
`MatrixHTMLRenderer`. It does not record the two-pass desktop live sanitizer,
the leftover `RenderBody` surfaces, or the exact still-misleading struct
comment. **Source wins** on those details; the snapshot’s “already-sanitized”
warning remains true.

Live room open consumes one Core row. Desktop `RoomView` mounts
`NativeTimelinePresenter` → `NativeFormattedBody`. iOS
`SharedCoreTimelineRows` maps `TimelineViewRowDto.formatted_body` into
`.formattedText` and `RoomTimelineView` calls `MatrixHTMLRenderer.segments`.
Neither adapter invents HTML; neither treats the Core string as a safe DOM or
attributed-string payload.

### Misleading Core comment (still present)

`crates/synara-core/src/app/timeline/view.rs` on this census commit still
documents `TimelineMessageRow.formatted_body` as:

```text
/// Already-sanitized rendering markup; never raw event content.
```

That sentence can be read as universally sanitized rendering markup. It is
not. `project_formatted_body` copies SDK `MessageFormat::Html` when the HTML
is non-empty and distinct from plaintext. It does not call Ruma
`sanitize_html`. Unit test `hermes_approval_html_crosses_the_shared_boundary_unchanged`
asserts the HTML crosses the boundary **unchanged**.

A second, misplaced comment sits on `project_poll_answers` (not on
`project_formatted_body`):

```text
/// Project SDK-sanitized Matrix HTML when present and distinct from plain text.
```

UniFFI `TimelineViewRowDto.formatted_body` has **no** sanitizer claim.
Presenters still sanitize (desktop `prepareNativeFormattedBody`, iOS
`sanitizingMatrixHTMLForNativeImport`). This memo does not edit `crates/**`.

| Concern | Rust/Core | Desktop | iOS | Evidence/tests |
| ------- | --------- | ------- | --- | -------------- |
| Inbound formatted HTML | Authority for the **protocol field only**. `project_formatted_body` copies HTML; no inbound sanitizer. `formatted_body` is untrusted event content, not a presentation AST. | Rendering. Live path: `prepareNativeFormattedBody` = `sanitizeCustomHtml` then `sanitizeMatrixV119PresentationHtml` (`sanitize-html`, nesting 100, 256 KiB UTF-8). `NativeFormattedBody` parses to React; Prism highlight is post-sanitize. | Rendering. `MatrixHTMLRenderer` quote-aware allowlist scanner (`sanitizingMatrixHTMLForNativeImport`), then typed `Segment` / `RichText` for SwiftUI. Same 256 KiB / nesting-100 caps. | `view.rs` `project_formatted_body`, field comment, `hermes_approval_html_crosses_the_shared_boundary_unchanged`; `nativeTimelineRichText.ts`; `nativeTimelineFormattedBody.tsx`; `nativeTimelineCodeHighlight.test.ts`; `TimelineService.swift` `MatrixHTMLRenderer`; `TimelineServiceTests.swift` |
| Outbound formatted HTML | Authority for **attach rule + Markdown helper**. `should_attach_formatted_body` omits redundant HTML. `markdown_to_html` uses Ruma `FormattedBody::markdown` + `HtmlSanitizerMode::Strict` (UniFFI). Live `send_text` then attaches caller HTML via `message_content` without a second inbound-style sanitizer. Edit path caps HTML at 65_536 bytes. | Observation / composer. Slate `toMatrixCustomHTML` + `trimCustomHtml`; editor *load* uses `sanitizeCustomHtml`. Send forwards composer HTML through the Core owner. Shell `normalize_formatted_body` (65_536) exists on the desktop send helper; live Core `send_text` does not repeat that cap. | Observation / composer. `ComposerMatrixFormatting.formattedBody` → `SynaraCore.markdownToHtml` (the Core Strict helper). Send uses `SharedCoreSendText` / `formattedBody`. | `lib.rs` `markdown_to_html`; `actions.rs` `should_attach_formatted_body`; `send/text.rs` `message_content`; `live.rs` `send_text` / `normalize_edit_formatted_body`; `RoomInput.tsx`; `editor/input.ts`; `ComposerMatrixFormatting.swift`; `ComposerMatrixFormattingTests.swift` |
| Reply fallback (`mx-reply`) | Protocol: reply **parent id / preview** already on `TimelineViewRow.reply` (ROE-03). Formatted HTML may still contain `<mx-reply>`. | Rendering. `nonTextTags` includes `mx-reply`; test expects `<p>current</p>` only. | Rendering. `contentDroppingTags` includes `mx-reply`; test expects `"Current message"`. | `nativeTimelineCodeHighlight.test.ts`; `TimelineServiceTests.swift` `testMatrixHTMLRendererStripsReplyFallbackAndCapsTagNestingAtOneHundred` |
| Mentions | Authority on **send**: `validated_mentions` parses `OwnedUserId`. Inbound HTML mention pills are not Core fields. | Rendering. v1.19 pass drops `data-mx-pill`; `https://matrix.to/#/@user` hrefs kept if scheme allowlisted. | Rendering. Same schemes; Hermes `matrix.to` mention kept, `javascript:` dropped. | `send/text.rs` `validated_mentions`; `nativeTimelineRichText.ts`; `TimelineServiceTests.swift` `testMatrixHTMLRendererPreservesExactHermesMentionAndRejectsUnsafeLink` |
| Spoilers | No Core spoiler field. HTML `data-mx-spoiler` is protocol content. | Rendering. `NativeSpoiler` reveal button; reason trimmed to 160 chars. | Rendering. Typed `SpoilerBlock`; concealed until reveal; selection omits spoiler text until opt-in. Reason is not length-capped in the scanner. | `nativeTimelineFormattedBody.tsx`; `TimelineService.swift` `selectionProjection`; `TimelineMessageCopyTests.swift` |
| Links / schemes | No Core URL AST. Identical **intended** allowlist: `https` `http` `ftp` `mailto` `magnet`. `matrix:` and relative hrefs are not links. | Rendering. `absoluteMatrixHref` plus control-character reject (`<= 0x1f` / `0x7f`). Adds `target=_blank` `rel=noreferrer noopener` (DOM tabnabbing). | Rendering. `isSafeMatrixHTMLLink` via `URLComponents`. No `target`/`rel` (no DOM). | `nativeTimelineRichText.ts`; `TimelineService.swift` `isSafeMatrixHTMLLink`; iOS `testMatrixHTMLRendererRetainsEverySafeAbsoluteMatrixLinkScheme` |
| Inline images | Authority: timeline **media** uses opaque handles (ADR 0005), not formatted-HTML `src`. | Rendering. Sanitizer may keep `mxc://` `img`; React replace is alt/title fallback — no webview fetch. | Rendering. Scanner replaces `img` with escaped alt/title immediately; `https` tracker images never load. | `nativeTimelineFormattedBody.tsx`; `TimelineService.swift` img branch; iOS `testMatrixHTMLRendererUsesInlineImageAltTextWithoutImportingAResource` |
| Lists / tables / code / headings | Not Core semantics. `TimelineViewRow` is the event/row model, not a paragraph AST. | Rendering. HTML → React; tables get a scroll region; `pre`/`code` → Prism with char limit. | Rendering. Typed heading/quote/table/details/code segments; exact pre/code whitespace tests; no Prism. | `nativeTimelineFormattedBody.tsx`; `TimelineServiceTests.swift` table/code/heading cases |
| Legacy `<strike>` / `<font>` / colors | Not Core. | Rendering. `strike` → `s`; `font` → `span`; colors must be `#` + 6 hex; case preserved. | Rendering. `font` → `span`; **no `strike` in the allowlist** (text kept, strikethrough lost). Colors uppercased. | `nativeTimelineRichText.ts`; `TimelineService.swift` allowedTags / font case |
| Size / nesting | Send/edit: Core edit + desktop helper cap outbound HTML at 65_536 **bytes of the string**. Live `send_text` does not apply that cap. Presentation: not a Core inbound cap. | Rendering. 256 KiB UTF-8; nestingLimit 100 (deep `<strong>` dropped, text kept). | Rendering. 256 KiB UTF-8; emit suppressed past 100 open tags; text still kept. | `live.rs` `normalize_edit_formatted_body`; `send/product_commands.rs` `normalize_formatted_body`; `MAX_NATIVE_FORMATTED_BODY_BYTES`; `maximumRichHTMLBytes` |
| Agent / Hermes card JSON | Authority. `project_agent_card_json` allowlists recognized objects (`org.hermes.agent`, …) and bounds at 200_000 bytes. Unrecognized event JSON (including tokens) does not cross. Formatted approval **HTML** still crosses as `formatted_body`. | Rendering of the HTML body; card JSON is a separate presenter surface. Approval classifier/action authority is ROE-08 / A2, not this lane. | Same: `SynaraAgentCardPayloadParser` on DTO JSON; HTML via `MatrixHTMLRenderer`. `AgentActionService` runs `sanitizedMarkdown` before classification (ROE-08). | `view.rs` `project_agent_card_json`; `SharedCoreTimelineRows.swift`; `AgentActionService.swift` |
| Leftover desktop HTML surfaces | Not an owner. | Rendering only. Inbox / pin menu / message-search still use `RenderBody` → `sanitizeCustomHtml` (editor profile, not the live v1.19 second pass). Not the live `RoomView` timeline. | No equivalent leftover HTML engine. Search/copy use `MatrixHTMLRenderer`. | `RenderBody.tsx`; `Notifications.tsx`; `RoomPinMenu.tsx`; `SearchResultGroup.tsx`; `RoomView.tsx` |
| Envelope / unknown fields | Authority. Command and stream payloads use `deny_unknown_fields`. Fail-closed deserialize. | Adapter. Tauri JSON must match the typed payload. | Adapter. UniFFI / leftover JSON must match. | `core.rs` request structs; `transport/command.rs`; `transport/stream_body.rs` |
| Canonical IDs | Authority. Room / event / user / thread / txn parse (`OwnedRoomId`, `OwnedEventId`, `OwnedUserId`; txn length 1..=255). Diagnostics do not echo the bad string. | Adapter forwards strings into Core. | Adapter forwards strings into UniFFI. | `send/text.rs`; `send/tests.rs` |
| Notes / account-data bounds | Authority. `MAX_NOTE_BODY_LENGTH` 4000, message-note 1000, 200 items/room; `limit_text` / `validate_note_item`. | Presenter editor only (ROE-09 already owned). | Presenter editor only. | `account_data/room_notes.rs`; `room_notes_live.rs` |
| Filenames / MIME | Authority. Attachment and content-upload reject empty, `>` 255 chars, `/` `\` NUL; MIME parse + 255-byte cap; errors omit the raw value. Timeline `media_filename` is projected, not inferred from `body`. | Shell file dialogs / display names (observation). | Upload display names (observation). | `send/attachment.rs`; `media/content.rs`; `view.rs` `project_media_filename_and_caption` |
| Avatar / media URLs | Authority. `parse_own_avatar_mxc` accepts `mxc://` only. Media **bytes** stay on ADR 0005 channels. | `synara-media://` resolve (not this lane). | UniFFI bytes by handle; Swift also rejects non-`mxc` sender avatars at the mapper. | `user_profile/live.rs`; `SharedCoreTimelineRows.senderAvatarURL`; ADR 0005 |
| Last-message preview | Authority. `sanitize_last_message_preview` trims, bounds, drops `mxc://` and token-like text. | Projection of the Core string. | Projection of the Core string. | `room_list/last_message.rs` |
| Diagnostics redaction | Authority for Core fixture tests; platform logs have their own redactors. | `sanitizeDiagnosticDetail` / desktop route sanitize (`sanitizeDesktopNotificationRoute`) — observation / chrome, not formatted-body. | iOS security-review notes sanitized external media descriptions. | `desktop.ts`; `app/diagnostics/tests.rs` |
| Widgets | No Core widget HTML owner found. | Comment-only “widget APIs” in room-list/ClientRoot; not a second HTML engine. | No widget sanitizer in product Swift. | `roomList.ts`; iOS glob empty |

**Classification.** Protocol identifiers, mention user IDs on send, notes
limits, filename/MIME, avatar `mxc://`, last-message preview, agent-card
object bounds, and envelope `deny_unknown_fields` are **Core authority**
(**hard invariant**: one Matrix/write owner; no generic-envelope secrets/bytes
— ADR 0003/0004/0005). Inbound formatted HTML is **protocol content Core
projects**, not output-safe markup (**hard invariant**: ADR 0004 “no universal
output sanitizer claim”). DOM/React sanitization, Swift attributed-text
sanitization, Prism, spoiler chrome, table scroll, and Dynamic Type are
**platform rendering** (**accepted platform boundary**). React vs SwiftUI vs
`sanitize-html` vs a hand-written Swift scanner are **current technology
preferences**. Viewport/focus remain **platform observation**.

**Earliest actual divergence.** There is no second formatted-body engine and
no Core AST. The earliest *appearance* of one is (1) the Core comment claiming
the field is already sanitized, and (2) two mature presenters implementing the
same Matrix v1.19 allowlist with different libraries. Observed differences
from existing tests (not a shared corpus run) are mostly presentation:
legacy `<strike>` styling on iOS, color hex case, spoiler-reason length,
DOM-only `rel`/`target`, leftover desktop `RenderBody` using only the editor
profile. Both live presenters drop `javascript:`, `matrix:`, relative hrefs,
`<script>`, `<mx-reply>`, oversize HTML, and remote `https` images.

Playbook §5 and the [goal-graph stop conditions](../../../shared-native-core/13-language-boundary-goal-graph.md)
treat P4-S16 timeline rows as landed and forbid inventing S38 or starting P5.
Docs-only PRs remain allowed. This memo does not claim P4 engine-ready.

## Boundary constraints

- ADR 0003: one Core; TypeScript/Swift adapters are not a second Matrix engine.
- ADR 0004 message-format layer: Core may expose protocol fields, validation,
  and bounded semantic metadata. Platforms parse/sanitize for output context
  and perform native rendering. A full paragraph/code/table/reply/spoiler AST
  requires an accepted ADR amendment or replacement, evidence that bounded
  fields cannot solve a proven problem, and still leaves sanitization in the
  presenters.
- ADR 0004 hard invariant 4: no universal output sanitizer claim.
- ADR 0005: formatted-HTML `img` must not become a second media-byte path.
- ROE-04 prior: stay platform-side; fixtures before types; small row fields
  only after a security-relevant semantic gap.
- ROE-12 prior: share protocol rules and fixtures, not one sanitizer crate.
- ROE-03: `TimelineViewRow` is already the event/row model. Do not add a
  parallel normalization layer or move viewport behavior.
- Playbook §5 / goal graph: do not invent S38; do not start P5; do not
  register leftover secret/byte commands on `Core::command`.
- D3 / D9: extract/proceed is a stop, not a start. No product code tonight.

## Alternatives

1. **No ownership change (stay-put, including “no shared corpus”).** Keep
   Core as the protocol-field projector. Keep desktop `sanitize-html` + React
   and iOS `MatrixHTMLRenderer` as independent output-context sanitizers.
   Rely on each platform’s existing adversarial tests (already covering
   schemes, `javascript:`, nesting, 256 KiB, `mx-reply`, Hermes mentions,
   tables, spoilers). Do not add `TimelineViewRow` presentation fields. Fix
   the misleading comment later in a separately authorized `crates/**` docs
   edit. **Falsified if** a shipped presenter inserts Core `formatted_body`
   into a DOM or `NSAttributedString` without an output-context sanitizer,
   or if desktop and iOS assign different *protocol* meaning (different
   mention user IDs, different reply parent ids, one client treating
   `javascript:` as a navigable link) to the same Core row.

   This is a real option: both live renderers already encode the same Matrix
   v1.19 scheme/tag story, and a corpus file does not by itself change
   ownership. The cost of staying here is slower *detection* of the next
   drift, not a second engine.

2. **Bounded extraction or shared fixture/contract (recommended follow-up
   shape, not a type extract).** Land a shared golden/adversarial corpus
   under `docs/future-projects/**` (paths and shape below) and run **both**
   existing presenters against it. Compare only security/protocol
   expectations (kept schemes, dropped `javascript:`, no remote image load,
   `mx-reply` stripped, oversize → plaintext `body`, nesting bound, mention
   href kept). Do **not** compare Prism tokens, Dynamic Type, or table
   chrome. Consider a small existing-row field (validated link list, mention
   MXID list, spoiler reason, reply-fallback flag) **only** if that run
   proves a security-relevant semantic gap the current string + plaintext
   `body` cannot express. **Falsified as necessary tonight** because no
   shared corpus was executed, independent tests already agree on the
   dangerous cases, and leftover `<strike>` / color-case drift is not
   security-relevant. A universal sanitizer crate is not this option.

3. **Broader Core model (presentation AST or one sanitizer for DOM and
   Swift).** Rejected. It would replace ADR 0004’s message-format boundary,
   serialize a versioned tree through IPC/UniFFI (1 MiB envelope pressure),
   and still leave React/SwiftUI sanitization, selection, and accessibility
   on the platforms. The asked bar is “bounded fields cannot solve a proven
   problem.” That proof does not exist.

Strongest stay-put case: the thick-looking Swift `MatrixHTMLRenderer` and the
desktop `sanitize-html` pipeline are output-context renderers over one Core
string. They are not a second Matrix engine. Sharing a crate that emitted
“safe HTML” would recreate the lie the struct comment already tells.

## Recommendation

**Stay platform-side.**

Confidence: high that ownership is already correct and that a Core AST or
universal sanitizer is not justified. Medium that a later corpus run will
ever prove a security-relevant row-field gap; that remains a candidate, not
a remainder to extract tonight.

Supporting evidence:

- Core projects `formatted_body` unchanged; presenters on **both** clients
  still sanitize before React DOM or Swift attributed text.
- Live desktop room does not mount leftover `RenderBody`; live iOS room uses
  `MatrixHTMLRenderer` only.
- Identical protocol allowlist intent for link schemes, `mx-reply` drop,
  nesting 100, 256 KiB presentation bound, and plaintext fallback.
- Protocol-identical rules already live in Core: IDs, mentions-on-send, notes
  limits, filename/MIME, avatar `mxc://`, last-message preview, agent-card
  object bounds, envelope `deny_unknown_fields`.
- Existing independent tests already reject the adversarial cases that would
  motivate a Core AST.
- Playbook §5 / goal graph do not open an S38 message-format slice.

Strongest stay-put objection: desktop leftover search/pin/inbox still use a
looser editor sanitizer, iOS drops `<strike>` styling, and live `send_text`
does not apply the 65_536 outbound cap that edit uses. Those are leftover
presenter hygiene and a small Core-internal send/edit inconsistency. They do
not prove a missing `TimelineViewRow` AST field or a single sanitizer crate.

Unresolved questions (explicit, not assumed):

- No shared corpus was executed against both renderers in this lane. Visual
  and leftover-path drift may exist beyond the tests cited.
- Whether product wants mention MXIDs or spoiler reason as **row fields** is
  unknown until a corpus shows a security-relevant gap.
- Whether live `send_text` should share the 65_536 outbound cap with edit is
  a later Core-internal consistency question, not a renderer extract.
- The “Already-sanitized rendering markup” comment remains misleading until
  a separately authorized product/docs edit in `crates/**`.
- Agent-approval classification/action authority remains ROE-08 / A2.

### Shared corpus design (paths and shape only)

Recommended later landing (after this memo is accepted), not implemented
tonight and not a renderer:

```text
docs/future-projects/rust-ownership-expansion/fixtures/message-format/
  README.md
  golden/
    replies.json
    mentions-hermes.json
    spoilers.json
    lists-ordered-start.json
    tables.json
    links-allowlisted-schemes.json
    code-fenced-language.json
    edits-plaintext-fallback.json
    hermes-approval.html.json
  adversarial/
    javascript-href.json
    matrix-scheme-and-relative.json
    mx-reply.json
    nesting-100.json
    oversize-256kib.json
    script-and-unknown-tags.json
    https-img-tracker.json
    mxc-img-no-fetch.json
    control-char-href.json
    legacy-strike-font.json
    malformed-overlap.json
```

Each file is one JSON object:

```json
{
  "id": "adv-javascript-href",
  "class": "adversarial",
  "body": "plaintext fallback",
  "formatted_body": "<p><a href=\"javascript:alert(1)\">tap</a></p>",
  "expect": {
    "navigable_schemes": [],
    "javascript_href": "dropped",
    "remote_image_fetch": false,
    "script_executed": false,
    "use_plain_body_if_empty": true
  },
  "not_compared": ["prism_tokens", "typography", "dynamic_type", "rel_target"]
}
```

Runners stay in each presenter test harness. Do not add a Core AST consumer
of this corpus.

Regression proof to keep the stay-put stable:

- Live desktop room continues to call `prepareNativeFormattedBody` before
  `html-react-parser` / Prism.
- Live iOS room continues to call `MatrixHTMLRenderer` before SwiftUI text.
- `project_formatted_body` continues to copy HTML without claiming a
  sanitizer in *behavior* (comment cleanup is separate).
- Both presenters continue to drop `javascript:`, keep plaintext `body` on
  oversize/empty sanitize, and not fetch formatted-HTML `https` images.
- No product path introduces a Core paragraph/text/strong/code/table AST or
  a shared DOM/attributed-string sanitizer crate.

## Next gate

The output-context ownership boundary is closed: keep DOM/React and Swift
attributed-text sanitization and rendering platform-side. The ROE-12 safety
claim is **not** closed until [A6](../program/ACTIONS.md) lands and runs the
shared golden/adversarial corpus in both presenter harnesses. Correct the
misleading `TimelineMessageRow.formatted_body` “already-sanitized” comment as
part of that bounded work and explicitly test the live send/edit size-policy
asymmetry. Do not add a renderer, sanitizer crate, or full AST by default. If
fixtures prove a security-relevant semantic gap, consider the smallest field
on the existing `TimelineViewRow`; a full AST still requires an ADR amendment
or replacement.
