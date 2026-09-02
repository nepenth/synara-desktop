# ROE-06 Research Memo: Room Sorting and Filtering Residual Census

Status: draft research; docs-only; not approved for implementation.

| Field              | Value                                                                                                                                          |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Workstream/cluster | ROE-06                                                                                                                                         |
| Research owner     | Isolated researcher on `roe/memo-06-room-sort`                                                                                                 |
| Reviewers          | Unassigned                                                                                                                                     |
| Source census      | 2026-09-01 against `b6797c3a1a223ce73e5d30838e5a634c09e3d9f7`                                                                                  |
| ADR baseline       | ADR 0003, 0004 last reviewed 2026-09-01 (index in [`docs/adr/README.md`](../../../adr/README.md)); same census commit                           |

[program/CENSUS.md](../program/CENSUS.md) recorded Core `sort.rs` / `filters.rs` as unused by product UIs on `main` `011cf39a`. Re-read source on this commit agrees. The snapshot is still accurate for this domain: clients consume Core **row fields**, not the Core sort/filter **helpers**.

## Observable problem

A user opening the room list must see rooms the account actually joined, with favorite and unread attention that match Matrix tags and counts, without two engines deciding “what is a favorite / unread / DM.” The residual question is not whether desktop and iOS look the same. It is whether existing Core predicates and sort helpers are the shared semantic owner, unused library code, or a silent competitor with desktop and iOS rules.

No current source evidence shows a second Matrix room-list engine. Desktop and iOS both read `matrix_room_list_snapshot`. They then apply native sections, device-local sort chrome, and locale collation. That is the split the [workstream brief](../workstreams/06-room-sorting-filtering.md) already named. The Core helpers are not called on that path.

## Current ownership census

