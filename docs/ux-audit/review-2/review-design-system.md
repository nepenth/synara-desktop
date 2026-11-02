# Synara design-system review: desktop ↔ iOS alignment map

Scope: cross-platform tokens and consistency only (type, colour, radii/spacing, avatars, icons, terminology, motion, light/dark). Method: code reading and grep on `main` (clean), no builds. All paths are relative to the repo root; desktop = `synara/src/…`, iOS = `synara-ios/Synara/…`. The rejected "UX Next" ideas (agent-activity folding, "Needs you", room-list chips/avatars redesign, glass composer, density setting) are deliberately not proposed.

## 1. Summary

1. The two platforms share a *structure* (4 chrome surfaces, 4 text roles + separator, 6 base-colour presets, rich-text token set, depth levels) but not the *values*: the accent itself differs (desktop `#1245A8`/`#8DB4F5`, iOS system blue via `Color.accentColor` with no `AccentColor` asset), so every accent-keyed element (own reaction, mention pill, badges, links) is a different blue per device.
2. The v2.7.5 timeline restyle landed on both platforms but iOS drifted on the reaction pill (32 pt, 9 % fill, 17 pt emoji vs the agreed 28 px, 8 %, 16 px) and the "mention gold" is `systemYellow` on iOS vs `#F5B400` on desktop; the unread badge is green (Success) on desktop and blue (accent) on iOS.
3. Type: the timeline is now 15 px/15 pt-class on both, but the rest of desktop still runs on folds 12/14 px (T200/T300/B300/L400 = 1,100+ uses) while iOS runs on 17/16/12 pt; desktop also bypasses folds weights with 550/650 literals, and iOS has ~30 fixed `.system(size:)` fonts that ignore Dynamic Type.
4. Iconography and words disagree on the shared nouns: Later is a bookmark on desktop and a clock on iOS; Notes is a pencil (same as Edit) on desktop and `note.text` on iOS; Threads use one glyph on desktop and four different SF Symbols on iOS; iOS says "Channels"/"DM" where desktop says "Rooms"/"Direct Messages"; iOS ships both "Reply in Thread" and "Reply in thread"; iOS cannot save to Later at all.
5. Light mode diverges most: desktop's light chrome stack is markedly darker (rail L 0.895, composer 0.91) than iOS's near-white stack (0.955/0.985), desktop has two extra themes (Silver, Butter) iOS lacks, and iOS has an increased-contrast token derivation desktop lacks.

## 2. Alignment map

Folds token values (from `synara/node_modules/folds/dist/style.css`): space S100…S700 = 4/8/12/16/20/24/32 px; radii R300/R400/R500 = 4/8/12 px, Pill = 9999; fontSize T200 12, T300 14, T400 15, T500 17, B300 12, B400 14, B500 16, H6 14, H5 16, H4 18, H3 20, H2 25, H1 35; Avatar 200/300/400/500 = 24/36/42/58 px.

### 2a. Type scale

| Role | Desktop today | iOS today | Proposed canonical (desktop px / iOS text style) | Where to change |
|---|---|---|---|---|
| Caption (badges, counts, uppercase labels) | `L400` 12 px badge text (`components/unread-badge/UnreadBadge.tsx:30`); 10 px sidebar avatar (`components/sidebar/Sidebar.css.ts:156`); 11 px nav group label (`components/settings-layout/style.css.ts:36`), 11 px DM glyph (`features/room-nav/styles.css.ts:26`) | `.caption2` 11 pt `fineMeta`/`fineMetaBold` (`SharedUI/SynaraDesign.swift:1291-1292`); `.system(size: 11)` ×3 (`SharedUI/SynaraMessageBubble.swift:97,127,134`, `Features/RoomListView.swift:1372`) | **11** / `.caption2` | desktop: replace 10 px with 11; iOS: replace `.system(size: 11)` with `fineMeta` |
| Meta (timestamps, previews, system rows) | `T200` 12 px timestamps (`features/room/NativeTimelinePresenter.tsx:1501,1564,2299`); 13 px system row (`features/room/nativeTimelineHtml.css.ts:561`) | `.caption` 12 pt `messageMeta` (38 uses); `.footnote` 13 pt `messageTime` (`SynaraDesign.swift:1283,1285`) | **12** / `.caption`; **13** / `.footnote` for timestamps beside 15 px body | aligned in effect; document which one is "meta" vs "time" |
| Label (reaction count, chips, nav rows) | 13 px/600 reaction count (`nativeTimelineHtml.css.ts:603-604`); `T300` 14 px room name (`features/room-nav/RoomNavItem.tsx:176`); 13 px header DM glyph (`features/room/RoomViewHeader.css.ts:15`) | `.subheadline` 15 pt semibold reaction count (`Features/RoomTimelineView.swift:8790`); `.caption` semibold `chipLabel` (`SynaraDesign.swift:1290`) | **13** / `.footnote` | iOS reaction count → `.footnote.weight(.semibold)`; desktop room row stays 14 (platform density) or moves to 15 (see DS-3) |
| Body (message text, list titles) | 15 px / 1.47 (`nativeTimelineHtml.css.ts:353-355,362-364`); folds `B300` 12 px and `T300` 14 px everywhere else (271 + 304 uses) | `.body` 17 pt `messageBody` tight leading (`SynaraDesign.swift:1282`); `.callout` 16 pt `supporting` (62 uses, line 1280) | **15** / `.body` (17 pt at default Dynamic Type — accepted platform offset, as Slack does) | desktop: promote list titles and settings body copy from 12/14 to 15 where they are reading text (DS-3) |
| Strong body (sender, day heading, room title) | 15 px / 700 sender (`nativeTimelineHtml.css.ts:447-451`), day header 15/700 (`:581-585`), room title 700 + `-0.01em` (`RoomViewHeader.css.ts:18-22`) | `.body.bold` sender (`SynaraDesign.swift:1284`), `.headline.bold` day header (`:1286`), `.system(size: 17, weight: .bold)` title (`RoomTimelineView.swift:4303`) | **15/700** / `.body.weight(.bold)` | iOS title → `SynaraTypography.messageSender` (same role) |
| Title (panel/sheet titles) | `H4` 18 px (68 uses); 22 px `#` glyph (`RoomViewHeader.css.ts:6`) | `.title2.semibold` `screenTitle` 22 pt (`SynaraDesign.swift:1277`); `.system(size: 19/18/20)` (`RoomTimelineView.swift:4391,5650,10325,10386,9658`) | **17** / `.headline` for inline titles; **20** / `.title3` for sheet titles | collapse 18/19/20/22 literals |
| Heading / display (auth, empty states) | 24/32/44 px (`pages/auth/styles.css.ts:90-103`, emoji board 32 `components/emoji-board/components/styles.css.ts:77,128`) | `.title`/`.title2`; `.system(size: 28/34/40/42/44/48)` (`App/RootShellView.swift:618,628`, `RoomTimelineView.swift:8220,10499`, `RoomNotesView.swift:31`, `RichLinkEmbeds.swift:289`) | **24** / `.title`; hero glyphs 44 | leave hero glyphs; name them |
| Weights | 400/500/600/700 plus **550** (`components/message/layout/layout.css.ts:180`) and **650** (`pages/auth/styles.css.ts:92`); `config.fontWeight.W600` used 6× vs literal `600` 12× | `.regular/.medium/.semibold/.bold` only | **400 / 500 / 600 / 700** ↔ regular / medium / semibold / bold | replace 550→600, 650→700; use `config.fontWeight` |
| Line height | 1.47 body, 1.3 headings, 1.35 system, 1.5 code, 1.65 file preview (`nativeTimelineHtml.css.ts:507`), 1.2/1.22/1.25/1.45/1.55 one-offs | SF default + `messageLineSpacing` 0.5 (`SynaraDesign.swift:1288`) | body 1.47, tight 1.3, code 1.5 | prune 1.45/1.55/1.65 |

