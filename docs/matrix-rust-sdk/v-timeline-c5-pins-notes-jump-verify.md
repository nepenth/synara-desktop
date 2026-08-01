# V-TIMELINE.C5 — pins / notes / jump live proof after cutover

| Field  | Value |
| ------ | ----- |
| Status | Docs-only verification checklist — **no product code** |
| Scope  | `synara/src/app/features/room/NativeTimelinePresenter.tsx` (pin/unpin, jump-to-latest, Later save), `nativeTimelineAction.ts` (`pinWithNativeTimelineAction`, `unpinWithNativeTimelineAction`), `nativeLaterOwner.ts` (`upsertLaterWithNativeOwner`, `createLaterItemFromIds`), `nativeRoomNotesOwner.ts` (`matrix_room_notes_*`) |
| Precondition | C1 (#285) selects `NativeTimelinePresenter` in `RoomView`; C2 (#289) deletes `RoomTimeline` + dead JS timeline path; C3 (#294) stream/delta checklist exists; C4 media/render checklist exists |
| Policy | [full-vertical-policy.md](full-vertical-policy.md); [cutover-operating-model.md](cutover-operating-model.md); dual_backend **false**; **never touch #39** |
| Related | [v-timeline-cutover-residual.md](v-timeline-cutover-residual.md) (C5 row), [v-timeline-full-replacement-contract.md](v-timeline-full-replacement-contract.md), [v-timeline-c3-stream-verify.md](v-timeline-c3-stream-verify.md) |

## 1. What C5 must prove

C5 is the **live authenticated proof** for pin/unpin, Later/notes, and
jump-to-latest on the selected `NativeTimelinePresenter`. It is a
**verification gate**, not a new implementation. The residual map records
"wired on the selected presenter; live authenticated proof unclaimed"; C5
closes that unclaimed live-proof half of the row.

The binding contract (from `NativeTimelinePresenter.tsx`, `nativeTimelineAction.ts`,
`nativeLaterOwner.ts`, `nativeRoomNotesOwner.ts`, and the contract doc) that must
be proven end-to-end on the selected path:

| # | Contract point | Implementation | Proof target |
| -- | -------------- | -------------- | ------------ |
| P1 | Pin/unpin is **capability-gated** and driven by projected `pinnedEventIds` | `NativeTimelineRowActions` gates Pin vs Unpin via `selectNativeTimelinePinAction(Boolean(pinned))`; `pinned` from `isNativeTimelineEventPinned(pinnedEventIds, eventId)` | Pin affordance appears only when `capabilities.pin`; label flips Pin↔Unpin from the projected pin list |
| P2 | Pin/unpin routes through native owners, never JS `sendStateEvent` | `pinWithNativeTimelineAction` / `unpinWithNativeTimelineAction` → `matrix_timeline_pin` / `matrix_timeline_unpin` | Pin and unpin mutate via native commands; no JS pin writer on the selected path |
| P3 | Pin-list changes project live on the stream (metadata-only batch) | `applyNativeTimelineViewDelta` accepts `pinnedEventIds` metadata-only batches; `snapshot.pinnedEventIds` drives the "Pinned" badge | A pin/unpin from another client updates the badge without a row op or revision gap |
| P4 | Later save is a room-event affordance for any remote item with an id | `NativeTimelineRowActions` "Save for later" → `upsertLaterWithNativeOwner(createLaterItemFromIds(roomId, eventId, 'saved'))` | Saving a row writes `in.synara.later` account data via `matrix_later_upsert`; no JS `setAccountData` |
| P5 | Later/notes read/write routes through native account-data owners | `nativeLaterOwner.ts` (`matrix_later_*`), `nativeRoomNotesOwner.ts` (`matrix_room_notes_*`) | Later and room-notes snapshot/mutate commands return typed readbacks; JS `setAccountData` writers deleted |
| P6 | Jump-to-latest is stream-addressed: closes prior stream, returns fresh live-bottom readback | `controller.jumpLatest()` → `matrix_timeline_jump_latest`; swaps `streamIdRef`/snapshot | Jump from a scrolled-up position returns a fresh live-bottom readback with a **new** `streamId`; prior stream closed |
| P7 | Jump affordance is position-gated | Presenter shows "Jump to latest" only when `snapshot.position.kind !== 'live_bottom'` | Button hidden at live bottom; shown when scrolled up / focused / restored / unread |

## 2. Existing tests (already green on tip)

| Test file | Covers | C5 gap |
| --------- | ------ | ------ |
| `__tests__/nativeTimelineActions.test.ts` | `pinWithNativeTimelineOwner` / `unpinWithNativeTimelineOwner` typed readbacks + off-desktop unavailability; `selectNativeTimelinePinAction` | **Unit-level only.** No live pin-list projection, no real `matrix_timeline_pin`/`unpin` IPC, no authenticated session |
| `__tests__/nativeTimelineViewDelta.test.ts` | Pure `applyNativeTimelineViewDelta`: pin-list metadata-only batches, empty-batch rejection | Pure reducer; no live stream, no real pin projection |
| `__tests__/nativeTimelineViewportPolicy.test.ts` | `shouldRestoreNativeTimelineViewport` TTL / unread / live-tail precedence | Pure policy; no jump-to-latest stream lifecycle |

**What the unit tests do NOT prove** (the C5 live gap): live pin/unpin
projection across clients, native pin/unpin mutation round-trip, Later save
writing `in.synara.later` account data, room-notes read/write round-trip,
jump-to-latest stream-addressed re-open with a new `streamId` and prior-stream
close, and the position-gated jump affordance — all require an authenticated
desktop session.

## 3. Suggested live proof steps (authenticated desktop, after C1/C2/C3/C4)

Run against the sole desktop user on a real homeserver (Synapse topology per
[test-matrix-synapse-topology.md](test-matrix-synapse-topology.md)). Record each
step with a timestamp + observed state.

1. **Pin.** From the row menu, pin a message. Assert `matrix_timeline_pin` fires
   (no JS `sendStateEvent`), the "Pinned" badge appears, and the pin persists
   across a room re-open.
2. **Unpin.** Unpin the same message. Assert `matrix_timeline_unpin` fires, the
   badge clears, and the pin list updates.
3. **Live pin projection.** From a second client, pin/unpin a message. Assert the
   badge updates via a metadata-only `pinnedEventIds` batch (no row op, no
   revision gap) on the selected presenter.
4. **Pin capability gate.** In a room where pinning is not permitted, assert the
   Pin affordance is absent (gated by `capabilities.pin`).
5. **Later save.** From the row menu, "Save for later". Assert
   `matrix_later_upsert` writes `in.synara.later` account data with the
   `{roomId, eventId}` id; the item appears in the Later panel.
6. **Room notes.** Create/update/delete a room note and a todo. Assert
   `matrix_room_notes_*` snapshot/mutate commands return typed readbacks and the
   panel reflects the change; no JS `setAccountData` writer is used.
7. **Jump-to-latest.** Scroll up (or open focused/unread), then jump. Assert a
   fresh live-bottom readback with a **new** `streamId` and the prior stream
   closed (no further deltas on the old id).
8. **Jump affordance gate.** At live bottom, assert the "Jump to latest" button
   is hidden; after scrolling up / focused open, assert it appears.
9. **Unmount close.** Navigate away from the room. Assert `matrix_timeline_close`
   fires with the exact `streamId` and no late `setState` warnings.

## 4. Fail-closed rules (non-negotiable)

- **No JS fallback.** Pin/unpin, Later/notes, and jump-to-latest must route
  through native owners — never a JS `sendStateEvent`, `setAccountData`, or JS
  timeline fetch, never a dual-backend flag.
- **Capability-gated affordances.** Pin and jump appear only when the native
  capability/position gate allows; no un-gated affordance.
- **Exact stream only.** Jump-to-latest swaps to the exact returned `streamId`;
  the prior stream is closed and never receives further deltas.
- **Monotonic revision.** A pin-list metadata batch with a revision gap is
  rejected; the hook errors rather than guessing.
- **Close on unmount.** Every opened stream is closed with its exact `streamId`;
  `disposed` guards all async callbacks.
- **No product code in this PR.** This doc only; C5 verification is a live
  proof gate, not a code change.

## 5. Done when

- C1 (#285), C2 (#289), C3 (#294), and C4 are merged and `NativeTimelinePresenter`
  is the sole active timeline owner.
- Steps 1–9 above pass on an authenticated desktop session.
- Pin/unpin, Later/notes, and jump-to-latest all route through native owners with
  no JS fallback reachable on the selected path.
- The C5 row in [v-timeline-cutover-residual.md](v-timeline-cutover-residual.md)
  is updated to "verified" with the live-proof evidence recorded.

## 6. Self-eval confidence

- **High** on the binding contract points (P1–P7) — read directly from
  `NativeTimelinePresenter.tsx`, `nativeTimelineAction.ts`, `nativeLaterOwner.ts`,
  `nativeRoomNotesOwner.ts`, and the contract doc; unit tests already cover the
  pure pin/later/notes owners and the pin-list delta reducer.
- **Medium** on live proof — steps require an authenticated session and the
  C1/C2/C3/C4 cutover to be merged first; this doc frames the gate, it does not
  claim the proof.
