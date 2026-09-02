# ROE-10 Research Memo: Draft Serialization and Reply Metadata

Status: ownership split accepted; desktop reply defect reopened; docs-only; not approved for implementation.

| Field              | Value                                                                                                                                 |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------- |
| Workstream/cluster | ROE-10                                                                                                                                |
| Research owner     | Isolated researcher on `roe/memo-10-drafts`                                                                                           |
| Reviewers          | Independent feature-branch review; original `ACCEPT` on PR `#1088` at `11aa9881e8c9d45e8d27cf57b61ae8656fe54ae9`; later promotion review elevated the reply defect |
| Source census      | 2026-09-01 against `00155b9fab2a29e1b477eafe3f3d7839e968c7bc`                                                                         |
| ADR baseline       | ADR 0003, 0004, 0005 last reviewed 2026-09-01 (index in [`docs/adr/README.md`](../../../adr/README.md)); same census commit            |

[program/CENSUS.md](../program/CENSUS.md) recorded drafts as split on
`main` `011cf39a`: reply/thread draft commands on the timeline owner; desktop
Slate/Jotai composer body local-only; iOS Core reply draft plus local SwiftUI
composer state. Re-read source on this commit agrees with that split. Source
wins on two consumption details the snapshot does not name: shipped iOS UI
does not call the Core reply-draft family, and the desktop Jotai reply atom
is no longer written.

This memo does not authorize product work, a new Core surface, a durable
draft schema, or a shared-Core phase change.

## Observable problem

Users type a message, start a reply or thread, leave the room, crash, or open
the other client. The residual question is whether reply/thread *identity*
and ordinary composer *bodies* still have competing owners, and whether any
accepted product rule requires crash-restored or cross-device *rich* drafts
that would justify a wire-neutral Core schema.

The user-visible risk is not whether Slate and SwiftUI look the same. It is
whether either client still invents reply/thread targets, persists composer
bodies into Matrix account data, or treats editor implementation state as
shared protocol truth.

No current source evidence shows a second draft engine or an accepted
cross-device rich-draft requirement. Reply/thread metadata already has a
Core command family. Composer bodies stay local.

## Current ownership census

