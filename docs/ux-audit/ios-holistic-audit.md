# Synara iOS holistic UI/UX audit

Status: audit and implementation brief; no production code changed  
Reviewed: 2026-08-24  
Repository baseline: `e2cdb55dc05289da6cb5c1c032f316d653c1c602` (`feature/holistic-apple-ux-review`)  
Primary target: iPhone 17 Pro, iOS 26.5, portrait; light, dark, and Dark + Increase Contrast at an accessibility text size

## Executive assessment

Synara has a coherent native SwiftUI foundation and already uses semantic type styles for most content. The strongest current screen is the normal-size timeline: sender, time, message body, reactions, replies, and attachments are recognizable without bubble-heavy visual noise. The current light theme also no longer has the full-screen gray cast visible in the earlier supplied screenshots; its near-white surfaces read as opaque in the current build.

The release-quality gap is now less about adding decoration and more about making the hierarchy resilient. Four issues dominate:

1. **Accessibility text sizes break the end-to-end reading and composing path.** At AX Extra Large, the composer placeholder escapes below its field and screen. Settings content also sits beneath the fixed-size floating tab bar; whether the final actions can scroll clear and receive taps is the critical unresolved test. The composer failure is a correctness defect, not polish.
2. **Routine connection chrome creates the reported off-color bands while it is visible.** In the captured fixture session, `Syncing history…` occupies a full-width green surface above every tab. In Dark Mode it becomes a dark-green status-bar region unrelated to the charcoal content below. The floating tab bar then samples a different underlying surface at the bottom. The screenshots do not establish how often or how long production sessions remain in this state.
3. **Screen backgrounds are not one system.** Rooms, Timeline, Settings, and Notifications use different combinations of custom opaque surfaces, hidden list backgrounds, system list backgrounds, and material. The Notifications screen is near-white/pure black while the other screens are lightly tinted/charcoal. The mismatch is most obvious in Dark Mode.
4. **Dense content loses scan hierarchy.** Room rows use small, visually similar avatar tiles; timeline messages have only 4–8 pt between most content units; reply metadata is low-emphasis; and notification rows make title, category chip, state chip, count, and disclosure indicator compete on one horizontal axis.

Recommended product direction: keep the restrained, native, content-first style, but make system semantics the default. Use custom theme color mainly for tint and selective emphasis; use system/elevated background roles for structure; give reading content a measured rhythm; and switch constrained horizontal layouts to stacked layouts at accessibility sizes.

## Evidence and method

The audit used an already signed-in local simulator session without reading or exposing credentials. Deterministic UI-test launch fixtures supplied stable room, timeline, Settings, and Notifications content. The source branch itself was not modified or rebuilt for this audit, so the screenshots are evidence of the locally installed 2.1.8 build; each finding was also checked against the source at the repository baseline above.

| Surface | Light | Dark | Accessibility stress |
| --- | --- | --- | --- |
| Live room list | [room-list-light](evidence/ios/room-list-light.png) | [room-list-dark](evidence/ios/room-list-dark.png) | Code review; add a canonical AX capture in implementation |
| Settings | [settings-light](evidence/ios/settings-light.png) | [settings-dark](evidence/ios/settings-dark.png) | [Dark, Increase Contrast, AX Extra Large](evidence/ios/settings-dark-axxl-contrast.png) |
| Timeline and composer | [timeline-light](evidence/ios/timeline-light.png) | [timeline-dark](evidence/ios/timeline-dark.png) | [Dark, Increase Contrast, AX Extra Large](evidence/ios/timeline-dark-axxl-contrast.png) |
| Notifications | [notifications-light](evidence/ios/notifications-light.png) | [notifications-dark](evidence/ios/notifications-dark.png) | Code review; add stacked AX row fixture in implementation |
| Later | Not captured in this pass | Not captured in this pass | Code review shows no shared floating-bar clearance; must be added to Slice A evidence |

The AX screenshots are intentionally failure-oriented. They show real layout behavior at a supported system setting and should become regression baselines, not aspirational mockups. The earlier user-supplied physical-device Settings screenshot is corroborating context but is not copied into this evidence directory; physical reachability still needs a clean recorded run.

### Independent review