| Concern | Rust/Core | Desktop | iOS | Evidence/tests |
| ------- | --------- | ------- | --- | -------------- |
| Joined-room snapshot | Authority. `snapshot_from_sync_owner` opens `matrix-sdk-ui` `all_rooms`, applies SDK `new_filter_joined()`, and projects `RoomSummary` (tags, counts, `last_activity_ts`). Registered `matrix_room_list_snapshot` returns that snapshot; it does **not** call `sort_rooms` or `filter_rooms_by_scope`. | Thin invoke. `useBindAllRoomsAtom` hydrates `orderedRoomIds` + summaries. Dual-backend JS room-list fallback is documented forbidden. | Thin UniFFI. `SharedCoreRoomListService` maps the same command plus invites and space-parent snapshots. `PlaceholderRoomListService` is preview/test only. | [`live.rs`](../../../../crates/synara-core/src/app/room_list/live.rs); [`core.rs`](../../../../crates/synara-core/src/core.rs) `matrix_room_list_snapshot`; desktop [`roomList.ts`](../../../../synara/src/app/state/room-list/roomList.ts); iOS [`SharedCoreRoomList.swift`](../../../../synara-ios/Synara/Services/SharedCoreRoomList.swift), [`SharedCoreProductServices.swift`](../../../../synara-ios/Synara/Services/SharedCoreProductServices.swift) |
| Core sort helpers (`RoomListSort`, `sort_rooms`) | Unused library. `ByName` / `RecentActivity` / `FavoritesThenRecent` / `LowPriorityLast`. ASCII `#`-stripped lowercase names; missing `last_activity_ts` last; stable by `room_id`. Linked only via `matrix_room_list_markers()` and unit tests. Not on UniFFI. | Product Home does **not** import these helpers. | Product list does **not** import these helpers. | [`sort.rs`](../../../../crates/synara-core/src/app/room_list/sort.rs); [`mod.rs`](../../../../crates/synara-core/src/app/room_list/mod.rs); Tauri re-export tests in [`src-tauri/.../room_list/tests.rs`](../../../../src-tauri/src/matrix/room_list/tests.rs). Grep of `synara/` and `synara-ios/` has no `sort_rooms` / `RoomListSort::` consumer. |
| Core filter helpers (`RoomListScope`, `room_matches_scope`, `partition_favorite_rooms`, `select_rooms_in_folder`) | Unused by product UIs. `room_matches_scope` is used only by `RoomListBadgeCounts::from_rooms` (Unread tab counter). Those badge counts are **not** on the snapshot DTO or UniFFI wire. `folder_id` is always `None` in live projection. | No call. | No call. | [`filters.rs`](../../../../crates/synara-core/src/app/room_list/filters.rs); [`counts.rs`](../../../../crates/synara-core/src/app/room_list/counts.rs); UniFFI [`RoomListSnapshotDto`](../../../../crates/synara-core/src/synara_core.udl) is `session_generation` + `ordered_room_ids` + rooms only. |
| Favorite tag | Authority. Live row `is_favorite` from SDK `m.favourite` / `is_favourite()`. | Projection. `favoriteRoomIdSet` reads Core `isFavorite`; `partitionHomeRooms` lifts those ids out of the Home orphan-room list. | Projection. `RoomListFavorites.partition` keeps joined favorites, leaves remaining (including invites). | Core `project_room` in `live.rs`; desktop [`homeRoomList.ts`](../../../../synara/src/app/pages/client/home/homeRoomList.ts); iOS [`RoomListService.swift`](../../../../synara-ios/Synara/Services/RoomListService.swift) `RoomListFavorites` |
| Direct chats | Authority for both `m.direct` snapshot and row `is_direct`. | Navigation. Home uses `useOrphanRooms` (not a DM, not a space child) driven by `mDirectAtom` ← `matrix_mdirect_snapshot`. Directs page is a separate root. | Section. `kind` is `.directMessage` when Core `isDirect` is true; Direct messages is a third section under the same list. | Desktop [`mDirectList.ts`](../../../../synara/src/app/state/mDirectList.ts), [`useHomeRooms.ts`](../../../../synara/src/app/pages/client/home/useHomeRooms.ts), [`Direct.tsx`](../../../../synara/src/app/pages/client/direct/Direct.tsx); iOS `SharedCoreRoomListRows` + `RoomListView` |
| Spaces | Authority for parent ids (`matrix_space_parents_snapshot`). | Navigation. `roomToParentsAtom` from that snapshot; Home hides space children; Space sidebar owns child lists. | Section / grouping. Channels with a parent render under space headers; space chip filters the same list. | Desktop [`roomToParents.ts`](../../../../synara/src/app/state/room/roomToParents.ts), [`hooks/roomList.ts`](../../../../synara/src/app/state/hooks/roomList.ts); iOS `RoomListSpaceGrouping` |
| Invites | Authority. Separate `matrix_invites_snapshot`; live room-list filter is joined-only. | Inbox Invites route, not Home sections. | Merges invite rows into the list and inbox sections. | Core `matrix_invites_snapshot`; iOS `RoomListSearchFilter.mergeInvitedRooms`; desktop Inbox paths |
| Unread / mentions (row fields) | Authority. `room_unread_presentation` writes `unread_count` / highlight; `marked_unread` stays a separate field. Core `RoomListScope::Unread` / `::Mentions` unused by UIs. | Projection. `unreadInfosFromNativeRooms` builds nav badges from those fields; excludes spaces and muted rooms. Collapsed Rooms category hides read rooms (local chrome). No Unread/Mentions filter chips. | Projection. Filter chips: unread = `unreadCount > 0`; mentions = `hasHighlight`. Inbox splits mentions / invites / leftover unread. | Core [`counts.rs`](../../../../crates/synara-core/src/app/room_list/counts.rs); desktop [`roomToUnread.ts`](../../../../synara/src/app/state/room/roomToUnread.ts); iOS `RoomListScopeFilter`, `NotificationsInboxSections` |
| Agents | No `RoomListScope::Agents`. | No agent room-list chip. | Observation. `isAgentRoom` from local agent-card / approval fields; Agents chip is iOS chrome. | iOS `RoomSummary.isAgentRoom`; `RoomListScopeFilter.Kind.agents` |
| Archived / left / low-priority | Live list is SDK joined rooms. `is_low_priority` is projected on the Rust DTO and used only by unused `LowPriority` / `LowPriorityLast` helpers. UniFFI `RoomListRoomDto` **drops** `is_low_priority`. Desktop `parseRoomSummary` has no `isLowPriority`. | Not surfaced. | Not surfaced. | `live.rs` `folder_id: None`; UDL `RoomListRoomDto`; [`room.ts`](../../../../synara/src/app/features/matrix-dto/room.ts) |
| Name / recent presentation | Unused `ByName` / `RecentActivity` (ASCII lowercase, no locale). | Device-local `recent` \| `name` per Favorites/Rooms; `localeCompare` (`sensitivity: 'base'`); recent uses Core `lastActivityTs` only. Directs and space-child lists use `factoryRoomIdByActivity(mx)`, which reads the native facade’s `getLastActiveTimestamp` → same `lastActivityTs`. | Device-local `recent` \| `name` per Favorites/Channels/Directs; `localizedCaseInsensitiveCompare`. Missing ts becomes `.distantPast` (sorts last). | [`homeRoomList.ts`](../../../../synara/src/app/pages/client/home/homeRoomList.ts) + [`homeRoomList.test.ts`](../../../../synara/src/app/pages/client/home/__tests__/homeRoomList.test.ts); [`sort.ts`](../../../../synara/src/app/utils/sort.ts); [`Home.tsx`](../../../../synara/src/app/pages/client/home/Home.tsx); iOS `RoomListSortOrder`, `RoomListFavorites.sorted`, [`RoomListView.swift`](../../../../synara-ios/Synara/Features/RoomListView.swift) |
| Search / virtualization / layout | Must not own | React virtualizer, Create/Join/Search nav, collapsed categories | SwiftUI search field, filter/space strips, section headers, animation | ADR 0004 layer map |

