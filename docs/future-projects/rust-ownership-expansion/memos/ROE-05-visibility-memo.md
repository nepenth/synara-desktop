# ROE-05 Research Memo: Visibility-contract residual census

Status: ownership boundary accepted; read/privacy remediation reopened; docs-only; not approved for implementation.

| Field              | Value                                                                                                                                                                                                 |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Workstream/cluster | ROE-05 (read and list semantics)                                                                                                                                                                      |
| Research owner     | Isolated researcher on `roe/memo-05-visibility`                                                                                                                                                       |
| Reviewers          | Independent feature-branch review; `ACCEPT_WITH_NITS` on PR `#1091` at `25c1ee02`                                                                                                                    |
| Source census      | 2026-09-01 against `5f81d9e7d1fccd762e16dad645bea8f07a216675`. [CENSUS.md](../program/CENSUS.md) is a `main` `011cf39a` snapshot only; product paths were re-read on this commit. Source wins.          |
| ADR baseline       | [ADR 0003](../../../adr/0003-shared-native-rust-core.md), [0004](../../../adr/0004-rust-language-boundaries.md), [0005](../../../adr/0005-native-media-handle-channel.md); index last reviewed 2026-09-01. Goal-graph and playbook §5 read on this census commit. |

This memo does not authorize product work, a new Core surface, UniFFI
changes, or a shared-Core phase change.

## Observable problem

A user who actually sees the live tail of a room expects that room to
become read on this device and, through Matrix, on their other devices.
A user who only opened a permalink, scrolled history, backgrounded the
app, or enabled “Hide Typing & Read Receipts” expects the opposite:
no receipt write from a non-visible tail.

The residual question is not whether desktop and iOS scroll the same
way. It is whether the *typed* platform-to-Core report of genuine
visibility is missing or disagreed, so that presenters independently
decide receipt eligibility after they already know what the user saw.

This memo does not move viewport geometry, scroll position, focus
detection, or badge chrome into Core.

## Current ownership census

`program/CENSUS.md` correctly names room-list counts,
`matrix_timeline_set_read_state`, `matrix_room_set_read_state`, desktop
auto-read / focus gating in the viewport policy, and iOS
foreground/background plus `SharedCoreReadMarkers`. Source on this
commit agrees those files still exist. Source also shows cutovers the
snapshot does not name:

- Live desktop mark-as-read from a mounted room uses
  `matrix_timeline_set_read_state` on the open view stream. Context-menu
  and “mark all” use `matrix_room_set_read_state`.
- Live iOS uses `SharedCoreRoomReadMarkerService` (P4-S24): open a
  temporary live stream, `timeline_set_read_state("mark_read")`, close
  it. Historical `MatrixRoomReadMarkerService` HTTP Bearer writes are
  test-only.
- Core `mark_live_timeline_read` always sends `m.fully_read` plus
  `m.read.private` to the SDK `latest_event_id()`. It never sends a
  public `m.read`. It never takes a presenter event id.
- `UnreadPositionStore` / `ReceiptIndex` / `AccountDataIndex` fully-read
  helpers are harnesses. They are not registered on `Core::command`.
- `in.synara.unread_anchor` remains a documented account-data *schema*
  only. No live Core/desktop/iOS writer was found. Timeline
  `unread_anchor_event_id` is the open-position Unread frontier, not
  that account-data type.

Where snapshot and source disagree, source wins.

Two product contracts must not be collapsed:

1. **Unread/count truth and Matrix writes** (SDK counts, `m.fully_read`,
   private receipt, marked-unread flag). Core authority.
2. **Genuine visibility** (focus, scene phase, painted live tail, hide-
   activity, explicit Mark as Read). Platform observation.

