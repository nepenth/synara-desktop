# ROE-03 Research Memo: Timeline rows and event relationships

Status: draft research; docs-only; not approved for implementation.

| Field              | Value                                                                                                                                                                                                                                                                                                                                 |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Workstream/cluster | ROE-03 (residual engine census; message-format cluster only as a boundary, not ROE-04/12)                                                                                                                                                                                                                                              |
| Research owner     | Residual-census researcher (this memo)                                                                                                                                                                                                                                                                                                |
| Reviewers          | Unassigned                                                                                                                                                                                                                                                                                                                            |
| Source census      | 2026-09-01 on worktree `roe/memo-03-timeline-rows` at `53d7b2c4968df0b9c9adb031ebfa46eab7685242`. [CENSUS.md](../program/CENSUS.md) is a `main` `011cf39a` snapshot only; source below was re-read on this commit.                                                                                                                      |
| ADR baseline       | [ADR 0001](../../../adr/0001-ios-repository-layout.md), [0002](../../../adr/0002-ios-architecture.md), [0003](../../../adr/0003-shared-native-rust-core.md), [0004](../../../adr/0004-rust-language-boundaries.md), [0005](../../../adr/0005-native-media-handle-channel.md); last reviewed 2026-09-01; source commit as above. |

## Observable problem

Users see one ordered room history on desktop and iOS: messages, edits, replies, thread chips, reactions, redactions, local sends, older pages, and late-decrypting events. The portfolio prior is that `TimelineViewRow` is already the event/row semantic model. The residual question is whether any of those relationships still produce a **concrete cross-client semantic gap the current row model cannot express**.

This memo does not ask whether desktop and iOS *look* the same. Scrolling, virtualization, visual grouping, typography, and selection stay platform observation/rendering. It does not propose a second semantic-row layer.

## Current ownership census

Re-verified against current source. Where this table disagrees with [CENSUS.md](../program/CENSUS.md), **source wins**. The snapshot still correctly names `app/timeline/live.rs`, `view.rs` (`TimelineViewRow`), desktop `nativeTimelineViewportPolicy.ts` / `nativeTimelineRichText.ts`, and iOS `SharedCoreTimeline*` plus `MatrixHTMLRenderer`. It does not record UniFFI flattening of the row, SDK `thread_root` on in-thread children, or the iOS pending-send overlay.

Live room open on both clients consumes the same Core owner: `NativeTimelineOwner` / `NativeTimelineRegistry` projects SDK items with `project_timeline_item_with_media` into `TimelineViewRow`, then emits `TimelineViewSnapshot` / `TimelineViewDeltaOp` (exact `VectorDiff` variants). Desktop Tauri JSON carries the full row. iOS UniFFI `TimelineViewRowDto` is a privacy-safe flattening of that row, not a second model.