Classification:

- Snapshot membership, `m.favourite` / `m.direct` / space-parent ids, unread counters, `marked_unread`, and `last_activity_ts` are **Core authority** and a **hard invariant** (ADR 0003: no second room-state engine; ADR 0004: no second Matrix owner).
- Home vs Directs vs Spaces vs Inbox, Favorites/Channels/Directs sections, filter chips, collapsed-category hide, search, virtualization, and sort menus are **platform observation / rendering** and an **accepted platform boundary**.
- React versus SwiftUI list chrome, and locale collation (`localeCompare` vs `localizedCaseInsensitiveCompare` vs unused Core ASCII `ByName`), are a **current technology preference**, not a reason to move presentation into Rust.

Earliest actual divergence is **native sections**, not a second favorite/unread writer. Desktop Home is orphan rooms (not DMs, not space children) with a Favorites lift; Directs is another root; Spaces live in the sidebar. iOS is one list with Favorites / Channels / Directs plus space grouping and chips. Both partitions read Core `isFavorite`. Both recent sorts read Core `lastActivityTs`. Neither calls `sort_rooms` or `room_matches_scope`.

The closest semantic remapping that is **not** a second engine: iOS `SharedCoreRoomListRows` sets `hasHighlight = highlightCount > 0 || markedUnread`. Mentions chips then include marked-unread rooms. Core’s unused `RoomListScope::Mentions` is highlight-only. Desktop has no mentions chip; its unread-info projection still treats `markedUnread` as unread attention, not as a highlight count. That is presenter attention chrome on Core fields.

## Boundary constraints

- ADR 0003: one Core for room state and Matrix writes. Presenters may project lists; they must not invent a second joined-room owner.
- ADR 0004 current layer map: desktop presentation owns navigation; Core owns room/timeline state. “Ordering” as Core authority means protocol/product tie-breakers when clients would otherwise compete — not locale collation, section headers, or filter-chip layout.
- ADR 0004 hard invariant 2: no UI framework in Core. Name collation that depends on device locale is locale UI context (observation), not a Core sort key.
- Workstream prior: keep navigation sections and locale/collation presentation local. Do not centralize locale presentation in Rust to force matching screenshots.
- Playbook §5 / goal-graph stop conditions: room-list live emit already landed (P4-S19 / S25–S26). This census must not invent S38, start P5, or register leftover secret/byte commands on `Core::command`. Promoting unused helpers onto UniFFI would be a new product surface, not a docs close.
- `RoomListFixtures.sorted` (highlight, then unread count, then activity) is placeholder/test chrome only. Live iOS uses `RoomListFavorites.sorted`.

Behaviors that must stay platform-side: section membership (Home/Directs/Spaces vs Favorites/Channels/Directs), filter chips, search, virtualization, animation, accessibility, and locale-aware name order.

## Alternatives

1. **No ownership change (stay-put / close).** Keep Core as the snapshot and row-field owner. Leave `sort.rs` / `filters.rs` as unused helpers. Keep section layout, chips, and collation on each presenter. Falsified if a shipped client sorts or filters from a JS/Swift Matrix room list instead of the Core snapshot, or if a presenter invents favorite/unread/DM truth that ignores Core fields.
2. **Bounded extraction or shared fixture.** Expose `RoomListSort` / `RoomListScope` on UniFFI and make both clients call them. Falsified as necessary tonight: there is no demonstrated product-semantic conflict that those helpers would resolve, and Core `ByName` would *replace* locale collation the brief forbids moving. Golden vectors for unused helpers would not change ownership.
3. **Broader Core model** (Core-owned sections, chips, search, or a single FavoritesThenRecent list). Would flatten intentional native navigation and fight ADR 0004 invariant 2. Falsified only by a new product requirement that both screens must be identical, including section topology.