Proposed single scale: **11 / 12 / 13 / 15 / 17 / 20 / 24** with roles caption / meta / label / body / title / heading / display. iOS maps to `caption2 / caption / footnote / body / headline / title3 / title` and keeps Dynamic Type; desktop pins px.

### 2b. Colour

| Role | Desktop | iOS | Proposed canonical | Where to change |
|---|---|---|---|---|
| Accent (Primary) | `#1245A8` light / `#8DB4F5` dark (`synara/src/colors.css.ts:30,225`; `app/utils/themeAccent.ts:2,5`); user-overridable `customAccentColor` (`app/state/settings.ts:39`, `app/pages/ThemeManager.tsx:197-243`) | `Color.accentColor` (`SharedUI/SynaraDesign.swift:582`) — **no `AccentColor` asset** in `Resources/Assets.xcassets` → system blue `#007AFF`/`#0A84FF`; text variant `#0058b8`/`#5eacff` (`:641`), fill variant `#0062cc`/`#1a6fd8` (`:644`) | One accent pair on both; `customAccentColor` synced through the app-group defaults like `themeBaseColor` | iOS: add `AccentColor.colorset` + read `customAccentColor` in `SynaraSharedConstants` (`synara-ios/SynaraShared/SynaraNotificationPreviewSupport.swift:116`) |
| Chrome surfaces, dark | rail/roomList/chat/composer L = 0.075/0.10/0.104/0.135 (`app/utils/themeBase.ts:312-336`) | grouped/secondary/surface/elevated L = 0.075/0.10/0.104/0.135 (`SynaraDesign.swift:170-173`) | **aligned** | — |
| Chrome surfaces, light | 0.895 / 0.952 / 1.0 / 0.91 (`themeBase.ts:355-378`) | 0.955 / 0.975 / 0.995 / 0.985 (`SynaraDesign.swift:233-237`, rationale `:225-229`) | Converge on one table; iOS's quieter stack is the better reference | desktop light stops in `themeBase.ts` + `a11yContrast.test.ts` |
| Text roles | heading/primary/secondary/tertiary/separator derived to 12/8/5.5/4.5/3 : 1 (`themeBase.ts:255-277`); static fallbacks `synara/src/index.css:65-69,123-127` | same five roles at fixed lightness (dark .97/.80/.66/.60; light .06/.12/.34/.40) (`SynaraDesign.swift:176-180,238-242`) | Keep five roles; share the contrast targets | iOS `SynaraThemeRamp.tokens` |
| Mention (others) | Primary 16 % fill, Primary text, W600 (`app/styles/CustomHtml.css.ts:263-267`) | accent 16 % fill, `accentText` (`Features/RoomTimelineView.swift:6383-6384`), `.subheadline.semibold` (`:6356`) | **aligned in recipe**, differs via accent hue (DS-1) | — |
| Mention (me, gold) | `#F5B400` 22 % fill; text = Warning.Main 75 % + `#F5B400` (`CustomHtml.css.ts:274-277`) | `systemYellow` (`#FFCC00`/`#FFD60A`) 22 %; text rgb(0.98,0.80,0.36)/(0.52,0.36,0) literals (`SynaraDesign.swift:584-589`) | `mentionGold = #F5B400`, text `#5C3D00` light / `#F8CC5C` dark | both: name the token |
| Reaction pill fill | heading 8 %, hover 13 %; own Primary 18 %/24 % + Primary border (`nativeTimelineHtml.css.ts:593-630`) | primaryText 9 %; own accent 18 % + accent 80 % stroke (`RoomTimelineView.swift:8782,8796,8800`) | heading 8 %; own accent 18 % + accent 100 % 1 px border | iOS `ReactionPill` |
| Unread badge | highlight → **Success (green) solid**, else Secondary solid (`components/unread-badge/UnreadBadge.tsx:23-26`) | highlight → **accent solid**, else accent 16 % + accent text (`SynaraDesign.swift:1406-1430`) | accent solid for mention/highlight; neutral (heading 12 %) for plain unread | desktop `UnreadBadge.tsx` |
| Success / Warning / Critical | `#017343`/`#864300`/`#9D0F0F` light, `#85E0BA`/`#E3BA91`/`#E69D9D` dark (`colors.css.ts:56,69,82` and dark block) | fills `systemGreen/Orange/Red`; text pairs `#1a6b2d/#30d158`, `#9a4500/#ff9f0a`, `#b3141a/#ff7f78` (`SynaraDesign.swift:591-593,638-640`) | one 3-role table with fill + text pair per appearance | both token files |
| Extra semantic hues | — | agent teal, secure orange, design purple, ops teal (`SynaraDesign.swift:590-597`) | desktop needs at least `agent` (approvals UI) | add folds-side CSS vars |
| Hard-coded literals outside token files | `#F5B400` (`CustomHtml.css.ts:274,277`), `rgba` shadow (`nativeTimelineHtml.css.ts:875`), `#000` (`:1068`), `rgba` (`features/settings/appearance/Appearance.tsx:361`), `#f7fffb`/`#10251f` (`ThemeManager.tsx:243`), `#1e1f22`/`#ffffff` (`:175`), focus-ring rgba (`themeBase.ts:347,383`) | `.systemBlue` tint (`Features/SelectableMessageTextView.swift:26`), `systemTeal` (`Features/Composer/ComposerTextView.swift:505`, `App/SynaraApp.swift:1135`), `Color.white` (`RoomTimelineView.swift:6526,9702,10295`, `SharedUI/SynaraRoomAvatar.swift:127`) | zero literals outside `colors.css.ts`/`index.css`/`SynaraDesign.swift` | see DS-17 |