| Concern | Rust/Core | Desktop | iOS | Evidence/tests |
| ------- | --------- | ------- | --- | -------------- |
| Ordered event/row identity | Authority. SDK `unique_id` → `item_id`. Remote `event_id` optional; absent only for a local echo that has not received a server id. Deltas are `append` / `set` / `insert` / `remove` / `reset` / … so presenters never infer a missing mutation. | Projection. `NativeTimelineViewRow` mirrors the JSON row; `applyNativeTimelineViewDelta` rejects schema/revision gaps instead of repairing from JS events. Live room is `RoomView` → `NativeTimelinePresenter` → `useNativeTimelineView`. | Projection. `SharedCoreTimelineService` opens/paginates/snapshots the same owner. `SharedCoreTimelineRows.item(from:)` maps DTO → `TimelineItem` (`id` = `itemId`). | `crates/synara-core/src/app/timeline/view.rs` `TimelineEventRowBase`, `project_timeline_diffs`; `nativeTimelineView.ts`; `SharedCoreProductServices.swift` `SharedCoreTimelineService`; `AppEnvironment.swift` live wiring |
| Edits | Authority. `message.is_edited()` → `TimelineMessageRow.edited`. Edit writes are `edit_message` / `timeline_edit_text` on the same owner. | Rendering. Edited chip from `row.edited`. No JS `m.replace` walk on the live native path. | Rendering. `isEdited: row.edited`. | `view.rs` `project_event_row_for_user`; `NativeTimelinePresenter.tsx`; `SharedCoreTimelineRows.swift`; `p4_s16_timeline_rows.rs` |
| Replies | Authority. SDK `in_reply_to` → `TimelineReplyPreview` (`event_id`, optional sender, body). Ready parent uses message body or `"Original message"`. `Unavailable` / `Pending` / `Error` still emit the parent id with body `"Jump to original"`. | Rendering. Reply surface uses `row.reply` and focuses `row.reply.eventId`. | Projection + local snippet. DTO keeps `reply_to_event_id` only. `TimelineReplyPreview` rebuilds a snippet from **already-loaded** `TimelineItem`s when the parent is in the window. | `view.rs` `project_reply`; `nativeTimelineView.ts` `NativeTimelineReplyPreview`; `TimelineReplyPreview.swift`; `TimelineReplyPreviewTests.swift` |
| Threads | Authority on **roots** that the SDK gives a `thread_summary`: `TimelineThreadSummary` (`root_event_id`, `reply_count`, `latest_event_id`; local-echo latest ids omitted). Send already accepts `thread_root` separately from `reply_to` (`Relation::Thread`). Live `TimelineBuilder` default is `TimelineFocus::Live { hide_threaded_events: false }` (matrix-sdk-ui 0.18). SDK `MsgLikeContent.thread_root` on **in-thread children** is **not** copied onto `TimelineViewRow`. | Rendering of the existing root field. Thread chip from `row.thread`; focus helper `nativeThreadFocusEventId`. Composer send forwards `threadRoot` + `replyTo`. | Adapter thinning + presenter grouping. UniFFI DTO has **no** thread field. Product `loadThreadTimeline` default focuses the root event in the room stream. Fixture `TimelineService.threadTimelineItems` keeps items whose `replyToEventID == root` (misses nested in-thread replies). Room composer `SharedCoreMessageSendService` currently passes `threadRoot: nil`; thread composer attachments can pass `threadRootEventID`. | `view.rs` `project_thread_summary`; `app/send/text.rs` `message_content`; `nativeTimelineView.ts`; `TimelineService.swift`; `SharedCoreProductServices.swift`; SDK `MsgLikeContent` in matrix-sdk-ui 0.18 |
| Reactions | Authority. Aggregated `key` / `count` / optional `own` from SDK `MsgLikeContent.reactions`. Writes: `reaction_ensure` / `reaction_redact` / `timeline_reaction_toggle`. Message rows carry the list; `Sticker` / `Poll` variants do not. | Rendering. Reaction chips from `row.reactions`. | Projection. Counts only (`[String: Int]`); `own` is dropped in the mapper. | `view.rs` `project_reactions`; `NativeTimelinePresenter.tsx`; `SharedCoreTimelineRows.reactionCounts`; `p4_s9_timeline_reactions.rs` |
| Redactions | Authority. `MsgLikeKind::Redacted` + remote id → `TimelineViewRow::Redacted` (`item_id`, `event_id`, summary `"Message removed"`). Local redacted echo without an id is `Other` (`"Redacted local event"`). | Rendering. `kind: 'redacted'`. | Rendering. `.redacted`. | `view.rs` `project_event_row_for_user`; `SharedCoreTimelineRows.displayKind`; `TimelineServiceTests.swift` |
| Local echoes | Authority. `event_id` absent until the server id arrives; item id remains the stable key. Capabilities that need a remote id stay false. | Projection of the Core row (no separate pending-message list on the live presenter). | **Platform send UX**, not a second protocol owner. `TimelineItem.pendingMessage` / `OutgoingSendService` overlay `$pending-*` rows; `TimelinePendingReconciler.merge` inserts unmatched pending by timestamp and never reorders server items. Empty DTO `eventId` is remapped to `itemId`. | `view.rs` `TimelineEventRowBase`; `TimelineService.swift` `TimelinePendingReconciler`; `OutgoingSendService.swift` |
| Pagination overlap | Authority. SDK `paginate_backwards` / `forwards`; rows arrive as diffs or a replacement snapshot; `pagination.backward` / `forward` are `available` / `exhausted` / …. Revision is monotonic per stream. | Observation. Viewport policy decides *when* to ask; `applyNativeTimelineViewDelta` applies the owner’s ops. | Observation + window cap. `loadOlderTimeline` paginates until new item ids appear or backward is exhausted. `TimelineWindowPolicy` dedups by event/item id and caps the presenter window (300). That is display bounding, not a second history. | `live.rs` `paginate`; `pagination.rs`; `nativeTimelineViewportPolicy.ts`; `SharedCoreProductServices.swift`; `SynaraCoreBindingsTests.swift` pagination bounds |
| Late decryption | Authority. `MsgLikeKind::UnableToDecrypt` → `EncryptedUnavailable` (`reason_code: "unable_to_decrypt"`). Later SDK `Set` replaces the row with the decrypted projection. Separate `UtdIndex` / recovery is owner-internal. | Rendering. `encrypted_unavailable` until the next authoritative row. | Rendering. `kind == "encrypted"` → `.encryptedPlaceholder`; `decryption_state` on the DTO. Snapshot refresh on the same stream picks up the replacement. | `view.rs`; `live.rs` `reconcile_utd`; `SharedCoreTimelineRows.swift` |
| Relation-before-parent | Authority. Same as replies: parent id is always on the preview; body is a fallback until the SDK marks the embed ready. No presenter fetch of raw event JSON. | Rendering of the Core preview. | Rendering of `replyToEventID`; snippet only if the parent is already in the loaded window. | `view.rs` `project_reply`; `TimelineReplyPreview.swift` |
| Virtual / date / read rows | Authority for SDK virtual kinds (`DateSeparator` timestamp_ms, `ReadMarker`, `TimelineStart`). Locale day labels stay presenter-side (comment on `DateSeparator`). | Rendering / grouping. `shouldGroupNativeTimelineRows`. | `SharedCoreTimelineRows` **skips** separators/markers (`nil` kind). Grouping is `TimelineMessageGroupingPolicy`. | `view.rs` `project_timeline_item`; `nativeTimelineGrouping.ts`; `SharedCoreTimelineRows.swift` |
| Formatted body (out of this lane) | Protocol field only. `project_formatted_body` copies SDK HTML when format is HTML and distinct from plain text. Struct comment on `TimelineMessageRow.formatted_body` still says “Already-sanitized rendering markup”. | Presenter sanitizes (`nativeTimelineRichText.ts`). | Presenter sanitizes (`MatrixHTMLRenderer`). | CENSUS leftover; belongs to ROE-04/12, not a row-relationship remainder |