Strongest stay-put case: unused Core helpers look like a latent shared owner, but product lists never call them. Treating that unused library as competing authority would mis-classify presenter section chrome as a second engine. Desktop Directs still using `factoryRoomIdByActivity` is the same Core timestamp through the native facade, not a js-sdk recency table.

## Recommendation

**Stay platform-side.**

Confidence: high that Core sort/filter helpers are unused by product UIs; high that favorite/DM/space/unread **fields** are already Core-owned and consumed; high that desktop vs iOS list topology is different native sections; medium that the iOS `markedUnread → hasHighlight` fold is only attention chrome (it would disagree with unused Core `Mentions` if those helpers were ever wired).

Supporting evidence:

- CENSUS.md snapshot still matches: `sort.rs` and `filters.rs` are unused by product UIs.
- Live snapshot path applies SDK joined filter and SDK entry order, then presenters re-sort.
- Desktop Home consumes Core `isFavorite` and `lastActivityTs` through local partition/sort chrome.
- iOS live list consumes the same snapshot fields through local partition/sort chrome.
- Neither client consumes `FavoritesThenRecent`, `LowPriorityLast`, `RoomListScope`, or `RoomListBadgeCounts`.
- Low-priority is not even on the UniFFI or desktop presenter DTO.
- Sort preference keys (`synara.roomListSort.*`) are device-local chrome, not Matrix account data.

Strongest objection: iOS mentions/unread chips and desktop muted-room badge exclusion do not match unused Core `RoomListScope` predicates. That is not two shipped owners of the same chip policy. Desktop has no those chips; iOS owns its chips; Core helpers are not on the wire. Wiring the helpers later would be a **new** shared-policy product decision, not a census remainder.

Unresolved questions that do **not** reopen ownership:

- Desktop Home/Directs use `matrix_mdirect_snapshot`; iOS section kind uses room-list `isDirect`. Both are Core projections. A mismatch would be a Core consistency bug, not a sort-helper extraction.
- Desktop Directs and space-child lists have no name/recent menu; iOS Directs does. Section chrome.
- Core `folder_id` is unused (`None` in live projection).
- Large-list performance of presenter sorts was not measured; the brief requires that evidence before a Core change, and none is present.

Regression proof to keep the boundary stable:

- Core: `matrix_room_list_snapshot` remains the joined-room owner; `sort_rooms` / `filter_rooms_by_scope` stay off the live snapshot path unless a later human implementation decision says otherwise.
- Desktop: Home continues to sort from Core `lastActivityTs` / `isFavorite`; `allRoomsAtom` continues to hydrate from the native snapshot only.
- iOS: `AppEnvironment.live()` keeps `SharedCoreRoomListService`; `PlaceholderRoomListService` stays test/preview-only.
- Neither presenter grows a js-sdk / independent Swift room-list sort that ignores Core row fields.

## Next gate

Stay platform-side. Close ROE-06. Do not write an implementation plan. Do not expose `RoomListSort` / `RoomListScope` on UniFFI. Do not move locale collation, filter chips, or navigation sections into Core. Unused helpers may remain as dead library code; deleting or wiring them is a later human product decision, not this census.

## Reviewer nits (`ACCEPT_WITH_NITS` on #1087)

Recorded from the independent review at `eff03f01`. They do not change the
close:

- Live iOS does not append `InviteDto` rows via
  `RoomListSearchFilter.mergeInvitedRooms`. `SharedCoreRoomListRows`
  overlays invite preview on already-listed snapshot rooms.
  `mergeInvitedRooms` only reinserts invited-membership rooms already in
  the in-memory array. Combined with joined-only `new_filter_joined()`,
  the census table overstates “merges invite rows into the list.”
- `PlaceholderRoomListService` is unused. `AppEnvironment.live()` uses
  `SharedCoreRoomListService`. The test/preview/UI-test stand-in that
  calls `RoomListFixtures.sorted` is `MockRoomListService`.
