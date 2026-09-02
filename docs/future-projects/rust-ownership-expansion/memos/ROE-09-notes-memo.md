# ROE-09 Research Memo: Notes and Account-Data Ownership

Status: draft research; docs-only; not approved for implementation.

| Field              | Value                                                                                                                                 |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------- |
| Workstream/cluster | ROE-09                                                                                                                                |
| Research owner     | Isolated researcher on `roe/memo-09-notes`                                                                                            |
| Reviewers          | Unassigned                                                                                                                            |
| Source census      | 2026-09-01 against `0b6c4297989746a95df21d4a6d286480ee61d570`                                                                         |
| ADR baseline       | ADR 0003, 0004, 0005 last reviewed 2026-09-01 (index in [`docs/adr/README.md`](../../../adr/README.md)); same census commit            |

[program/CENSUS.md](../program/CENSUS.md) recorded notes as already owned on
`main` `011cf39a`. Re-read source on this commit agrees. The snapshot is still
accurate for this domain.

## Observable problem

Personal notes, ToDos, and message anchors must stay private to the signed-in
user, survive across desktop and iOS, and not fork into two Matrix account-data
writers. The user-visible risk is not editor chrome. It is whether either
shipped client still normalizes, persists, or syncs `in.synara.room_notes`
without going through the existing Core schema and CRUD path.

No current source evidence shows a second notes engine on the shipped write
path. Editors, list sort, and drag/reorder remain platform-owned, as ADR 0004
already requires.

## Current ownership census