**Classification.** Event identity, edit flag, reply parent id/preview, root thread summary, reaction aggregates, redaction kind, local-echo id absence, pagination state, and UTD-to-message replacement are **Core authority** (hard invariant: one timeline/relationship owner — ADR 0003/0004). Viewport, scroll, virtualization, date-label locale, visual grouping, text selection, and pending-send chrome are **platform observation/rendering** (accepted platform boundary). React vs SwiftUI cells are **technology preferences**.

**Earliest actual divergence.** There is no second protocol-row engine. The earliest *appearance* of one is iOS `TimelineItem` plus the pending-send overlay, and the UniFFI DTO dropping `thread` / reply preview / capabilities / poll answers that `TimelineViewRow` already has. Those are presenter projection and transport flattening. Desktop leftover `Message.tsx` / pin-menu JS event helpers are not the live `RoomView` path.

SDK `MsgLikeContent.thread_root` on in-thread children is the only protocol field this census found on the live item that `TimelineViewRow` does not project. Both clients therefore lack child-in-thread membership **equally**. That is a shared omission, not desktop and iOS inventing different thread roots. iOS `replyToEventID == root` filtering is presenter grouping (keep-closed), not a competing Core model.

Playbook §5 and the [goal-graph stop conditions](../../../shared-native-core/13-language-boundary-goal-graph.md) treat P4-S16 / S18 / S29 / S31–S33 as landed. This memo does not invent S38, start P5, or move grouping/virtualization into Core.

## Boundary constraints

- ADR 0003: one Core for timeline state and Matrix writes; Swift/TS adapters stay thin.
- ADR 0004: timeline event relationships are Core-shaped authority; native grouping, cells, scroll, selection, and typography stay platform-owned. No second Matrix engine. No UI framework in Core. No universal sanitizer claim on `formatted_body`.
- ADR 0005: media bytes stay on opaque handles / dedicated channels; not a row-relationship remainder.
- ROE-03 prior: extend `TimelineViewRow` only when a proven protocol-semantic field is missing. Do not add a parallel normalization layer.
- Workstream keep-closed: scrolling, virtualization, gesture arbitration, visual grouping, typography, selection, invalidation.
- Playbook §5 / goal graph: do not invent S38; do not start P5; docs-only PRs remain allowed.
- Message-format AST and fixture corpus are ROE-04/12. The still-misleading “already-sanitized” comment is that cluster, not a ROE-03 extract.

## Alternatives

1. **No ownership change (stay-put).** Keep `TimelineViewRow` as the only semantic row. Leave UniFFI flattening, iOS `TimelineItem` mapping, pending-send overlay, and reply-snippet reconstruction as presenter/adapter work. Leave SDK `thread_root` on children unprojected until permutation fixtures prove a product-visible, cross-client mis-attribution that the existing `reply` + root `thread` fields cannot express. **Falsified if** a shipped TS/Swift path still walks raw `m.relates_to` / `m.replace` / reaction annotations to decide edit/reply/thread/redaction identity for the live room timeline, or if desktop and iOS assign different parent/thread/redaction ids to the same Core row.

