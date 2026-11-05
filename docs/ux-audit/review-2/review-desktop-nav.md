# Desktop navigation shell — deep UX/UI review (Slack-informed)

Scope: space rail, room list (Home / Space / Direct), unread badges, page chrome, lobby, search entry points, splash and welcome screens. Code reading only, on `main` (clean). Token values resolved from `folds` dist: T300 = 14px/20px, T200 = 12px/18px, B300 = 12px, L400 = 12px, H4 = 18px, H5 = 16px, H6 = 14px; S100/200/300/400/500/700 = 4/8/12/16/20/32px; R300/400/500 = 4/8/12px; Header size 300 = 32px, size 600 = 54px; Chip size 400 = 24px tall; Badge size 400 = 16px, size 200 = 8px dot.

## Summary

1. The room list is structurally close to Slack already (32px rows, bold unreads, collapsible sections that keep unread rows visible, hover-revealed kebab, resizable pane, virtualised at 50+ rooms) but it still uses the pre-restyle 14px type while the timeline moved to 15px, so the two panes no longer share one scale.
2. The biggest usability gaps are behavioural, not visual: launch lands on a "pick a room" Welcome page instead of the last room; the unread badge disappears the moment you hover a row; mentions are green; muted and private rooms are not distinguishable from ordinary read rooms.
3. There are three "search" entry points (rail icon → quick switcher, a 40px "Message Search" row in every list, Cmd+K) and three permanent 40px utility rows above the Home list that cost ~130px of list height on every window.
4. Section headers are 24px pill chips with 12px text and an always-on sort icon; Slack-style would be a full-width, hover-revealed, 14px semibold header. Space lists have no sort, no favourites, no empty/loading state; Direct ignores Core's sort presentation.
5. Cross-platform drift: iOS shows Favorites / Channels / Direct messages / Low priority in one list with lock glyphs, star glyphs and a primary-coloured `#` when unread; desktop splits DMs into a rail tab, never shows locks or stars, and names the same concepts "Rooms", "Chats", "channel" and "Direct Messages" in different places.

## Findings (ordered by impact)

### NAV-1 · P0 · Launch and "nothing selected" land on a Welcome page, not the last room
- **Where:** `synara/src/app/pages/Router.tsx:190`, `:225` (index routes render `WelcomePage`); `synara/src/app/pages/client/WelcomePage.tsx:61-63` ("Pick a room or direct message from the sidebar."); last-active path already tracked in `navToActivePath` and used by the rail (`pages/client/sidebar/HomeTab.tsx:68-74`).
- **Today:** Every cold start and every click on Home/Direct with no room selected shows a centred logo, version link, three external links and an instruction sentence.
- **Proposal:** Index route redirects to `navToActivePath.get('home')` (or the most recent room in `presentation.recentOrder[0]`) when one exists; only a brand-new account sees Welcome. Keep Welcome content as the empty state of the timeline pane, but rewrite it as a task prompt ("Pick a conversation, or press ⌘K to jump to one") and move the three external links into Settings → About.
- **Effort:** S. **Risk:** Low (routing only; respect `pathSearchParam` deep links). **iOS:** No counterpart (tab roots restore themselves).

### NAV-2 · P0 · Hovering a row hides its unread count, typing badge and mute icon
- **Where:** `synara/src/app/features/room-nav/RoomNavItem.tsx:371`, `:415-433` (`!optionsVisible && …` gates the badge, typing badge and notification-mode icon), `:437-487` (kebab renders in `NavItemOptions` only while hovered).
- **Today:** With 50 rooms and several unreads, sweeping the pointer down the list makes the counts blink out one by one; keyboard focus (`useFocusWithin`) does the same, so an arrow-key user never sees the count of the focused room.
- **Proposal:** Keep the badge and mute icon mounted; render the kebab *beside* them in a fixed 24px slot that is `opacity: 0` at rest and `opacity: 1` on `:hover`/`:focus-within` (Slack pattern). Reserve the slot width always so the name truncation does not jump. Drop the typing badge from the sidebar entirely (Slack never shows typing in the list; it adds motion to a scanning surface).
- **Effort:** S. **Risk:** Low; `areRoomNavItemPropsEqual` unaffected. **iOS:** no hover; no change.