### 2c. Radii and spacing

| Use | Desktop | iOS | Proposed canonical | Where |
|---|---|---|---|---|
| Inline code / mention / tag | R300 4 px (`CustomHtml.css.ts:266`; 443 `radii="300"` uses) | `small` 6 (`SynaraDesign.swift:919`) | **4** | iOS `SynaraRadius.small` → 4 |
| Cards, controls, message hover, avatars ≤ 40 | R400 8 px (55 uses) | `card`/`control` 8 (`:920-921`); room tile literal 11 / `size*0.28` (`SharedUI/SynaraRoomAvatar.swift:28,32`) | **8**; avatars 22 % of size | iOS room tile → `SynaraRadius.avatar` |
| Popovers, media, sheets | R500 12 px (5 uses); media viewer | literals 12, 9, 10, 16 (`RoomTimelineView.swift:8118,8217,8423`, `SynaraDesign.swift:1485,1692`) | **12** | iOS add `SynaraRadius.large = 12` |
| Pills (reactions, badges, filter chips) | `toRem(14)` on 28 px pill (`nativeTimelineHtml.css.ts:600`), `radii="Pill"` 151 uses | `Capsule()` 25 uses; literal 14 (`RoomTimelineView.swift:10369-10371`) | **pill** | desktop pill → `config.radii.Pill` |
| Composer | R400 8 px (`components/editor/Editor.css.ts:13`) | `composer` 22 (`SynaraDesign.swift:922`) | platform-appropriate; leave | — |
| Spacing steps | S100–S700 = 4/8/12/16/20/24/32 | 4/8/12/16/24 (`SynaraDesign.swift:7-13`) | 4-pt grid, add 20 and 32 on iOS | `SynaraSpacing` |
| Odd literals | gap 5 / pad 9 / 7 in reaction pills (`nativeTimelineHtml.css.ts:596-598,639`); `paddingRight: toRem(96)` row (`:258`) | pad 10 (`RoomTimelineView.swift:8783`) | 4/8/12 only inside pills | both |

### 2d. Avatars