| Concern | Rust/Core | Desktop | iOS | Evidence/tests |
| ------- | --------- | ------- | --- | -------------- |
| Event type, v1 schema, item kinds | Authority. `ROOM_NOTES_EVENT_TYPE` = `in.synara.room_notes`; version forced to `1`; kinds `note` / `todo` / `message`; camelCase fields | Projection types only in `synara/src/types/matrix/accountData.ts` | Projection structs in `RoomNotesService.swift`; UniFFI `RoomNoteItemDto` | [`room_notes.rs`](../../../../crates/synara-core/src/app/account_data/room_notes.rs); [`synara-room-notes-contract.md`](../../../../synara/docs/synara-room-notes-contract.md); [`synara-room-notes-content.schema.json`](../../../../synara/docs/contracts/synara-room-notes-content.schema.json) |
| Normalize, limits, malformed remote | Authority. Drops unknown kinds, empty ids, non-finite timestamps, wrong-room items, empty note/todo bodies, message items without `eventId`; caps bodies (4000 / 1000); keeps 200 newest-by-`updatedAt` items per room | Display-time `normalizeRoomNotesContent` on Core snapshots only; leftover `put` / `remove` / `complete` / `move` helpers are unit-test local and do not persist | Live path maps Core DTOs; does not re-encode the wire event | Core: `normalize_room_notes_content`; desktop tests [`roomNotes.test.ts`](../../../../synara/src/app/utils/__tests__/roomNotes.test.ts), [`contractSchemas.test.ts`](../../../../synara/src/app/contracts/__tests__/contractSchemas.test.ts); Tauri re-export tests in [`src-tauri/.../room_notes.rs`](../../../../src-tauri/src/matrix/account_data/room_notes.rs) |
| Create / edit / delete / complete | Authority. Registered `matrix_room_notes_{snapshot,upsert,delete,complete_todo,move_todo}`; live RMW on `Client` account data | Thin Tauri invoke → `Core::command`. No `setAccountData(in.synara.room_notes)` | Live `AppEnvironment` uses `SharedCoreRoomNotesService` → UniFFI → same five commands | Core register/handlers in [`core.rs`](../../../../crates/synara-core/src/core.rs); live I/O [`room_notes_live.rs`](../../../../crates/synara-core/src/app/account_data/room_notes_live.rs) via `NativeImagePackOwner`; desktop [`nativeRoomNotesOwner.ts`](../../../../synara/src/app/features/room/nativeRoomNotesOwner.ts), [`bridge/room_notes.rs`](../../../../src-tauri/src/bridge/room_notes.rs); iOS [`SharedCoreRoomNotes.swift`](../../../../synara-ios/Synara/Services/SharedCoreRoomNotes.swift), [`AppEnvironment.swift`](../../../../synara-ios/Synara/Services/AppEnvironment.swift) `live()`; fail-closed [`p4_s9_room_notes.rs`](../../../../crates/synara-core/tests/p4_s9_room_notes.rs), [`SynaraCoreBindingsTests.swift`](../../../../synara-ios/SynaraTests/SynaraCoreBindingsTests.swift) |
| Reorder persistence | Authority for ToDo adjacent swap (`move_room_todo_item`) and for any item upsert that includes `order` | ToDo chevrons call Core `move_todo`. Note chevrons compute a fractional `order` locally, then Core `upsert` | Drag / chevrons compute fractional `order` locally (`RoomNoteOrdering`), then Core `upsert` via `setItemOrder`. `moveTodo` exists on the service but the shipped notes screen does not call it | Core `move_room_todo_item`; desktop [`RoomNotesPanel.tsx`](../../../../synara/src/app/features/room/room-notes/RoomNotesPanel.tsx); iOS [`RoomNotesView.swift`](../../../../synara-ios/Synara/Features/RoomNotesView.swift) |
| Stable IDs | Authority stores the id the presenter supplies; empty id fails closed | Mints `kind:<base36-ts>:<rand>` for note/todo before upsert | Mints `kind:<uuid>` for note/todo and `message:<uuid>` for pins | Presenter mint + Core validate; not a second identity store |
| Versioning / migration / downgrade | Authority. Readers coerce `version` to `1` and drop unknown kinds/fields. No v2 writer exists | Same v1 contract types | Same v1 DTO fields | `ROOM_NOTES_ACCOUNT_DATA_VERSION = 1`; schema `const: 1` |
| Concurrent / offline updates | Authority. Last-write-wins global-event RMW (`load` → mutate → `set_account_data_raw`). No etag / tombstone / CRDT. Missing event → empty default | 1s snapshot poll; preserves last projection on transient failure; no local write queue | Refresh / `.task` reload; optimistic list only for drag, rolled back on Core failure; no local write queue | `mutate_room_notes` in `room_notes_live.rs`; [`roomNotesList.ts`](../../../../synara/src/app/state/roomNotesList.ts) |
| Tombstones | None. Delete removes the item; empty room buckets drop | Same via Core delete | Same via Core delete | `remove_room_note_item` |
| Clock independence | Authority requires finite `createdAt` / `updatedAt`; complete/move stamp `room_notes_now_ms()` | Presenter supplies millisecond timestamps on create/upsert | Same | Finite checks in normalize + live validate |
| Message anchors | Authority: `message` items require `eventId`; body/sender/`eventTs` are optional helpers | Helper `createMessageRoomNoteItemFromIds` exists and would Core-upsert. Shipped `NativeTimelinePresenter` does not expose “Pin to Notes”. Legacy `Message.tsx` `onAddToNotes` has no caller | Timeline / thread menus call `pinMessage` → Core upsert. Requires durable `serverEventID` | iOS [`RoomTimelineView.swift`](../../../../synara-ios/Synara/Features/RoomTimelineView.swift); desktop presenter has Later/pin-state, not notes-pin ([`NativeTimelinePresenter.tsx`](../../../../synara/src/app/features/room/NativeTimelinePresenter.tsx), [`nativeTimelinePresenterActions.test.ts`](../../../../synara/src/app/features/room/__tests__/nativeTimelinePresenterActions.test.ts)) |
| Privacy | Authority writes user global account data, not room state. DTOs are documented “no tokens” | Panel copy: personal notes | Footer: private to the account; never posted to the room | Contract + `RoomNotesSnapshotDto` comments in [`shared_core_ffi.rs`](../../../../crates/synara-core/src/shared_core_ffi.rs) |
| Export | No notes-specific export owner in Core or either client | None found | None found | Census search; not a second engine |
| Editors / drag / presentation | Must not own | React composer, chevrons, header summary, side panel | SwiftUI editor, Edit-mode drag, sheets | ADR 0004 layer map; workstream prior |

Classification:

- Schema, normalize, limits, Matrix read-modify-write, and the five commands are
  **Core authority** and a **hard invariant** (ADR 0003: no second account-data
  owner; ADR 0004: no second Matrix engine).
- Viewport polling, ID minting, fractional rank, SwiftUI/React editors, and
  list chrome are **platform observation / rendering** and an **accepted
  platform boundary**.
- React versus SwiftUI editor widgets are a **current technology preference**,
  not a reason to move editors into Core.

Earliest actual divergence is presentation, not persistence: desktop ToDo
chevrons use Core `move_todo`; iOS drag persists rank through Core `upsert`.
Both write the same v1 event. Desktop does not currently offer a native-timeline
“Pin to Notes” affordance; iOS does. That is a missing presenter menu, not a
second writer.

## Boundary constraints

- ADR 0003: one Core for account-data and Matrix writes. Swift/JS must not
  compete for that authority.