| Concern | Rust/Core | Desktop | iOS | Evidence/tests |
| ------- | --------- | ------- | --- | -------------- |
| Reply / thread *target identity* | Authority. Registered `matrix_composer_set_reply_draft` / `get` / `clear` on the timeline owner. Set loads the Matrix event, rejects invalid/redacted/unsupported targets, and stores a per-room preview (`event_id`, `sender_id`, target `body` / `formatted_body`, optional `thread_root_event_id` when `start_thread` or an existing thread relation). In-process `ComposerDraftRegistry` only; not account data and not durable across process death. Send does **not** auto-read this registry; `matrix_send_text` takes explicit `reply_to` / `thread_root`. | Product set/get/clear through Tauri → `Core::command`. `NativeTimelinePresenter` Reply / Reply-in-thread call `setNativeComposerReplyDraft` and render the Core readback banner. `RoomInput` clears Core on cancel/send. Leftover Jotai `roomIdToReplyDraftAtomFamily` is only *cleared*; no shipped writer populates it. `mapNativeReplyDraftToJs` is unused on the product path. | UniFFI wrapper `SharedCoreComposerReplyDraft` exists (P4-S9-21). Shipped `RoomTimelineView.beginReply` does **not** call it. Reply identity is `@State replyTarget` built from the already-loaded `TimelineItem` (`ComposerRelationTarget`). Send passes that `eventID` as `replyToEventID` on the Core send path. Thread screens use the thread root as view identity, not the reply-draft registry. | Core [`composer.rs`](../../../../crates/synara-core/src/app/timeline/composer.rs), [`live.rs`](../../../../crates/synara-core/src/app/timeline/live.rs) `set_reply_draft` / `load_reply_draft_preview`; register/handlers in [`core.rs`](../../../../crates/synara-core/src/core.rs); census names in [`census.rs`](../../../../crates/synara-core/src/transport/census.rs); UniFFI in [`shared_core_ffi.rs`](../../../../crates/synara-core/src/shared_core_ffi.rs) / `synara_core.udl`. Tests: [`p4_s9_composer_draft.rs`](../../../../crates/synara-core/tests/p4_s9_composer_draft.rs). Desktop: [`timeline_composer.rs`](../../../../src-tauri/src/bridge/timeline_composer.rs), [`nativeComposerDraftOwner.ts`](../../../../synara/src/app/features/room/nativeComposerDraftOwner.ts), [`NativeTimelinePresenter.tsx`](../../../../synara/src/app/features/room/NativeTimelinePresenter.tsx), [`RoomInput.tsx`](../../../../synara/src/app/features/room/RoomInput.tsx). iOS: [`SharedCoreComposerReplyDraft.swift`](../../../../synara-ios/Synara/Services/SharedCoreComposerReplyDraft.swift); fail-closed [`SynaraCoreBindingsTests.swift`](../../../../synara-ios/SynaraTests/SynaraCoreBindingsTests.swift); product [`RoomTimelineView.swift`](../../../../synara-ios/Synara/Features/RoomTimelineView.swift) `beginReply`. |
| Reply *preview chrome* | Authority for the privacy-safe preview DTO of the *target* event (not the unsent composer body). Schema version `1`. | Rendering: presenter banner from Core readback. RoomInput still has a leftover Jotai banner that stays empty because the atom is never set. | Rendering: `ComposerRelationBanner` from local `ComposerRelationTarget` snippet/sender. | `ComposerReplyDraftPreviewDto`; desktop presenter banner; iOS [`TimelineReplyPreview.swift`](../../../../synara-ios/Synara/Services/TimelineReplyPreview.swift) |
| Ordinary composer *body* | Must not own. No body field on the reply-draft commands. No `in.synara.room_draft` account-data type. | Platform. Slate `Descendant[]` in Jotai `roomIdToMsgDraftAtomFamily` plus `localStorage` key `in.synara.room_draft.<userId>.<roomId>` (v1, 64 KiB cap). Autosave on editor change and room unmount. | Platform. In-memory `DraftStore` (`[roomID: String]`). `RoomTimelineView` writes on every `draft` change. Logout wipe calls `clearAll()`. Not UserDefaults, not files, not Core. | [`drafts.ts`](../../../../synara/src/app/utils/drafts.ts), [`drafts.test.ts`](../../../../synara/src/app/utils/__tests__/drafts.test.ts), [`RoomInput.tsx`](../../../../synara/src/app/features/room/RoomInput.tsx); iOS [`ComposerService.swift`](../../../../synara-ios/Synara/Services/ComposerService.swift), [`LocalWipeService.swift`](../../../../synara-ios/Synara/Services/LocalWipeService.swift), [`ComposerServiceTests.swift`](../../../../synara-ios/SynaraTests/ComposerServiceTests.swift) |
| Editor implementation state (typing, selection, attributed/Slate tree) | Must not own. | Slate editor + React selection. Persisted value *is* the Slate tree. | SwiftUI / UIKit composer focus, selection, attributed editing. | ADR 0004 layer map; charter closed boundary |
| Mentions | Authority validates mention user ids / room mention on *send* (`validated_mentions`). No mention list in the reply-draft registry. | Observation at send time from the live editor (and leftover Jotai reply user). Not stored in `StoredRoomDraft`. | Not persisted on `DraftStore`. | [`send/text.rs`](../../../../crates/synara-core/src/app/send/text.rs); `RoomInput.tsx` submit |
| Edit identity | Not the reply-draft family. Edit is a separate send/edit command with `editEventID`. | Native edit affordance on the timeline presenter; not a Core draft document. | `ComposerEditSession` + `ComposerEditFlow` hold the edit target and restore the previous local draft on cancel. | iOS `ComposerService.swift`; Core `matrix_edit_message` (out of this body’s owner) |
| Attachment *handles / bytes* | Metadata/handles only (ADR 0005). Reply-draft commands carry no attachment list. | Jotai upload list + native send queue; bytes stay off `Core::command`. | `ComposerAttachmentDraft` holds local `Data` in the presenter; send uses the dedicated native upload path. | ADR 0005; iOS `ComposerAttachmentDraft.swift` |
| Crash restore / process death | Reply registry dies with the attached timeline owner. No durable Core draft store. | Device-local restore of Slate JSON via `localStorage` for the same user/room/profile. Explicitly not Matrix account data. | `DraftStore` is process memory. Relaunch starts empty. | [`synara-namespaces.md`](../../../../synara/docs/synara-namespaces.md); FR-7.4-009 in [`feature-parity-traceability.md`](../../../matrix-rust-sdk/feature-parity-traceability.md) |
| Cross-device / account-data draft sync | None. Account-data types in the shipped namespace doc are Later, room notes, unread anchors, and spaces — not drafts. | Documented local-only. No `setAccountData` draft writer. | No Core or Swift account-data draft writer. | [`synara-namespaces.md`](../../../../synara/docs/synara-namespaces.md); [`synara-modernization-roadmap.md`](../../../../synara/docs/synara-modernization-roadmap.md) |