### NAV-3 · P0 · Mention badges are green ("Success"), not the accent used for mentions elsewhere
- **Where:** `synara/src/app/components/unread-badge/UnreadBadge.tsx:23` (`variant={highlight ? 'Success' : 'Secondary'}`); same component on the rail (`sidebar/HomeTab.tsx:103-105`, `sidebar/SpaceTabs.tsx:476-478`). Success Main is `#017343` (`synara/src/colors.css.ts:56`). The same green is reused for LiveCallChip (`room-nav/LiveCallChip.tsx:13`), "Suggested" badges (`features/lobby/RoomItem.tsx:167`), and drag-drop lines (`components/sidebar/Sidebar.css.ts:45,55`).
- **Today:** "Someone mentioned you" and "a call is live" look identical; the timeline's mention pill for *me* is gold, so the badge contradicts the thing it points at.
- **Proposal:** Add a `Mention` badge role: `color.Critical.Main`-adjacent red or the timeline's gold mention token, white text, W700 L400. Unread-without-mention stays neutral (`Secondary`). Keep Success for live calls only. Define it once in `UnreadBadge` and reuse on rail + rows + Inbox.
- **Effort:** S. **Risk:** Low; check 4.5:1 on `--synara-selected-surface` light `#e4e7eb` and dark `#292c31`. **iOS:** yes — iOS unread/mention accent should use the same role (`RoomListView.swift:1357` area).

### NAV-4 · P1 · Room-list type is 14px while the restyled timeline is 15px
- **Where:** `synara/src/app/components/nav/NavItemContent.tsx:8` (`size="T300"` = 14px/20px); `features/room-nav/styles.css.ts:12-20` (`#` glyph 17px, weight 500); timeline body `features/room/nativeTimelineHtml.css.ts:353-355` (15px / 1.47) and bold 15px sender names.
- **Today:** Sidebar names read one step smaller and lighter than message text; Slack uses the same 15px for both.
- **Proposal:** `NavItemContent` → `fontSize: toRem(15)`, `lineHeight: 1.47`; keep rows at 32px (fits: 22px line in 32px). Unread weight W700 (not W600) to match sender names; `#` glyph `toRem(15)` W600 at `width: toRem(18)`. Apply the same size to section headers (NAV-7) and the utility rows (NAV-6) so the pane has one scale.
- **Effort:** S. **Risk:** Low; re-measure truncation at 196px min width (`components/page/Page.tsx:42`). **iOS:** already `SynaraTypography.body` semibold/regular (`RoomListView.swift:1357`) — matches after the change.

### NAV-5 · P1 · Three "search" entry points with the same icon; message search is a permanent 40px row
- **Where:** rail `pages/client/sidebar/SearchTab.tsx:13-19` (opens quick switcher); Home row "Message Search" `pages/client/home/Home.tsx:456-479`; Space row `pages/client/space/Space.tsx:507-526`; Cmd+K `features/search/Search.tsx:620-633`; the modal's own placeholder is just "Search" (`Search.tsx:418`).
- **Today:** Two `Icons.Search` affordances 70px apart do different things. Message search costs a 40px row above *every* room list.
- **Proposal:** Slack pattern: one search field at the top of the room-list pane (below the header, 32px, `Background` variant, placeholder "Jump to or search…", trailing `⌘K` kbd) that opens the quick switcher; add "Search messages in Home / this space" as the first `>` action and as an item in the header kebab; remove the "Message Search" rows and the rail Search tab. Quick-switcher placeholder → "Jump to a room, person or space".
- **Effort:** M. **Risk:** Medium (route `getHomeSearchPath` must stay reachable; update `useHomeSearchSelected` consumers and e2e). **iOS:** iOS already has a single `.searchable`; no change.

### NAV-6 · P1 · Permanent utility rows ("Create Room", "Join with Address", "Lobby", "Create Chat") eat list height and misalign with room rows
- **Where:** `pages/client/home/Home.tsx:407-480` (3 × `minHeight: toRem(40)` rows, `components/nav/styles.css.ts:57`), `pages/client/space/Space.tsx:487-526`, `pages/client/direct/Direct.tsx:236-256`. Icon column is `Avatar size="200"` (24px) + gap S200 → text starts at 40px, while room rows start text at 8 + 18 + 8 = 34px (`NavItemContent` paddingLeft S200 + glyph 18 + gap 200).
- **Today:** ~130px of every Home list is fixed chrome before the first room; the two row kinds have different heights (40 vs 32) and different text x-offsets.
- **Proposal:** Move "Create Room" / "Join with Address" into the header kebab and into a `+` at the end of the Rooms section header (Slack: "Add channels" row at the *bottom* of the section). Keep "Lobby" in Space but as a 32px row using the 18px glyph column (`Icons.Flag` at 15px). Any remaining utility row: 32px, same `NavItemContent`, glyph column 18px.
- **Effort:** M. **Risk:** Low–medium (route selection states `useHomeCreateSelected`, `useHomeJoinSelected` keep working from the menu). **iOS:** iOS puts compose/new-room in the toolbar; no change.