| Concern | Rust/Core | Desktop | iOS | Evidence/tests |
| ------- | --------- | ------- | --- | -------------- |
| Public / private receipts and `m.fully_read` *writes* | **Core authority (hard invariant).** `exact_read_receipts` sets `fully_read_marker` and `private_read_receipt` on one remote event; `public_read_receipt` is `None`. `mark_live_timeline_read` uses SDK `latest_event_id()` (same resolver as `Timeline::mark_as_read`) so presenters do not hand-walk visible items. Empty live timeline only clears the unread flag. | **Observation then Core write.** Auto-read calls `matrix_timeline_set_read_state` `{ streamId, action: mark_read }` after `nativeLiveReadTarget`. Explicit room-level write is `matrix_room_set_read_state`. Leftover `notifications.ts` `markAsRead` still *can* send public or private via js-sdk, but the desktop branch returns through the native owner and never reaches that path. | **Observation then Core write.** Product `SharedCoreRoomReadMarkerService.markRoomAsRead` opens a new live-bottom stream and calls UniFFI `timelineSetReadState(..., "mark_read")`. `markFullyRead(roomID:eventID:)` validates the id then *discards* it and marks the live tail. Leftover HTTP `MatrixRoomReadMarkerService` is not wired in `AppEnvironment.makeLive`. | Core [`live.rs`](../../../../crates/synara-core/src/app/timeline/live.rs) `exact_read_receipts` / `mark_live_timeline_read` / `set_read_state` / `set_room_read_state`; [`native.rs`](../../../../crates/synara-core/src/app/timeline/native.rs) `NativeTimelineReadAction`; [`core.rs`](../../../../crates/synara-core/src/core.rs) `matrix_timeline_set_read_state` / `matrix_room_set_read_state`. Desktop [`nativeRoomReadStateOwner.ts`](../../../../synara/src/app/utils/nativeRoomReadStateOwner.ts), [`nativeTimelineView.ts`](../../../../synara/src/app/features/room/nativeTimelineView.ts), [`notifications.ts`](../../../../synara/src/app/utils/notifications.ts) desktop early-return. iOS [`SharedCoreProductServices.swift`](../../../../synara-ios/Synara/Services/SharedCoreProductServices.swift) `SharedCoreRoomReadMarkerService`; [`AppEnvironment.swift`](../../../../synara-ios/Synara/Services/AppEnvironment.swift); [`RoomReadMarkerService.swift`](../../../../synara-ios/Synara/Services/RoomReadMarkerService.swift) leftover HTTP. Tests: Core `live.rs` `exact_read_receipts_target_one_event_*`; `p4_s9_timeline_read_state.rs`; desktop `nativeRoomReadStateOwner.test.ts`; iOS `SynaraCoreBindingsTests.testSharedCoreReadMarkers*`. |
| Genuine-visibility *observation* before `mark_read` | **Must not own.** No typed visibility/focus/lifecycle payload on the read-state commands. Core accepts `streamId`+`action` or `roomId`+`action` only. | **Platform observation.** `nativeLiveReadTarget` requires selected==snapshot room, `document.visibilityState === 'visible' && document.hasFocus()`, `hideActivity === false`, painted live bottom, `positionKind === 'live_bottom'`, `capabilities.markRead`, and a `$` tail id. A second rAF/scroll/resize pass re-checks paint+focus before `setReadState('mark_read')`. Jump-to-latest marks only if `!hideActivity`. | **Platform observation, different gates.** `RoomTimelineReadAcknowledgementPolicy` requires live + confirmed pin + not jumping + not dragging. Debounce 1s / max 2s. `onDisappear` `flushMarkFullyRead` can still write from `lastAcknowledgementCandidateEventID` after the user has scrolled off the tail (`cancelMarkFullyRead` does not clear that candidate). No `scenePhase` / `applicationState` check on the acknowledgement path. `SynaraForegroundMatrixMutationPolicy` gates approval session resume, not receipts. | [`nativeTimelineViewportPolicy.ts`](../../../../synara/src/app/features/room/nativeTimelineViewportPolicy.ts); [`NativeTimelinePresenter.tsx`](../../../../synara/src/app/features/room/NativeTimelinePresenter.tsx); [`nativeTimelineViewportPolicy.test.ts`](../../../../synara/src/app/features/room/__tests__/nativeTimelineViewportPolicy.test.ts). iOS [`RoomTimelineView.swift`](../../../../synara-ios/Synara/Features/RoomTimelineView.swift) `RoomTimelineReadAcknowledgementPolicy` / `scheduleMarkFullyRead` / `flushMarkFullyRead`; [`SynaraApp.swift`](../../../../synara-ios/Synara/App/SynaraApp.swift) `SynaraForegroundMatrixMutationPolicy`. Tests: `StableTimelineViewportTests` queue/latency. |
| `hideActivity` (“Hide Typing & Read Receipts”) | **No Core field.** Writes are always private+fully-read when invoked. Unused harness `ReceiptPrivacy` defaults Private. | **Inconsistent presenter gate.** Auto-read and jump-to-latest suppress on `hideActivity`. Sidebar / Home / Escape / inbox `markAsReadInBackground(mx, roomId, hideActivity)` pass the flag as the old *privateReceipt* argument; the desktop native branch **ignores** it and always `mark_read`. Context-menu Mark as Read also ignores it. Settings copy still says the switch turns off read receipts. | **Suppresses every Core write**, including swipe “Read” on the room list. `markRoomAsRead` returns `nil` when the setting is on. | Desktop [`General.tsx`](../../../../synara/src/app/features/settings/general/General.tsx); [`RoomNavItem.tsx`](../../../../synara/src/app/features/room-nav/RoomNavItem.tsx); [`Room.tsx`](../../../../synara/src/app/features/room/Room.tsx); [`notifications.ts`](../../../../synara/src/app/utils/notifications.ts). iOS [`SettingsView.swift`](../../../../synara-ios/Synara/Features/SettingsView.swift); [`SharedCoreProductServices.swift`](../../../../synara-ios/Synara/Services/SharedCoreProductServices.swift); [`RoomListView.swift`](../../../../synara-ios/Synara/Features/RoomListView.swift). |
| Fully-read *open position* / last-read frontier | **Core authority on desktop open.** `own_read_signals` loads `fully_read_event_id` plus unthreaded public and private receipts; `plan_unread_open` prefers a frontier already in the live window, else a receipt outside the window, and ignores a stale `m.fully_read` that is not in the live graph. `project_live_read_state` exposes `own_read_event_id` / `unread_anchor_event_id` / `is_marked_unread`. Unused `UnreadPositionStore.effective_frontier` prefers fully-read first and is **not** the live path. | **Observation of Core position.** Normal open sends a viewport *hint* (at-bottom, restored anchor, live-tail id). Core `resolve_normal_open_position` decides LiveBottom / Unread / Restored. Presenter does not pick the unread event. | **Presenter placement on Core fields, not a second receipt loader.** `fullyReadEventID` maps Core `ownReadEventId` else last ackable row. `RoomTimelineFocusPolicy` then picks live vs unread from *already-loaded* live items (newer receipt beats older fully-read; fully-read outside the graph is not a target). | Core [`live.rs`](../../../../crates/synara-core/src/app/timeline/live.rs) `unread_open_plan` / `plan_unread_open` / `own_read_signals` / `resolve_normal_open_position`. Unused [`unread/state.rs`](../../../../crates/synara-core/src/app/unread/state.rs). iOS [`RoomTimelineView.swift`](../../../../synara-ios/Synara/Features/RoomTimelineView.swift) `RoomTimelineFocusPolicy`; [`SharedCoreReadMarkers.swift`](../../../../synara-ios/Synara/Services/SharedCoreReadMarkers.swift). Tests: Core `unread_plan_*`; iOS `TimelineServiceTests.testRoomTimelineFocusPolicy*`. |
| Marked-unread | **Core authority.** `MarkUnread` is `set_unread_flag(true)` on the SDK room (view stream or room-level). `MarkRead` clears via the SDK receipt path. Room-list `marked_unread` is `room.is_marked_unread()`. `room_unread_presentation` lifts a zero joined count to 1 when the flag is set; highlight stays mention-only. | **Explicit intent → Core.** Context-menu `mark_unread` through `setRoomReadStateWithNativeOwner`. Auto-read treats `isMarkedUnread` as a reason to write even if the tail id already matches `ownReadEventId`. | **No product Mark unread.** Live list swipe is Mark Read only. `RoomUnreadPresentation.make` exists as a UniFFI helper around the same Core function but is not the live list mapper. | Core [`counts.rs`](../../../../crates/synara-core/src/app/room_list/counts.rs); desktop [`RoomNavItem.tsx`](../../../../synara/src/app/features/room-nav/RoomNavItem.tsx); iOS [`RoomListView.swift`](../../../../synara-ios/Synara/Features/RoomListView.swift); [`MatrixClientPolicies.swift`](../../../../synara-ios/Synara/Services/MatrixClientPolicies.swift) `RoomUnreadPresentation`. |
| Notification / highlight *counts* | **Core authority.** Live `project_room` uses `room_unread_presentation` (`num_unread_messages.max(num_unread_notifications)`, mentions → highlight) plus raw `marked_unread`. Snapshot DTO carries `unread_count` / `highlight_count` / `marked_unread`. `RoomListBadgeCounts` is unused by product UIs (ROE-06). | **Projection / rendering.** `unreadInfosFromNativeRooms` skips spaces and muted rooms; attention is `markedUnread \|\| unreadCount \|\| highlightCount`. Badge chrome is desktop-local. | **Projection / rendering.** Live rows copy Core counts. `SharedCoreRoomListRows` folds `markedUnread` into `hasHighlight` for chips (attention chrome, not a second counter). `hasUnreadMessages` is count or highlight. | Core [`live.rs`](../../../../crates/synara-core/src/app/room_list/live.rs) `project_room`; desktop [`roomToUnread.ts`](../../../../synara/src/app/state/room/roomToUnread.ts); iOS [`SharedCoreRoomListRows.swift`](../../../../synara-ios/Synara/Services/SharedCoreRoomListRows.swift). Tests: `counts.rs`; `roomToUnread.test.ts`; `RoomUnreadPresentationTests.swift`. |
| Threads | **Core write is unthreaded-only.** `own_read_signals` loads `ReceiptThread::Unthreaded` only. Receipt DTO *may* carry `thread_id`; harness `ReceiptIndex` can store it; live `exact_read_receipts` does not. Timeline rows may expose a root `thread` summary (ROE-03). | Thread chip focuses a root/latest event on the **room** stream. Auto-read still requires `live_bottom`. No thread-scoped receipt command. | Thread screen uses `threadTimelineUpdates` / in-room focus. It does **not** call `scheduleMarkFullyRead`. Reading a thread does not report thread visibility to Core. | Core [`live.rs`](../../../../crates/synara-core/src/app/timeline/live.rs) `ReceiptThread::Unthreaded`; [`dto/receipt.rs`](../../../../crates/synara-core/src/dto/receipt.rs). Desktop [`NativeTimelinePresenter.tsx`](../../../../synara/src/app/features/room/NativeTimelinePresenter.tsx) thread chip. iOS [`RoomTimelineView.swift`](../../../../synara-ios/Synara/Features/RoomTimelineView.swift) thread view. ROE-03 memo for the shared `thread_root` omission. |
| Late decryption | **Core authority for the row.** UTD → `EncryptedUnavailable`; later SDK `Set` replaces the row. `mark_live_timeline_read` still targets SDK latest remote id, including an encrypted tail. | Rendering until the next snapshot. If the user remains at live bottom, a later revision can re-arm auto-read (deduped when `ownReadEventId` already matches). | Rendering. Bottom-pin can re-schedule after the list updates. No separate decrypt eligibility. | Core [`live.rs`](../../../../crates/synara-core/src/app/timeline/live.rs) `reconcile_utd`; ROE-03 late-decrypt census. |
| Local echoes | **Core authority.** `latest_event_id()` skips a local echo without a server id; comment forbids targeting it. | `latestNativeReadEventId` requires `$…`. | `MatrixServerEventIDPolicy` rejects `$pending-` / `$local-`. Pending rows are presenter send UX (ROE-03). | `live.rs` `mark_live_timeline_read`; `nativeTimelineViewportPolicy.ts`; `RoomReadMarkerService.swift` `MatrixServerEventIDPolicy`. |
| Sync / pagination / room-change / retry races | **Core authority for the write and stream.** Mark-read on a view requires that stream; room-level mark opens/reuses the live entry. SDK receipt dedup is documented for 0.18. | Room change bumps `liveTailMarkGenerationRef` and clears the submitted key. `setReadState` failure clears the key so a later paint can retry. Pagination of a non-live position cannot satisfy `positionKind === 'live_bottom'`. | Debounce coalesces; flush on leave is a separate invoke. Opening a *new* live stream to mark can race the viewed unread/focused stream (write still goes to the temporary live tail, not the historical window). | `live.rs` SDK 0.18 comment; desktop presenter generation effect; iOS `RoomTimelineReadMarkerQueuePolicy`. |
| Multiple devices | **Core / homeserver.** `m.fully_read` is room account data (other own devices). `m.read.private` is this-device. Counts come from the SDK after sync. Core does not emit public `m.read`, so *other users* do not see Synara read receipts. | Same Core write. Leftover `RoomViewFollowing` / `useRoomEventReaders` still walks js-sdk `getUsersReadUpTo` under the composer; on the native facade that is leftover *display*, not a write. | Same Core write. No public-receipt reader chrome in the thread/room composer. | `exact_read_receipts`; [`RoomView.tsx`](../../../../synara/src/app/features/room/RoomView.tsx); [`useRoomEventReaders.ts`](../../../../synara/src/app/hooks/useRoomEventReaders.ts). Opt-in Synapse proof [`receipt_synapse_proof.rs`](../../../../src-tauri/src/matrix/timeline/live_synapse_proof/receipt_synapse_proof.rs) still sends a **public** receipt — that is a harness, not the product write. |
| Badge / unread chrome | Must not own presentation. | Rendering. Mute skip and space rollup from Core fields. | Rendering. Chips, inbox sections, accessibility “N unread”. | ADR 0004 layer map; ROE-06 / ROE-07. |