2. **Bounded extraction / shared fixture.** Smallest honest candidate is an optional child `thread_root_event_id` copied from SDK `MsgLikeContent.thread_root`, plus (separately) UniFFI carrying the **already-owned** root `thread` summary. Workstream preference is event-permutation fixtures first. **Falsified as necessary tonight** because no fixture corpus was run, grouping is keep-closed, and both clients already share the same omission. Widening the DTO to carry existing JSON fields is adapter completeness, not a missing `TimelineViewRow` field.

3. **Broader Core model (second semantic-row layer, Core-owned grouping/virtualization, or a message AST).** Rejected. It would violate the ROE-03 prior, ADR 0004 (no UI in Core), and the message-format ladder (fixtures before types; AST needs an ADR amendment).

Strongest stay-put case: the thick-looking iOS `TimelineItem` and the thinner UniFFI DTO are projections of one Core row. They do not re-own edits, redactions, reaction aggregates, or parent ids. A field the SDK has and Core does not copy is not automatically a census extract; the asked bar is a **cross-client** gap the row cannot express.

## Recommendation

**Already correctly owned.**

Confidence: high that no second semantic-row layer exists and that the asked relationships are already on `TimelineViewRow` (or are presenter/transport). Medium that child `thread_root` will ever need a row extension; that remains a candidate, not a proven remainder.

Supporting evidence:

- Edits, reply parent id (including relation-before-parent fallback), root thread summary, reactions, redaction kind, local-echo id absence, pagination metadata, and UTD replacement are projected from matrix-sdk-ui and consumed by both product timelines.
- Desktop live room has no `matrix-js-sdk` importer on `NativeTimelinePresenter`.
- iOS live `AppEnvironment` uses `SharedCoreTimelineService` → UniFFI `timeline_open` / `snapshot` / `paginate`.
- Relation-before-parent does not require a new field: the parent event id is present when the embed is not ready.
- Pagination overlap is owned by SDK diffs + stream revision; iOS id-dedup is a window, not a second history.
- Late decryption is a Core `Set` of the same `item_id`, not a presenter decrypt.

Strongest stay-put objection: iOS cannot show a thread chip or nested in-thread membership from the DTO, and Core does not project SDK `thread_root` on children while the live timeline includes threaded events. Those facts do not prove desktop and iOS *disagree* on a relationship the row already expresses. DTO drop of existing `thread` is thinning. Child `thread_root` is a shared omission. iOS `replyTo == root` filtering is grouping.

Unresolved questions (explicit, not assumed):

- No shared event-permutation / malformed-relation fixture corpus was executed (workstream preference). That does not authorize a second row model or a field add tonight.
- Whether product iOS thread sheets must list nested replies (`reply_to` ≠ thread root) is a presenter/product question. If fixtures later prove users cannot attribute those children, the smallest row extension is optional `thread_root_event_id` on the **existing** message row — still not a second layer.
- UniFFI completeness for already-owned JSON fields (`thread`, reply preview body/sender, poll answers, `own` reaction, capabilities) is adapter work, not a ROE-03 extract of new semantics.
- `formatted_body` “already-sanitized” wording remains misleading; ROE-04/12.
- Leftover desktop `Message.tsx` / pin-menu JS event helpers are not the live room owner; hygiene only.

Regression proof to keep the close stable:

- Live desktop room continues to consume `TimelineViewRow` JSON via `NativeTimelinePresenter` without reconstructing relations from raw Matrix events.
- Live iOS room continues to map `TimelineViewRowDto` without a Swift Matrix timeline.
- `edited`, `reply` / `reply_to_event_id`, `reactions`, `Redacted` / `kind == "redacted"`, and `EncryptedUnavailable` / `kind == "encrypted"` remain Core-projected.
- Presenters still do not own pagination identity; they apply Core diffs/snapshots.
- No product path introduces a parallel semantic-row type.

## Next gate

Already owned: close the research item. No implementation plan, no Core command, no second row model, no move of scrolling/virtualization/grouping/typography/selection into Core. Do not treat UniFFI flattening or the iOS pending-send overlay as a missing `TimelineViewRow` field. Do not start P5 or invent S38 from this memo. If a later fixture corpus proves child-in-thread mis-attribution, that is a human decision to extend the **existing** row — and that decision is a stop, not a start.