### NAV-7 · P1 · Section headers are 24px pill chips with 12px text and an always-on sort icon
- **Where:** `features/room-nav/RoomNavCategoryButton.tsx:8-25` (`Chip` radii Pill, B300 12px, chevron opacity P400); `components/nav/NavCategoryHeader.tsx:11-16` (Header size 300 = 32px); Home sort button `pages/client/home/Home.tsx:488-492`, `:526-530`, `Home.css.ts:5-18` (28px, always visible); Favorites header is a plain `Text size="B300"` and is *not* collapsible (`Home.tsx:484-494`). Section gap `gap="300"` (`Home.tsx:406`) but Space sub-space headers use `paddingTop: S400` (`Space.tsx:549`).
- **Today:** Headers look like filter chips, not section titles; the chevron sits inside a pill; Favorites cannot be collapsed; sort icons add a permanent 28px control per section.
- **Proposal:** One `NavSectionHeader` component: full-width 28px button, left-aligned 14px (H6) W600 `--synara-content-secondary` label in the row's glyph column, chevron (12px) visible only on hover or when collapsed, right slot for hover-revealed sort kebab (reuse `RoomListSortIcons` as its menu) and optional `+`. Same component for Favorites, Rooms, Chats and every Space sub-space; one vertical rhythm (S300 above every header). Keep the "collapsed still shows unread + selected" behaviour (`Home.tsx:358-360`, `Space.tsx:440-447`, `Direct.tsx:204-207`) — that is exactly Slack's.
- **Effort:** M. **Risk:** Low. **iOS:** `RoomSectionHeader` (`RoomListView.swift:1172-1190`) already is label + sort + count; align label weight and drop the count on desktop (Slack has none).

### NAV-8 · P1 · Muted, private and favourite rooms are indistinguishable from ordinary read rooms
- **Where:** glyph always `#` (`components/room-glyph/roomGlyph.ts:26-27`); `RoomSummary` carries `joinRule`, `encryptionStatus`, `isFavorite`, `notificationMode` (`features/matrix-dto/room.ts:63-73`); mute shows only as a 12px bell icon at the row's right (`RoomNavItem.tsx:425-433`); muted rooms are excluded from unread so they never bold (`e2e/room-list-harness/presentation.ts:38`, mirrors Core); favourites are a section in Home only, nothing in Space lists; context menu uses `Icons.Pin` for "Favorites" (`RoomNavItem.tsx:197`).
- **Today:** In a 50-room list, a muted room and a quiet room look the same; invite-only rooms look public; in a Space you cannot tell which rooms you favourited.
- **Proposal:** (a) Private (`joinRule !== 'public'`) → lock glyph in the `#` slot (15px `Icons.Lock`), as iOS does (`RoomListView.swift:1360-1364`) and Slack does. (b) Muted → `RoomNameMuted` colour `--synara-content-tertiary` + glyph tertiary; keep the bell icon but only on hover/selected (the dimming carries the state). (c) Favourite → small star (11px, `--synara-content-tertiary`) after the name in Space lists; rename the menu icon to a star to match iOS `star.fill` and the word "Favorites".
- **Effort:** S–M. **Risk:** Low; factual state only (audit P0-4). **iOS:** already shows lock + star; desktop catches up.