Classification:

- Reply/thread target resolution, validation, and the typed in-session
  preview DTO are **Core authority** and a **hard invariant** for Matrix
  identity (ADR 0003/0004: no second Matrix write/relation owner). The
  registry is session-scoped, not a durable document store.
- Composer bodies, Slate/Swift editor trees, typing, selection, focus, and
  attachment *UI* lists are **platform observation / rendering** and an
  **accepted platform boundary** (ADR 0004: Slate and Swift editor state stay
  platform-owned).
- React/Slate versus SwiftUI composer widgets are a **current technology
  preference**, not a reason to move editor state into Rust.
- Desktop `localStorage` draft JSON is **platform local persistence** of
  editor implementation state. It is the accepted FR-7.4-009 “drafts remain
  local” behavior, not a missing Core schema.
- iOS `replyTarget` is **platform observation** of which already-projected
  timeline row the user tapped. It is not a second event loader.

Earliest actual divergence is presenter *consumption*, not competing draft
authority. Desktop writes reply identity through Core and shows a Core
banner; `RoomInput` still reads an unwired Jotai atom for `replyTo` on send.
iOS never writes the Core reply-draft family and instead passes the tapped
row’s event id into Core send. Both send paths still go through Core
`reply_to` validation. Neither client writes composer bodies to Matrix
account data.

The CENSUS.md note “Core reply draft + local SwiftUI composer state” remains
true as *surface* (UniFFI + local `DraftStore`). It is slightly stale as
*product invocation*: the iOS wrapper is fail-closed tested and unused by
`RoomTimelineView`. That is leftover adapter, not a missing shared owner.

## Boundary constraints

- ADR 0003: one Core for session, timeline, account data, and Matrix writes.
  Swift/JS must not become a second relation or account-data writer.
- ADR 0004 current layer map: desktop presentation owns the Slate composer;
  “a mature platform library owns editor or renderer state and crossing the
  boundary would duplicate that state.” Hard invariant 2: no UI framework in
  Core. Closed charter boundary: no Slate or Swift editor state in Rust
  absent an accepted product requirement and boundary decision.
- ADR 0005: attachment bytes and filesystem paths stay off `Core::command`.
  A hypothetical durable draft may carry handles only. No such product
  schema exists today.
- Playbook §5: P4-S9-21 (`#969`) already landed the iOS reply-draft UniFFI
  family. This census must not invent S38, start P5, or register leftover
  secret/byte commands on `Core::command`.
- Goal-graph stop conditions: P4 engine-ready remains blocked; leftover
  secret/byte commands must not cross the envelope; do not start P5. Docs-only
  memos are still allowed.
- Accepted product rule for bodies is **local, not account data**:
  [`synara-namespaces.md`](../../../../synara/docs/synara-namespaces.md),
  modernization roadmap, FR-7.4-009 / `D-PRESERVE-LOCAL` (drafts survive
  *SDK migration on this device*; they are not a cross-device sync feature).
- No ADR, no numbered program decision, and no shipped account-data type
  require crash-restored or cross-device *rich* drafts.

Behaviors that must stay platform-side: Slate/Swift typing and selection,
composer layout, reply/edit banners as chrome, autosave cadence, IME,
accessibility, and ordinary local composer bodies.

## Alternatives

1. **No ownership change (stay-put / close).** Keep Core as the only reply-
   draft command owner and the only send-time `reply_to` / `thread_root`
   validator. Keep Slate `localStorage` and iOS `DraftStore` as platform
   body stores. Do not add a durable Core draft schema. Falsified if a
   shipped client writes composer bodies through js-sdk / raw HTTP / a
   Swift Matrix client, if Core reply-draft commands are unregistered, or
   if an accepted ADR/product requirement demands cross-device rich drafts.

2. **Bounded extraction or shared fixture.** Hydrate desktop `RoomInput`
   from Core `get`, or call `SharedCoreComposerReplyDraft` from iOS
   `beginReply`. That would be presenter consumption of an *existing* owner,
   not a new Core schema. A shared fixture of reply-target previews would
   not change ownership. Falsified as *necessary for this census* because
   the shared owner already exists; missing calls are not a missing engine.
   A wire-neutral durable body schema would belong here only after a real
   persistence/cross-device requirement. No such accepted requirement was
   found.

