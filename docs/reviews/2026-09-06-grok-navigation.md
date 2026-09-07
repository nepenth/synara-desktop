# Grok4.6 High source review

I'll start by reading the review skill and the full prompt so the source review follows the contract exactly.This is a read-only source review of the named head, not a skill-orchestrated PR review. I’ll read the review doc and the changed sources next.I’ll read the Core, presenter, and iOS navigation paths in parallel so remaining invariants can be judged from the actual code.**REQUEST_CHANGES**

The three previously rejected defects are present in this head as described: one recovery overlay per iOS viewport (`RoomTimelineView.swift` 833 and 939), latest placement only after adopted jump + unchanged navigation input (`nativeTimelineView.ts` 753–769, `NativeTimelinePresenter.tsx` 2416–2435), and shared recovery stacks (`NativeTimelinePresenter.tsx` 2544–2572, `RoomTimelineView.swift` 2618–2635). Core last-read mapping, hidden-edit predecessor tests, privacy/read write gates, send-vs-edit, and iOS missing-marker pinning look consistent with the contract on source inspection.

Desktop missing-marker placement and live-follow still fail the user contract.

---

## P1

### 1. Missing last-read lands at the oldest live row instead of keeping the current viewport

- **File/line:** `synara/src/app/features/room/NativeTimelinePresenter.tsx:2346-2363`
- **Route/scenario:** Room entry (or re-entry) whose Core position is `unread` and `anchor_event_id` is not in the live rows (`missing` / long live window). Core keeps the live provider (`live.rs` 1441–1454, 2537–2558).
- **Failing invariant:** Missing last-read must retain the current viewport and expose Jump to Last Read. On first entry that viewport is the live tail; on re-entry it is the saved event/pixel hint. iOS matches this (`RoomTimelineView.swift` 1617–1654, UITest expects `pinned=true` and the latest synthetic row). This branch only scrolls for `live_bottom` or a found anchor, so a missing unread target leaves the virtualizer at index 0 (oldest live row), not the live tail or saved hint. That is an invented location, not last-read and not “current.”
- **Repair:** When `selectedPosition.kind === 'unread'` and `anchorIndex < 0`, place as live bottom if there is no saved non-bottom anchor, otherwise restore `savedViewport.anchor` / offset. Keep `pendingLastRead` set. Do not treat the unread id as a scroll target until the user taps Jump to Last Read.

### 2. Follow-live promotion drops Jump to Last Read

- **File/line:** `synara/src/app/features/room/NativeTimelinePresenter.tsx:2341-2345` with `2168-2176`; `synara/src/app/features/room/nativeTimelineView.ts:734-735`
- **Route/scenario:** Sparse or short live window with an unavailable last-read (`sparse-missing`: one event or a list that fits). Geometry reports `atLiveBottom`, `nativeFollowLiveTarget` accepts any non-`live_bottom` kind, and Core `follow_live_tail` succeeds because this stream is the live provider at the SDK tail (`live.rs` 1841–1862).
- **Failing invariant:** A missing/loading marker must keep Jump to Last Read until an explicit jump; later promotion/sync must not take that control away. `setPendingLastRead` runs on every new `placementKey`. Follow-live flips `selectedPosition` to `live_bottom`, so the next layout writes `pendingLastRead` to `undefined`. The Chromium `sparse-missing` case cannot see this: the harness throws on `matrix_timeline_follow_live`.
- **Repair:** Store the unavailable target until Jump to Last Read succeeds or the user jumps to latest. Follow-live may flip provider kind for down-arrow / live follow; it must not clear that target. Skip follow-live while a pending last-read exists only if you cannot keep the button after promotion — keeping the button is what the contract requires, including while following the live tail.

P1.1 and P1.2 are coupled: placing missing last-read at the live tail (the P1.1 repair, and the sparse case today) makes follow-live succeed and then P1.2 removes the button.

### 3. Pointer-down at the live tail stops following new messages