### NAV-9 · P1 · `#` glyph never changes with unread; selected rows carry a 3-layer shadow
- **Where:** glyph colour fixed `--synara-content-secondary` (`features/room-nav/styles.css.ts:15`); selected row = `--synara-selected-surface` + `ContainerLine` border + `raisedShadow` (two inset edges + 3px and 8px drop shadows, `components/nav/styles.css.ts:83-89`, `styles/Depth.css.ts:29-33`); every hovered row also gets `raisedShadow` + border (`features/room-nav/styles.css.ts:52-56`).
- **Today:** Hovering down a dense list makes each 32px row "lift" with a shadow; the selected row is a bordered, shadowed tile inside a flat list; the glyph stays grey when the name is bold.
- **Proposal:** Slack-flat: hover = `Surface.ContainerHover` tint only, no border/shadow; selected = `--synara-selected-surface` fill + the existing 3px primary bar, no shadow, no border (keep `raisedShadow` for `prefers-contrast: more` → solid edge). Glyph follows the name: `--synara-content-primary` when unread, else secondary (iOS `roomGlyphColor`, `RoomListView.swift:1330-1336`).
- **Effort:** S. **Risk:** Low; keep 3:1 on the 3px bar. **iOS:** glyph already matches.

### NAV-10 · P1 · Space lists have no sort, no favourites, no loading or empty state
- **Where:** `pages/client/space/Space.tsx` imports no `NavLoading`/`NavEmpty` (grep count 0), no `RoomListSortIcons`; hierarchy order is `m.space.child` order key then timestamp (`hooks/useSpaceHierarchy.ts:168`, `:291-300`) and switches to activity only when the sub-space is collapsed.
- **Today:** A space with 40 rooms is ordered by whoever last edited `m.space.child`; a user who sorts Home by name cannot do so in a space; a just-joined empty space shows only "Lobby" and "Message Search" with no explanation.
- **Proposal:** Per-space sort (`readRoomListSort(storage, 'space')`, default "Space order", options Recent / Name) in the section header kebab; a "Favorites" section at the top of each Space list filtered from `favoriteRoomIdsFromPresentation`; `NavLoading("Loading space…")` while `useNativeRoomListLoaded()` is false and a `NavEmptyLayout` ("No rooms yet in this space" → Explore space / Create room) when the hierarchy has no joined rooms.
- **Effort:** M. **Risk:** Medium (hierarchy sort callback is already pluggable). **iOS:** iOS groups by parent space inside one list; sort options should match names.

### NAV-11 · P1 · Direct list sorts locally and ignores Core's presentation; labels diverge from iOS
- **Where:** `pages/client/direct/Direct.tsx:203` (`factoryRoomIdByActivity()`), header "Direct Messages" (`:117-119`), section "Chats" (`:264`), empty-state button "Direct Message" (`:175-178`), create row "Create Chat" (`:249`); Home uses `orderRoomIdsByPresentation` (`pages/client/home/homeRoomList.ts:70-74`). iOS: "Direct messages" with its own persisted sort (`RoomListView.swift:22`, `:227-231`).
- **Today:** DMs cannot be sorted by name; Home and Direct use different sort code paths; four labels for one concept.
- **Proposal:** Use `sortHomeRoomIds(…, 'directs')` with a `RoomListSort` persisted under `synara.roomListSort.directs` (key already defined, `homeRoomList.ts:17`); section label "Direct messages"; header "Direct messages"; action "New message". See NAV-12 for the bigger structural option.
- **Effort:** S. **Risk:** Low. **iOS:** label parity.

### NAV-12 · P1 · DMs live behind a separate rail tab; Slack and iOS show Channels + Direct messages in one list
- **Where:** `pages/client/SidebarNav.tsx:31-34` (Home and Direct tabs); scope filter `state/hooks/navigationUnread.ts:24-29` (`home` = non-direct, non-space-child; `direct` = isDirect); iOS one list: `RoomListView.swift:92-107`, `:164-246`.
- **Today:** Replying to a DM while reading a room means a rail switch, a list change and a lost scroll position; the Home rail badge and the DM rail badge split one attention count.
- **Proposal (structural, confirm first):** Home pane shows Favorites / Rooms / Direct messages sections (Direct messages collapsible, activity-sorted, same `RoomNavItem`); the Direct rail tab becomes a filter that scrolls/expands to that section (or is removed). This is the single biggest Slack alignment available and matches iOS. Not part of the rejected UX-Next work (no chips/avatars/activity folding).
- **Effort:** L. **Risk:** Medium–high (routing `/direct/*` paths, `navToActivePath` scopes, e2e `NavigationUnreadFixture`). **iOS:** already this shape.