Grok 4.6 High reviewed this brief, the ten evidence images, the cited Swift owners, and Apple guidance in read-only plan mode. Its accepted pushback materially changed the audit: Liquid Glass peek-through is no longer labelled a defect without reachability/tap proof; adaptive horizontal rows moved from P0 to P1 until a real clip is captured; ownership references were corrected; the composer diagnosis now identifies the unconstrained multiline placeholder; custom theme surfaces are preserved rather than replaced wholesale; timeline sender spacing is stated accurately; verification steps must not hide real SAS states; and the stale 44 pt claim in the existing accessibility checklist is now called out as a regression.

## Apple design basis

The recommendations apply the following current primary guidance:

- [Typography](https://developer.apple.com/design/human-interface-guidelines/typography): use system text styles, support Dynamic Type, avoid truncation in scrollable content, and change inline layouts to stacked layouts when large text creates crowding.
- [Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility/): audit with accessibility tools, check both appearances, support larger text, avoid color-only meaning, and target at least 4.5:1 for ordinary text up to 17 pt (3:1 for larger or bold text).
- [Color](https://developer.apple.com/design/human-interface-guidelines/color) and [Dark Mode](https://developer.apple.com/design/human-interface-guidelines/dark-mode): use adaptive colors and define light/dark/increased-contrast behavior for custom colors. Use 4.5:1 as the ordinary-text gate (3:1 for qualifying large/bold text); treat the repository's existing 7:1 checks as a desirable internal stretch target, not the published minimum.
- [Layout](https://developer.apple.com/design/human-interface-guidelines/layout): respect safe areas and system bars, and validate multiple sizes, orientations, localizations, and text settings.
- [Tab bars](https://developer.apple.com/design/human-interface-guidelines/tab-bars): treat the system tab bar as navigation chrome that floats above content; content must remain readable and operable around it.
- [Buttons](https://developer.apple.com/design/human-interface-guidelines/buttons): make controls visually understandable and generally provide a 44×44 pt hit region for primary touch controls. Apple documents 28×28 pt as the minimum in some accessibility sizing guidance, but minimum-sized controls still require adequate separation and are inappropriate for frequent primary actions.
- [Text fields](https://developer.apple.com/design/human-interface-guidelines/text-fields): use a text view for long text, use a useful placeholder/label, and size the field for the expected input.
- [Settings](https://developer.apple.com/design/human-interface-guidelines/settings): keep settings focused, prefer good defaults, respect system-wide settings, and place task-specific controls in their task context.
- [Notifications](https://developer.apple.com/design/human-interface-guidelines/notifications): keep system-delivered alerts concise and useful and protect private content. The in-app Notifications tab is an inbox/list and should primarily follow List, Typography, and Layout guidance rather than borrowing every rule for Lock Screen notifications.
- [Inclusion](https://developer.apple.com/design/human-interface-guidelines/inclusion): use inclusive generic-person imagery and avoid encoding identity assumptions in fallback avatars.

## Priority 0 — fix before visual polish

### P0.1 Make the composer functional at every Dynamic Type size

**Observed:** In the AX Extra Large capture, `Send a message…` wraps outside the rounded text container and below the screen. The leading attachment and sticker controls stay 34 pt while text becomes much larger. The composer consumes the bottom of the viewport without a deliberate compact-to-stacked transition.

**Cause in source:** `ComposerTextContainer` makes the multiline placeholder `UILabel` a sibling of the text view, constrains its leading/trailing/top edges but not its bottom, and does not clip the container. The collapsed empty editor remains one callout line even when the placeholder wraps in the width left by the fixed horizontal tool row. The text height is separately capped at a fixed 112 pt and side controls remain fixed at 34/28 pt.

**Required behavior:**

- Placeholder, typed text, caret, and selection must remain within the editor at all supported sizes.
- Constrain and clip the placeholder correctly first; then, at accessibility categories, move secondary tools into a separate toolbar/menu or stack them above the editor. Keep the text view and Send action dominant.
- Make Attach, Stickers, Formatting, and Send hit regions at least 44×44 pt even when their visible glyph treatment stays compact.
- Replace the constant maximum height with a Dynamic Type-aware cap based on available viewport height; once capped, keep internal editor scrolling usable and obvious.
- Verify keyboard shown/hidden, portrait/landscape, reply/edit banners, attachments, and 1–5 line drafts at Large, XXXL, AX Medium, AX Extra Large, and AX XXXL.

**Likely owners:**

- `synara-ios/Synara/Features/Composer/ComposerTextView.swift:7-14,47-124,204-229,278-334`
- `synara-ios/Synara/Features/RoomTimelineView.swift:5295-5475,5631-5679`
- `synara-ios/SynaraUITests/SynaraUITests.swift` (add matrix screenshots and hit-region assertions)

### P0.2 Prove and fix final-action reachability around the floating tab bar

**Observed:** In normal Settings captures, Support and the next section are visible through/under the floating tab bar. In the AX capture, `About` appears beneath it. Liquid Glass is designed to float above edge-to-edge scroll content, so peek-through itself is not the defect. The unresolved defect is whether Support, Log Out, Unread rooms, and the final Later item can scroll fully above the material and receive taps without the tab bar taking the hit. The room list reserves 68 pt, Settings reserves 104 pt, and Notifications/Later reserve no explicit clearance.

**Cause in source:** where extra clearance exists, it is encoded as hard-coded transparent heights rather than derived from the actual system bar/safe area. Coverage is inconsistent across tabs.

**Required behavior:**

- The final row/action of every tab must scroll completely above the bar with readable separation and remain hittable. Prove this with actual taps, not screenshots alone.
- Use one shell-owned tab-bar avoidance contract instead of per-screen constants. Prefer the system-adjusted scroll/content safe area; if an additional content margin is needed for the iOS 26 floating bar, derive and test it centrally.
- Keep scroll content edge-to-edge beneath Liquid Glass as Apple intends, with the bar sampling a consistent screen background. Do not treat all peek-through as a failure.
- Keep the tab bar at its system size for Dynamic Type and support Large Content Viewer where useful; the content avoidance contract, not a scaled custom tab bar, owns reachability.
- Validate iPhone 17e/17 Pro/Pro Max, portrait and landscape, all standard and accessibility sizes, keyboard present, and Reduce Transparency.

**Likely owners:**

- `synara-ios/Synara/Features/SettingsView.swift:119-129,1464-1475`
- `synara-ios/Synara/Features/RoomListView.swift:241-260`
- `synara-ios/Synara/Features/LaterListView.swift:8-77` (missing adopter)
- `synara-ios/Synara/App/AppTab.swift:3-104` (Notifications missing adopter)
- `synara-ios/Synara/App/RootShellView.swift:71-82,189-200` (owner of the future shared shell contract, not the current spacer cause)

## Priority 1 — visual system and reading quality

### P1.1 Demote routine sync state; reserve the banner for actionable status

**Observed:** `Syncing history…` paints the status-bar/top-safe-area green on Settings, Timeline, and Notifications throughout the captured fixture session. In Dark Mode it reads as a separate dark-green app header; in Light Mode it introduces a pale-green band like the one reported by the user. Source confirms `.syncing` presents immediately, but this evidence does not prove that real connected production sessions remain there; confirm that `.connected` hides it before treating the banner as the sole production root cause.

**Recommendation:**

- Do not show a full-width banner for healthy, routine sync. Use a small progress indicator near the room-list title or a transient, neutral status treatment that does not reflow every tab.
- Keep the persistent banner for offline, delayed, retryable, or authentication-required states.
- When visible, use semantic label/fill pairs rather than foreground color plus the same color at low opacity. Ensure Increased Contrast has a materially stronger variant.
- Announce meaningful state changes to VoiceOver without repeatedly announcing routine sync progress.

**Likely owners:**

- `synara-ios/Synara/SharedUI/ConnectionStatusBanner.swift:10-78`
- `synara-ios/Synara/Services/ConnectionStatus.swift`
- `synara-ios/Synara/App/RootShellView.swift:71-84`

### P1.2 Complete the existing adaptive appearance contract

**Observed:** Rooms and Settings use custom tinted near-white/charcoal roles, Timeline uses another custom role, and Notifications leaves the system List background visible. Consequently Notifications is white in Light Mode and pure black in Dark Mode while Settings is light gray/dark charcoal. The top banner and bottom material add two more unrelated stops.

**Recommendation:**

- Preserve the existing theme-ramp product decision, but define its roles semantically: base background, grouped background, elevated surface, separator, material-underlay, primary/secondary/tertiary label. Make Notifications and Later adopt the same contract instead of replacing the ramp with an unrelated system background.
- Continue using semantic system colors and materials where they improve platform behavior, especially labels and status meanings, while keeping the theme hue in the established surface/tint system.
- If custom surface colors remain, make the dynamic color provider inspect `accessibilityContrast` as well as `userInterfaceStyle`; currently it only switches on light/dark.
- Add token tests for every preset across Light, Dark, Light + Increase Contrast, and Dark + Increase Contrast. Include text, icons, separators, disabled controls, placeholder text, reply excerpts, and tint-on-fill combinations—not only primary text on the main surface.

**Likely owners:**

- `synara-ios/Synara/SharedUI/SynaraDesign.swift:15-256,401-423`
- `synara-ios/Synara/App/AppTab.swift:9-104`
- `synara-ios/Synara/Features/RoomListView.swift:24-261`
- `synara-ios/Synara/Features/SettingsView.swift:9-129`
- `synara-ios/Synara/Features/RoomTimelineView.swift:464-745`
- `synara-ios/SynaraTests/SynaraThemeRampTests.swift`

### P1.3 Refine timeline text hierarchy without double-spacing groups

**Observed:** Normal text is legible. The list contributes 4 pt between rows and an ungrouped sender adds 7 pt top padding, yielding about 11 pt before a new sender; within-group spacing is 4 pt, sender-to-body is 5 pt, and the message-content stack is 8 pt. The clearly weak element is reply metadata: caption-sized secondary/tertiary text with a two-line limit. Rich-text segments also receive only 4 pt separation.

**Recommendation:**

- Keep message body at the system `.body` style, but add approximately 2–3 pt line spacing for multiline prose and test with Bold Text. Do not globally increase weight; preserve semibold for sender and true emphasis.
- Keep the current roughly 11 pt new-sender separation unless comparison screenshots show a regression; do not add another 10–12 pt on top. Tune the 4 pt within-group and rich-block spacing selectively, and keep about 8 pt around attachments/reactions/thread links.
- At accessibility sizes, stack sender and timestamp instead of forcing a shared baseline. Keep timestamp available but subordinate.
- Raise reply-excerpt legibility: body-sized or callout-sized snippet where space permits, stronger secondary contrast, and a maximum that preserves enough context. Provide a VoiceOver action to open the replied-to event.
- On iPad/landscape, cap long reading lines to a comfortable content column rather than stretching prose edge to edge.

**Likely owners:**

- `synara-ios/Synara/Features/RoomTimelineView.swift:4277-4460,4632-4673`
- `synara-ios/Synara/SharedUI/SynaraMessageBubble.swift:14-48,254-283`
- `synara-ios/Synara/SharedUI/SynaraDesign.swift:408-422`

### P1.4 Make rich messages document-like, not merely decoded

**Observed:** The table renderer already alternates row surfaces—an important improvement over the supplied earlier Mac comparison—but body cells have fixed 112/168 pt widths, horizontal-scroll indicators are hidden, and every rich-text segment is separated by only 4 pt. Markdown parsing is effectively inline within each text segment, so heading and paragraph hierarchy is weaker than the source content implies.

**Recommendation:**

- Introduce semantic rich-message blocks: heading levels, paragraphs, lists, quotes, code, details, and tables with distinct but restrained spacing/type roles.
- Tables: retain zebra rows; strengthen header fill/weight; use 10–12 pt cell insets; align numeric columns; adapt first/data column widths to content and viewport; and provide a visible overflow affordance because hidden scroll indicators make wide content undiscoverable.
- Code: preserve monospaced text and Copy, but increase contrast between header/body surfaces and expose language when known. Ensure line numbers do not compete with code.
- Preserve the Matrix event as a message; do not invent a separate document viewer as part of this effort. On larger screens, cap prose reading width while letting tables/code scroll inside that column.
- Build one shared comparison fixture for iOS, macOS, and Linux containing the same headings, paragraph breaks, lists, inline code, quote, code block, details, and table. Use it to expose semantic drift; do not make exact cross-platform pixel equality a release gate.

**Likely owners:**

- `synara-ios/Synara/Features/RoomTimelineView.swift:3371-3604`
- `synara-ios/Synara/Services/TimelineService.swift:1250-1780`
- `synara-ios/SynaraTests/TimelineMessageCopyTests.swift`
- `synara-ios/SynaraUITests/SynaraUITests.swift` (golden rich-message fixture)

### P1.5 Rebuild room rows around recognition and predictable priority

**Observed:** The live list is spacious but low-information: 32 pt room avatars are smaller than the row’s typographic emphasis, most fallbacks are the same gray tile, and room names/previews truncate. Source permits favorite/mention/approval/agent/time/unread elements to occupy one line, but the sparse live screenshots do not demonstrate the worst-case collision. The hostname is the dominant header title.

**Recommendation:**

- Use 40–44 pt room avatars. Use circles for people/DMs and rounded rectangles for rooms/spaces. Derive a stable, contrast-safe fallback hue from the Matrix ID so rows remain recognizable without loaded media.
- Keep the first line to identity + one highest-priority state + time. Put other state in the second line or trailing unread accessory. Do not show Favorite, Mention, Approval, Agent, time, and count simultaneously inline.
- Let unread state use weight plus count/shape, not color alone. Preserve a minimum two-line row at accessibility sizes and stack time/count as needed.
- Keep active-account context for multi-account safety, but make it compact enough that rooms remain primary. Preserve the full homeserver in the account sheet and expose it accessibly rather than simply hiding it.
- Replace the two 16×16 sort buttons with one 44×44 menu labelled “Sort”; the current hit regions are below Apple’s general touch-target recommendation.

**Likely owners:**

- `synara-ios/Synara/Features/RoomListView.swift:720-775,1004-1110,1197-1270`
- `synara-ios/Synara/SharedUI/SynaraRoomAvatar.swift:6-76`
- `synara-ios/Synara/SharedUI/SynaraDesign.swift:522-606`
- `synara-ios/Synara/Services/RoomListService.swift`

### P1.6 Use one avatar identity model across clients and native presentations

**Observed:** Room avatars, timeline avatars, Settings account icons, and notification avatars use different shapes, sizes, and fallback logic. Timeline fallbacks special-case three fixture users, while all other senders receive the same accent-to-secondary gradient. This weakens rapid speaker recognition.

**Recommendation:**

- Share only platform-neutral avatar data where appropriate: display name, media URI, entity kind, initials, and a stable color seed. Keep shape, sizing, image decoding, and platform rendering in Swift/AppKit/web presentation code so the iOS fix is not blocked by a core migration.
- iOS presentation: people circular; rooms rounded-square; spaces distinct but not reliant on color alone; account icon consistent with person treatment.
- Use actual profile media where available, a deterministic ID-derived fallback otherwise, and sufficient letter/background contrast in Light, Dark, and Increase Contrast.
- Do not use generic gendered imagery for unknown users; current `person.fill` is acceptable as a nongendered SF Symbol.

**Likely owners:**

- `synara-ios/Synara/SharedUI/SynaraRoomAvatar.swift`
- `synara-ios/Synara/Features/RoomTimelineView.swift:3607-3712`
- `synara-ios/Synara/Features/SettingsView.swift:1238-1261`
- shared room/member/profile projection contracts in `crates/synara-core` and `synara-ios/Synara/Contracts/SynaraContracts.swift` only if seed/entity data is genuinely missing

### P1.7 Simplify Notifications into an inbox rather than a badge showcase

**Observed:** The normal-size notification rows truncate titles (`Access revi…`, `Deploy app…`) while category and pending chips occupy large horizontal space. The same “Unread rooms” wording appears in both the section header and disclosure row. Dark Mode uses a different base background from the other tabs.

**Recommendation:**

- Give title the first line. Put room/sender + preview on the second/third line. Move category/status into one compact accessory or a leading semantic icon; never sacrifice the action title to show both `Agent` and `pending`.
- Remove the duplicate Unread rooms label. Either use a single disclosure section header with count or show expanded room rows directly.
- Add dates/times where they help triage and preserve deep-link focus.
- Use the shared screen surface and adaptive row layout described above.

**Likely owners:**

- `synara-ios/Synara/App/AppTab.swift:3-261`
- `synara-ios/Synara/Services/RoomListService.swift:610-730`

### P1.8 Make verification status and decisions explicit at every stage

**Observed from source:** The verification sheet has a sound security boundary—comparison requires a large detent, terminal states pair icon with text, and mismatch is destructive—but progress states are only a spinner beneath explanatory copy. Emoji cells use fixed metrics; decimals remain a single horizontal row; request identity is a horizontal key/value row. These layouts are vulnerable to long names and accessibility sizes.

**Recommendation:**

- If a step model is added, make it additive and map every real state rather than collapsing the protocol into a misleading five-step summary. Keep exact current-state wording such as `Exchanging keys`, `Waiting for the other device`, and `Waiting for codes` as the source of truth. Highlight state with text/icon, not color alone, and add a nontechnical recovery action when progress times out or fails.
- At accessibility sizes, stack emoji cells in one/two columns, stack decimal groups if necessary, and stack User/Device label over value.
- Pin the decision footer with full-width, 44 pt minimum actions. Keep “They Do Not Match” visually distinct and require no ambiguous confirmation wording.
- Add deterministic previews/UI tests for every `CryptoVerificationState` in both appearances and accessibility sizes; the current live-only path is inadequate for visual regression.

**Likely owners:**

- `synara-ios/Synara/App/RootShellView.swift:314-574`
- `synara-ios/Synara/Services/AppServices.swift:376-448`
- `synara-ios/Synara/Services/MatrixClientPolicies.swift`
- `synara-ios/SynaraUITests/SynaraUITests.swift`

### P1.9 Adapt constrained information rows before they fail in real data

**Observed:** The AX Settings screenshot still renders `@alice:matrix.org` on one line; `Security & Recovery` wraps normally. This is not a demonstrated P0 failure. Source does show that account identity uses `lineLimit(1)` plus `minimumScaleFactor`, verification decimals and request identity stay horizontal, notification rows stay horizontal, and there is no Dynamic Type layout switching in the iOS target. Longer Matrix IDs, device names, localizations, and AX XXXL therefore remain a high-risk inference to test.

**Required behavior:**

- Capture the smallest supported iPhone at AX XXXL with long expansion-prone fixtures before selecting the final layout.
- Use `ViewThatFits`, size-category branching, or adaptive layouts so account identity, setting title/value, notification state, and verification identity can move from horizontal to vertical without shrinking.
- Do not use `minimumScaleFactor` as the normal escape hatch for user-selected text size.
- Long Matrix IDs, homeservers, device names, and localized labels must wrap or expose their full value in a detail view.

**Likely owners:**

- `synara-ios/Synara/Features/SettingsView.swift:1238-1271`
- `synara-ios/Synara/App/RootShellView.swift:393-425,561-574`
- `synara-ios/Synara/App/AppTab.swift:176-261`
- `synara-ios/Synara/Features/RoomListView.swift:720-775,1086-1110,1197-1270`

## Priority 2 — consistency and refinement

### P2.1 Reduce one-off geometry and opacity

Create named, role-based metrics for minimum touch target, compact control glyph, room-row/avatar sizes, message group spacing, readable column width, and floating-bar content separation. Current 16, 28, 32, 34, 38, 42, 44, 68, 104, and 112 pt constants encode local fixes rather than a predictable system.

Audit every `.opacity(...)` applied to semantic colors. A semantic color with low opacity over a custom background does not automatically satisfy contrast and can change substantially when material or content moves underneath it.

### P2.2 Preserve native affordances before custom chrome

The app generally benefits from native `List`, `Form`, `NavigationStack`, and `TabView`. Keep those interaction semantics. Remove custom intervention where the system can own behavior, especially bar avoidance, list background behavior, control sizing, and Dynamic Type scaling. Custom styling should be additive—brand tint, avatars, rich message blocks—not a parallel layout system.

### P2.3 Clarify task versus global preferences

Keep notification privacy, appearance theme, account/security, and general notification behavior in Settings. Keep room sorting/filtering in Rooms and room-specific notification behavior in Room Details. Avoid exposing a custom system text-size control; the existing “Uses iOS Dynamic Type” explanation correctly points to the system setting.

### P2.4 Add polish gates for motion and assistive settings

- Verify Reduce Motion disables send/reaction/keyboard transitions without hiding state changes.
- Verify Reduce Transparency leaves bars, composer, and sheets clearly separated.
- Verify Bold Text does not clip chips, metadata, or composer tools.
- Verify Differentiate Without Color and grayscale still distinguish unread, mention, secure, approval, warning, and destructive states.
- Give selected room/space filters a non-color cue in addition to blue fill versus outline.
- Verify VoiceOver rotor/actions for room swipe actions, message actions, open thread, reply target, and verification decisions.
- Confirm the composer placeholder's spoken value matches its visible state, routine sync is not re-announced on every tab, tab-bar-covered rows precede tab items in a sensible swipe order, and emoji/decimal comparison reads in the intended order.
- Verify Large Content Viewer for compact tab/sort controls where the system bar or design appropriately remains fixed-size.

### P2.5 Correct the existing accessibility record

`synara-ios/docs/accessibility-checklist.md` currently says icon-only composer controls are 44×44 pt, while the current source uses 34×34 pt for Attach/Stickers/Send and 28×28 pt for Formatting. Treat this as a documented regression. Update the checklist only after restoring and verifying the intended hit regions; do not use the stale claim as release evidence.

## Recommended implementation slices

Keep changes reviewable and measurable rather than shipping a broad visual rewrite in one commit.

### Slice A — accessibility correctness

1. Adaptive composer and 44 pt hit regions.
2. Central tab-bar/safe-area contract.
3. Adaptive Settings, Notifications, and Verification rows.
4. Deterministic screenshot grid across Dynamic Type and contrast settings.

Acceptance: no overlap, clipping, off-screen placeholder, scaled-down identity text, or unreachable final action at AX XXXL on the smallest supported iPhone. The last Settings, Notifications, and Later actions must be scrolled fully clear and successfully tapped. Repeat the composer path with the keyboard open, home indicator present, reply/edit banners, and the editor at its height cap.

### Slice B — surface and status system

1. Semantic screen/elevation roles.
2. Increased Contrast variants.
3. Notifications background parity.
4. Routine-sync banner demotion.

Acceptance: paired Light/Dark/Increase Contrast screenshots show a continuous base plane from top safe area through the bottom material, with consistent elevation roles across tabs.

### Slice C — reading and identity

1. Timeline rhythm and reply legibility.
2. Rich-message block hierarchy and responsive tables.
3. Shared deterministic avatar presentation.
4. Room-list and notification-row reprioritization.

Acceptance: the same long technical comparison fixture is comfortably scannable on iPhone and Mac, and users can identify speaker/room/state without decoding a row of chips. Exact pixels may differ by platform; semantic hierarchy and content completeness may not.

## Validation matrix and release gates

| Dimension | Required coverage |
| --- | --- |
| Devices | smallest supported iPhone; iPhone 17 Pro; Pro Max; representative iPad split/full width |
| Orientation | portrait and landscape |
| Appearance | Light, Dark, Light + Increase Contrast, Dark + Increase Contrast |
| Type | Large, XXXL, AX Medium, AX Extra Large, AX XXXL; Bold Text |
| Accessibility | VoiceOver manual path; Reduce Motion; Reduce Transparency; Differentiate Without Color; grayscale |
| Content | empty/loading/error/full; very long room/user/device names; RTL and expansion-prone localization; long prose; headings/lists/quote/code/table; encrypted unavailable; failed send |
| Interaction | tab switching, room search/filter/sort, open room, read/reply/react/thread, compose with keyboard/attachment/reply/edit, Settings final action, Notifications unread disclosure/final row, Later final row, notification deep link, all verification states |

Release gates for this effort:

1. Accessibility Inspector has no high-impact hit-region, clipping, contrast, label, or focus-order failures in the primary path.
2. Screenshot baselines exist for each matrix extreme, with explicit accepted deltas rather than “looks reasonable” notes.
3. The physical-device Settings bottom row and composer remain clear of the iOS floating tab bar/keyboard.
4. The rich-message comparison fixture is reviewed in iOS, macOS, and Linux and any semantic hierarchy/content-completeness drift is recorded; it is not a pixel-equality gate.
5. A human VoiceOver pass completes: open room → read sender/body/reply context → compose/send → open notification → review Settings → start and complete/cancel verification.

## Definition of “Apple-quality” for Synara

The goal is not to imitate every first-party screen pixel for pixel. It is to make the app behave like a first-party citizen:

- content remains primary while chrome is quiet and adaptive;
- text respects the user’s size instead of shrinking to preserve a composition;
- system bars, safe areas, keyboard, and materials never obscure meaning or actions;
- Light, Dark, and Increased Contrast are intentional appearances, not opacity variants;
- hierarchy survives long real-world Matrix content and localization;
- security and status flows say what is happening, what the person must compare or decide, and how to recover;
- iOS, macOS, and Linux share information architecture and rendering semantics while each client uses its native interaction patterns.