**Earliest actual divergence.** Not a second Matrix write engine: both
live clients invoke Core `mark_read` / `mark_unread`, and Core alone
chooses the remote event and the private+fully-read pair. Not a second
unread counter: both project Core room-list fields.

The remaining split is the **observation seam**. The typed command is
only `{ streamId \| roomId, action }`. Platforms never send “I painted
this live tail while focused.” They decide *whether* to invoke. Those
decisions differ:

1. Desktop requires document visibility **and** focus **and** a painted
   live bottom on the **current** view stream.
2. iOS requires live + pin + debounce, then marks by opening a
   **temporary** live stream. It has no scene-phase gate. Leave-room
   flush can use a stale last-candidate after the user left the tail.
3. `hideActivity` is a local preference that Core does not see. Desktop
   auto-read honors it; desktop explicit Mark as Read does not; iOS
   honors it for every write, including swipe Read.
4. Thread visibility is never a Core input; writes stay unthreaded.

That is platform observation quality and a missing *shared description*
of the gates, not two receipt engines.

**Constraint classes.**

- Unread/count truth, receipt/read-marker writes, and the marked-unread
  flag are **hard invariants** (ADR 0003/0004: one Matrix owner; no
  presenter-chosen public vs private vs event id once the invoke
  happens).
- Viewport geometry, scroll position, focus, scene phase, painted-tail
  detection, debounce, and badge chrome are **accepted platform
  boundaries**.