- ADR 0004 current layer map: “Notes/account data — Core schema, normalization
  and Matrix synchronization / Editors, drag/reorder affordances and
  presentation.”
- ADR 0005: unused here. Notes bodies and ids travel on the generic envelope;
  no media bytes or filesystem paths.
- Playbook §5 / goal-graph stop conditions: notes already landed as P4-S9-7
  (#955). This census must not invent S38, start P5, or register leftover
  secret/byte commands on `Core::command`. The generic desktop
  `setAccountData` facade remains a documented fail-closed GAP
  ([`nativeClientFacade.ts`](../../../../synara/src/app/features/native-client/nativeClientFacade.ts)
  F3) and is not a notes write path.
- `MockRoomNotesService` is preview / UI-test only
  (`AppEnvironment.mock`, `SYNARA_UI_TEST_ROOM_NOTES`). Shipped `live()`
  wires `SharedCoreRoomNotesService`.
- Developer-tools account-data listing is not a product notes engine.

Behaviors that must stay platform-side: text editing, accessibility, gestures,
drag handles, chevron enablement, locale timestamps, and room-scoped list
layout.

## Alternatives

1. **No ownership change (stay-put / close).** Keep Core as the only
   `in.synara.room_notes` writer. Keep leftover desktop mutation helpers as
   unused local codec, not authority. Keep editors and drag local. Falsified
   if a shipped client writes the event type through js-sdk / raw HTTP / a
   Swift Matrix client, or if Core commands are unregistered.
2. **Bounded extraction or extra fixture.** Shared v1 schema and fixtures
   already exist under `synara/docs/contracts/`. iOS already has a gated
   two-client live smoke (`testLiveRoomNotesSyncWhenConfigured`) that writes
   the event from an independent Matrix client and reads the iOS write back
   from global account data. A new portfolio Synapse fixture would not change
   ownership. Falsified if current schema/fixtures disagreed with Core
   normalize (they do not on this commit).
3. **Broader Core model** (Core-owned editor state, Core-owned drag geometry,
   or a second notes store). Would fight ADR 0004 and the workstream rule
   against a second engine for a native UX gap. Falsified only by a new
   product requirement that rich note documents must be identical byte-for-byte
   in both editors.

Strongest stay-put case: leftover `putRoomNoteItem` / `completeRoomTodoItem` /
`moveRoomTodoItem` in `synara/src/app/utils/roomNotes.ts` look like a second
codec, but product writes never call them. Treating that leftover as an engine
would mis-classify a presenter projection as competing authority.

## Recommendation

**Already correctly owned.**

Confidence: high for shipped write/sync ownership; medium for live
homeserver cross-client proof (iOS smoke is gated; desktop C5 notes live
proof remains unclaimed and is not an ownership gate).

Supporting evidence:

- One codec and one live RMW in `crates/synara-core`.
- Desktop product mutations go only through `matrix_room_notes_*`.
- iOS product mutations go only through the same five UniFFI methods.
- No `setAccountData(in.synara.room_notes)` on the shipped desktop path.
- Editors and drag stay in React/SwiftUI and persist through Core upsert or
  `move_todo`.
- CENSUS.md snapshot matches current source.

Strongest objection: unused TypeScript mutation helpers plus an unwired
desktop message-pin helper could be mistaken for residual dual ownership.
They do not persist and do not sync. Wiring “Pin to Notes” on
`NativeTimelinePresenter` would be a presenter change, not a Core extraction,
and is out of scope for this census.

Unresolved questions that do **not** reopen ownership:

- Desktop native timeline still lacks a notes-pin menu item.
- iOS list reorder uses Core upsert rather than `move_todo`.
- Last-write-wins RMW can lose a concurrent edit from another device; that is
  existing Core policy, not a second engine.
- No notes export surface exists on either client.

Regression proof to keep the boundary stable:

- Core: family remains registered; no-session fail-closed; normalize still
  drops malformed items and enforces caps.
- Desktop: `nativeRoomNotesOwner.ts` remains the only product writer; no new
  js-sdk `setAccountData` for `in.synara.room_notes`.
- iOS: `AppEnvironment.live()` keeps `SharedCoreRoomNotesService`;
  `MockRoomNotesService` stays test/preview-only.
- Contract fixtures in `synara/docs/contracts/fixtures/synara-room-notes-content.json`
  continue to match Core v1.

## Next gate

Already owned. Close ROE-09. Do not write an implementation plan. Do not move
editors or drag into Core. Do not add a second notes store. Shared schema and
fixtures already exist; no new portfolio fixture is required to close.
A later product owner may add a desktop notes-pin menu without changing this
ownership decision.
