# Synara iOS/iPad shell review (everything outside the timeline)

Reviewed 2026-11-01 against `main` (3231c7bb, v2.7.6) by code reading only. All paths are under `synara-ios/Synara/`. Desktop counterpart column refers to `synara/src/app/...`.

## Summary

1. The shell's bones are right for the Slack direction (flat `#`/person rows, bold-when-unread, one accent, central tab-bar clearance, split canvas on iPad), but the Rooms top chrome is too tall and doubles up on space navigation (chip strip *and* collapsible groups), costing roughly two rows on a 50-room list.
2. Room-list rows are ~66 pt tall with a mandatory preview line and three independently-sorted sections; Slack-style density needs ~56 pt rows, uniformly collapsible section headers, and one sort control.
3. Several state gaps are visible in code: a mute action with no muted indicator or unmute, a lock glyph driven by the room *name* rather than `encryptionStatus`, a `No Rooms` empty state whose message is the connection banner text, and a Later tab whose rows never show the saved message.
4. iPad is half-done: Rooms/Later/Notifications use the sidebar+detail canvas, but Settings is a stretched phone `Form`, column headers don't share a height, and the sidebar cannot be collapsed.
5. Copy and surfaces drift: three sign-in verbs (`Login`/`Sign In`/`Log In`), a tab and a settings page both called "Notifications", `Danger Zone`, and Settings sub-pages that alternate between the theme ramp and the stock grouped background.

## Findings (ordered by impact)

| ID | P | Title | Where |
|---|---|---|---|
| IOSS-1 | P0 | Rooms top chrome stacks four bands before the first row | `Features/RoomListView.swift:117-146` |
| IOSS-2 | P0 | Space navigation is duplicated (chips + disclosure groups) | `RoomListView.swift:135-141, 186-204` |
| IOSS-3 | P0 | Room rows are ~66 pt; 50+ rooms need Slack density | `RoomListView.swift:582-584, 608, 1455` |
| IOSS-4 | P1 | Section headers aren't collapsible and don't match the space-group label | `RoomListView.swift:1172-1192, 1244-1268` |
| IOSS-5 | P1 | Three per-section sort menus persist three different orders | `RoomListView.swift:20-22, 93-108, 1194-1222` |
| IOSS-6 | P1 | Mute has no indicator and no unmute | `RoomListView.swift:632-639, 698-708`; `Services/RoomListService.swift:18-45` |
| IOSS-7 | P1 | Lock glyph keyed on room name, not `encryptionStatus` | `RoomListView.swift:1329-1334, 1363-1368`; `SharedUI/SynaraRoomAvatar.swift:200-204` |
| IOSS-8 | P1 | "No Rooms" empty state says "Connected" and offers no action | `RoomListView.swift:64-69`; `SharedUI/ConnectionStatusBanner.swift:114-127`; `Services/ConnectionStatus.swift:9, 200-202` |
| IOSS-9 | P1 | Unread name weight is semibold while timeline sender is bold; read rows aren't dimmed | `RoomListView.swift:1356-1358, 1407-1409` |
| IOSS-10 | P1 | Later rows never show the message; preview is "Saved\nRoom: X" | `Features/LaterListView.swift:183-230, 293-299, 364-379` |
| IOSS-11 | P1 | Later pull-to-refresh isn't awaited and blanks the list to skeletons | `LaterListView.swift:61-63, 86-88` |
| IOSS-12 | P1 | iPad Settings is a stretched phone Form | `App/AppTab.swift:411-418, 428, 455`; `App/RootShellView.swift:252-266` |
| IOSS-13 | P1 | iPad split: sidebar pinned open, column headers don't align | `App/SynaraCanvasLayout.swift:136-145`; `RoomListView.swift:812-865`; `Features/RoomTimelineView.swift:4366-4470` |
| IOSS-14 | P1 | "Notifications" names both a tab and a Settings page; Approvals hidden behind a toolbar glyph | `AppTab.swift:47-59, 481-483`; `Features/SettingsView.swift:129-135` |
| IOSS-15 | P1 | Notifications rows use `#` for DMs; header/row styles differ from Rooms | `AppTab.swift:157-161, 305-310, 349-353` |
| IOSS-16 | P1 | Connection banner is a layout-shifting full-width band, shown on every cold launch | `RootShellView.swift:80-111`; `ConnectionStatus.swift:151-165, 125`; `ConnectionStatusBanner.swift:24-57` |
| IOSS-17 | P1 | Settings surfaces alternate between theme ramp and stock grouped background | `SettingsView.swift:215-221, 1542-1543, 1680-1681` vs `278-612, 1095-1340, 1922-2053, 2737-2774` |
| IOSS-18 | P1 | Notification settings mix user preferences with push diagnostics | `SettingsView.swift:1120-1142, 1186-1191, 1206-1251` |
| IOSS-19 | P1 | Sign-in uses three verbs and two titles | `Features/LoginView.swift:39, 114, 141`; `Features/HomeserverSelectionView.swift:19-20, 86` |
| IOSS-20 | P2 | Rooms header title is the hostname with a fixed 210 pt width and scale-down | `RoomListView.swift:556-564, 822-831` |
| IOSS-21 | P2 | Custom search field: fixed 44 pt height, no Cancel, Done-to-close | `RoomListView.swift:1270-1310, 764-774` |
| IOSS-22 | P2 | Skeleton rows don't match the loaded row anatomy | `SharedUI/SynaraDesign.swift:1674-1699`; `RoomListView.swift:52-55` |
| IOSS-23 | P2 | Account row in Settings is a generic glyph; "Danger Zone" header | `SettingsView.swift:109-127, 189-205, 2641-2665` |
| IOSS-24 | P2 | Appearance page: wrong icon, inert "Appearance: System" row, `-11` padding hack | `SettingsView.swift:140, 1575, 1605, 1668-1678` |
| IOSS-25 | P2 | Invite card uses a warning-orange stroke in the room list | `RoomListView.swift:1500-1526` |
| IOSS-26 | P2 | Swipe copy and context menu are thin ("Read", favorite-only menu) | `RoomListView.swift:622-630, 648-657` |
| IOSS-27 | P2 | Haptics only on filter chips and invite accept | `RoomListView.swift:516, 1138`; `SharedUI/SynaraHaptics.swift` |
| IOSS-28 | P2 | Approvals row carries a literal "Open message" line plus a chevron | `Features/ApprovalsView.swift:217-256` |
| IOSS-29 | P2 | 28 pt toolbar icon buttons; fixed-size header glyphs under Dynamic Type | `SynaraDesign.swift:1791-1809`; `RoomListView.swift:843-858`; `ConnectionStatusBanner.swift:25-33` |
| IOSS-30 | P2 | Dead shell code: `roomTint`, `RoomListSyncBanner`, `SynaraRoomAvatarTile`, `SynaraLoadingState` | `RoomListView.swift:1484-1498, 1592-1610`; `SynaraRoomAvatar.swift:6-77`; `SynaraDesign.swift:1645-1659` |