- React virtualizer vs UIKit pin, 1s iOS debounce vs desktop rAF, and
  leftover JS reader chrome are **technology preferences**.

## Boundary constraints

- ADR 0003: one Core for room/timeline state and Matrix writes.
  Presenter projections are not a second domain owner. Thin adapters
  (`SharedCoreRoomReadMarkerService`, Tauri bridges) are not a second
  engine.
- ADR 0004 (2026-09-01): Core owns “receipt eligibility/writes.”
  Platforms own observations Core cannot independently know, “such as
  whether a message is genuinely visible.” Viewport geometry must not
  enter Core. A platform observation may be a typed *input* to Core
  without Core owning the observation.
- ADR 0004 invariant 5: no permanent dual owner. Leftover HTTP and
  js-sdk `markAsRead` writers must not become a live second path.
- ADR 0005: unused here (no media bytes on `Core::command`).
- Workstream [05](../workstreams/05-read-marker-unread.md): propose a
  contract clarification only if the earliest divergence is observation
  vs Core authority. Do not let presenters independently decide
  eligibility *once observations cross that contract*. Do not move
  viewport math into Core.
- Playbook §5 / goal-graph stop conditions
  ([13-language-boundary-goal-graph.md](../../../shared-native-core/13-language-boundary-goal-graph.md)):
  P4-S24/S26 already landed. Next required node is the P4 engine-ready
  *gate* (pending/blocked). Docs-only PRs remain allowed. Do not invent
  S38. Do not start P5. Do not register leftover secret/byte commands.