- **File/line:** `synara/src/app/features/room/NativeTimelinePresenter.tsx:2290-2294`
- **Route/scenario:** Active focused room, `live_bottom`, painted at the real tail. User pointer-downs a row (react, copy, open actions) without leaving the 8px bottom band. A live append then arrives.
- **Failing invariant:** At actual latest bottom, follow all new messages. `onUserInput` always sets `followingLiveRef.current = false`. `onScroll` would recompute follow from geometry, but pointer-down does not scroll. `observeNativeTimelineBottom` only measures on scroll/resize/childList, so follow stays off; the next append grows `scrollHeight` and the user is no longer at bottom. Wheel that stays inside the band is recovered by `onScroll`; click is not. iOS resumes follow on interaction end if still pinned (`RoomTimelineView.swift` 2702–2720).
- **Repair:** User input should only clear `programmaticScrollUntilRef` (and set `userInitiatedScrollRef`). Drive `followingLiveRef` from `live_bottom && atBottom` on scroll and pointer-up/cancel, not unconditionally on pointer-down.

---

## P2

### 4. Jump to Last Read is removed before the focused open succeeds

- **File/line:** `synara/src/app/features/room/NativeTimelinePresenter.tsx:2562-2567`
- **Route/scenario:** User taps Jump to Last Read; focused open fails, is superseded, or the marker is still absent.
- **Failing invariant:** Missing last-read must remain an explicit control until the location is actually restored. This click sets `pendingLastRead` to `undefined` immediately. iOS keeps the marker unless the returned feed contains the anchor (`RoomTimelineView.swift` 2638–2651, 2775–2776).
- **Repair:** Clear `pendingLastRead` only after the focused/unread window contains the target. On failure, leave the button and the current rows.

---

## Independently confirmed (not defects)

- Hidden edit of an old row maps to the chronological visible predecessor in Core (`live.rs` 2562–2632; `p4_s38_timeline_follow_live.rs` 435–487). iOS reads that Core `unreadAnchorEventId` (`SharedCoreProductServices.swift` 2656–2661).
- Own send requests latest after native send success; edits use a separate owner and do not (`RoomInput.tsx` 659, 691 vs `editTextWithNativeTimelineAction`). iOS `performSend` returns before jump when `editEventID != nil` (`RoomTimelineView.swift` 2002–2020). Other-room send is room-scoped (`nativeTimelineNavigation.ts` 6–8).
- Sparse open requests one bounded backwards page and keeps Available on timeout/failure (`live.rs` 1346–1368, `PAGINATION_BATCH_SIZE` 30).
- Automatic receipt writes still require active document, live provider, painted tail, and `canMarkRead` (`nativeTimelineViewportPolicy.ts` 101–114). `SharedCoreReadMarkers` still rejects non-server ids.

---

## Verification limits (not treated as code defects)

- No Shell: HEAD `c4379c1f` was not re-hashed; this is the worktree source.
- Author Core 43 / SDK 9 / Chromium 7 / frontend 954 were not re-run.
- Signed iOS compile and UI execution are still unconfirmed; iOS findings above are source-only.
- Harness IPC is mocked. `sparse-missing` forces follow-live to throw, so it does not prove P1.2. This is source review, not runtime proof.


## Follow-up review of ddda4f3f

Grok 4.6 High, session `01a0797b-e7a0-7ef2-88d7-5d2fde5c9110`; verdict REQUEST_CHANGES. Exact source head reviewed: `ddda4f3f4bb2dc3a585255d0761e89a1cfd47a5d`.

I'll read the full prompt and then inspect the current-head stream-ownership and last-read paths directly.I’ll read the controller buffer, restore, and presenter navigation paths next so stream ownership can be checked against the contract.**REQUEST_CHANGES**

The four prior desktop defects are fixed in this head: missing last-read uses live tail or a saved event/pixel (`NativeTimelinePresenter.tsx` 2347–2375), `pendingLastRead` survives follow-live and room-local snapshots (2359–2360, 2053), pointer-down no longer drops follow (2293–2298), and `restoreLastRead` keeps the current provider until a focused snapshot contains the target, closing only unused returned streams (`nativeTimelineView.ts` 916–986). Candidate replay is bounded (64 batches / 2048 items), scoped by room and known generation, fail-closed on gap/overflow/removed target, and released on adopt/fail/supersede/cleanup.

One remaining stream-ownership hole can still replace a healthy adopted view.

---

## P1

### Paginate treats a lagging same-stream snapshot as a fatal desync