---

### IOSS-1 · P0 · Rooms top chrome stacks four bands before the first row
**Where:** `Features/RoomListView.swift:117-146` (header VStack), `812-865` (`RoomListHeader`), `1112-1140` (`RoomFilterStrip`), `1142-1170` (`SpaceFilterStrip`).
**Today:** The navigation bar is hidden (`:291`) and replaced by a custom stack: 38 pt brand mark + title row, optional 44 pt search field, a 32 pt "All / Unread / Mentions / Agents" chip row, and (whenever any space exists) a second 32 pt "All spaces / Space…" chip row, with 12 pt gaps and 12/4 pt outer padding. That is ~130 pt (≈170 pt with search open) of chrome on every visit, plus the floating tab bar at the bottom. On a 6.1" phone this leaves room for ~10 rows (see IOSS-3).
**Proposal (Slack-informed):** One header row (title/account on the left, search + compose on the right) and one chip row at most. Fold the space chips into the title menu (IOSS-2). Make the filter strip collapse when scrolling down (Slack hides its filter row on scroll) or move "Unread/Mentions" into a segmented control that shares the header line at regular widths. Target ≤ 90 pt of chrome.
**Effort:** M. **Risk:** Medium (UI tests key on `RoomFilterStrip`, `SpaceFilterStrip` identifiers). **Desktop counterpart:** desktop sidebar has a single filter row; no change needed there.

