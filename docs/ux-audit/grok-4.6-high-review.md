# Grok 4.6 High: holistic cross-client UX review

Date: 2026-08-24  
Branch: `feature/holistic-apple-ux-review`  
Reviewer: Grok 4.6, reasoning effort `high`, run read-only through the Grok CLI  
Adjudication: Codex review of Grok's claims against the checked-out source, supplied screenshots, and current Apple guidance

## Outcome

The strongest conclusion is that this should not be treated as a cosmetic “Apple polish” pass. The release-quality gaps are concentrated in four contracts:

1. content must never be obscured by platform chrome;
2. message structure must survive from Matrix HTML to an accessible rendered hierarchy;
3. security and state affordances must come from factual shared-core state;
4. feature parity must mean shared semantics and capabilities, not identical pixels.

The screenshots establish real problems in a captured build, but they do **not** prove that every problem reproduces on this branch. Current light-theme code explicitly says it was changed to remove the gray “veil,” and current Settings code contains a later 104-point clearance attempt. These changes remain unverified at runtime. A device evidence gate is therefore a P0 requirement, not a documentation afterthought.

No production source was changed as part of this review. Grok did not run the clients, simulator, VoiceOver, Accessibility Inspector, or contrast tooling.

## Evidence rules

- **Fact** means directly visible in supplied/repository screenshots, present in the checked-out source, or stated by official Apple guidance.
- **Inference** means a likely runtime/user effect that has not been reproduced in this review.
- A screenshot finding is evidence about the captured build, not automatically current `HEAD`.
- The 8:04 desktop screenshot is Synara. The 8:05 screenshot is Element (the `All Chats` IA and `Send an unencrypted message...` copy are absent from this repository); it is a competitive reference, not implementation evidence.

## Apple baseline used for adjudication

This plan uses current official guidance as constraints, not as a recipe for making every client look identical:

- [Typography](https://developer.apple.com/design/human-interface-guidelines/typography): use text styles and Dynamic Type, adapt layouts across sizes, and avoid truncating important content.
- [Layout](https://developer.apple.com/design/human-interface-guidelines/layout): account for safe areas and bars so essential content remains reachable while backgrounds may extend underneath.
- [Dark Mode](https://developer.apple.com/design/human-interface-guidelines/dark-mode) and [Color](https://developer.apple.com/design/human-interface-guidelines/color): prefer adaptive semantic colors, supply light/dark/increased-contrast variants, test in context, meet at least 4.5:1 for small custom text and strive for 7:1.
- [Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility): Apple currently recommends a 44×44-point default control size on iOS (28×28 minimum) and 28×28 on macOS (20×20 minimum), plus larger text, VoiceOver, and reduced-motion support.
- [Tab bars](https://developer.apple.com/design/human-interface-guidelines/tab-bars), [Materials](https://developer.apple.com/design/human-interface-guidelines/materials), and [Motion](https://developer.apple.com/design/human-interface-guidelines/motion): keep top-level navigation stable, use system materials purposefully, and make motion brief, optional, and responsive to accessibility settings.
- [Sidebars](https://developer.apple.com/design/human-interface-guidelines/sidebars), [Settings](https://developer.apple.com/design/human-interface-guidelines/settings), and [Lists and tables](https://developer.apple.com/design/human-interface-guidelines/lists-and-tables): retain platform conventions rather than porting iPhone chrome to desktop or desktop density to touch UI.

Correction to Grok: 44×44 is Apple's recommended default iOS target, not the absolute minimum in the current HIG. The 16-point sort buttons still fail even the 28-point minimum; 28- and 34-point composer controls require contextual evaluation but are below the recommended default for this high-frequency touch surface.

## Critical adjudication of Grok's pushback

| Grok claim | Verdict | Reason and decision |
| --- | --- | --- |
| The floating iOS tab bar obscures the last Settings action and room rows. | **Agree on captured build; current reproduction required.** | Both supplied screenshots visibly show overlap. Current code relies on two different fixed clearances—68 points in `RoomListView.swift:241-253` and 104 points in `SettingsView.swift:1464-1474`—around a system `TabView` (`RootShellView.swift:61-82`). The durable fix is system/geometry ownership, not a still-larger constant. |
| The gray light-mode veil is caused by theme surfaces. | **Partially agree.** | The screenshots show a cast. Current `SynaraDesign.swift:102-120` explicitly replaces raw theme-base mixing with near-white opaque surfaces to address exactly that problem. We should not reopen colors blindly; first reproduce current `HEAD` on device and check seams, materials, and contrast numerically. |
| iOS markdown destroys message hierarchy. | **Agree.** | `RoomTimelineView.swift:3422-3426` uses `inlineOnlyPreservingWhitespace`; `TimelineService.swift` converts block HTML into display markdown, including headings to bold. Tests encode this flattening. A semantic block renderer is higher leverage than spacing tweaks. |
| The desktop renderer is already fully strong. | **Partially disagree.** | It preserves HTML structure and already has zebra table rows (`nativeTimelineHtml.css.ts:231-260`), a 672px readable measure, 16px body, and 1.55 line height (`:64-82`). But `h1`–`h6` share weight/margins without level-specific sizes (`:152-165`), wide tables lack an explicit scroll container, and syntax colors are hard-coded. |
| Tables need gradients to match Element. | **Disagree with the prescription.** | Gradients are not the cause of readability. Synara desktop already differentiates table header and alternating rows. Priority is semantic headers, level hierarchy, wrapping/overflow, text measure, and tested contrast. Subtle zebra surfaces are useful; decorative gradients are optional. |
| Make all clients use one visual token system. | **Disagree if this means literal values.** | Share semantic role names, state meanings, and content fixtures. Map them to SwiftUI adaptive/system colors on iOS, a macOS-appropriate system font/material layer, and Linux-friendly web tokens. Identical geometry, blur, fonts, and chrome would be uniformity theater. |
| Make all platforms use Apple's system font. | **Disagree.** | macOS should use the system font stack where it improves native fit. Linux should keep Inter or the user's OS sans-serif; SF is neither guaranteed nor appropriate. Current desktop globally forces Inter (`synara/src/index.css:21-23,46-55`), so a platform-qualified stack is the right experiment. |
| Replace list rendering with native `List` everywhere on iOS. | **Disagree for message blocks.** | A nested scrolling `List` is not an appropriate renderer for bullets inside a timeline cell. Use semantic block views with hanging indents and explicit accessibility grouping/traits. Native `List`/`Form` remains appropriate for room/settings screens. |
| Increase the 104-point Settings spacer until overlap disappears. | **Disagree.** | The inconsistency between 68 and 104 is the problem. Use the actual container safe area/system bar geometry and a single shell contract, then validate visual material bounds and focus frames. |
| Force an identical bubble/flat-message style across iOS and desktop. | **Disagree.** | Preserve semantic content and action parity. Bubble vs flat presentation can differ by platform and viewport. Desktop benefits from readable flat document-like messages; iOS may use restrained grouping/bubbles. |
| Existing “release-prep accepted” status is sufficient. | **Disagree.** | The existing visual matrix tests default Dynamic Type and limited appearances, while Settings/login/Later/Notifications remain partial (`ios-visual-fidelity-matrix.md:10-30`). Its own lines 97-99 require physical-device review before “Complete.” Acceptance must include accessibility modes and the last reachable item. |

## Priority findings

### P0 — blockers to an Apple-quality acceptance claim

#### P0-1 — One safe-area contract for every iOS tab root

- **Fact:** Supplied Rooms and Settings screenshots show content under the floating capsule. Current code uses a native `TabView` but separate 68- and 104-point transparent insets.
- **Inference:** Either constant can fail on a different device, orientation, Dynamic Type size, tab-bar material geometry, or OS revision.
- **Impact:** Last rooms and destructive Settings actions can be obscured or difficult to activate.
- **Recommendation:** Give the root shell ownership of bottom navigation geometry. All tab-root scroll containers consume the same actual safe-area/bar inset; backgrounds may extend below, interactive rows may not. Do not infer the visual capsule from a tab button's accessibility frame.
- **Acceptance:** On a small iPhone, current Pro-size iPhone, Pro Max, portrait/landscape where supported, default and AX5 text, with and without the software keyboard: the last room and final Settings action scroll fully above the **visual** material with at least 8 points of separation, remain hittable, and receive VoiceOver focus. Capture light/dark and Reduce Transparency evidence.
- **Scope:** iOS shell, Rooms, Settings, Later, Notifications, loading/empty/error variants.

#### P0-2 — Message rendering must preserve semantics, not only characters

- **Fact:** iOS inline parsing at `RoomTimelineView.swift:3422-3426` cannot create block heading/list structure. Tables use fixed 112/168-point columns and one generic accessibility label (`:3430-3477`). Code blocks are structurally better (`:3546-3604`). Desktop has semantic HTML, readable measure, code treatment, zebra tables, but no heading-level size scale (`nativeTimelineHtml.css.ts:64-82,152-165,231-260`).
- **Inference:** Long agent reports like the supplied Spectre example are slower to scan on iOS and insufficiently hierarchical on desktop.
- **Impact:** This is the central reading surface; hierarchy loss harms comprehension more than any chrome improvement can repair.
- **Recommendation:** Define a shared presentation AST for paragraph, heading level, ordered/unordered list (including nesting/start), quote, thematic break, inline/fenced code with language, link, spoiler, and table with header/data cells. Keep sanitization in the shared/core boundary. Render that AST natively in SwiftUI and semantically in React. Do not share pixels.
- **Acceptance:** One cross-client fixture containing all block types yields the same content and actions. iOS VoiceOver identifies headings, list position, links, spoiler toggle, and table row/header relationships; desktop exposes correct DOM semantics. Headings have visibly distinct but restrained levels. Tables wrap or scroll within the message without widening the timeline. Large Dynamic Type and 200% desktop zoom remain usable.
- **Scope:** shared presentation contract, iOS timeline, macOS/Linux timeline.

#### P0-3 — Audit every high-frequency target against actual platform guidance

- **Fact:** iOS room sort buttons are 16×16 (`RoomListView.swift:1086-1107`); search/new-room buttons are 42×42 (`:750-774`); filter chips are 32 points high (`SynaraDesign.swift:560-598`). Composer source includes 28/34-point controls. The 16-point controls fail Apple's current 28-point minimum and all are below the 44-point recommended default except where the effective container enlarges them.
- **Impact:** High-frequency navigation and composing controls are harder to acquire, especially with motor impairment or while moving.
- **Recommendation:** Measure effective hit regions, not icon frames. Consolidate the two tiny sort icons into one labeled menu or picker; target the 44-point default in the iPhone room list and composer. On desktop preserve compact visuals but provide at least macOS-recommended targets, keyboard access, and visible focus.
- **Acceptance:** Accessibility Inspector reports compliant effective frames; an automated UI test asserts non-overlap and minimum target dimensions; all controls have concise labels, selected state, and keyboard/focus behavior. No control relies on `contentShape` to enlarge a frame that was never expanded.
- **Scope:** iOS room list/composer/toolbar first, then macOS/Linux sidebar/composer.

#### P0-4 — Factual security and state affordances only

- **Fact:** `SynaraRoomAvatar.swift:116-120` computes `isSecureRoom` from room-name substrings (`security`, `secure`, `e2e`). A repository evidence screenshot shows a lock on `test-e2e-room`.
- **Impact:** A security-looking symbol can disagree with actual Matrix encryption state. That is a trust defect, not icon polish.
- **Recommendation:** Put room encryption, verification state, favorite state, unread/highlight, mute, send lifecycle, and capability flags in the shared semantic contract. If a state is unavailable, omit the affordance or label it unknown—never infer from copy/name.
- **Acceptance:** An unencrypted fixture named `test-e2e-room` has no lock; an encrypted room named `Alerts` does. iOS and desktop consume the same state fixture. Tests forbid substring-based encryption inference.
- **Scope:** shared core, iOS room list/timeline/composer, macOS/Linux corresponding surfaces.

#### P0-5 — Replace checklist acceptance with a device-and-accessibility evidence gate

- **Fact:** The current visual matrix targets default Dynamic Type, mostly light mode, one simulator, and lists multiple screens as partial (`ios-visual-fidelity-matrix.md:10-30`). Its own completion rule requires physical-device review (`:97-99`). Current code has an infinite shimmer animation without a Reduce Motion branch (`SynaraDesign.swift:765-790`) and an agent-room-wide forced dark scheme (`RoomTimelineView.swift:348-353`). No desktop `prefers-reduced-motion`, `prefers-contrast`, or `forced-colors` handling was found by repository search.
- **Recommendation:** Make the validation matrix below a merge/release artifact. A unit-test count is not visual or assistive-technology evidence.
- **Acceptance:** Each P0 screen has dated screenshots and results for light/dark, large text/zoom, VoiceOver or screen reader, Increased Contrast/high contrast, Reduce Motion, Reduce Transparency, keyboard/focus, smallest and largest supported viewport. Failures block “Complete.”
- **Scope:** all clients and UX documentation.

### P1 — high-value readability and interaction improvements

#### P1-1 — A platform-aware type system and readable measure

- Keep SwiftUI Dynamic Type roles already centralized in `SynaraTypography` (`SynaraDesign.swift:408-423`), but replace residual fixed-size text and validate at accessibility sizes.
- Retain the desktop message body's 16px/1.55 line height/672px measure as a good baseline (`nativeTimelineHtml.css.ts:64-82`), then create an actual h1–h6 scale and tune paragraph/list/metadata spacing.
- Use a macOS system font stack in the macOS Tauri shell after A/B screenshot review; retain Inter or OS sans on Linux. Monospace should stay platform-qualified (`SFMono`/Menlo on macOS, suitable Linux fallbacks).
- **Acceptance:** body text, metadata, link, code, quote, and all heading roles have documented tokens; no meaningful body text truncates at AX5/200%; long prose remains approximately 55–80 characters per line where the viewport permits.

#### P1-2 — Semantic color roles before new gradients

- **Fact:** iOS has a custom HSL theme ramp and a contrast helper; current light surfaces explicitly address the earlier veil (`SynaraDesign.swift:79-137`). Desktop uses Folds/Cinny colors plus timeline CSS and hard-coded syntax colors.
- Define roles for canvas, grouped canvas, row, elevated surface, separator, primary/secondary/tertiary text, link, accent/on-accent, unread, mention, destructive, encrypted, warning, agent, selection, focus, and code tokens.
- Map roles to adaptive native colors on iOS and platform-qualified CSS variables on desktop. Brand tint may remain, but it must not tint the entire reading plane unless each appearance/contrast combination is tested.
- **Acceptance:** automated contrast checks use the **actual foreground/background pair** for every role in light, dark, and increased-contrast modes. Small text meets 4.5:1; aim for 7:1. Selected accent chips/badges have a tested `onAccent` role rather than unconditional white.

#### P1-3 — Room-list hierarchy should be mobile-native, not desktop compressed

- The current iOS header stacks account identity, search, compose, filters, optional spaces, multiple sections, per-section sort, and counts. The screenshot's `clock Aa 9` reads as one ambiguous cluster.
- Preserve room semantics across clients but simplify mobile controls: one labeled sort menu, count incorporated into the section label, system-toolbar behavior where possible, stable row anatomy, and progressive disclosure for secondary filters.
- Give avatars stable identity behavior: authenticated MXC image first, deterministic fallback by stable ID, a documented distinction between people (circle) and rooms (rounded square) if product adopts it. Do not copy Element's colored circles solely because they look lively.
- **Acceptance:** at default and AX5, room name/unread/mention/time/security remain comprehensible; preview may yield before identity/state. VoiceOver reads one coherent row summary and exposes independent actions through actions/menus, not tiny adjacent glyphs.

#### P1-4 — Composer hierarchy and clearance

- The composer should own keyboard/safe-area behavior and guarantee the last message scrolls above it on every client.
- On iOS, use recommended-default touch targets, reveal formatting progressively, preserve Draft/Reply/Edit/Upload/Send lifecycle, and honor Reduce Motion. On desktop, preserve keyboard-first formatting and visible focus; keep compact actions without reducing target size to the icon.
- Encryption status must be factual and consistent. Whether unencrypted rooms use warning placeholder copy is a product decision; it must not be inferred from room names or stale local state.
- **Acceptance:** last message and jump-to-bottom control never sit behind composer; VoiceOver labels attach/format/sticker/send and state; hardware keyboard and IME composition work; failure/retry remains visible; no animation runs when Reduce Motion is enabled.

#### P1-5 — Settings should use each platform's settings idiom

- iOS `Form`/`NavigationLink` is the correct base; stop fighting it with guessed bottom space and an unrelated background plane. Clarify whether “Appearance” controls system light/dark behavior or only a chrome tint.
- macOS should provide standard Settings-window behavior, `Command-,`, expected pane navigation, keyboard focus, and last-pane restoration. Linux may keep an in-app settings page and platform packaging/security integration.
- Do not share the rendered settings hierarchy blindly; share account/security/notification capabilities and preference schemas.
- **Acceptance:** destructive actions are fully visible, confirmations use platform roles, all settings remain reachable by keyboard/screen reader, and unavailable capabilities are honestly omitted or explained.

#### P1-6 — Motion, transparency, focus, and non-default accessibility modes

- Disable shimmer and nonessential slide/pop transitions under Reduce Motion; use a static skeleton.
- Use opaque semantic surfaces when Reduce Transparency is enabled. Liquid Glass must not be the only separator between controls and content.
- Add desktop `prefers-reduced-motion`, high-contrast/forced-colors behavior, strong focus-visible rings, and screen-reader names for custom controls.
- **Acceptance:** no continuous or spatial animation in reduced mode; chrome stays readable when transparent materials are removed; a full room→timeline→compose→settings workflow is keyboard operable on macOS/Linux.

### P2 — polish after contracts are proven

| Item | Recommendation | Scope |
| --- | --- | --- |
| Loading/empty/error parity | Apply the same safe-area and accessibility behavior as loaded content; keep honest retry/sign-in actions; static skeleton under Reduce Motion. | iOS first, all clients |
| Code presentation | Preserve language label, copy, selection, horizontal containment, and syntax colors tested in both appearances. Line numbers should remain hidden from screen readers unless useful. | all clients |
| Table refinement | Use subtle header/zebra surfaces, wrapping, adaptive column sizing and optional horizontal scrolling. Avoid decorative gradients unless testing shows a material scanability gain. | all clients |
| Metadata rhythm | Tune sender/time/reaction grouping and vertical rhythm after semantic blocks land; metadata must remain readable but subordinate. | all clients |
| iPad adaptation | Use sidebar/split-view navigation where appropriate instead of stretching the phone tab composition. | iOS/iPadOS |
| Agent identity | Represent agent status semantically; do not force the entire room to dark mode without an explicit product choice and accessibility proof. | all clients |
| Localization stress | Test long German/French labels, right-to-left layout, CJK, emoji, and long unbroken code/URLs. | all clients |

## Shared parity contract

“Parity” should be specified at three layers:

| Shared across clients | Platform adapter | Intentionally native |
| --- | --- | --- |
| Room identity, image reference/fallback seed, favorite/mute/unread/highlight, encryption/verification, capabilities | Swift model / TypeScript view model mapping | Tab bar vs sidebar/split view |
| Message presentation AST, sanitization outcome, reply/edit/thread/reaction relationships, send lifecycle | SwiftUI blocks / semantic React DOM | Bubble vs flat grouping, density, hover |
| Composer commands, attachment capability, draft state, retry/failure, encryption truth | UIKit text input / web editor | Keyboard shortcuts, menus, pickers, drag/drop |
| Preference schema and account/security/notification capability | iOS Form / macOS Settings / Linux page | Window/pane behavior and OS integrations |
| Status/error taxonomy and user-facing copy keys | Native accessibility/focus adapters | Materials, font families, target geometry |

The shared core should not own SwiftUI views, CSS, corner radii, blur values, tab geometry, touch sizes, or desktop keyboard/menu behavior.

## Compact token recommendation

Use the same **role names**, then map them per platform:

| Role | iOS/iPadOS mapping | macOS mapping | Linux mapping |
| --- | --- | --- | --- |
| `surface.canvas/grouped/row/elevated` | adaptive system colors/materials, with opaque reduced-transparency variant | system/window/sidebar-aware CSS variables | Folds/Inter-friendly opaque variables |
| `text.primary/secondary/tertiary/link` | Dynamic Type + adaptive colors | system font stack + rem/user zoom | Inter/OS sans + rem/user zoom |
| `state.unread/mention/destructive/encrypted/warning/agent` | semantic `Color` and SF Symbols where appropriate | semantic CSS + platform icons | same semantics, Linux-appropriate icons |
| `control.accent/onAccent/selection/focus` | app tint, tested `onAccent`, native focus | CSS focus-visible + macOS accent behavior | CSS focus-visible + high-contrast fallback |
| `content.code.*` | platform monospace, semantic syntax palette | SFMono/Menlo fallback palette | system monospace fallback palette |

Spacing and type roles may share names (`space.1…6`, `body`, `meta`, `heading.1…6`, `mono`) but not necessarily identical values.

## Screen-by-screen validation matrix

Every applicable cell needs a dated result and evidence path. “N/A” requires a reason.

| Screen/surface | Core visual states | Accessibility modes | Content/state fixtures | Viewports/platforms | Critical assertions |
| --- | --- | --- | --- | --- | --- |
| Rooms | light, dark, loading, empty, error, offline | AX5/AX7, VoiceOver, Increased Contrast, Reduce Motion/Transparency | favorite, unread, mention, DM, real encrypted/unencrypted, missing avatar, long/localized names | small iPhone, Pro, Pro Max, iPad split; desktop sidebar on macOS/Linux | last row clears chrome; factual lock; coherent row; compliant targets |
| Timeline | light/dark, initial load, pagination, send failure, offline | large text/200% zoom, VoiceOver/screen reader, contrast, reduced motion | shared markdown HTML fixture, reply/edit/thread/reaction, media, agent event | phone/tablet, narrow/wide desktop | semantic blocks; readable measure; no horizontal page blowout; last message clears composer |
| Composer | empty, focused, formatting, reply/edit, upload, send/fail/retry | VoiceOver, keyboard/focus, reduced motion | encrypted/unencrypted, IME, multiline, attachments | software/hardware keyboard; macOS/Linux | truthful state; target size; no overlap; draft preserved |
| Settings | signed in/out, light/dark, logout/error | large text, VoiceOver/screen reader, contrast/transparency, keyboard | long account ID, unavailable capability, destructive confirmation | all supported iPhones/iPad; macOS Settings; Linux page | final action visible; roles correct; platform navigation idiom |
| Later | loading/empty/error/populated | large text, VoiceOver/screen reader, reduced motion | overdue/today/future/completed, long title | mobile + desktop | row actions reachable; time/urgency not color-only |
| Notifications | loading/empty/error, badge states | large text, VoiceOver/screen reader | mention/invite/agent/unread room | mobile + desktop | badge semantics; deep-link target; no chrome overlap |
| Login/account switcher | light/dark, loading, invalid/error | large text, VoiceOver/screen reader, keyboard | long homeserver/MXID, SSO/password states | all clients | error associated with field; secrets not exposed; progress recoverable |
| Agent approval | light/dark/system, pending/approved/rejected/error | large text, VoiceOver/screen reader, reduced motion | long report, all content blocks, crypto states | all clients | no forced-dark surprise; actions explicit; state not color-only |

Minimum manual devices/environments: smallest supported iPhone class, current standard/Pro class, large class, one iPad size, current supported macOS with keyboard/VoiceOver, and supported Linux desktop with keyboard/screen reader/high-contrast theme. Simulator automation remains useful but cannot be the only evidence for material bounds, physical readability, or assistive technology.

## Implementation order

1. **Baseline:** capture current branch on the validation matrix; confirm whether the veil and tab overlap still reproduce. Add the shared long-message fixture.
2. **P0 shell/trust:** one tab-safe-area contract, factual room security state, target audit, accessibility-state harness.
3. **P0 content:** shared message AST and semantic renderers; desktop heading/table containment; iOS heading/list/table/spoiler semantics.
4. **P1 core screens:** typography/color tokens, room list, composer, Settings, avatars, empty/loading/error.
5. **Platform fit:** macOS font/settings/menu/focus behavior, Linux font/integration/high-contrast behavior, iPad split-view adaptation.
6. **Evidence and only then polish:** complete device matrix, compare against Element for scanability, tune rhythm/zebra surfaces/motion without weakening native behavior.

## Highest-leverage changes

1. Replace per-screen 68/104-point bottom guesses with one measured/system-owned iOS navigation clearance contract.
2. Build one shared semantic message fixture and presentation AST, then make both renderers pass visual and accessibility criteria.
3. Remove room-name security inference and source all trust/state icons from shared factual state.
4. Audit the room list and composer against effective target size, large text, focus, motion, and contrast—not default screenshots alone.
5. Make physical-device and assistive-technology evidence a prerequisite for “Complete” and release-quality UX claims.

## Product decisions required

1. Is Element only a readability benchmark, or is its unified `All Chats` information architecture a desired product direction?
2. Should unencrypted rooms show persistent composer copy/status, a contextual warning, or only security details—and what shared state is authoritative?
3. Are iOS bubbles and desktop flat messages intentional platform adaptations?
4. Is Graphite/Blurple a durable brand surface or merely a user-selectable accent? If durable, what contrast budget and non-color fallback are required?
5. What is the authoritative cross-client favorite/star contract and which management actions must exist on mobile versus desktop?

## Explicit non-goals for this branch

- No production UI implementation in this review document.
- No attempt to make macOS, Linux, iOS, and iPadOS pixel-identical.
- No new custom Liquid Glass imitation.
- No release/PR claim based solely on mockups, simulator frame assertions, or unit-test counts.
- No decorative gradient work before semantic hierarchy, containment, and contrast are proven.