- **File/line:** `synara/src/app/features/room/nativeTimelineView.ts:739-746`
- **Route/scenario:** Adopted live (or unread) stream; user loads older messages; a live delta is applied in JS (revision N+1) before `matrix_timeline_paginate` resolves with the snapshot Core copied at revision N. Same room, same `streamId`, navigation revision unchanged, so `superseded()` is false.
- **Failing invariant:** An adopted provider must not be discarded because an in-flight command readback is older than a delta already applied on that stream. `setReadState` already documents this race (`isNativeTimelineReadbackStale`, 293–307, 791–798) and ignores the lagging readback. Paginate does not: any `acceptSnapshot` failure, including `next.revision < current.revision`, calls `setState({ status: 'error' })` and tears down the working rows. The Chromium delayed-paginate cases only cover an **old** stream after last-read adoption, not this same-stream lag.
- **Repair:** After the superseded check, if `!result.available || !result.value`, throw to the presenter action error **without** replacing ready state, or keep the current snapshot. If `isNativeTimelineReadbackStale(snapshotRef.current, result.value)`, return (leave current rows; pagination can be retried). Call `setState` error only when the readback is newer/same-generation and still cannot be applied.

---

## Independently confirmed (not defects)

- Buffer cannot grow without bound: overflow sets `invalid` and drops batches (`nativeTimelineView.ts` 611–614). `finishOpen` / effect cleanup / superseded paths cancel it.
- Unused streams closed are only IDs returned by this controller’s own open/jump (`881–883`, `949–951`, `1086–1089`, `1104–1106`). Adopted `streamIdRef` is closed on effect cleanup or after a successful last-read swap (`983–985`).
- Missing marker at first entry goes to live tail; re-entry uses saved history offset; Jump to Last Read stays through follow-live and appends (presenter 2353–2368; `pendingLastRead` not cleared on `live_bottom`).
- Click at the tail only clears the programmatic lock; follow is recomputed from geometry on scroll.
- Send still issues room-scoped latest; jump adoption still requires `accepted && mountedNavigationRef.current === navigation`. Explicit latest clears the recovery target by intent (`2451`).

---

## Verification limits (not code defects)

- No Shell: HEAD `ddda4f3f` was not re-hashed; this is the worktree source.
- The 33 Chromium cases, 954 modernization tests, and type/lint runs were not re-executed here.
- Core/Swift were not in this delta and were not re-reviewed.
- Signed iOS / observer `2feb` live-app results do not substitute for this desktop source review. Mocked-native browser proof does not establish live Matrix behavior.


## Pagination correction acceptance

Grok 4.6 High; exact head `7a3487622eb09078089f4e34e636accb9641afce`, session `01a0797b-e7a0-7ef2-88d7-5d2fde5c9110`. Independent agent review also accepted this correction.

I'll re-read the paginate/read/follow/poll ownership paths at this head and check for the same stale-readback failure on remaining commands.**ACCEPT**

The same-stream paginate hole is fixed in source: after stream/navigation ownership checks, an equal-or-older snapshot is dropped and the already-applied view stays (`nativeTimelineView.ts` 739–743). Unavailable pagination and read-state no longer swap ready rows for a sync error; they throw to the existing action handlers (740–741, 787–788). Incompatible non-stale snapshots still fail closed (744–749, 800–804).

Analogous commands on the adopted stream:

| Command | Lagging success | Unavailable / reject |
|---|---|---|
| Paginate | `isNativeTimelineReadbackStale` then return | throw, ready state kept |
| Read | stale ignored (791–793) | throw, ready state kept |
| Follow | `canAcceptNativeTimelineFollowReadback` requires `revision >= current` (835–837); no `setState('error')` on miss | superseded return / throw to presenter catch that keeps the stream |
| Poll | `acceptSnapshot` false is ignored (1062) | `.catch(() => undefined)` then return (1036–1045) |

`jumpLatest` / `restoreLastRead` still error or discard on a failed **replacement** open; that is a new-stream path, not a lagging snapshot on the current provider. Live subscription gaps still fail sync in `applyBatch` (1012–1018), which is the actual stream, not a stale RPC copy.

Prior last-read/follow/click repairs are unchanged in this delta.

---

**Verification limits (not defects):** HEAD `7a348762` was not re-hashed (no Shell). The 45 Chromium cases, 954 modernization tests, and type/lint runs were not re-executed here. This is Desktop source review only. It is not live Matrix, signed iOS, physical-device, or release proof. Core/Swift were not in this delta. The separate iOS upload/latest CI defect is out of scope.