- D1–D10: this memo cannot open an implementation gate, amend an ADR,
  or claim a shared-Core phase.

Recorded observation contract (documentation only — not a new command):

A platform may invoke Core `mark_read` only when all of the following
are true, unless the user issued an explicit Mark as Read:

1. The room is the selected room on that client.
2. The process is foreground-active (desktop: document visible and
   focused; iOS: scene active **and** `applicationState == .active`).
3. `hideActivity` is false.
4. The viewed position is the live tail, not unread/focused/history.
5. That live tail is painted in the viewport at invoke time.
6. The tail has a server event id (not `$pending-` / `$local-` / no id).

Explicit Mark as Read / Mark Unread is user intent, not visibility.
Whether `hideActivity` suppresses those explicit writes is a **product**
choice the two clients currently disagree on. Thread-scoped receipts are
absent on both; that is a missing Core write, not a presenter engine.

Once such an observation is true, Core remains the only eligibility
owner for *what* is written (latest remote id, private+fully-read,
unread-flag clear). Presenters must not pick public vs private or walk
visible items.

## Alternatives

1. **No ownership change (stay-put).** Keep Core as the sole count and
   write owner. Keep measuring visibility on each presenter. Treat gate
   mismatch as accepted native observation (and, where it is a bug,
   a later presenter fix). Leave harnesses and leftover HTTP/JS writers
   unwired. Falsified if a shipped client still *writes* receipts or
   unread flags without Core, or if a presenter chooses the event id /
   public-vs-private pair that Core then blindly sends.

