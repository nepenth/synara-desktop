# Desktop and cross-client UI/UX audit

Date: 2026-08-24

Scope: macOS and Linux desktop presentation, with iOS parity implications

Status: review only; no production implementation is part of this branch yet

## Outcome

Synara already has the right structural foundation: a three-pane desktop shell, a virtualized native timeline, a bounded message measure, a rich-content renderer, a semantic room-list DTO, and separate platform-owned UIs over one Rust core. The largest remaining gap is not a missing visual effect. It is the lack of a single, deliberate reading system across text roles, rich-content surfaces, list density, and controls.

The Element comparison is easier to read primarily because it creates more hierarchy *within* the content: body text is softer than headings, metadata is clearly secondary, tables have a visibly separate surface and more cell breathing room, and controls recede until needed. Synara currently gives many elements nearly the same bright foreground and uses dark surface stops that are technically distinct but perceptually very close. Increasing contrast further would make this worse. The next pass should introduce semantic text and content-surface roles, then tune spacing and hierarchy against measurable accessibility floors.

One P0 accessibility defect is proven by the static audit: every hard-coded Prism token color tested against the default light code-panel surface falls below the 4.5:1 normal-text contrast floor. Repair that defect before the broader P1 readability tranche.

## Evidence and design basis

The review inspected the current desktop React/Tauri code, the SwiftUI iOS presentation code, the shared Rust room/timeline DTOs, the supplied Synara-versus-Element macOS screenshots, and the checked-in iOS visual evidence. It also used Grok 4.6 High as an independent read-only reviewer; recommendations below include only findings that agree with the repository evidence and existing architecture.

The Apple guidance used here is intentionally principle-level, not a request to imitate every current Apple material:

- [Typography](https://developer.apple.com/design/human-interface-guidelines/typography): use a coherent text hierarchy and platform-appropriate system typography.
- [Dark Mode](https://developer.apple.com/design/human-interface-guidelines/dark-mode): use adaptive semantic foreground/background roles, preserve depth, and test at least 4.5:1 contrast for text while striving for 7:1 for small custom text.
- [Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility/): support larger text, meet contrast floors in both appearances, and communicate state with more than color.
- [Sidebars](https://developer.apple.com/design/human-interface-guidelines/sidebars): keep hierarchy shallow, allow hiding where useful, and respect familiar platform interactions.
- [Split views](https://developer.apple.com/design/human-interface-guidelines/split-views): support resizable panes and narrow/intermediate window widths.
- [Lists and tables](https://developer.apple.com/design/human-interface-guidelines/lists-and-tables): keep rows scannable, persistently identify navigation selection, and consider alternating fills for wide multicolumn tables.
- [Toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars): keep action groups deliberate and consistent instead of exposing every possible command at once.
- [UI design tips](https://developer.apple.com/design/tips/): preserve legible text size, ample contrast, nonoverlapping content, spacing, and alignment.

For Linux, the same hierarchy and accessibility contract should apply, but the shell should retain Linux system fonts, Ctrl-based shortcuts, native file/secret-service integration, distro window decoration expectations, and WebKitGTK rendering behavior. “Apple quality” means clarity and discipline, not hard-coded SF fonts, fake macOS chrome, or glass effects on every platform.

## What is already working well

- `NativeTimelinePresenter.tsx` is the shipping native-session timeline and already groups consecutive messages, separates sender metadata from body content, virtualizes rows, restores viewport state, and keeps row actions transient.
- `nativeTimelineHtml.css.ts` already sets a 16 px body, 1.55 line height, a 672 px maximum measure, tabular line numbers, horizontal code scrolling, table headers, and zebra-row intent.
- `themeBase.ts` already generates distinct rail/list/chat/composer stops and tests primary contrast. This should be extended, not replaced with unrelated per-screen colors.
- The Rust `RoomSummary` already carries `last_activity_ts` and `last_message_preview`; iOS consumes those fields. Desktop can improve its room rows without inventing a second data source.
- The Rust `TimelineViewRow` already owns event semantics, sender presentation, actions, reply/thread data, reactions, media metadata, and formatted content. UI shells can remain renderers.
- Composer controls have explicit accessibility labels and pressed/expanded state, and code blocks preserve whitespace instead of wrapping source code.
- Fallback room avatars include initials, so identity does not depend on color alone.

## P0 — repair light-mode syntax contrast

### 1. Replace the dark-only Prism palette with adaptive, tested token roles

Current issue: `nativeTimelineHtml.css.ts` applies one hard-coded One Dark-like palette in every appearance. Against the default light `Surface.Container` (`#F2F3F5`), the measured WCAG contrast ratios are:

| Token color | Current use | Ratio |
|---|---|---:|
| `#7a8478` | comments/prolog/doctype | 3.50:1 |
| `#9aa0a6` | punctuation | 2.38:1 |
| `#e06c75` | properties/tags/constants | 2.88:1 |
| `#d19a66` | booleans/numbers | 2.22:1 |
| `#98c379` | strings/selectors/builtins | 1.82:1 |
| `#56b6c2` | operators/entities/URLs | 2.13:1 |
| `#61afef` | functions/classes/attributes | 2.13:1 |
| `#c678dd` | keywords | 2.65:1 |
| `#e5c07b` | regex/important | 1.56:1 |

The code font is `0.92em`, so the normal-text 4.5:1 requirement applies. Line numbers and the language label are also reduced with opacity, which compounds the problem. This is a concrete light-mode readability failure, not a preference about visual softness.

Recommended change:

- Define semantic syntax roles for light, dark, and increased-contrast appearances, each measured against the actual code-panel surface.
- Keep Prism parsing and the existing 50,000-character highlighter bailout unchanged; switching themes should change CSS variables, not re-highlight every visible row.
- Replace opacity-based line-number/language-label treatment with opaque semantic secondary text.
- Preserve horizontal source scrolling and language labels; add a discoverable copy action without introducing a new block-parser architecture.

Likely files:

- `synara/src/app/features/room/nativeTimelineHtml.css.ts`
- `synara/src/app/features/room/nativeTimelineCodeHighlight.ts`
- `synara/src/app/features/room/NativeTimelinePresenter.tsx` (`NativeCodeBlock`)
- `synara/src/app/features/room/__tests__/nativeTimelineCodeHighlight.test.ts`
- `synara/src/app/features/room/__tests__/nativeTimelinePresenterActions.test.ts`

Owner: desktop React. Syntax semantics and colors do not belong in shared Rust.

Acceptance:

- Comment, punctuation, keyword, string, function, number, line-number, and language-label text each measure at least 4.5:1 in system light, system dark, custom-base, and increased-contrast fixtures.
- A one-line fence and a 200-line Rust fence remain readable and selectable; large blocks still take the existing no-highlight path.
- Appearance changes do not re-run Prism over rows whose highlighted markup is already available.

## P1 — establish one readable desktop content system

### 2. Add semantic text roles instead of using one bright `OnContainer` plus opacity

Current issue: `MessageBody`, formatted content, room names, headings, and many labels resolve to the same near-white `OnContainer`. Secondary information is often produced with `opacity: 0.7`, `0.78`, or `0.48`. Opacity composes unpredictably over custom theme ramps and makes contrast hard to prove. The screenshot therefore feels simultaneously high-contrast and flat: everything important is bright, while some metadata becomes too faint.

Recommended change:

- Define opaque semantic roles for `contentPrimary`, `contentSecondary`, `contentTertiary`, `interactive`, `separator`, and `selectedSurface` in both light and dark derived ramps.
- Keep normal message body text at or above 7:1 where practical, never below 4.5:1; keep secondary small text at or above 4.5:1; keep focus/selection and essential non-text state at or above 3:1.
- Soften the default dark body from near-white to a system-label-like light gray while preserving the contrast floor. Reserve the brightest role for headings, selection, and direct emphasis.
- Stop rendering native sender names with unrestricted `colorMXID(...)`. Use primary text or a contrast-adjusted semantic sender accent; color can remain in the avatar/accent edge but must not be the only identity cue.
- Add explicit `prefers-contrast: more`, `prefers-reduced-motion`, and forced-colors behavior. Do not emulate increased contrast by globally increasing every foreground.

Likely files:

- `synara/src/app/utils/themeBase.ts`
- `synara/src/app/pages/ThemeManager.tsx`
- `synara/src/colors.css.ts`
- `synara/src/index.css`
- `synara/src/app/features/room/nativeTimelineHtml.css.ts`
- `synara/src/app/features/room/NativeTimelinePresenter.tsx`
- `synara/src/app/utils/__tests__/themeBase.test.ts`

Owner: desktop React theme/presentation. Rust should not own colors.

Acceptance:

- Automated contrast checks cover every preset and valid custom base in light/dark; body/secondary/non-text floors are 4.5/4.5/3, with default body targeted at 7:1 or higher.
- Dark screenshots show at least three legible foreground levels without opacity stacking.
- Sender names, links, selected rows, unread state, and errors remain understandable in grayscale and forced-colors mode.

### 3. Make the system font stack and type hierarchy platform-native

Current issue: `--font-secondary` starts with an undeclared `InterVariable` face and falls back directly to generic `sans-serif`. This does not guarantee SF on macOS or a distro-native UI font on Linux. Rich-message headings also inherit browser defaults rather than an intentional desktop scale.

Recommended change:

- Use `system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` for UI and prose. Keep the existing cross-platform monospace stack for code.
- Retain 16 px/1.5–1.55 for message prose; Apple’s smaller macOS control text sizes are not a reason to shrink long-form content.
- Define explicit message heading sizes/weights/line heights and paragraph/list rhythms. Target a prose measure of roughly 60–78 characters; allow rich tables and media to use a wider content lane than prose.
- Keep page zoom, but add a text-size/readability control only if product testing shows page zoom is insufficient. Do not create independent font sliders for every surface.

Likely files:

- `synara/src/index.css`
- `synara/src/app/features/room/nativeTimelineHtml.css.ts`
- `synara/src/app/components/message/layout/layout.css.ts` (legacy timeline compatibility only)
- `synara/src/app/features/settings/general/General.tsx`

Owner: desktop React. Each native shell selects its own system font; no font choice belongs in shared Rust.

Acceptance:

- macOS uses the system UI font and Linux uses the desktop’s system sans without bundled Apple assets.
- At 75–150% page zoom, headings, prose, metadata, and controls retain hierarchy with no clipping.
- 200-line messages remain readable and timeline anchor-drift/performance budgets remain green.

### 4. Rebuild rich tables as readable, horizontally safe content surfaces

Current issue: table intent exists, but the derived dark `Surface` and `SurfaceVariant` stops are so close that header and zebra fills can disappear. Cell padding is compact, `overflow: hidden` can clip a wide table, and parent `overflow-wrap: anywhere` can break proof/code-like strings at arbitrary points. This is the clearest gap in the supplied Element comparison.

Recommended change:

- Wrap each table in a semantic horizontal-scroll container in `NativeFormattedBody`, leaving the actual `<table>` semantics intact.
- Give table canvas, header, odd row, even row, separator, and hover/focus distinct semantic content-surface roles derived from the active theme.
- Increase cells from the current compact spacing to approximately 10–12 px vertically and 14–16 px horizontally; keep header weight around 600 and body weight 400.
- Prefer natural word boundaries in prose cells. Keep inline code unbroken where possible and let the table scroll rather than exploding row height.
- Add a subtle edge/shadow affordance when more columns are offscreen. Do not add gradients inside individual cells; “layered cells” should mean a quiet luminance ramp, not decoration.

Likely files:

- `synara/src/app/features/room/nativeTimelineFormattedBody.tsx`
- `synara/src/app/features/room/nativeTimelineHtml.css.ts`
- `synara/src/app/features/room/__tests__/nativeTimelinePresenterActions.test.ts`
- a new focused rich-table browser/screenshot test under `synara/e2e/`

Owner: desktop React renderer. A future normalized semantic rich-content tree can be shared, but scroll behavior and cell styling remain platform UI.

Acceptance:

- A four-column table with long prose, hashes, inline code, links, and 20 rows is readable at 640, 900, and 1200 px detail-pane widths.
- Nothing is clipped; horizontal scrolling is keyboard/trackpad accessible; headers remain associated with cells for accessibility APIs.
- Dark/light screenshots make header and alternate rows perceptible without any text falling below its contrast floor.

### 5. Finish the whole Matrix rich-content reading rhythm

Recommended change:

- Apply the same semantic roles and spacing to paragraphs, lists, quotes, horizontal rules, inline code, code panels, replies, mentions, spoilers, edit labels, state rows, and agent cards.
- Underline links within message content by default (or provide an equally non-color-only cue); app navigation links can keep the current hover treatment.
- Treat the P0 syntax-palette repair as the baseline for code. Keep code unwrapped by default, add a discoverable copy action, and preserve the existing large-block highlighter limit.
- Give unread and date separators dedicated, quiet system-row treatments. The unread row should be an accent-labeled rule rather than body text; the date row should be a centered caption.
- Give replies a visible “Replying to {display name}” relationship and an accessible name based on the sender, not a raw event ID.
- Render membership/state/redaction rows as quieter centered/system-event rows with icon/text semantics, rather than giving them the same weight as a normal message.

Likely files:

- `synara/src/app/features/room/nativeTimelineHtml.css.ts`
- `synara/src/app/features/room/nativeTimelineFormattedBody.tsx`
- `synara/src/app/features/room/NativeTimelinePresenter.tsx`
- `synara/src/app/plugins/react-prism/ReactPrism.css`
- `synara/src/app/components/hermes/HermesAgentCard.tsx` and its styles

Owner: desktop React now; semantic event kind and safe content stay in Rust.

Acceptance:

- Golden cases cover headings, nested lists, quotes, spoilers, mentions, links, code, tables, replies, state events, agent cards, and custom sender colors in both appearances.
- Every link and interactive mention is identifiable without relying on hue alone.
- Wide tables scroll without clipping, unread/date rows are recognizable without reading surrounding messages, and reply controls announce the sender rather than an event identifier.
- Rich messages do not change virtual-row height after fonts settle by more than the existing 2 px anchor budget.

## P1 — improve scanning and task flow around the timeline

### 6. Use the existing shared room summary instead of a name-only desktop row

Current issue: `RoomNavItem` looks up native unread/favorite state but displays only the Matrix room name. The shared DTO already includes `lastMessagePreview` and `lastActivityTs`, and iOS displays them. Desktop is discarding useful shared information.

Recommended change:

- Introduce `Compact` and `Comfortable` room-list densities. Keep compact close to the current 40 px row. In comfortable mode, show a one-line secondary preview and a locale-aware trailing activity time, hiding them progressively when the pane becomes narrow.
- Keep unread rooms semibold, preserve the count badge, and add a persistent selected-row cue beyond a hover-equivalent background (for example, a small accent edge plus selection fill).
- Make favorite/room section sorting a single discoverable menu or adequately sized buttons. The current 14 px sort targets are too small for comfortable pointer use.
- Pass the `RoomSummary` directly to each row or index summaries once; do not add per-row polling or re-read Matrix state.

Likely files:

- `synara/src/app/features/room-nav/RoomNavItem.tsx`
- `synara/src/app/features/room-nav/styles.css.ts`
- `synara/src/app/pages/client/home/Home.tsx`
- `synara/src/app/pages/client/home/Home.css.ts`
- `synara/src/app/features/matrix-dto/room.ts`
- `synara/src/app/state/room-list/roomList.ts`

Owner: `RoomSummary` fields/sort semantics in shared Rust; density, truncation, time formatting, and row styling in desktop React.

Acceptance:

- 5,000-room updates still remap only the changed room within the existing 100 ms budget.
- Compact and comfortable modes work from the minimum window width through a large display; no room name, badge, timestamp, or menu overlaps.
- The selected room, unread room, mention, muted room, and favorite are distinguishable in color-blind/grayscale review.

### 7. Make navigation panes adaptable and platform-familiar

Current issue: the rail is fixed at 66 px and the room/settings pane is fixed at 222/256 px. There is no normal macOS-style show/hide-sidebar path or user-resizable list pane. Rail items translate sideways on hover, adding motion without information.

Recommended change:

- Add a draggable room-list divider with a bounded persisted width and an automatic compact threshold. Provide View-menu/keyboard commands to show or hide the leading navigation panes on macOS; expose equivalent menu/shortcut behavior on Linux.
- Treat the existing 960×720 minimum and the 960–1124 px breakpoint range as a first-class compact-desktop contract. Do not lower the minimum to 750 px merely to reactivate the retired mobile-friendly web chrome; decide pane collapse behavior before changing shell bounds.
- Keep the three-pane hierarchy. Do not merge the global rail and room list into a deeper, harder-to-scan tree.
- Remove hover translation from the rail or disable it under reduced motion. Keep stable hit regions, tooltips, focus rings, and a non-color-only selected indicator.
- Ensure important actions are not available only at the bottom of the rail; shortcuts/menu commands should duplicate them where appropriate.

Likely files:

- `synara/src/app/pages/client/ClientLayout.tsx`
- `synara/src/app/components/sidebar/Sidebar.css.ts`
- `synara/src/app/components/page/style.css.ts`
- `synara/src/app/features/room-nav/*`
- `src-tauri/src/menu.rs` and the existing window command owner in `src-tauri/src/lib.rs`
- `src-tauri/tauri.conf.json`

Owner: React owns pane layout; Tauri owns native menus/window commands; platform shortcut labels remain platform-specific.

Acceptance:

- Pane widths persist, keyboard focus never disappears behind a collapsed pane, and content remains reachable at narrow/intermediate/large widths.
- macOS and KDE/GNOME users can show/hide navigation using familiar commands, without changing shared navigation state.
- At 960×720, rail, room list, timeline, and composer remain usable without a mobile back-stack; at 1280×900, the people drawer does not cover the composer.

### 8. Simplify the composer’s default state

Current issue: the composer can expose formatting, sticker, emoji, GIF, poll, and send actions simultaneously in a floating panel while separately showing attachment. The editor reserves 184 px on its first line for that cluster. This works mechanically but makes writing feel like operating a toolbar, especially at narrow widths.

Recommended change:

- Keep attach, emoji, and send immediately available. Move infrequent sticker/GIF/poll actions into the existing leading `+` affordance or a single overflow menu, while retaining keyboard access and clear labels.
- Place formatting in a stable secondary row only while active; do not float a translucent mini-surface over draft text.
- Style reply/edit/upload/error states as integrated composer subregions with clear separators and an explicit dismiss/retry action. Send failures should use a critical semantic role and icon, not low-priority gray text.
- Maintain a comfortable single-line height and grow to a bounded multi-line editor; continue to scroll internally only after the cap.

Likely files:

- `synara/src/app/features/room/RoomInput.tsx`
- `synara/src/app/features/room/RoomComposer.tsx`
- `synara/src/app/features/room/RoomComposer.css.ts`
- `synara/src/app/components/editor/Editor.css.ts`
- `synara/src/app/components/editor/Editor.tsx`

Owner: platform UI. Rust owns send capability/state/error categories, not toolbar arrangement.

Acceptance:

- At every supported width, the caret and draft text never render beneath actions and every function remains keyboard reachable.
- The default empty composer shows no more than four primary controls, and the send action remains visually dominant.
- Error, queued, uploading, replying, and editing states are distinct without layout jumps that violate timeline anchoring.

## P2 — settings, avatars, and parity infrastructure

### 9. Turn Settings from a sequence of cards into a scan-friendly preference hierarchy

Current issue: many individual settings are each wrapped in a separate rounded card, while `SettingTile` always lays description and trailing control in one horizontal row. This creates “card soup” on large windows and squeezes explanatory copy/control groups at intermediate widths.

Recommended change:

- Group related settings into one section surface with separators; reserve standalone cards for genuinely distinct status/action panels.
- Implement a responsive setting row: label/description column plus aligned control column at wide widths, stacked control below at narrow widths.
- Keep destructive actions in a clearly separated final section. Keep diagnostics/developer tools visibly advanced.
- Shorten technical appearance copy and preview the effect beside the control. Preserve system-theme default and make custom colors secondary.
- Keep the native-timeline `Message Layout` row read-only or hide it: the current code correctly explains that Compact/Bubble affect only the retired JavaScript timeline. Do not spend this effort reimplementing Bubble merely to make an obsolete selector active.

Likely files:

- `synara/src/app/features/settings/Settings.tsx`
- `synara/src/app/features/settings/styles.css.ts`
- `synara/src/app/features/settings/general/General.tsx`
- `synara/src/app/components/setting-tile/SettingTile.tsx`
- `synara/src/app/components/sequence-card/style.css.ts`
- `synara/src/app/components/page/style.css.ts`

Owner: desktop React. Settings semantics may sync through the existing shared settings contract only after an explicit product/privacy decision; layout never belongs in Rust.

Acceptance:

- Settings remain scannable at 600, 900, and 1200 px content widths and at 150% zoom.
- Labels, descriptions, controls, warning text, and destructive actions never overlap and follow one predictable alignment grid.

### 10. Standardize avatar semantics without forcing identical platform rendering

Recommended change:

- Keep `avatar_url`, resolved display name, room kind, and an optional stable fallback seed in shared DTOs.
- Consolidate duplicate initials test fixtures across TypeScript and Swift, or expose a small core-derived fallback label/seed if localization and naming rules can be made stable.
- Let each platform own crop, corner radius, placeholder symbol, palette, and material. Desktop may use smaller rounded-square room avatars; iOS may keep larger touch-friendly tiles.
- Ensure avatar loading has a stable placeholder with no layout shift and useful alt/accessibility text at the parent row level.

Likely files:

- `crates/synara-core/src/app/room_list/summary.rs`
- `crates/synara-core/src/app/timeline/view.rs`
- `synara/src/app/utils/common.ts`
- `synara/src/app/components/room-avatar/*`
- `synara/src/app/components/user-avatar/*`
- `synara-ios/Synara/SharedUI/SynaraRoomAvatar.swift`

Owner: identity data/seed in Rust; all visual avatar rendering is platform-native.

### 11. Define a cross-client rich-content presentation contract

Current issue: Rust emits plain body plus sanitized HTML; desktop sanitizes/parses HTML again and iOS converts HTML into Markdown-like text. This can produce visible parity gaps for tables, nested structure, spoilers, mentions, code labels, and agent content even when both clients consume the same event.

Recommended change:

- First, document a parity corpus of safe Matrix content and expected semantic output.
- If platform drift remains material, evolve `TimelineViewRow` to optionally expose a bounded, versioned semantic render tree (paragraph, heading, list, quote, code, table, link, mention, spoiler, media caption) while retaining plain-body fallback.
- Keep renderer layout, fonts, colors, selection, menus, and platform links native. Do not ship CSS or SwiftUI styling from Rust.
- Treat this as a staged architecture task, not a prerequisite for the P1 desktop styling pass; a new render-tree boundary affects UDL, IPC size, sanitization, and timeline performance.

Likely files:

- `crates/synara-core/src/app/timeline/view.rs`
- `crates/synara-core/src/synara_core.udl`
- `synara/src/app/utils/matrixHtmlProfile.ts`
- `synara/src/app/utils/sanitize.ts`
- `synara/src/app/features/room/nativeTimelineFormattedBody.tsx`
- `synara-ios/Synara/Services/TimelineService.swift` (`MatrixHTMLRenderer`)

Owner: safe semantic content in Rust; final rendering in React/SwiftUI.

Acceptance:

- One fixture corpus produces equivalent semantic hierarchy on desktop and iOS for every supported Matrix rich-content construct.
- Malformed/deep/large HTML stays bounded and falls back to readable plain text.
- The optional semantic tree causes no material regression in snapshot size, row projection latency, memory, or scroll stability.

## Implementation order

1. Repair light-mode syntax contrast and add palette contrast tests.
2. Add semantic foreground/content-surface tokens and contrast tests.
3. Tune the shipping native timeline’s prose, rich tables, replies, separators, and event rows against the comparison fixture.
4. Simplify the composer and make errors/statuses semantically clear.
5. Add the desktop room-row density option using the existing Rust summary fields; fix selection and sort hit regions.
6. Make panes adaptable/resizable and add native menu equivalents.
7. Consolidate settings rows/sections.
8. Decide from parity fixtures whether a shared semantic rich-content tree is justified.

Each tranche should include light/dark visual snapshots on macOS and Linux, keyboard-only review, VoiceOver/Orca smoke coverage where available, increased-contrast/reduced-motion checks, and the existing timeline performance/anchor gates.

## Avoid these tempting regressions

- Do not clone Element pixel-for-pixel; use it as evidence about hierarchy, spacing, and scanability.
- Do not reduce “harsh” contrast by applying container opacity to text. Derive opaque semantic colors and prove their ratios.
- Do not force SF Pro or Apple-only icons onto Linux. Use platform system fonts and a shared semantic icon vocabulary.
- Do not add blur/transparency merely to look current; WebKitGTK, Reduce Transparency, GPU compatibility, and legibility all argue for restraint.
- Do not spend the first tranche polishing `message/Message.tsx` or Compact/Bubble layouts. Native sessions ship `NativeTimelinePresenter`, and Settings already labels the older layouts as retired.
- Do not lower the minimum window to 750 px and reuse `MobileFriendly.tsx`; that path is web/mobile chrome, not a sound compact-desktop idiom on macOS or Linux.
- Do not make every desktop room row permanently two lines. Offer responsive density so large accounts can remain compact.
- Do not put spacing, color, platform font, panel width, or toolbar layout into `synara-core`.
- Do not sync personal visual preferences through Matrix account data without a separate privacy/product decision.

## Proposed review gate

Before implementation is called ready, capture the same seeded public and encrypted rooms on:

- macOS: light, dark, Increase Contrast, Reduce Motion, 100% and 150% zoom.
- Linux: KDE/Wayland and GNOME/X11 or Wayland, light/dark, 100% and 150% zoom.
- Widths: 960×720 minimum, 1280×900 default, and a large/Retina or HiDPI window; add a Linux tiled-width fixture before any decision to lower the shell minimum.
- Content: short/long prose, every heading/list/quote form, a wide table, a 200-line code block, inline code/hashes/URLs, replies, reactions, edits, media, agent card, unread marker, state/redaction/UTD events, and a multi-line draft with attachments.

The release gate should require no clipping/overlap, no keyboard trap, proven contrast floors, selected/unread/error state without color alone, and no regression against the existing 2 px desktop anchor-drift and timeline performance budgets.