### NAV-13 · P1 · Rail badges show counts for plain unreads; badge sits top-left
- **Where:** `components/sidebar/Sidebar.css.ts:124-132` (`top/left: -6px`), `sidebar/HomeTab.tsx:103-105`, `DirectTab.tsx:103-105`, `SpaceTabs.tsx:476-478`, `:616-618` all pass `count={unread.total}`.
- **Today:** With 50 rooms the rail shows e.g. "212" on Home and "37" on a space — numbers nobody acts on — anchored at the top-left, opposite to macOS/Slack convention.
- **Proposal:** Slack rule: unread without mention → 8px dot (`UnreadBadge count={0}` already renders size 200); mention → count badge in the mention colour (NAV-3). Anchor top-right (`top: -4px; right: -4px`). Inbox keeps counts (they are actionable items).
- **Effort:** S. **Risk:** Low. **iOS:** tab-bar badge should follow the same dot/count rule.

### NAV-14 · P1 · Keyboard navigation is DOM-scraped and lacks the Slack set
- **Where:** row Arrow/Home/End query `a[data-room-nav-link]` (`RoomNavItem.tsx:356-365`, comment admits only rendered rows are reachable); Alt+↑/↓ prev/next (`features/desktop-keyboard/appShortcuts.ts:67-69`, `DesktopAppShortcuts.tsx:37-45`); Cmd+K (`features/search/Search.tsx:620`); no next-unread, no sidebar toggle, no back/forward.
- **Today:** Home/End stop at the virtualiser's overscan window; there is no "jump to next unread" (Slack Alt+Shift+↑/↓), no ⌘⇧K (DMs), no ⌘[ / ⌘] history, and the prior audit's "show/hide navigation" command still does not exist.
- **Proposal:** Drive Alt+↑/↓ and Home/End from the ordered id arrays (`mainRoomIds`, `hierarchy`, `sortedDirects`) via a small `RoomListOrderContext`, then `virtualizer.scrollToIndex`; add Alt+Shift+↑/↓ = next/previous room in `presentation.unread`; add ⌘⇧D (toggle room-list pane) persisted next to `synara.navigationPaneWidth`; add ⌘[ / ⌘] using router history. Show the shortcuts in the header kebab.
- **Effort:** M. **Risk:** Low–medium (menu.rs duplicates for macOS View menu). **iOS:** n/a.

### NAV-15 · P1 · Terminology: Rooms / Channels / channel / Chats / Direct Messages
- **Where:** "Rooms" (`Home.tsx:523`), "Chats" (`Direct.tsx:264`), error copy "Couldn't mark this channel as read." (`RoomNavItem.tsx:92`, `:111`), "Room Settings" / "Leave Room" (`RoomNavItem.tsx:273`, `:292`), iOS "Channels" (`RoomListView.swift:213`), composer placeholder "Message #room".
- **Proposal:** Pick one per concept and apply across desktop+iOS: section "Channels" (matches `#`, composer and iOS) or "Rooms" everywhere; "Direct messages"; never "Chats"/"channel" in error copy. Add a `t()` key table so both clients read from one list.
- **Effort:** S. **Risk:** Low. **iOS:** yes.

### NAV-16 · P2 · Only the Home header is a window drag region
- **Where:** `pages/client/home/Home.tsx:225` has `data-tauri-drag-region`; `pages/client/space/Space.tsx:289`, `direct/Direct.tsx:114`, `explore/Explore.tsx:178` do not; `RoomViewHeader.tsx:480` does.
- **Today:** On macOS you can drag the window from the Home list header but not from a space's or DM list header.
- **Proposal:** Put `data-tauri-drag-region` on `PageNavHeader` itself (`components/page/Page.tsx:139-149`) so every pane header behaves the same.
- **Effort:** S. **Risk:** None. **iOS:** n/a.

### NAV-17 · P2 · Pane header hierarchy: logo + server name in Home, name + lock in Space, title in Direct
- **Where:** `Home.tsx:228-237` (22px PNG + H4 server name), `Space.tsx:292-296` (H4 + 12px lock), `Direct.tsx:117-119`; room header title is W700 `--synara-content-heading` with −0.01em (`features/room/RoomViewHeader.css.ts:18-22`); `PageNavHeader` is 54px.
- **Today:** Three header compositions with a 18px W-default title that is lighter than the room title beside it.
- **Proposal:** One `PageNavTitle`: 15px W700 `--synara-content-heading`, −0.01em (same as `RoomTitle`), 48px header; Home shows the server/workspace name only (logo belongs on splash/about); Space shows avatar 20px + name + lock; chevron-down (Slack workspace menu) opens the existing kebab menu on title click.
- **Effort:** S. **Risk:** Low. **iOS:** `SynaraTypography.screenTitle.weight(.bold)` (`RoomListView.swift:1231`) already bold.