### IOSS-2 · P0 · Space navigation is duplicated (chips + disclosure groups)
**Where:** `RoomListView.swift:135-141` (`SpaceFilterStrip`), `172-204` (`SpaceSelectedHeader` vs per-space `DisclosureGroup`), `747-762` (only unread spaces seeded expanded).
**Today:** Spaces appear twice: as filter chips in the header and as collapsible groups inside the Channels area. Selecting a chip swaps the list to a flat space view with a 22 pt bold `SpaceSelectedHeader`; with "All spaces" selected, the same spaces show as `DisclosureGroup`s whose expansion state is seeded to "spaces with unreads" and lost on relaunch (`@State`). Favorites and DMs ignore the chip filter's grouping entirely. Users have two mental models for one concept.
**Proposal:** Keep one. Recommended: drop `SpaceFilterStrip`; keep the collapsible space groups (they match Slack's collapsible sections) and persist expansion per space in `UserDefaults` like sort order. Expose "jump to space" from the title menu (`RoomHeaderAccountMenuButton`) as a Slack-style workspace/space switcher sheet with unread counts. If a space is chosen there, render `SpaceSelectedHeader` with a clear "All spaces" back affordance.
**Effort:** M. **Risk:** Medium (space filtering is tested in `RoomListScopeFilter`). **Desktop counterpart:** desktop uses a space rail (`pages/client/sidebar/SpaceTabs.tsx`); the iPad regular-width layout could borrow that rail later, but not the phone.

### IOSS-3 · P0 · Room rows are ~66 pt; 50+ rooms need Slack density
**Where:** `RoomListView.swift:581-584` (`.padding(.horizontal, 12)`, `.padding(.vertical, 8)`), `:608` (`listRowInsets` top/bottom 1), `:1455` (`minHeight: 48`), `:1354` (`VStack spacing 4`).
**Today:** Content is name (17 pt body) + preview (15 pt subheadline) ≈ 40 pt, forced to 48 pt, plus 16 pt padding and 2 pt insets ⇒ ~66 pt per row. Slack's channel rows are ~44 pt (no preview) and DM rows ~60 pt. With IOSS-1's chrome, a 50-room account shows ~10 rows per screen.
**Proposal:** Drop `minHeight` to 44, vertical padding to 6 (row ≈ 56 pt). Consider making the preview line conditional: show it for DMs and for unread rooms, omit it for read channels (Slack Home shows no preview for channels). Keep the 20 pt glyph column. Verify with `RoomListFixtures` 50-room fixture (`RoomListService.swift:597-622`).
**Effort:** S. **Risk:** Low; UI tests asserting title frames still pass. **Desktop counterpart:** desktop `RoomNavItem` is already dense; no change.

### IOSS-4 · P1 · Section headers aren't collapsible and don't match the space-group label
**Where:** `RoomSectionHeader` `:1172-1192` (body-semibold `headingText`, count, 44 pt sort menu), `SpaceDisclosureLabel` `:1244-1268` (body-semibold `primaryText`, unread badge, count, system chevron inside a list row), `SpaceSelectedHeader` `:1224-1242` (22 pt bold).
**Today:** Favorites/Channels/Direct messages/Low priority are static `Section` headers with a sort menu; spaces are `DisclosureGroup` rows with a chevron and an unread badge. Three header styles coexist in one list; only the space groups collapse; collapsing a space hides its unread rooms.
**Proposal:** One `RoomListSectionHeader` used by every section: title (headline-weight, `headingText`), unread badge when collapsed, trailing chevron, tap to collapse, long-press/menu for sort. Persist collapsed state per section. When collapsed and unreads exist, keep showing only the unread rows (Slack behaviour). Retire `SpaceSelectedHeader`'s 22 pt title in favour of the same header.
**Effort:** M. **Risk:** Low. **Desktop counterpart:** desktop `Home.tsx:486-489` has collapsible "Favorites"; match its semantics.

### IOSS-5 · P1 · Three per-section sort menus persist three different orders
**Where:** `RoomListView.swift:20-22, 93-108, 354-362`; `RoomListSortOrder` `Services/RoomListService.swift:152-222`.
**Today:** Favorites, Channels, and Direct messages each have a 44×44 clock/textformat menu and their own stored order; Low priority borrows the Channels order. A user can have Favorites by name and Channels by recency without any visible indication beyond the small glyph.
**Proposal:** One sort control for the list (in the title menu or as the header's long-press), applied to all sections, with the option surfaced as text ("Sorted by recent activity"). Keep the per-section keys readable for migration but write one value.
**Effort:** S. **Risk:** Low. **Desktop counterpart:** desktop stores a single `synara.roomListSort`; aligning iOS simplifies cross-client parity.

### IOSS-6 · P1 · Mute has no indicator and no unmute
**Where:** trailing swipe `RoomListView.swift:632-639`; `muteRoom` `:698-708` always sets `.mute`; `RoomSummary` has no notification-mode field (`RoomListService.swift:18-45`).
**Today:** Swiping a muted room still offers "Mute"; the row shows nothing to distinguish muted rooms; "Mute" gives no feedback (no reload, no haptic). Slack shows muted channels dimmed with a `bell.slash` and offers Unmute.
**Proposal:** Add `notificationMode` to `RoomSummary` (Core already exposes it to Room Details), render muted rows with `secondaryText` name and a trailing `bell.slash` glyph (no unread badge, only a dot for mentions), and toggle the swipe label Mute/Unmute.
**Effort:** M (data plumbing). **Risk:** Medium (projection contract change). **Desktop counterpart:** yes, desktop `RoomNavItem` dims muted rooms; match.

### IOSS-7 · P1 · Lock glyph keyed on room name, not `encryptionStatus`
**Where:** `isSecureRoom` `SharedUI/SynaraRoomAvatar.swift:200-204` (name contains "security"/"secure"/"e2e"); used at `RoomListView.swift:1330-1334` (orange glyph) and `1363-1368` (lock icon), accessibility `:1543-1545`.
**Today:** Rooms named "secure-ops" get an orange `#` and a lock; actual encrypted rooms don't. `RoomSummary.isEncrypted` exists (`RoomListService.swift:42-45`) and is used 57 times elsewhere but not here. The orange `SynaraColor.secure` also breaks the one-accent rule.
**Proposal:** Replace `isSecureRoom` with `room.isEncrypted` (or show nothing: Slack doesn't mark encryption per row; Matrix clients usually mark the *unencrypted* public room instead). If kept, use `secondaryText`, not orange.
**Effort:** S. **Risk:** Low. **Desktop counterpart:** check desktop `RoomNavItem` for the same heuristic; it should match whichever rule is chosen.

### IOSS-8 · P1 · "No Rooms" empty state says "Connected" and offers no action
**Where:** `RoomListView.swift:64-69` → `HeldConnectionEmptyState` (`ConnectionStatusBanner.swift:114-127`) → `emptyStateMessage` = `bannerText` (`ConnectionStatus.swift:200-202`) → `"Connected"` (`:9`).
**Today:** A freshly signed-in user with no rooms sees "No Rooms / Connected" (or "Connecting…") and a chat-bubble glyph, with no way forward except noticing the compose icon. `SynaraEmptyState` also paints `SynaraColor.surface` (`SynaraDesign.swift:1641`) over the `roomList` plane, so the empty tab is chat-coloured.
**Proposal:** Keep connection copy only while not connected. When connected and empty: "No rooms yet" + "Join a room or start a direct message." + a prominent "Browse Rooms" / "New Message" button that opens `RoomManagementSheet`. Give `SynaraEmptyState` a transparent background by default and let the screen paint its plane.
**Effort:** S. **Risk:** Low. **Desktop counterpart:** desktop `WelcomePage.tsx` already has an actionable empty state; match its copy.

### IOSS-9 · P1 · Unread name weight is semibold while timeline sender is bold; read rows aren't dimmed
**Where:** `RoomListView.swift:1357` (`.semibold` when unread), `1408` (preview `.medium` when unread), `1444-1445` (time weight/colour).
**Today:** Unread rows use `.semibold` names and a *medium* preview; read rows use regular `primaryText`. The timeline direction set sender names at 15 pt **bold**. Slack's list makes unread names bold and leaves read names at regular weight in a slightly lower-emphasis colour; preview text stays regular.
**Proposal:** Unread: name `.bold`, `headingText`; preview regular `secondaryText`; badge. Read: name `.regular`, `primaryText`; preview `tertiaryText`. Drop the medium-weight preview. Keep weight + badge as the unread signal (not colour alone).
**Effort:** S. **Risk:** Low. **Desktop counterpart:** yes, desktop unread rows are bold; align the read-state colour too.

### IOSS-10 · P1 · Later rows never show the message; preview is "Saved\nRoom: X"
**Where:** `LaterListView.swift:183-230` (row), `293-299` (`stateLabel` returns the same string in both branches), `364-379` (`preview` embeds `\n`).
**Today:** A Later row reads "Saved" (caption) above a body line "Saved / Room: #design" — the kind is repeated, there is no sender or message excerpt, and a literal newline formats the body. `SynaraLaterListItem` carries only `roomID`/`eventID` (`Services/TimelineService.swift:772-779`).
**Proposal:** Resolve the event preview (sender + first line, as Slack's Later does) through `TimelineService` with a cached lookup; row = room name (headline) · time, then the message excerpt (2 lines), then a due pill if any. Remove the duplicated kind label; keep the icon.
**Effort:** M (needs an event-preview lookup). **Risk:** Medium (offline events). **Desktop counterpart:** desktop Inbox/Later shows the message; match.

### IOSS-11 · P1 · Later pull-to-refresh isn't awaited and blanks the list to skeletons
**Where:** `LaterListView.swift:61-63` (`.refreshable { load() }`), `86-88` (`load()` sets `state = .loading`).
**Today:** The refresh indicator dismisses immediately (nothing awaited) while the list is replaced by `SynaraSkeletonList` and then re-rendered. Rooms (`reloadRoomsForRefresh` `:393-409`) and Notifications (`reloadInbox` `:237-242`) do this correctly.
**Proposal:** Add an `async` reload that keeps `.loaded` while fetching and await it in `.refreshable`; only show skeletons from `.idle`.
**Effort:** S. **Risk:** Low. **Desktop counterpart:** none.

### IOSS-12 · P1 · iPad Settings is a stretched phone Form
**Where:** `AppTab.swift:411-418` (`usesConversationCanvas` false for `.settings`), `428, 455` (unused "Select a Setting" copy), `RootShellView.swift:258-264`.
**Today:** On iPad the Settings tab is a full-width `NavigationStack` + `Form`; each sub-page pushes and replaces the whole screen. The empty-pane copy for Settings exists but is never used.
**Proposal:** Give Settings the same split canvas as the other tabs: list column (Account, Notifications, Appearance, Security, GIFs & Links, About…) and a detail column that shows the selected page, defaulting to Account. Reuse `SynaraConversationEmptyPane` copy already defined. On iPhone nothing changes.
**Effort:** M. **Risk:** Low. **Desktop counterpart:** desktop Settings is already sidebar + detail (`features/settings/Settings.tsx`); this brings iPad in line.

### IOSS-13 · P1 · iPad split: sidebar pinned open, column headers don't align
**Where:** `SynaraCanvasLayout.swift:136-145` (`columnVisibility: .constant(.all)`, `.balanced`, width 280/320/400), `RoomListView.swift:812-865` (custom header: 38 pt mark, 12 pt top / 4 pt bottom padding, nav bar hidden), `RoomTimelineView.swift:4366-4470` (`TimelineHeader`, 44 pt controls, glass capsule).
**Today:** The sidebar can never be hidden (no toggle, `.constant(.all)`), so landscape reading of a wide table or code block is capped at the detail width. The sidebar column has no navigation bar and a ~54 pt custom header, while the detail column draws its own 44 pt header + capsule; their baselines are set independently, so the two columns' top rows won't line up. There is no `.sidebarAdaptable`/column-toggle affordance, and `.balanced` on an 820 pt iPad gives a 320 pt sidebar + 500 pt detail.
**Proposal:** Bind `columnVisibility` to state and show the system sidebar toggle in the detail header (Slack iPad has one). Share a `SynaraColumnHeaderHeight` constant (44 pt content + 8 pt padding) between `RoomListHeader` and `TimelineHeader`, or let both columns keep the system nav bar with `.inline` titles. Use `.prominentDetail` in portrait so the conversation wins width.
**Effort:** M. **Risk:** Medium (timeline header is in the other reviewer's area; only the height contract is shared). **Desktop counterpart:** desktop sidebar is collapsible; match.

### IOSS-14 · P1 · "Notifications" names both a tab and a Settings page; Approvals hidden behind a toolbar glyph
**Where:** tab label `AppTab.swift:481-483`; tab title `:47`; Settings row `SettingsView.swift:129-135`; Approvals toolbar `AppTab.swift:48-59` (`checkmark.shield`, no text).
**Today:** The Notifications *tab* is an inbox (mentions, invites, agent actions, unread rooms); Settings > Notifications is push configuration. Users asked to "turn off notifications" will land in the inbox. Approvals is reachable only via an unlabelled shield icon.
**Proposal:** Rename the tab "Activity" (Slack's term for the same inbox; `bell` icon stays) and keep "Notifications" for the settings page. Give Approvals a visible row at the top of Activity ("Approvals · N" with chevron) instead of, or in addition to, the toolbar glyph. Update `AppDeepLink` aliases to accept both `activity` and `notifications`.
**Effort:** S. **Risk:** Low (string + identifier churn in UI tests). **Desktop counterpart:** desktop names it "Inbox" (`pages/client/inbox`); pick one word for both clients.

### IOSS-15 · P1 · Notifications rows use `#` for DMs; header/row styles differ from Rooms
**Where:** `NotificationsInboxRow` `AppTab.swift:305-310` and `AgentPendingApprovalRow` `:349-353` hard-code `Text("#")`; section headers `:157-161` are `supporting`/`secondaryText` "Mentions · 3"; room list headers are body-semibold `headingText`.
**Today:** A mention in a DM shows a `#` glyph; the same room shows a person glyph in Rooms. Section headers in the inbox are lighter and smaller than in Rooms; the inbox also uses `listStyle(.plain)` with default insets while Rooms uses custom 8 pt insets, so left edges differ by tab.
**Proposal:** Reuse `SynaraRoomGlyph.glyph(for:)` and `SynaraPersonGlyph` in both inbox rows; reuse the shared section header from IOSS-4; align `listRowInsets` with Rooms.
**Effort:** S. **Risk:** Low. **Desktop counterpart:** none needed.

### IOSS-16 · P1 · Connection banner is a layout-shifting full-width band, shown on every cold launch
**Where:** `RootShellView.swift:80-111` (`VStack { ConnectionStatusBanner; TabView }`); `ConnectionStatus.swift:151-165` (`presentsBanner(.starting) == true`), `:125` (12 s connected flash); `ConnectionStatusBanner.swift:24-57` (caption2-bold text, 13 pt icon), `:100-111` (status colour at 12-14% opacity).
**Today:** `.syncing` is now silent (prior audit P1.1 partially done), but `.starting` still paints "Connecting…" above all tabs on launch, and recovery paints a green "Connected" band for 12 s. Because the banner is a sibling above `TabView`, every tab's content jumps when it appears/disappears. Text is caption2 (11 pt) with `minimumScaleFactor(0.85)`, and copy like "Connection Lost! Reconnecting..." uses an exclamation mark and three periods.
**Proposal:** Two tiers, Slack-style. Routine (`starting`, connected flash): an inline status under the Rooms title ("Connecting…" in `secondaryText`, footnote) that doesn't reflow other tabs; drop the connected flash or make it a 2 s toast. Actionable (`disconnected`, `reconnecting`, `restoreFailed`, sliding-sync unsupported): keep the band but as a `safeAreaInset(edge: .top)` *inside* each tab's content so the tab bar and nav don't move, with footnote text, proper ellipsis "…", no "!" and the existing Retry/Log Out buttons.
**Effort:** M. **Risk:** Medium (UI tests assert `ConnectionStatusBanner` identifiers). **Desktop counterpart:** desktop `SyncStatus.tsx` is a top band too; copy should match (`syncStatusCopy.ts`).

### IOSS-17 · P1 · Settings surfaces alternate between theme ramp and stock grouped background
**Where:** theme-ramp plane: root `SettingsView.swift:215-221`, GIFs & Links `:1542-1543`, Appearance `:1680-1681`, Agent Notifications `AgentNotificationSettingsView.swift:157-158`. Stock background: Account `:278-612`, Notifications `:1095-1340`, Security `:1922-2053`, About (`List`) `:2737-2774`, Licenses, Privacy, Support.
**Today:** Pushing from Settings (tinted) to Account (system grey) and back visibly shifts the background hue, most obviously in Dark Mode with a custom base colour. Prior audit P1.2 called this out; it is fixed on three pages only.
**Proposal:** One `.synaraSettingsChrome()` modifier (`scrollContentBackground(.hidden)` + `SynaraChrome.settings.ignoresSafeArea()` + `.navigationBarTitleDisplayMode(.inline)`) applied to every Settings page and sheet (`IdentityResetSheet`, `VerifyThisDevicePromptSheet`).
**Effort:** S. **Risk:** Low. **Desktop counterpart:** none.

### IOSS-18 · P1 · Notification settings mix user preferences with push diagnostics
**Where:** `SettingsView.swift:1120-1142` (Delivery: registration status + "Re-register Push"), `1186-1191` (orphan "Agent Notifications" link in its own unlabelled section), `1206-1251` (Local Delivery Diagnostics: 24 log rows, Refresh/Copy/Share/Clear).
**Today:** The Notifications page runs Permission → Delivery → Agent Approvals → (Agent Notifications) → Privacy → Diagnostics → push rules (1-to-1, Rooms, Mentions, Keywords). The diagnostics table alone can be 24 rows. Prior audit P2.3 ("task vs global") persists.
**Proposal:** Order by user intent: Permission status (one row with "Open iOS Settings" when denied) → push rules (1-to-1 / Rooms / Mentions / Keywords) → Agent Approvals (fold the Agent Notifications link into this section) → Privacy. Move "Delivery" and "Local Delivery Diagnostics" to Settings > Support > Diagnostics, keeping a one-line "Push: Registered" summary here.
**Effort:** S. **Risk:** Low (UI test identifiers move). **Desktop counterpart:** desktop notification settings have no diagnostics; aligns.

### IOSS-19 · P1 · Sign-in uses three verbs and two titles
**Where:** `HomeserverSelectionView.swift:86` ("Choose Server"), `:19-20` (subtitle "Matrix chat with agent approvals."); `LoginView.swift:39` ("Sign In" header), `:114` ("Log In" button), `:141` (nav title "Login"), `:86-89` (homeserver as plain text), `:91-118` (fields stay enabled while loading).
**Today:** Screen 1 says "Choose Server / Synara / Matrix chat with agent approvals."; screen 2 is titled "Login", headed "Sign In", with a "Log In" button; Settings and the banner say "Log Out". The homeserver is displayed as inert text; changing it means the back button. Fields remain editable during the request.
**Proposal:** Use "Log In"/"Log Out" everywhere (it is already the Settings verb) and make the nav title match the header. Replace the subtitle with something that serves a first-time user ("Chat on Matrix with your team and your agents."). Render the homeserver as a `LabeledContent("Server", host)` row with a trailing "Change" that pops. Disable fields while `state.isLoading`. Keep the Reset Local Data path as is.
**Effort:** S. **Risk:** Low (xcstrings catalog test will flag new keys; see memory note). **Desktop counterpart:** desktop login uses "Log In"; match.

### IOSS-20 · P2 · Rooms header title is the hostname with a fixed 210 pt width and scale-down
**Where:** `RoomListView.swift:556-564` (`accountMenuTitle` = host), `822-831` (`.title3.semibold`, `lineLimit(1)`, `minimumScaleFactor(0.78)`, `maxWidth: 210`).
**Today:** "matrix.example-company.org" is the most prominent text on the screen and shrinks to fit 210 pt. Slack puts the workspace name here; Matrix has no workspace, but the user's display name or "Rooms" is more human than a host. Prior audit P1.5 noted this.
**Proposal:** Title "Rooms" (or the selected space name per IOSS-2) in `.title2.bold` matching the timeline day-heading weight, with the account shown as a 32 pt avatar button (`SynaraAvatar`) instead of the brand mark; host stays in the account sheet. Remove `minimumScaleFactor` and the fixed 210 pt cap; let the title truncate.
**Effort:** S. **Risk:** Low. **Desktop counterpart:** desktop shows the space/home name in the sidebar header; consistent.

### IOSS-21 · P2 · Custom search field: fixed 44 pt height, no Cancel, Done-to-close
**Where:** `RoomSearchField` `RoomListView.swift:1270-1310` (`.frame(height: 44)`, `submitLabel(.done)`), `dismissSearch` `:764-774`.
**Today:** Search is a custom field toggled by a toolbar icon. At AX sizes the body-font `TextField` exceeds 44 pt and clips. There is no "Cancel"; with text typed, the only exits are clearing via the x or pressing Done (which keeps the query and the field open). Scope is name or last-preview only (`RoomListScopeFilter` `RoomListService.swift:1044-1045`).
**Proposal:** Either adopt `.searchable(text:isPresented:)` on the list (gets Cancel, Dynamic Type, scroll-to-reveal for free; works even with a hidden nav bar on iOS 17+), or add a "Cancel" button and replace `frame(height: 44)` with `frame(minHeight: 44)`. Add search by Matrix ID/alias for DMs.
**Effort:** S–M. **Risk:** Medium (UI tests inject `SYNARA_UI_TEST_ROOM_SEARCH`). **Desktop counterpart:** none.

### IOSS-22 · P2 · Skeleton rows don't match the loaded row anatomy
**Where:** `SynaraSkeletonRow` `SynaraDesign.swift:1680-1693` (42 pt rounded tile, 148/212 pt bars, 28×20 pill); used in Rooms `RoomListView.swift:52-55`, Notifications `AppTab.swift:16`, Later `LaterListView.swift:14`, Approvals `ApprovalsView.swift:74`.
**Today:** The loading state shows large 42 pt avatar tiles that then snap to 20 pt `#` glyphs and two-line text; Later/Approvals pass `showsAvatar: false` but still get the trailing pill.
**Proposal:** A `RoomRowSkeleton` with a 20 pt circle, 17/15 pt bars and no pill, matching IOSS-3 row height; keep the generic skeleton for Later/Approvals.
**Effort:** S. **Risk:** None. **Desktop counterpart:** none.

### IOSS-23 · P2 · Account row in Settings is a generic glyph; "Danger Zone" header
**Where:** `SettingsView.swift:109-121` (`SettingsSummaryRow` with `person.crop.circle.fill`, title = `userID` with `minimumScaleFactor(0.8)` `:2656`), `189-205` ("Danger Zone" / "Log Out").
**Today:** The user's uploaded avatar (`AccountSettingsView` supports upload, `:359-372`) and display name never appear in Settings; the row is the raw `@user:host`. "Danger Zone" is GitHub vocabulary, not iOS; the row is the only destructive action.
**Proposal:** Account row: 44 pt `SynaraAvatar` (seeded like timeline senders), display name as title, `@user:host` as subtitle, no scale-down. Replace "Danger Zone" with an un-headed last section containing "Log Out" (destructive), as Apple's own apps do.
**Effort:** S. **Risk:** Low. **Desktop counterpart:** desktop Settings has no "Danger Zone" for the account (`Settings.tsx:217` "Log Out"); room/space settings keep it, which is fine.

### IOSS-24 · P2 · Appearance page: wrong icon, inert "Appearance: System" row, `-11` padding hack
**Where:** `SettingsView.swift:140` (`textformat.size` for Appearance), `1575` (`SettingsInfoRow("Appearance", "System")` with no control), `1605` (`.padding(.leading, -11)`), `1668-1678` ("Text Size: Uses iOS Dynamic Type").
**Today:** The Appearance entry uses a text-size glyph though the page is about theme colour; the first row looks like a Light/Dark picker but is read-only; preset swatches are nudged with a negative pad; the Text section is an informational row that looks like a setting.
**Proposal:** Icon `paintpalette`. Either add a real `Picker("Appearance", System/Light/Dark)` bound to `preferredColorScheme` or drop the row. Put the preset swatches in an `HStack` with `listRowInsets(EdgeInsets())` instead of negative padding. Move the Dynamic Type note into the section footer and delete the inert row.
**Effort:** S. **Risk:** Low. **Desktop counterpart:** desktop has a Light/Dark/System setting; adding it here aligns.

### IOSS-25 · P2 · Invite card uses a warning-orange stroke in the room list
**Where:** `InviteRoomListRow` `RoomListView.swift:1500-1526` (`synaraCard(fill: warning 8%, stroke: warning, depth: .layered)`), used in Rooms `:568-575` and the inbox `AppTab.swift:179-186`.
**Today:** Invites are orange-outlined cards inside an otherwise flat list, with Accept/Decline buttons on every row. Orange is also "secure" (IOSS-7) and "warning".
**Proposal:** Flat row with the room glyph, name, "Invited by X" preview, and a single accent "Accept" button; Decline in the swipe/context menu. Group invites in an "Invites" section at the top of Rooms instead of inlining them in Channels/DMs by sort order.
**Effort:** S. **Risk:** Low. **Desktop counterpart:** desktop lists invites separately in Inbox; consistent.

### IOSS-26 · P2 · Swipe copy and context menu are thin
**Where:** `RoomListView.swift:622-630` ("Read" label), `648-657` (context menu offers only Favorite).
**Today:** The leading swipe shows "Read" (ambiguous: read vs mark read). Long-press shows one item; Mute/Mark Read/Leave/Copy link/Room details are swipe- or screen-only. Slack's long-press offers Mark read, Mute, Star, Copy link, Leave.
**Proposal:** Rename to "Mark Read". Context menu: Mark Read / Favorite / Mute(Unmute) / Room Details / Copy Link / Leave. Keep swipes as the quick path.
**Effort:** S. **Risk:** Low. **Desktop counterpart:** desktop `RoomNavItem.tsx:203` context menu has a fuller set; match it.

### IOSS-27 · P2 · Haptics only on filter chips and invite accept
**Where:** `RoomListView.swift:516` (`.success` on accept), `:1138` (`.selection` on chips). None for favorite toggle (`:683-696`), mark read (`:665-681`), mute (`:698-708`), leave (`:710-726`), space expand/collapse, pull-to-refresh completion, or Later "Complete" (`LaterListView.swift:169-179`). The timeline uses `SynaraHaptics` 17 times.
**Proposal:** `.lightImpact` on favorite/mute/collapse, `.success` on mark-read and Later complete, `.warning` on leave confirmation, via the existing `synaraHapticFeedback` modifier so Reduce Motion/UI tests stay silent.
**Effort:** S. **Risk:** None. **Desktop counterpart:** n/a.

### IOSS-28 · P2 · Approvals row carries a literal "Open message" line plus a chevron
**Where:** `ApprovalsView.swift:251-253` (`Text("Open message")` in every row), `236-239` (monospaced summary at caption size), `217-219` (`NavigationLink` already draws a chevron).
**Today:** Each row has five text lines: room, sender, summary, "Decided …", "Open message". Summary is caption-size monospaced, hard to read at 3 lines.
**Proposal:** Remove "Open message" (the chevron and accessibility hint already say it). Promote summary to `SynaraTypography.monoBody` (callout mono) with 2 lines; put decision + date on one trailing meta line.
**Effort:** S. **Risk:** None. **Desktop counterpart:** desktop Approvals tab shows a similar card; copy can stay.

### IOSS-29 · P2 · 28 pt toolbar icon buttons; fixed-size header glyphs under Dynamic Type
**Where:** `SynaraToolbarIconButton` `SynaraDesign.swift:1799-1805` (28×28 frame, used in Later `LaterListView.swift:70` and `PlaceholderScreen.swift:19`); `RoomListHeader` icons `.system(size: 17)` `RoomListView.swift:843-858`; brand mark fixed 38 `:822`; banner icon `.system(size: 13)` `ConnectionStatusBanner.swift:26`.
**Today:** Toolbar accounts button is 28 pt visually (the nav bar pads the hit area, but it looks undersized next to system items). Header glyphs and the banner icon do not scale with Dynamic Type while their neighbouring text does. Prior audit P2.1 (one-off geometry) persists: 20, 24, 28, 32, 38, 42, 48, 210 pt constants.
**Proposal:** Use `.font(.body.weight(.semibold))` or `@ScaledMetric` for glyphs; drop the custom 28 pt chip in favour of a plain toolbar `Image` (system sizing); define `SynaraMetrics.roomGlyph = 20`, `.listAvatar = 32`, `.headerHeight = 44` in `SynaraDesign` and reference them.
**Effort:** S. **Risk:** Low. **Desktop counterpart:** n/a.

### IOSS-30 · P2 · Dead shell code
**Where:** `roomTint` `RoomListView.swift:1592-1610` (name-based purple/teal tints), `RoomListSyncBanner` `:1484-1498`, `SynaraRoomAvatarTile` `SynaraRoomAvatar.swift:6-77`, `SynaraLoadingState` `SynaraDesign.swift:1645-1659` — zero call sites outside their declarations (and the token gallery).
**Proposal:** Delete; they encode the pre-Slack visual language and will mislead the next pass.
**Effort:** S. **Risk:** None. **Desktop counterpart:** n/a.

---

## Already good — leave alone

- **Central tab-bar clearance.** `RootShellView.swift:104-110, 267, 274-283` measures the real `UITabBar` frame and applies one `safeAreaInset` to every tab root; the compact conversation drops it to 0. This closes prior P0.2 (Later/Notifications no longer lack clearance).
- **Row anatomy direction.** `#` glyph in a fixed 20 pt column, `SynaraPersonGlyph` seeded by Matrix ID (`SynaraAvatarPalette`), 1-line name + 1-line preview, time + `SynaraUnreadBadge` (accent fill when highlighted, tinted outline otherwise) — matches Slack and the new timeline. Accessibility-size stacking (`SynaraDynamicTypeLayout.stacksRoomRowMetadata`, `TimelineAccessibility.swift:267-294`) is the right pattern.
- **Selected-row treatment in split layout** (`RoomListView.swift:585-597`, `.isSelected` trait at `:605`).
- **Swipe set and leave confirmation** (`:610-647`, `306-331`): favorite + mark-read leading with full swipe; mute + leave trailing without full swipe; leave is a confirmation dialog.
- **Connection chrome policy** (`ConnectionStatus.swift:120-165`): syncing is silent, lost states are held 4 s, Retry and Log Out never co-exist, banner text contrast pairs are precomputed (`SynaraStatusTextPalette`).
- **Theme ramp handles Increase Contrast** (`SynaraDesign.swift:707-716` reads `accessibilityContrast`), closing prior P1.2's contrast gap.
- **Verification sheet** (`RootShellView.swift:491-764`): measured detent, 2-column emoji grid at AX sizes, destructive "They Do Not Match", interactive dismiss blocked mid-protocol.
- **Login keyboard flow** (`LoginView.swift:5-35, 58-71`): Return advances, Log In scrolls above the keyboard, `textContentType` set, destructive local-data reset behind a confirmation.
- **Agent notification editor** (`AgentNotificationSettingsView.swift:7-71`): optimistic toggle with rollback and inline "Saved to your account." is exactly the right settings feel.
- **VoiceOver on rows**: combined row label (`RoomListView.swift:1529-1555`), hidden decorative glyphs, section sort menu labelled with its current value (`:1219`), tab identifiers stable.
- **Empty detail pane on iPad** (`SynaraCanvasLayout.swift:169-183`) with brand mark for Rooms only.

## Prior-audit items (docs/ux-audit/ios-holistic-audit.md) that persist

| Prior item | Status now |
|---|---|
| P0.2 tab-bar reachability | **Resolved** by the shell-owned contract (`RootShellView.swift:104-110, 267`). |
| P1.1 demote routine sync banner | **Partially resolved**: `.syncing` hidden. `.starting` and the 12 s connected flash still paint the band and reflow all tabs (IOSS-16). |
| P1.2 one surface system | **Partially resolved**: Rooms/Notifications/Later share the plane, Increase Contrast handled. Settings sub-pages still mix ramp vs stock (IOSS-17); Later/Approvals use `.insetGrouped` while Rooms/Inbox use `.plain`. |
| P1.5 room rows | Avatar sizing recommendation superseded by the Slack `#`/20 pt glyph direction (intentional). Two 16 pt sort buttons → one 44 pt menu: **resolved** (`:1194-1222`), but still one per section (IOSS-5). Hostname as header title: **persists** (IOSS-20). |
| P1.6 one avatar identity model | **Partially resolved**: `SynaraAvatarPalette` seeds DM glyphs and timeline senders alike; Settings account row still a generic glyph (IOSS-23); `SynaraRoomAvatarTile` is dead (IOSS-30). |
| P1.7 Notifications as an inbox | **Mostly resolved**: title first, reason as a meta `Label`, "Unread rooms" under a "Messages" header. Remaining: `#` for DMs and header style drift (IOSS-15). |
| P1.9 `minimumScaleFactor` as escape hatch | **Persists** at `RoomListView.swift:829`, `SettingsView.swift:2656`, `ConnectionStatusBanner.swift:33`, `SynaraStatusChip` `SynaraDesign.swift:1390`. |
| P2.1 one-off geometry | **Persists** (IOSS-29). |
| P2.3 task vs global preferences | **Persists** for push diagnostics inside Notification settings (IOSS-18). |
| P2.4 polish gates (Reduce Motion etc.) | `SynaraListRowButtonStyle` and haptics respect Reduce Motion/UI-test; selected filter chips still rely on fill colour + `.isSelected` trait only (no shape cue) — minor, unchanged. |
