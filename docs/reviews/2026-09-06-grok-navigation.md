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