3. **Broader Core model** (Core-owned Slate/Swift trees, Core-owned typing
   / selection, or Matrix account-data sync of rich drafts). Would fight
   ADR 0004, the workstream rule against moving editor state, envelope size
   / schema churn, and the shipped “drafts remain local” product rule.
   Falsified only by a new accepted product and boundary decision that rich
   drafts must be identical and durable across devices.

Strongest stay-put case: the leftover Jotai reply atom and the unused iOS
UniFFI wrapper look like dual ownership if adapters are mistaken for
engines. Product writes of reply *identity* on desktop already go through
Core. Product iOS reply identity is a row-tap projection plus Core send
validation. Composer bodies are local by documented product rule. Treating
those leftovers as a missing shared owner would invent work the prior told
this lane not to assume.

## Recommendation

**Already correctly owned.**

The prior split holds. Core already owns reply/thread metadata via the
registered composer reply-draft family. Ordinary composer bodies, Slate/Swift
editor state, typing, and selection stay platform-owned. There is **no**
accepted product requirement for crash-restored or cross-device *rich*
drafts, so a wire-neutral durable Core draft schema is not justified.

Confidence: high for ownership and the absence of a cross-device/rich-draft
requirement; medium for live send-time desktop reply attachment (see
unresolved presenter wiring below). That wiring gap is not a second engine
and does not reopen extraction.

Supporting evidence:

- Three Core commands are registered, fail-closed without a session, and
  backed by `p4_s9_composer_draft.rs`.
- The Core DTO is target preview + thread root, not a composer document.
- Desktop native timeline set/get/clear those commands.
- Desktop body drafts are localStorage Slate JSON, namespaced as local-only.
- iOS body drafts are an in-memory `DraftStore`; wipe is local.
- No `in.synara.*` account-data draft type exists.
- FR-7.4-009 / `D-PRESERVE-LOCAL` preserve *local* drafts through migration;
  they do not charter Core sync.
- CENSUS.md snapshot matches the split; source only adds consumption detail.

Strongest objection: unused Jotai `IReplyDraft` plus unused iOS
`SharedCoreComposerReplyDraft` could be mistaken for residual dual
ownership, and desktop `RoomInput` still reads the empty Jotai atom for
`replyTo`. Those are leftover presenter seams, not a second persistence or
protocol owner.

Confirmed product defect and unresolved parity questions:

- `RoomInput` does not call `getNativeComposerReplyDraft` before send and reads
  the separate Jotai `roomIdToReplyDraftAtomFamily`, for which no shipped
  writer was found. A reply banner set by `NativeTimelinePresenter` can
  therefore be visible while the send path omits `replyTo`. That is a real
  relation-integrity defect in presenter consumption, not a Core extraction.
- iOS product UI does not invoke `composer_set_reply_draft`. Wiring it
  would be presenter consumption of the existing owner.
- iOS drafts do not survive process death; desktop Slate drafts do, locally.
  That asymmetry is accepted platform persistence, not competing authority.
- `composer.rs` still mentions a “legacy presenter”; JS `RoomTimeline` is
  gone. Comment-only stale wording.
- The V-TIMELINE contract line that `RoomTimeline`/`RoomInput` consume the
  reply-draft owner is stale for `RoomTimeline` and only half-true for
  `RoomInput` (clear, not set/get).

Regression proof to keep the boundary stable:

- Core: the three `matrix_composer_*_reply_draft` names stay registered;
  no-session fail-closed; set still validates event/room; registry remains
  in-memory; send still takes explicit `reply_to` rather than silently
  serializing editor trees.
- Desktop: `drafts.ts` remains localStorage-only; no new
  `setAccountData` / account-data event for `in.synara.room_draft`;
  `NativeTimelinePresenter` remains the product set/get caller.
- iOS: `DraftStore` stays process-local; `AppEnvironment.live()` does not
  grow a second draft writer; `Mock` / UI-test drafts stay off the live
  Core path.
- No Slate JSON, Swift attributed storage, selection ranges, or attachment
  bytes appear on `Core::command`.

## Next gate

The ownership split is closed: do not move Slate/Swift editor state into Rust
or invent a durable Core draft schema without a separate product requirement.
The desktop reply defect is reopened as
[A1](../program/ACTIONS.md#a1--reply-relation-integrity). Establish one
authoritative reply target for banner and send, cover text, attachment, poll,
GIF, cancellation, send-clear, and reply-in-thread paths, and prove the Matrix
relation rather than only the UI state. iOS consumption of the existing reply-
draft owner remains parity work and must not create another identity source.