| Context | Desktop | iOS | Match? | Proposed |
|---|---|---|---|---|
| Room list row | `#` glyph 17 px in 18 px slot; DM person glyph 18 (`features/room-nav/styles.css.ts:12-27`) | `#` glyph 20 slot; DM person glyph 20 (`Features/RoomListView.swift:1347-1351`) | ✓ (glyph model) | 20 on both |
| Room header | glyph 22 (`features/room/RoomViewHeader.css.ts:4-16`) | person glyph 22 (`RoomTimelineView.swift:4412`) | ✓ | — |
| Timeline sender | folds `size="300"` = 36 px, `radii="400"` 8 px (`features/room/NativeTimelinePresenter.tsx:1669`) | 38 compact / 40 regular, radius `size*0.22` (`RoomTimelineView.swift:8030-8032`; `SynaraDesign.swift:925-927`) | ≈ | 36 desktop / 40 iOS acceptable |
| Fallback colour | `--mx-uc-1…8` hsl palette, hues 208/302/163/343/24/181/242/94 (`synara/src/index.css:44-51,114-121`) via `colorMXID` (`components/user-avatar/UserAvatar.tsx:42`) | FNV hash over hues 212/248/285/330/8/28/140/195, s .34 l .37, white initials (`SynaraDesign.swift:892-915`) | ✗ same user, different colour per device | adopt one hash + hue list (iOS's) on both |
| Member list | 24 px (`features/room/MembersDrawer.tsx:156`) | mention picker 30 (`RoomTimelineView.swift:10050`); details list uses text rows (`:7029-7070`) | ✗ | 28 |
| Reaction viewer | 24 px (`features/room/reaction-viewer/ReactionViewer.tsx:177`) | 36 (`RoomTimelineView.swift:8918`) | ✗ | 28 |
| Settings profile / room profile | 58 px (`features/settings/account/Profile.tsx:180`, `features/common-settings/general/RoomProfile.tsx:209,351`) | `SynaraAvatar` default 38 (`SynaraDesign.swift:1304`), brand mark 38 (`RoomListView.swift:822`) | ✗ | 44 row / 64 hero |
| Sidebar space avatars | 16 / 34 / 42 (`components/sidebar/Sidebar.css.ts:153-165`) | n/a | — | — |

### 2e. Iconography

| Concept | Desktop (folds `Icons.*`) | iOS (SF Symbol) | Verdict |
|---|---|---|---|
| Thread | `Thread` (`RoomViewHeader.tsx:658`, `components/message/Reply.tsx:52`) | `text.bubble` "Reply in Thread" (`RoomTimelineView.swift:7429`); `bubble.left.and.bubble.right` "Open Thread" (`:7433`) — also the **Rooms tab** icon (`App/AppTab.swift:477`); `arrowshape.turn.up.left` "Reply in thread" (`:10646`) — same as Reply; `arrow.up.left.and.arrow.down.right` "Open thread" (`:10652`) | ✗ four metaphors on iOS; pick `text.bubble` for thread everywhere, keep `bubble.left.and.bubble.right` for Rooms |
| Reply | text-only menu item "Reply" (`NativeTimelinePresenter.tsx:600-700`) | `arrowshape.turn.up.left` (`:7428`) | ok (desktop menu has no icons) |
| React | `SmilePlus` (`NativeTimelinePresenter.tsx:1278,1842`) | `face.smiling` (`:7890,7899,8853`) | ✓ |
| Edit | `Pencil` | `pencil` (`:7436`) | ✓ |
| Delete | `Delete` | `trash` (`:7412,7472`) | ✓ |
| Pin / Pinned messages | `Pin` + "Pinned Messages" (`RoomViewHeader.tsx:683-714`) | no pin glyph anywhere; "Pin to Notes" = `note.text.badge.plus` (`:7464`) | ✗ different feature ("pin" means Notes on iOS) |
| Later | `Bookmark` (`pages/client/inbox/Inbox.tsx:62`, `pages/client/inbox/Later.tsx:326`); reminder = `RecentClock` (`Later.tsx:170`) | `clock` tab (`AppTab.swift:485`); items `clock`/`alarm`/`checkmark.seal.fill` (`Features/LaterListView.swift:349`) | ✗ bookmark vs clock |
| Notes | `Pencil` (`RoomViewHeader.tsx:778`, `features/room/room-notes/RoomNotesPanel.tsx:432`) — identical to Edit | `note.text` (`Features/RoomNotesView.swift:30,350`) | ✗ desktop should use a note glyph |
| Search | `Search` | `magnifyingglass` | ✓ |
| Members | `User` (`RoomViewHeader.tsx:823`) | `person.2` is used for **View Reactions** (`:7442`); members live in Room details (`:6696`) | ✗ |
| Settings | `Setting` | `gearshape` | ✓ |
| Approvals | `Shield` (`pages/client/sidebar/ApprovalsTab.tsx:44`, `features/approvals/Approvals.tsx:184,355`) | `checkmark.shield` (`AppTab.swift:53,292`) | ≈ (shield vs check-shield) |
| More | `VerticalDots` | `ellipsis` | ✓ |
| Mark unread | `MessageUnread`/`CheckTwice` (`RoomViewHeader.tsx:174`) | — | — |

### 2f. Terminology and casing

Desktop strings are mostly hard-coded in TSX (en.json has 155 keys, `synara/public/locales/en.json`); iOS has 1,023 keys in `Resources/Localizable.xcstrings`.

| Concept | Desktop | iOS | Verdict |
|---|---|---|---|
| Rooms vs Channels | "Rooms" (`features/add-existing/AddExisting.tsx:254`, `features/settings/notifications/Notifications.tsx:173`); nav "Home"/"Spaces" (`pages/client/sidebar/HomeTab.tsx:88`, `pages/client/SidebarNav.tsx:27`) | tab "Rooms" (`AppTab.swift:477`) **and** section "Channels" (`RoomListView.swift:213`) | ✗ pick one noun |
| DMs | "Direct Messages" (`pages/client/sidebar/DirectTab.tsx:88`) | "DM" chip (`RoomListView.swift:870-880`) | ✗ |
| Inbox / Notifications | sidebar "Inbox" (`pages/client/sidebar/InboxTab.tsx:61`) holding Notifications, Invites, Later | tab "Notifications" (`AppTab.swift:481`); Later is its own tab | ✗ naming of the container |
| Later | "Later", "Save for later" (`pages/client/inbox/Later.tsx:328`, `NativeTimelinePresenter.tsx:843`) | "Later" tab; **no save action** (only `environment.later.loadItems`/`completeItem`, `LaterListView.swift:90,125`) | ✗ feature gap |
| Approvals | "Approvals" (`ApprovalsTab.tsx:29`, `Approvals.tsx:356`) | "Approvals" (`AppTab.swift:45-54`) | ✓ |
| Notes | "Notes" / "Pin" / "Unpin" (`NativeTimelinePresenter.tsx:820`) | "Notes", "Pin to Notes" (`:7464`) | ≈ |
| Threads | "Threads", "Thread", "Reply in thread" (`RoomViewHeader.tsx:645-655`, `features/room/NativeInlineThread.tsx:27`, `RoomInput.tsx:265`) | "Reply in Thread"/"Open Thread" (`:7429,7433`, `Features/TimelineAccessibility.swift:31,33`) **and** "Reply in thread"/"Open thread" (`:10646,10652`) | ✗ casing duplicates on iOS |
| Favorites | "Favorites" (`pages/client/home/Home.tsx:486-489`) | "Favorites" (`RoomListView.swift:165`) | ✓ |
| Composer placeholder | "Message #{{room}}" / "Message {{name}}" / "Reply in thread" (`RoomInput.tsx:262-276`) | "Message #\(title)" / "Message \(title)" (`RoomTimelineView.swift:1912-1914`) | ✓ |
| Casing policy | mixed: "Pinned Messages", "Mark as Read" (`features/room-nav/RoomNavItem.tsx:177`) vs "Add reaction" (`NativeTimelinePresenter.tsx:1272`), "Save for later", "Edit message" (`features/room/NativeTimelineMessageEditor.tsx:256`) | mostly Title Case (HIG) with the two sentence-case strays above | ✗ set one rule per platform |

### 2g. Motion and haptics

| Tier | Desktop | iOS | Proposed canonical |
|---|---|---|---|
| Micro (hover, press, toggle) | 140 ms ease-out ×17 (`app/styles/Depth.css.ts:95,138,196`), 120 ms (`nativeTimelineHtml.css.ts:58`, `components/page/style.css.ts:56`), 100 ms (`features/room/RoomViewTyping.css.ts:26`, `features/room/MembersDrawer.css.ts:38`, `components/image-viewer/ImageViewer.css.ts:40`) | easeOut 0.14 (`SynaraDesign.swift:1185,1443`), 0.12 (`RoomTimelineView.swift:9500`), 0.15 (`SharedUI/SynaraMotion.swift:70`), 0.16 (`RoomTimelineView.swift:1607,1625`, `RoomNotesView.swift:270`) | **140 ms ease-out** |
| Transition (panels, rows, sheets) | 200 ms (`components/sidebar/Sidebar.css.ts:85`, `layout.css.ts:123` cubic-bezier(0,.8,.67,.97)) | 0.18 (`RoomListView.swift:729,770`, `RoomTimelineView.swift:9498-9499,9674`, `SharedUI/SynaraNavigation.swift:166`), 0.2 (`AppTab.swift:112`, `RoomTimelineView.swift:1795,4858`), 0.22 send slide (`SynaraMotion.swift:21`) | **200 ms ease-out** |
| Emphasis (reaction pop, send) | none | spring(response 0.28, damping 1) + 40 ms stagger (`SynaraMotion.swift:52,71`) | **spring 280 ms, no overshoot**; desktop gets a 200 ms scale-in keyframe |
| Long ambient | 1400 ms reveal (`components/settings-layout/style.css.ts:88`), 2000 ms highlight (`layout.css.ts:63`), 400 ms (`pages/client/sidebar/UnverifiedTab.css.ts:15`) | — | keep highlight; drop 1400 ms |
| Reduced motion | `prefers-reduced-motion` in 11 files; keyframes **without** guard: `MembersDrawer.css.ts`, `components/message/layout/layout.css.ts`, `components/scroll-top-container/style.css.ts` | `accessibilityReduceMotion` in 4 files; `withAnimation` without guard: `AppTab.swift`, `SynaraNavigation.swift`, `RoomNotesView.swift`, `RoomListView.swift` (2) | guard everything |
| Haptics | n/a | lightImpact 4, success 4, warning 8, selection 1 (`SharedUI/SynaraHaptics.swift:7-12`) | document a map: send → lightImpact, reaction → selection, approve → success, destructive → warning |

## 3. Findings (ordered by impact)

**DS-1 · P0 · The accent is a different blue on each platform, and the user's custom accent never reaches iOS**
- Desktop Primary `#1245A8`/`#8DB4F5` (`synara/src/colors.css.ts:30,225`), overridable via `customAccentColor` (`synara/src/app/state/settings.ts:39`, `synara/src/app/pages/ThemeManager.tsx:197-243`). iOS uses `Color.accentColor` (`synara-ios/Synara/SharedUI/SynaraDesign.swift:582`) and there is no `AccentColor.colorset` under `synara-ios/Synara/Resources/Assets.xcassets`, so it is system blue; the shared defaults carry only `themeBaseColor` (`synara-ios/SynaraShared/SynaraNotificationPreviewSupport.swift:116`). Own reactions, mention pills, badges, links and selected filter chips therefore differ in hue.
- Proposal: add an `AccentColor` asset with `#1245A8`/`#8DB4F5`; read `customAccentColor` from the app-group defaults and feed `SynaraStatusTextPalette.accent/accentFill` from it with the same contrast soften used on desktop. Effort M. Risk: iOS contrast tests for `accentText` pairs need re-running; iOS `SynaraMessageBubble` paints accent at 12 % (`SynaraDesign.swift:449`).

**DS-2 · P0 · Unread/mention badge is green on desktop, blue on iOS**
- `synara/src/app/components/unread-badge/UnreadBadge.tsx:23-26` uses `variant="Success"` solid for highlight and `Secondary` solid otherwise; `SynaraDesign.swift:1406-1430` uses accent solid / accent 16 %. Prior desktop audit (#2 "semantic text roles") did not cover this.
- Proposal: accent solid + white text for highlight on both; neutral heading-12 % fill with heading text for plain unread; Success stays for "done/approved". Effort S. Risk: none beyond snapshot updates.

**DS-3 · P1 · Two type scales on desktop: 15 px timeline vs 12/14 px everything else**
- Timeline body/sender/day header are 15 px (`synara/src/app/features/room/nativeTimelineHtml.css.ts:353,449,582`), but the app runs on `T200` 12 px (327 uses), `T300` 14 px (304), `B300` 12 px (271), `L400` 12 px (204); room list names are `T300` 14 (`synara/src/app/features/room-nav/RoomNavItem.tsx:176`), panel titles `H4` 18. iOS is uniformly `.body` 17 / `.callout` 16 / `.caption` 12 (`SynaraDesign.swift:1276-1296`). Reading text in settings, search results, Later and Notes thus sits two steps below the timeline on desktop.
- Proposal: adopt the 11/12/13/15/17/20/24 scale; move reading text (`B300`/`T300` in Later, Notes, search results, settings descriptions, room list name) to 15 and labels to 13; keep 12 for timestamps only. Prior audit desktop #3 ("type hierarchy platform-native") persists in this form. Effort L (hundreds of call sites; do by surface). Risk: room-list density — keep the row at 32 px by lowering padding, not font.

**DS-4 · P1 · iOS reaction pill misses the agreed 28/14/8 % spec**
- `synara-ios/Synara/Features/RoomTimelineView.swift:8781-8800`: height 32, fill `primaryText` 9 %, emoji 17 pt, count `.subheadline.semibold` (15 pt), padding 10, own = accent 18 % + accent 80 % stroke. Desktop: 28 px, radius 14, heading 8 %, emoji 16, 13 px/600, padding 9, own = Primary 18 % + Primary border (`nativeTimelineHtml.css.ts:593-630`).
- Proposal: `@ScaledMetric` 28, `headingText.opacity(0.08)`, emoji 16, count `.footnote.weight(.semibold)`, stroke accent 1.0; desktop replace `toRem(14)` with `config.radii.Pill`. Effort S. Risk: 28 pt still meets the 44 pt hit area via `contentShape`? Verify (`:8754`).

**DS-5 · P1 · "Mention gold" is `#F5B400` on desktop and `systemYellow` on iOS**
- `synara/src/app/styles/CustomHtml.css.ts:274-277` vs `SynaraDesign.swift:584-589` (systemYellow `#FFCC00`/`#FFD60A` plus rgb literals for text). Both are untokenised literals.
- Proposal: `--synara-mention-gold: #F5B400` on desktop, `SynaraColor.mentionGold = "#F5B400"` on iOS, text `#5C3D00`/`#F8CC5C` (≥ 4.5:1 on the 22 % fill over both surfaces — verify with `SynaraThemeRamp.contrastRatio`). Effort S.

**DS-6 · P1 · Light-mode chrome stacks diverge (dark is identical)**
- Desktop light rail/list/chat/composer L = 0.895/0.952/1.0/0.91 (`synara/src/app/utils/themeBase.ts:355-378`); iOS 0.955/0.975/0.995/0.985 (`SynaraDesign.swift:233-237`, with a comment at `:225-229` explaining why the desktop-style mix "looked like a translucent gray veil"). Dark stops match exactly (`themeBase.ts:312-336` vs `SynaraDesign.swift:170-173`).
- Proposal: publish one stop table (dark already shared) and move desktop light toward iOS while keeping ≥ 1.1:1 steps; keep the desktop composer one step darker than chat for affordance. Effort M. Risk: `synara/src/app/utils/__tests__/a11yContrast.test.ts` floors; secondary text AA on the composer.

**DS-7 · P1 · Later, Notes, Threads and Members use different metaphors per platform**
- Later: `Icons.Bookmark` (`synara/src/app/pages/client/inbox/Inbox.tsx:62`, `Later.tsx:326`) vs `clock` (`synara-ios/Synara/App/AppTab.swift:485`, `Features/LaterListView.swift:349`). Notes: `Icons.Pencil` (`features/room/RoomViewHeader.tsx:778`, `room-notes/RoomNotesPanel.tsx:432`) — the same glyph as Edit — vs `note.text` (`Features/RoomNotesView.swift:30,350`). Threads: one `Icons.Thread` vs `text.bubble` / `bubble.left.and.bubble.right` / `arrowshape.turn.up.left` / `arrow.up.left.and.arrow.down.right` (`RoomTimelineView.swift:7429,7433,10646,10652`), and `bubble.left.and.bubble.right` is also the Rooms tab (`AppTab.swift:477`). Members: `Icons.User` (`RoomViewHeader.tsx:823`) while iOS uses `person.2` for View Reactions (`:7442`).
- Proposal: Later = bookmark on both (`bookmark` SF), reminders keep clock; Notes = note glyph on both (folds `Icons.File` or a custom note icon); Thread = `text.bubble` everywhere on iOS; Members = `person.2` for members only, `face.smiling`-style for reactions viewer. Effort S–M (desktop needs a note icon asset). Risk: low.

**DS-8 · P1 · Terminology: Rooms/Channels, DM/Direct Messages, Inbox/Notifications, and iOS casing duplicates**
- "Channels" section header (`synara-ios/Synara/Features/RoomListView.swift:213`) vs "Rooms" tab (`AppTab.swift:477`) and desktop "Rooms" (`synara/src/app/features/add-existing/AddExisting.tsx:254`); "DM" (`RoomListView.swift:870-880`) vs "Direct Messages" (`synara/src/app/pages/client/sidebar/DirectTab.tsx:88`); desktop "Inbox" (`InboxTab.tsx:61`) vs iOS "Notifications" (`AppTab.swift:481`). iOS has both "Reply in Thread"/"Open Thread" (`RoomTimelineView.swift:7429,7433`, `Features/TimelineAccessibility.swift:31,33`) and "Reply in thread"/"Open thread" (`:10646,10652`) as separate xcstrings keys. Desktop mixes "Pinned Messages"/"Mark as Read" (`RoomViewHeader.tsx:683`, `features/room-nav/RoomNavItem.tsx:177`) with "Add reaction"/"Save for later"/"Edit message" (`NativeTimelinePresenter.tsx:1272,843`, `NativeTimelineMessageEditor.tsx:256`).
- Proposal: nouns — Rooms (Matrix term, `#` glyph already says channel), Direct Messages (DMs in tight chips), Inbox for the container on both; casing — Title Case for iOS buttons/menus (HIG), sentence case for desktop menus/tooltips except product nouns (Later, Notes, Approvals, Threads); delete the lower-case iOS duplicates. Effort S. Risk: de.json/xcstrings translations.

**DS-9 · P1 · iOS has no "Save for later"; Later is read-only there**
- Desktop offers "Save for later" on every timeline item (`synara/src/app/features/room/NativeTimelinePresenter.tsx:824-843`); iOS only loads/completes (`LaterListView.swift:90,125`; no other `environment.later.*` call). The Later noun therefore means different things.
- Proposal: add "Save for Later" (bookmark glyph) to the iOS message menu and swipe actions, reusing the desktop item shape. Effort M (service already exists). Risk: Rust/shared store API parity — check `environment.later` protocol first.

**DS-10 · P1 · Same user, different avatar colour on each device**
- Desktop `colorMXID` → `--mx-uc-1…8` (`synara/src/util/colorMXID.ts:20-27`, `synara/src/index.css:44-51,114-121`) using a Java-style hash and hues 208/302/163/343/24/181/242/94; iOS `SynaraAvatarPalette` FNV-1a over hues 212/248/285/330/8/28/140/195 (`SynaraDesign.swift:892-915`). Prior iOS audit P1.6 ("one avatar identity model") persists.
- Proposal: port the iOS FNV + 8-hue list to desktop (`--mx-uc-*` values become the iOS hues at the two lightness pairs). Effort S. Risk: "Legacy Username Color" setting (`NativeTimelinePresenter.tsx:2011-2016`) changes colours for existing users — acceptable, it is already legacy.

**DS-11 · P1 · Avatar sizes per context differ outside the timeline**
- Member lists 24 px (`synara/src/app/features/room/MembersDrawer.tsx:156`) vs 30 pt (`RoomTimelineView.swift:10050`); reaction viewer 24 px (`features/room/reaction-viewer/ReactionViewer.tsx:177`) vs 36 pt (`:8918`); profile 58 px (`features/settings/account/Profile.tsx:180`) vs 38 pt default (`SynaraDesign.swift:1304`); iOS room tile radius literal 11 / `size*0.28` (`SharedUI/SynaraRoomAvatar.swift:28,32`) vs `SynaraRadius.avatar` 22 %.
- Proposal: canonical ladder 20 (row glyph) / 24 (dense) / 28 (lists, reaction viewer) / 36–40 (timeline) / 44 (profile row) / 64 (hero); radius 22 % everywhere. Effort S. Risk: none.

**DS-12 · P1 · Status colours are two unrelated palettes**
- Desktop Success/Warning/Critical `#017343`/`#864300`/`#9D0F0F` light, `#85E0BA`/`#E3BA91`/`#E69D9D` dark (`synara/src/colors.css.ts:55-95` and dark block ~`:250-290`); iOS system fills + text pairs `#1a6b2d/#30d158`, `#9a4500/#ff9f0a`, `#b3141a/#ff7f78` (`SynaraDesign.swift:591-593,638-640`), plus `agent` teal with no desktop token (`:590,642`).
- Proposal: one table of {fill, text, container} × {light, dark} for success/warning/critical/agent; iOS text pairs are already AA-verified and can seed it. Effort M. Risk: desktop Critical containers used in `TimelineNotice` (`nativeTimelineHtml.css.ts:116-120`).

**DS-13 · P2 · Weight literals bypass tokens on desktop; fixed-size fonts bypass Dynamic Type on iOS**
- Desktop `fontWeight: 550` (`synara/src/app/components/message/layout/layout.css.ts:180`), `650` (`synara/src/app/pages/auth/styles.css.ts:92`), `500` literals (`features/room/RoomViewHeader.css.ts:7`, `features/room-nav/styles.css.ts:17`), `600` ×12 vs `config.fontWeight.W600` ×6. iOS ~30 `.system(size:)` calls (`RoomTimelineView.swift:4303,4327,4353,4391,4455,4464,5650,5662,7305,7900,8183,8220,8788,8959,9658,9680,9699,10097,10100,10293,10325,10382,10386,10499`; `RoomListView.swift:844,855,1214,1372`; `SynaraMessageBubble.swift:97,127,134`).
- Proposal: 400/500/600/700 via `config.fontWeight`; iOS replace with `SynaraTypography` roles or `@ScaledMetric`. Effort M. Risk: layouts tuned to fixed sizes (header 44 pt buttons).

**DS-14 · P2 · Radii: iOS `small` is 6, desktop R300 is 4; stray literals on both**
- `SynaraDesign.swift:919` vs folds R300; iOS literals 11/12/14/16/9/10 (`SharedUI/SynaraRoomAvatar.swift:28`, `RoomTimelineView.swift:8118,8217,8423,10369-10371`, `SynaraDesign.swift:1485,1682,1692`); desktop `toRem(14)`/`toRem(16)` (`nativeTimelineHtml.css.ts:600` and one more), `borderRadius: 6` (`synara/src/app/pages/client/home/Home.tsx:233`).
- Proposal: 4 / 8 / 12 / pill; add `SynaraRadius.large = 12`, set `small = 4`. Effort S.

**DS-15 · P2 · Spacing scale: iOS lacks 20 and 32; pills use 5/7/9/10 px literals**
- `SynaraSpacing` (`SynaraDesign.swift:7-13`) vs folds S500/S700; reaction pill literals `nativeTimelineHtml.css.ts:596-598,639`, `RoomTimelineView.swift:8783`.
- Proposal: add `xxLarge = 32`, `large2 = 20` only if used; pills use 8/12. Effort S.

**DS-16 · P2 · Motion durations are scattered on both platforms**
- Desktop canonical 140 ms ease-out (`synara/src/app/styles/Depth.css.ts:95,138,196`) but 100/120/200/400/1400/2000 ms elsewhere (table 2g); iOS 0.12–0.22 s literals in seven files plus `SynaraMotion` spring. Desktop has no reaction-add animation; iOS has none of desktop's 2000 ms highlight.
- Proposal: `SynaraMotion.Duration { micro = 0.14, transition = 0.2 }` + `emphasis` spring; desktop `--synara-motion-micro: 140ms`, `--synara-motion-transition: 200ms`; add a 200 ms scale-in on new reaction pills (guarded). Effort S–M.

**DS-17 · P2 · Colour literals outside the token files**
- Desktop: `CustomHtml.css.ts:274,277` (`#F5B400`), `nativeTimelineHtml.css.ts:875` (rgba scroll shadow), `:1068` (`#000` X-post media), `features/settings/appearance/Appearance.tsx:361` (rgba inset), `ThemeManager.tsx:175,243` (`#1e1f22`, `#f7fffb`, `#10251f`), `themeBase.ts:347,383` (focus-ring rgba). iOS: `Features/SelectableMessageTextView.swift:26` (`.systemBlue` tint — should be accent), `Features/Composer/ComposerTextView.swift:505` and `App/SynaraApp.swift:1135` (`systemTeal` — should be `SynaraColor.agent`), `Color.white` over avatars (`SharedUI/SynaraRoomAvatar.swift:127`, `RoomTimelineView.swift:6526,9702,10295`), self-mention rgb literals (`SynaraDesign.swift:584-589`).
- Proposal: route each through a named token; `Color.white` on media overlays is acceptable but should be `SynaraColor.onMedia`. Effort S.

**DS-18 · P2 · Reduced-motion guards are incomplete**
- Desktop keyframes without `prefers-reduced-motion`: `synara/src/app/features/room/MembersDrawer.css.ts:23`, `synara/src/app/components/message/layout/layout.css.ts:44`, `synara/src/app/components/scroll-top-container/style.css.ts:4`. iOS `withAnimation` without `accessibilityReduceMotion`: `App/AppTab.swift:112`, `SharedUI/SynaraNavigation.swift:166`, `Features/RoomNotesView.swift:270`, `Features/RoomListView.swift:729,770`. Prior iOS audit P2.4 persists.
- Proposal: a `synaraAnimation(_:)` helper that returns `nil` under Reduce Motion; a desktop `reducedMotion` style helper. Effort S.

**DS-19 · P2 · Light/dark parity: extra desktop themes, missing desktop increased-contrast, hue-blind iOS light agent surface**
- Desktop ships light, silver, dark, butter (`synara/src/app/hooks/useTheme.ts:19-37`; `synara/src/colors.css.ts:102,298`); iOS has light/dark + `increasedContrast` token variants (`SynaraDesign.swift:163-166,176-179`) and no Silver/Butter. Desktop handles `prefers-contrast: more` with per-component overrides in 10 files but no token-level derivation. iOS light `agentReviewBackground` uses fixed hue 196 (`SynaraDesign.swift:243-244`) while dark follows the theme hue (`:183-184`). Desktop hard-codes `Primary.OnContainer` to `#f7fffb`/`#10251f` when a custom accent is set (`ThemeManager.tsx:243`).
- Proposal: retire Silver and Butter (base-colour presets already give six looks on both); add an increased-contrast multiplier pass to `deriveThemeSurfaceRamp`; derive `agentReview*` from hue in both appearances. Effort M. Risk: users on Butter — migrate to Dark + Amber.

**DS-20 · P2 · Text-role derivation differs (contrast-targeted vs fixed lightness)**
- `themeBase.ts:255-277` targets 12/8/5.5/4.5/3:1; `SynaraDesign.swift:176-180,238-242` fixes lightness. Roles match; values drift per preset.
- Proposal: port `softenToContrast` to Swift (contrast helpers exist at `SynaraDesign.swift:309-323`). Effort S. Risk: iOS snapshot diffs.

**DS-21 · P2 · Letter-spacing and uppercase labels exist only on desktop**
- `RoomViewHeader.css.ts:20` (`-0.01em`), `features/room-nav/styles.css.ts:29` (`-0.005em`), `components/settings-layout/style.css.ts:33-36` (uppercase 11 px 0.06em). iOS uses SF tracking and plain section headers.
- Proposal: drop manual tracking (system fonts already track); keep uppercase group labels as a desktop-only idiom or switch to 13 px sentence-case like iOS sections. Effort S.

**DS-22 · P2 · Haptic map is undocumented**
- `SynaraHaptics.swift:7-12` defines four kinds; 17 call sites (warning 8, success 4, lightImpact 4, selection 1). No stated rule for send / reaction / approve / destructive.
- Proposal: document the map in `SynaraHaptics` and assert it in UI tests. Effort S.

## 4. Already aligned — leave alone

- Dark-mode chrome stops (rail/list/chat/composer L 0.075/0.10/0.104/0.135) and the 6 base-colour presets (`themeBase.ts:6-13` ↔ `SynaraDesign.swift:116-123`), synced via `themeBaseColor`.
- The five text roles + separator model, the rich-text token set (inline code, code block, spoiler, table header/odd/even) on both platforms (`themeBase.ts:49-68` ↔ `SynaraDesign.swift:80-102`).
- Timeline: 15-class body, bold sender, bold day heading over a rule, flat rows without bubbles (`SynaraSurfaceDepthRole.standardMessageShowsBackground = false`, `SynaraDesign.swift:1072`), "Message #room" / "Message Name" placeholders, mention recipe (accent 16 %, 600, gold for self), timeline avatar 36/40 at ~22 % radius.
- Room list and header use the `#` / person-glyph model at 18–22 px on both, not avatar tiles.
- Depth vocabulary (resting / raised / floating / critical / avatar) and the "text never receives depth" rule (`Depth.css.ts:3-7` ↔ `SynaraDepthLevel`).
- Font stack: desktop prefers the OS font (`-apple-system`, Segoe UI) with Inter fallback and a Linux override (`synara/src/index.css:57-59,176-178`); iOS is SF. Prior desktop audit #3 is resolved.
- Icons for Search, Settings, Edit, Delete, React, More, Approvals (shield family) and the words Later, Approvals, Notes, Favorites, Settings, the composer placeholder.
- Spacing base grid (4/8/12/16/24) and 8 px card/control radius.
- Muted-text opacity is tuned for AA on desktop (`themeBase.ts:167-170`) and iOS avoids opacity for secondary text — different means, same outcome; leave.
