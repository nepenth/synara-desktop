# V-TIMELINE.C3 — stream/delta re-verify after cutover

| Field  | Value |
| ------ | ----- |
| Status | Docs-only verification checklist — **no product code** |
| Scope  | `synara/src/app/features/room/nativeTimelineView.ts` (`useNativeTimelineView`, `applyNativeTimelineViewDelta`), `NativeTimelinePresenter.tsx` stream consumption |
| Precondition | C1 (#285) selects `NativeTimelinePresenter` in `RoomView`; C2 (#289) deletes `RoomTimeline` + dead JS timeline path |
| Policy | [full-vertical-policy.md](full-vertical-policy.md); [cutover-operating-model.md](cutover-operating-model.md); dual_backend **false**; **never touch #39** |
| Related | [v-timeline-cutover-residual.md](v-timeline-cutover-residual.md) (C3 row), [v-timeline-full-replacement-contract.md](v-timeline-full-replacement-contract.md) |

## 1. What C3 must prove

C3 is the **live authenticated viewport proof** that the native stream/delta
binding — already implemented on the unselected presenter — stays correct once
`NativeTimelinePresenter` is the sole active timeline owner. It is a
**re-verify gate**, not a new implementation. The residual map records "no gap
found at tip"; C3 closes the unclaimed live-proof half of that row.

The binding contract (from `nativeTimelineView.ts` + contract doc) that must be
proven end-to-end on the selected path:

| # | Contract point | Implementation | Proof target |
| -- | -------------- | -------------- | ------------ |
| S1 | Register the `matrix-timeline-view-updated` listener **before** `matrix_timeline_open` | `listen(...)` runs before `invokeDesktopWithAvailability('matrix_timeline_open', …)` in `open()` | No delta is missed between subscribe and open readback |
| S2 | Keep only the **exact** `streamId` returned by the open readback | `streamIdRef.current = readback.streamId`; batches with a different `streamId` are dropped | No cross-stream / stale-stream rows leak in |
| S3 | Reject **revision gaps** and **malformed ops** instead of repairing via JS | `applyNativeTimelineViewDelta` returns `undefined` on schema/session/room/revision mismatch or invalid op; hook sets `status: 'error'` | A gap or bad op fails closed (error state), never a guessed render |
| S4 | Abort with the session timeline registry on unmount / close | cleanup calls `matrix_timeline_close` with the exact `streamId`; `disposed` guards async callbacks | No orphaned stream, no late setState after unmount |
| S5 | Buffer early batches and replay them against the open readback | `earlyBatchesRef` collects pre-open batches; replayed after `streamId` is known | No lost live rows during the open race |
| S6 | Metadata-only batches (readState / pagination / pinnedEventIds) project live frontier, pagination, and pin-list signals | `applyNativeTimelineViewDelta` accepts metadata-only batches; empty+no-metadata rejected | Live read-frontier / pin / pagination updates render without row ops |
| S7 | Jump-to-latest is stream-addressed: closes prior stream, returns fresh live-bottom readback | `jumpLatest()` invokes `matrix_timeline_jump_latest`, swaps `streamIdRef`/snapshot | Live-bottom re-open after jump; old stream closed |

## 2. Existing tests (already green on tip)

| Test file | Covers | C3 gap |
| --------- | ------ | ------ |
| `__tests__/nativeTimelineViewDelta.test.ts` | Pure `applyNativeTimelineViewDelta`: metadata-only read-frontier, pagination + pin-list metadata, empty-batch rejection, pin/forward/thread/format pure helpers | **Unit-level only.** No live stream, no open/close lifecycle, no real `matrix_timeline_*` IPC, no authenticated session |
| `__tests__/nativeTimelineViewportPolicy.test.ts` | `shouldRestoreNativeTimelineViewport` TTL / unread / live-tail precedence | Pure policy; no stream |
| `__tests__/nativeTimelineActions.test.ts` | Native action owners (edit/redact/forward/report/pin/poll/call) typed readbacks + off-desktop unavailability | Action owners, not the view stream |

**What the unit tests do NOT prove** (the C3 live gap): listener-before-open
ordering, exact-streamId retention against a real registry, revision-gap
fail-closed under live load, early-batch replay, unmount close/abort, and
metadata-only live projection — all require an authenticated desktop session.

## 3. Suggested live proof steps (authenticated desktop, after C1/C2)

Run against the sole desktop user on a real homeserver (Synapse topology per
[test-matrix-synapse-topology.md](test-matrix-synapse-topology.md)). Record each
step with a timestamp + observed state.

1. **Open live-bottom.** Log in, open a room with existing history. Assert
   `status: 'ready'`, `selectedPosition.kind === 'live_bottom'`, snapshot rows
   render, and `streamId` is stable across re-renders.
2. **Live append.** From a second client, send a text message. Assert the row
   appears via a delta `append`/`push_back` (not a full re-fetch) and `revision`
   increments by exactly 1.
3. **Live edit / reaction / redact.** Send an edit, a reaction, and a redact
   from the second client. Assert `set`/`remove` ops apply in place and the
   `revision` chain stays contiguous.
4. **Pagination.** Scroll backward until `backward: 'exhausted'`; scroll forward
   back to live. Assert `matrix_timeline_paginate` readbacks are accepted and
   `revision` stays monotonic.
5. **Read-frontier / pin metadata.** Mark read/unread and pin/unpin from the
   menu. Assert metadata-only batches project `readState` / `pinnedEventIds`
   without row ops and without a revision gap.
6. **Jump-to-latest.** Scroll up, then jump. Assert a fresh live-bottom readback
   with a **new** `streamId` and the prior stream closed (no further deltas on
   the old id).
7. **Fail-closed drill (optional, dev-only).** Force a revision gap or a
   malformed op (e.g. via a temporary harness or a stale streamId). Assert the
   hook enters `status: 'error'` and does **not** fall back to any JS timeline
   fetch. Restore by re-opening the room.
8. **Unmount close.** Navigate away from the room. Assert `matrix_timeline_close`
   fires with the exact `streamId` and no late `setState` warnings.

## 4. Fail-closed rules (non-negotiable)

- **No JS fallback.** A stream gap, malformed op, or lost sync must surface the
  native error state — never a JS timeline fetch, never a dual-backend flag.
- **Exact stream only.** Any batch whose `streamId` differs from the open
  readback is dropped; the hook never adopts an inferred stream.
- **Monotonic revision.** A batch with `revision !== snapshot.revision + 1` is
  rejected; the hook errors rather than guessing.
- **Schema/session/room lock.** A batch with a different `schemaVersion`,
  `sessionGeneration`, or `roomId` is rejected.
- **Close on unmount.** Every opened stream is closed with its exact `streamId`;
  `disposed` guards all async callbacks.
- **No product code in this PR.** This doc only; C3 verification is a live
  proof gate, not a code change.

## 5. Done when

- C1 (#285) and C2 (#289) are merged and `NativeTimelinePresenter` is the sole
  active timeline owner.
- Steps 1–6 and 8 above pass on an authenticated desktop session; step 7 is
  demonstrated at least once (dev harness acceptable).
- No JS timeline fallback is reachable on the selected path; `RoomTimeline` is
  deleted.
- The C3 row in [v-timeline-cutover-residual.md](v-timeline-cutover-residual.md)
  is updated to "verified" with the live-proof evidence recorded.

## 6. Self-eval confidence

- **High** on the binding contract points (S1–S7) — read directly from
  `nativeTimelineView.ts` and the contract doc; unit tests already cover the
  pure delta reducer.
- **Medium** on live proof — steps require an authenticated session and the
  C1/C2 cutover to be merged first; this doc frames the gate, it does not claim
  the proof.