### NAV-18 · P2 · Rail composition: two bottom stacks of five fixed items; Explore/Create as separate 42px tiles
- **Where:** `pages/client/SidebarNav.tsx:36-52` (Explore, Create after spaces; sticky Search, Approvals, Unverified, Inbox, Settings); `components/sidebar/Sidebar.css.ts:19-29` (42px tiles, 12px gaps).
- **Today:** At 720px tall the scrolling space area is ~400px once 5 sticky items (~270px) and Home/DM are placed; Explore and Create (rare actions) hold two permanent tiles.
- **Proposal:** Merge Explore + Create into one `+` tile ("Add a space" menu: Create, Join, Explore); remove the Search tile (NAV-5); keep Approvals/Unverified only when they have something to say (Unverified already is conditional); order bottom stack Inbox → Settings. Keep the current tile size/avatar design (no redesign).
- **Effort:** S. **Risk:** Low. **iOS:** n/a.

### NAV-19 · P2 · Truncated room names have no tooltip; context menu is 160px wide
- **Where:** `RoomNavItem.tsx:402-409` (`truncate`, no `title`); `RoomNavItemMenu` `maxWidth: toRem(160)` (`RoomNavItem.tsx:153`).
- **Proposal:** `title={roomName}` only when the span overflows (ResizeObserver or `scrollWidth > clientWidth` on hover); menu `maxWidth: toRem(220)` so "Remove from Favorites" / "Notifications: mentions and keywords" never truncate.
- **Effort:** S. **Risk:** None. **iOS:** n/a.

### NAV-20 · P2 · Quick switcher: generic empty state, 400px cap, no recents/unreads grouping
- **Where:** `features/search/Search.tsx:402-427` (Modal `maxHeight: 400`, placeholder "Search"), `:295-304` ("No Rooms" / "No Match Found"), `:287` shows `topActiveRooms` when empty, footer hint `:601-604`.
- **Proposal:** Placeholder "Jump to a room, person or space"; empty list shows "Unread" then "Recent" groups (data available via `roomToUnread`, `presentation.recentOrder`); `maxHeight: min(560px, 70vh)`; row 36px with glyph column (reuse `roomGlyphFor`), unread rows bold; show the kbd hint inline in the input's trailing slot rather than a footer sentence.
- **Effort:** M. **Risk:** Low. **iOS:** n/a.

### NAV-21 · P2 · Splash duplicates the brand twice and uses playful copy
- **Where:** `components/splash-screen/SplashScreen.tsx:18-20` (footer "Synara" H2) plus `pages/client/ClientRoot.tsx:68-83` (`SynaraMark` 88px, "Heating up", status).
- **Proposal:** One mark (88px), status line in `T300 priority 400`, drop the H2 wordmark footer or the mark; copy "Connecting…" / "Syncing rooms…" (same tone as the restyled timeline). Keep tokens; background already `Background.Container` in both themes.
- **Effort:** S. **Risk:** None. **iOS:** n/a.

### NAV-22 · P2 · Lobby and Explore cards: fixed 3-column grid, Success green for "Suggested"
- **Where:** `components/room-card/style.css.ts:5-9` (`repeat(3, 1fr)`), `features/lobby/RoomItem.tsx:167-169` ("Suggested" Success badge), `RoomItem.css.ts:4-13` (card padding S400, radius 0).
- **Proposal:** `gridTemplateColumns: repeat(auto-fill, minmax(260px, 1fr))`; "Suggested" → `Secondary` soft badge (green stays reserved for live calls); lobby room rows 44px with the 18px glyph column so they rhyme with the sidebar.
- **Effort:** S. **Risk:** None. **iOS:** no lobby.

