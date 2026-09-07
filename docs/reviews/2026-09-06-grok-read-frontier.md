# Grok 4.6 High source review: read-frontier

Read-only review; runtime evidence is separate. Session `01a07959-bb46-7353-bd3d-d14c4a28d0cb`.

I'll pick up from the remaining Core write path, iOS merge/mark-read, and follow-live pairing so this review can actually close.## Verdict: **ACCEPT**

The paired-frontier contract matches the reported failure (auto-read stuck when `latest_event_id` is a folded edit/reaction while the painted row ID is unchanged). Core still owns the Matrix write and keeps the exact-tail guard. Clients only submit a receipt token when their rendered remote tail equals Core’s `visibleTailEventId`, and they dedup on the transport identity. I did not treat the earlier reviewer’s debounce approval as given; the current immutable `(visible, receipt)` queue drain is consistent.

No additional broken invariant beyond the stated restore-navigation dependency (existing hidden `m.fully_read` targets must resolve to a chronological visible anchor in a later change).

---

### What holds

**Race / pairing.** `project_live_read_state` samples `latest_event_id()` before `items()`. A new message between those reads yields `visible=new, receipt=old`. Clients reject that pair if they have not painted the new row; if they have, Core’s exact-tail write no-ops against the newer SDK tail. The reverse order would have authorized an unseen message.

SDK `latest_event_id` reads `all_remote_events` under the same controller state as items, including folded aggregates. Same-snapshot `receipt != visible` is the intended edit/reaction case, not a new-message leak.

**Row / read-state ordering.** Live item diffs now emit ops and `read_state` together. Desktop `applyNativeTimelineViewDelta` applies ops, then metadata, in one snapshot. A metadata-only `room_info` batch with `visible=new` while rows are still old fails the client match. Folded activity with unchanged rows is authorized only when rendered tail still equals `visibleTailEventId`.

**Stream filtering.** Desktop still takes the unfiltered snapshot tail (`latestNativeReadEventId` on `snapshot.rows`), which is what Core’s last remote event item is. Presentation filters (`hideMembershipEvents`) are not used as the read observation. HideActivity is unchanged.

**Local echo.** Core skips `is_local_echo()`. iOS skips `serverEventID == nil` pending rows. Desktop skips rows without a `$` event id. A pending echo at the end does not attach a mismatched receipt token; auto-read stays fail-closed until the remote echo lands.

**Hidden edit/reaction after prior ack.** Dedup keys on `receiptTailEventId` / `readReceiptEventID`, not the painted message id. A later fold re-arms. Core still no-ops a stale token after a newer live event.

**Lifecycle.** iOS cancel/generation still bumps `markFullyReadTaskGeneration` and clears the queue on jump/reset/drag. Desktop increments `liveTailMarkGenerationRef` on room change and keys attempts per receipt identity. Focused providers cannot mark: Core `set_read_state` requires `LiveBottom`; follow-live rejects non-live providers; iOS `shouldSchedule` requires `timelineProviderIsLive`.

**Swift wakeup / pairing.** Token is attached only when the last remote row’s `serverEventID` equals `visibleTailEventId`. `TimelineItem` equality includes `readReceiptEventID`; `withDeliveryStatus` / `withSenderAvatarURL` copy it. A token-only change is a real row-value change, so the stable viewport applies and `reportBottomPinnedIfChanged(force: true)` runs. `scheduleMarkFullyRead` then uses the attached receipt id. The debounce queue stores one pair; the in-flight task drains that pair and cannot pick up a later observation’s visible id.

**FFI / schema.** New fields are optional (`serde(default)`, UDL `string?`). `TIMELINE_VIEW_SCHEMA_VERSION` stays 1. Rust DTO, UDL, and both clients agree on names.

**Tests (source).** The mock-server case uses the SDK’s real `EventFactory.edit` / `reaction` folding shape, asserts visible tail stays the original message, asserts the fold is not a displayed message row, writes the receipt id, and rejects the old token after a newer message. HTTP bodies check private `m.fully_read` / `m.read.private` only. TS tests cover fold identity changes and metadata-before-row rejection. Swift tests cover token-only reconfigure, copy preservation, and A→B queue replacement.

---

### Code defects

None that break the stated invariants.

---

### Verification limits (not defects)

These were not executed here and remain pending:

- UniFFI Swift regeneration / native compile
- `p4_s38` mock-server run
- iOS unit tests and signed simulator
- `SYNARA_LIVE_READ_SMOKE` (edit + reaction markers, background/offscreen negatives)

Do not treat this source ACCEPT as live-path proof. The user-reported room still needs that evidence.