2. **Bounded extraction (typed visibility observation).** Add a Core
   payload such as “live tail painted + focused + hideActivity” and
   refuse `mark_read` unless that observation is present (explicit Mark
   as Read as a separate action). Falsified if the current boolean
   invoke already preserves one write owner *and* no live path shows
   Core accepting a write the platform did not mean — i.e. if the
   remainder is presenter gate quality, not missing Core authority.

3. **Broader Core model.** Move viewport geometry, focus detection,
   scene phase, scroll pinning, or badge presentation into Core.
   Falsified immediately by ADR 0004 and the workstream “keep closed”
   list. Productizing `UnreadPositionStore` as a second open-position
   owner would also fight the live `plan_unread_open` path.

Stay-put is the default unless harmful duplicated *authority* is
proven. The proven remainder is an informal observation invoke plus
inconsistent presenter gates — not two unread tables and not a missing
Core writer.

## Recommendation

**Already correctly owned** for unread/count truth and Matrix
receipt / fully-read / marked-unread writes.

**Stay platform-side** for genuine-visibility measurement, lifecycle,
viewport math, debounce/paint, and badge chrome.

Confidence: high that live writes and counts are Core-only on both
clients; high that leftover HTTP and js-sdk writers are not the product
path; medium that iOS leave-room flush and the `hideActivity` split can
produce a receipt the *user* would not call “seen,” which is an
observation defect, not a second engine; medium that thread-scoped
receipts are an accepted product absence rather than a residual Core
extract.