### NAV-23 · P2 · Unused compact signal and a 16px count pill with 12px text
- **Where:** `components/page/Page.tsx:116` sets `data-compact` below 232px but nothing consumes it (repo grep); `UnreadBadge.tsx:25-32` (size 400 = 16px, L400 12px, `millify`).
- **Proposal:** Use `data-compact` to hide the sort kebab and mute icon and tighten `NavItemContent` paddingRight to S200; badge → 18px tall, min-width 18px, 11px W700 (Slack's proportions) and cap display at "99+" instead of `millify` ("1.2k" on a 16px pill is unreadable).
- **Effort:** S. **Risk:** None. **iOS:** badge proportions should match.

## Already good — leave alone

- 32px rows, virtualised with a 32px estimate and overscan 10 (`Home.tsx:382-389`, `Space.tsx:460-467`); 50+ rooms are cheap.
- Collapsed sections keep unread and selected rooms visible (`Home.tsx:358-360`, `Space.tsx:440-447`, `Direct.tsx:204-207`) — exactly Slack's behaviour.
- Unread = weight + colour, read = opaque secondary colour (not opacity) with the contrast rationale documented (`features/room-nav/styles.css.ts:34-40`).
- 3px primary selection bar with `prefers-contrast` and `prefers-reduced-motion` branches (`components/nav/styles.css.ts:63-73`, `:100-120`).
- Resizable pane with persisted width, keyboard resizing and a labelled divider (`components/page/Page.tsx:65-136`).
- Rail: no hover translation any more; `::before` indicator grows 16→24px; faint outline at rest, full on hover (`Sidebar.css.ts:76-107`, `:169-183`).
- DM rows show the peer in the `#` slot with initial fallback (`components/room-glyph/*`, `RoomNavItem.tsx:394-400`); live-call chip with participant count (`LiveCallChip.tsx`).
- Context menu: Mark read/unread toggles on real state, favourite and notification mode with busy spinners and inline error text (`RoomNavItem.tsx:84-134`, `:164-233`); Escape handling and focus return through `FocusTrap`.
- Sort preference per section, persisted per device, ordered by Core's presentation (`homeRoomList.ts`), with a 28px target and a labelled menu (`Home.css.ts`, `Home.tsx:127-208`).
- Loading vs empty states are distinguished by `useNativeRoomListLoaded()` (`Home.tsx:398-403`, `Direct.tsx:227-232`).
- Linux custom title bar is minimal and shares one icon size (`features/desktop-titlebar/DesktopTitleBar.tsx`).

## Prior-audit items: persist vs fixed

From `docs/ux-audit/desktop-cross-client-audit.md`:
- §6 "persistent selected-row cue beyond hover" — **fixed** (`nav/styles.css.ts:83-89`).
- §6 "14px sort targets too small / single discoverable menu" — **fixed** (28px button + menu, `Home.css.ts:5-11`).
- §6 "Compact / Comfortable densities with preview + time" — **intentionally not adopted** (density setting is on the rejected list); row stays name-only, which is also Slack's choice.
- §6 "selected, unread, mention, muted, favourite distinguishable in grayscale" — **persists**: muted and favourite have no row treatment (NAV-8); mention is colour-only green (NAV-3).
- §7 "draggable room-list divider with persisted width and compact threshold" — **fixed** for the divider (`Page.tsx:41-106`); the compact threshold is emitted but unused (NAV-23).
- §7 "View-menu/keyboard command to show/hide navigation panes" — **persists** (no action in `appShortcuts.ts:8-9`) (NAV-14).
- §7 "remove hover translation from the rail" — **fixed** (`Sidebar.css.ts` has no transform).
- §7 "important actions not only at the bottom of the rail" — **partly fixed**: Settings has ⌘, (`appShortcuts.ts:63`); Inbox and Search have no shortcut.

From `docs/ux-audit/grok-4.6-high-review.md`:
- P0-3 "desktop sidebar targets ≥ macOS-recommended with keyboard access and visible focus" — **mostly fixed** (rows 32px, 28px sort, focus outline `nav/styles.css.ts:92-95`); row kebab vanishes on focus loss (NAV-2).
- P0-4 "favourite/mute/unread/encryption from the shared contract, never inferred" — **data is in the DTO** (`room.ts:63-73`) but desktop rows do not surface lock/favourite (NAV-8).
- P1-6 "reduced motion / increased contrast handling" — **fixed** in nav and row styles.
- Compact token recommendation (one type scale) — **persists** between list (14px) and timeline (15px) (NAV-4).

From `docs/ux-audit/ios-holistic-audit.md` §P1.3 (one sort menu, weight-not-colour unread) — iOS fixed; desktop equivalent fixed for sort, open for mention colour (NAV-3).