The typed invoke is underspecified as a *visibility observation*
(`action` only) and the two clients’ gates are inconsistent. That
answers the bounded question. It does **not** justify extracting a Core
visibility engine or starting a new command family. The clarification
that is earned is the gate list in Boundary constraints, plus optional
later **docs-only** focus/visibility fixtures under
`docs/future-projects/**` if a human wants them. Required live evidence
named by the workstream (ordering/property tests, two-client Synapse
receipts, desktop/iOS lifecycle tests) is not claimed here; the opt-in
Synapse proof still exercises a public receipt the product does not
send.

Do not productize `UnreadPositionStore`, `ReceiptIndex`, or
`AccountDataIndex.set_fully_read`. Do not register leftover HTTP
read-marker I/O on `Core::command`. Do not move viewport, focus, or
badges into Core. Do not treat this memo as permission to delete
leftover TypeScript/Swift helpers or to add UniFFI fields.

Strongest objection (extract a visibility DTO anyway): ADR 0004 already
allows a typed observation input; presenters today decide eligibility
before Core sees anything, which is exactly the workstream’s “once
observations cross the contract” fear; iOS can flush a live-tail write
after the user left history; desktop explicit Mark as Read ignores
`hideActivity` while iOS does not. Those are real seams. They are
still presenter observation / product-preference defects on top of one
write owner. Wiring a new Core observation would start product work
while the goal graph is stopped on the P4 engine-ready gate (D3/D9).

Unresolved product defects and decisions:

- Product: does “Hide Typing & Read Receipts” suppress *all* receipt
  writes, including explicit Mark as Read, or only automatic ones?
  Desktop and iOS disagree.
- Product: Synara never sends public `m.read`. The settings string and
  the leftover js-sdk helper still talk as if public receipts exist.
- iOS currently lacks the documented scene/application-active gate, and
  cancellation does not clear the last candidate. A leave-room flush can
  therefore invoke Core after the user moved away from the tail. Core still
  chooses the latest event on a newly opened live stream; that does not make
  the stale presenter observation correct.
- Thread-scoped receipts if a later product requires them. That would
  be a new Core write, not a visibility DTO, and is not authorized here.

Regression proof to keep this close stable: live desktop and iOS
mark-read/unread go only through `matrix_timeline_set_read_state` /
`matrix_room_set_read_state` (or the UniFFI wrappers of those
commands); Core continues to target SDK `latest_event_id()` with
private+fully-read only; leftover HTTP `MatrixRoomReadMarkerService`
stays unwired; desktop js-sdk `markAsRead` stays behind the native
early-return; `UnreadPositionStore` stays unregistered; presenters keep
viewport/focus/lifecycle; NSE never starts sync or writes receipts.

## Next gate

The write/count ownership question is closed and visibility remains platform-
observed. Product correctness and privacy are reopened as
[A4](../program/ACTIONS.md#a4--readprivacy-contract): add the missing active-
scene gate, clear pending and last-candidate state on cancellation/scroll-away,
prevent stale leave-room flushes, and decide one explicit-versus-automatic
`hideActivity` contract across clients. Cover transition races and prove the
actual two-client receipt/unread result on Synapse. Leftover
`markAsReadAtEvent` / `setUnreadAnchor` have no desktop native early return but
also no product callers; keep them unwired or remove them through a separate
hygiene change.
