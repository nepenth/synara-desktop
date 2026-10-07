# Unsent local echo — desktop presentation and discard

Branch: `feature/2026-10-07-client-session-and-chrome`.
Baseline: HEAD `6f964d3f`, tag `v2.1.46`.
Scope: how a text send that never received a server event id is shown, discarded, and optionally retried. Session reconnect, logout, the connection banner, and mark-read belong to the session task. This behavior is defined without that fix.

## Problem statement

On desktop, a message composed while the client was in a bad connection state comes back every time the app is opened. It has the same presentation as a sent message, and it has no message actions: no vertical-ellipsis / 3-dot menu, so the user cannot delete it. The user cannot tell whether it was sent.

The desktop log `~/.local/share/com.whylandcreative.synara.desktop/logs/synara-desktop.log` records exactly two `frontend matrix_send_text failed: native command rejected` lines in the recent window, at unix milliseconds `1791213552671` and `1791290307016` (about 2026-10-05 11:19 EDT and 2026-10-06 08:38 EDT). There is no later send success or failure line for that command. Those two lines are composer invocations of `matrix_send_text`. A background SDK retry would not write that frontend line, so the log proves two failed send commands and the absence of a recorded success. It does not, by itself, prove which diagnostic fired.

`formatDesktopInvokeError` in `synara/src/app/utils/desktop.ts` returns `native command rejected (${diagnosticId})` only when `diagnosticId` is in `SAFE_NATIVE_DIAGNOSTIC_IDS`. Every other rejection, including the send-queue ids produced by `crates/synara-core/src/app/send/room_queue.rs` (`d0.4-send-queue-timeout`, `d0.4-send-queue-closed`, `d0.4-send-queue-wedged`, `d0.4-send-queue-cancelled`, and the subscribe/handle ids), becomes the bare string `native command rejected`. The observed lines have no parenthetical id.

## Current states

Two layers exist. Only the SDK timeline item survives relaunch.

### SDK timeline send state (what the row can know after restart)

`EventTimelineItem::send_state()` on matrix-sdk-ui 0.19.1 is `Option<EventSendState>`:

| SDK state | Meaning | Server `event_id` |
| --- | --- | --- |
| `NotSentYet { progress }` | Still queued or in flight (sending) | Absent |
| `SendingFailed { error, is_recoverable }` | Failed. `is_recoverable: false` is the wedged case that blocks later sends in that room | Absent |
| `Sent { event_id }` | Homeserver accepted the send | Present |
| `None` | Remote event, or a local echo the SDK no longer treats as in-flight | Present once the server created the event |

`EventTimelineItem::event_id()` is `Some` only after the server has created the event. `is_local_echo()` stays true until sync echoes the event back. `transaction_id()` is the queue key for a local echo and is kept until the remote echo arrives. `identifier()` is that transaction id while the echo is unsent, and the event id once the item has been sent.

### Product send queue (process memory)

`LocalEchoState` in `crates/synara-core/src/dto/timeline.rs` is `Sending`, `Sent`, `Failed`, `Cancelled`, and `Wedged`. `SendQueue` in `crates/synara-core/src/app/send/queue.rs` stores those states for the live command. `NativeTimelineOwner` holds that queue in `sends` (`crates/synara-core/src/app/timeline/live.rs`), created empty in `NativeTimelineOwner::new` for the session generation. It is not written to the state store. After a relaunch it cannot mark the restored row failed.

`send_text_via_queue` maps the live command onto that memory queue:

- enqueue → `Sending`
- `QueuedSendAck` → `Sent`
- wedged `QueuedSendError` → `Wedged`
- cancelled → `Cancelled`
- any other error → `Failed`

### Where the persisted queue is reloaded

Composer text is enqueued with `Room::send_queue()` in `enqueue_event_via_room_queue` (`crates/synara-core/src/app/send/room_queue.rs`). matrix-sdk 0.19.1 persists that request in the state store (`save_send_queue_request` / `load_send_queue_requests`). The SDK module documents `SendQueue::respawn_tasks_for_rooms_with_unsent_requests()` as the client-initialization reload: it calls `load_rooms_with_unsent_requests` and reopens each room queue, which respawns the sending task. The same respawn runs from the global `SendQueue::set_enabled(true)`. The module states that if respawn is not called during initialization, persisted unsent events are re-sent when that room's send queue is reopened.

This repository does not call `respawn_tasks_for_rooms_with_unsent_requests`. `restore_session_onto_client` in `crates/synara-core/src/app/lifecycle/session_restore.rs` installs the vault session and does not touch the send queue. The room queue is reopened by the first `Room::send_queue()` in this crate: enqueue, `subscribe` inside abort/unwedge/inspect, and `set_enabled(true)` on that room after a recoverable failure, an unwedge, or an abort (`room_queue.rs`). Recoverable `SendError` values stay queued; `wait_for_queued_send` re-enables the room queue so the SDK retries them in order, then returns the failure to the command. The desktop command therefore logs a failure while the request remains in the store.

Opening a room builds an SDK timeline in `NativeTimelineOwner::open` via `TimelineBuilder`. That timeline is the surface that shows the restored local echo on the next launch. The in-memory `SendQueue` is empty at that point, so the row is whatever `project_event_row` copies from the SDK item.

## Root cause

`project_event_row_base` in `crates/synara-core/src/app/timeline/view.rs` copies `event.event_id()` and does not copy `EventTimelineItem::send_state()`. The field comment on `TimelineEventRowBase::event_id` already says the id is absent only for a local echo that has not received a server event id. `TimelineMessageRow` has body, edit, reply, and reaction fields, and no sending / failed / sent status. `project_row_action_capabilities` closes server-mutating actions when `event_id()` is missing (`react`, `reply`, `redact`, `report`, `pin`, `forward`, `vote`, `decline_call`). `edit` follows `event.is_editable()` and can be true for a local echo, but the desktop shell never reaches it.

`NativeTimelineRowActionSurface` in `synara/src/app/features/room/NativeTimelinePresenter.tsx` sets `hasActionMenu = Boolean(eventId && capabilities)` and renders the vertical-ellipsis rail only when that is true. A restored echo therefore has the same message surface as a sent row and no discard, retry, or delete control.

`abort_send` and `unwedge_send` already exist on `NativeTimelineOwner` and call `abort_queued_send` / `unwedge_queued_send` by transaction id. They are not desktop commands and the presenter does not call them. Redact cannot replace discard: redact is gated on a server event id.

iOS already shows `TimelineDeliveryStatus` (`sending`, `queued`, `sent`, `failed`) from `OutgoingSendService`, with a Retry chip on `SynaraMessageBubble` for `.failed`. That status is applied in the iOS outgoing path. `SharedCoreTimelineRows.item(from:)` does not read a delivery field, and when `eventId` is empty it uses `itemId` as `eventID`. The desktop native presenter has no equivalent status or chip.

## Requirements

### FR-ECHO-1 — Unsent rows are visibly not sent

A timeline row whose SDK send state is `NotSentYet` or `SendingFailed`, or whose server `event_id` is absent, uses an explicit unsent presentation on the desktop native timeline.

- `NotSentYet` is **sending** (still queued or in flight).
- `SendingFailed` is **failed**. `is_recoverable: false` remains the wedged subtype: later sends in that room stay blocked until unwedge or discard, which is existing queue behavior.
- `Sent { event_id }`, and a remote row that already has a server event id, stay the current sent presentation.

The unsent presentation is visible without hovering and has an accessible name a unit test can assert (sending versus failed). Classifying the row uses the SDK enum and `is_recoverable` only. `SendingFailed.error` is untrusted and is not rendered.

A missing status field on a row that also lacks `event_id` uses the unsent presentation. A missing status field on a row that has a server event id keeps the sent presentation.

### FR-ECHO-2 — Discard without a server event id

The user can discard a sending or failed local echo from that row. Discard calls the existing `abort_send` / `abort_queued_send` path with the room id and the SDK transaction id (`EventTimelineItem::transaction_id()`). It does not invent a `$` event id, call redact, or send `m.room.redaction`.

After a successful discard the row leaves the timeline, the persisted `RoomSendQueue` request for that transaction id is gone, and the next launch does not show it and does not send it. `SendQueue::cancel` updates the in-memory product queue in the same process; the abort of the SDK request is what makes relaunch safe.

Discard stays available when `event_id` is absent. It is a local control on the unsent row, separate from the server-event ellipsis menu.

### FR-ECHO-3 — Retry only through the existing queue owner

Desktop has no retry control on the native timeline today. iOS already has a Retry chip for failed outgoing bubbles; this spec does not add or change that chip.

If desktop offers retry on a failed or wedged echo, the control calls the existing owner on the **same** transaction id:

- wedged (`SendingFailed` with `is_recoverable: false`, product `Wedged`): `unwedge_send` / `unwedge_queued_send`
- recoverable failure that is still queued: the existing room-queue re-enable path that `wait_for_queued_send` already uses, not a second implementation

Retry does not call `matrix_send_text` again and does not allocate a new transaction id. A sending (`NotSentYet`) row offers discard and the sending presentation. It does not offer a failed-retry chip.

### FR-ECHO-4 — Relaunch does not present a rejected send as sent

A local echo that is still `NotSentYet` or `SendingFailed` after process start is projected again with unsent status and no server event id. The desktop row uses the FR-ECHO-1 presentation. The empty in-memory `SendQueue` from `NativeTimelineOwner::new` is not evidence that the echo was sent.

This requirement does not depend on the session task's reconnect, logout, banner, or mark-read work. A rejected send stays visibly unsent even while the session is unhealthy.

### FR-ECHO-5 — Server event ids keep the ellipsis menu

A row with a server event id keeps `hasActionMenu` from `eventId && capabilities`, the vertical-ellipsis control ("More message actions"), and the current capability gates in `project_row_action_capabilities`. Discard and retry for unsent echoes do not replace that menu and do not appear as the way to delete a sent message.

### FR-ECHO-6 — No token or body logging

Send failure logs stay on closed diagnostic ids. The message body, access token, refresh token, raw SDK error `Display` (`SendingFailed.error`), homeserver response body, and transaction payload are not written to `synara-desktop.log` or to the timeline DTO as a diagnostic. Widening `SAFE_NATIVE_DIAGNOSTIC_IDS` is optional and, if done, admits only existing static send-owner ids. It is not required to satisfy FR-ECHO-1 through FR-ECHO-5.

## Acceptance criteria

A reviewer can check these from tests and code, without a live homeserver:

1. Projection of an SDK item in `NotSentYet` omits `event_id` and is classified sending.
2. Projection of `SendingFailed` omits `event_id` and is classified failed. The wedged case is distinguishable from a recoverable failure using `is_recoverable`, and the error value is not copied into the DTO or a log string.
3. Projection of `Sent { event_id }` keeps that event id and the sent presentation. Remote rows with an event id are unchanged.
4. The desktop presenter test shows a sending row and a failed row as not sent, with a discard control that does not require `eventId`.
5. The presenter test for a row that has `eventId` still requires `eventId && capabilities` for the ellipsis rail and still exposes "More message actions".
6. Discard invokes the abort owner with the transaction id. A follow-up read of that room's local echoes (`RoomSendQueue::subscribe` or `queued_send_is_wedged` / the echo list) does not still contain that transaction as an active send.
7. A fixture that rebuilds the timeline projection the way a new `NativeTimelineOwner` would, with an empty in-memory `SendQueue` and an SDK item still in `NotSentYet` or `SendingFailed`, does not take the sent branch.
8. Retry, if present in the presenter, is wired to `unwedge_send` or the existing re-enable path for that same transaction id. Source for the unsent row does not call `matrix_send_text`.
9. Grep of the new log and DTO paths shows no message body, token, or `SendingFailed.error` display.

## Testing requirements

Add unit tests next to the code they lock:

- Timeline projection, in `crates/synara-core/src/app/timeline/view.rs` tests or the existing timeline view suite: the three `EventSendState` variants above, plus a remote row with an event id. Construct items with the SDK's timeline test helpers or a pure mapper over `EventSendState` and `event_id`. The assertion is the projected status and whether `event_id` is present.
- Desktop presenter, in `synara/src/app/features/room/__tests__/` (the native timeline presenter action tests are the current home of the `hasActionMenu` rail): unsent rows expose sending or failed and a discard control; a row with an event id keeps the ellipsis.
- Queue, in `crates/synara-core/src/app/send/tests.rs` and the existing mock-server suite `crates/synara-core/tests/messaging_core_send_queue.rs`: abort removes the transaction from the SDK local-echo list; unwedge remains the retry for a wedged transaction; a recoverable failure stays non-wedged. An in-memory `mark_failed` alone does not satisfy the relaunch criterion.

A live send test would add one thing these fixtures do not: a real state-store round trip. Enqueue text, fail it before a server event id exists, drop the `Client`, restore that same store through `restore_session_onto_client`, open the room with `TimelineBuilder`, and assert the projected row is unsent and that discard removes it from `load_send_queue_requests`. That live or cold-restart test is not required to land this branch when the projection, presenter, and queue fixtures above exist.

## Allowed paths

- `crates/synara-core/src/app/timeline/view.rs` — project send state onto the row.
- `crates/synara-core/src/dto/timeline.rs` — only if the public status enum needs a timeline-row form. `LocalEchoState` already names Sending, Sent, Failed, Cancelled, and Wedged.
- `crates/synara-core/src/app/timeline/live.rs` — call the existing `abort_send` / `unwedge_send` from a desktop command handler. Do not change mark-read or decryption retry.
- `crates/synara-core/src/app/send/room_queue.rs` and `crates/synara-core/src/app/send/queue.rs` — only to expose abort/unwedge status already implemented there.
- `crates/synara-core/src/core/messaging.rs`, `src-tauri/src/matrix/send/product_commands.rs`, `src-tauri/src/bridge/send_text.rs`, and the command census — only for a discard/retry command whose payload is room id plus transaction id.
- `synara/src/app/features/room/nativeTimelineView.ts` — carry the additive status. A missing field on a row with `eventId` stays sent.
- `synara/src/app/features/room/NativeTimelinePresenter.tsx` — unsent presentation, discard, and optional retry.
- Unit tests beside those files, including `synara/src/app/features/room/__tests__/nativeTimelinePresenterActions.test.ts`, `crates/synara-core/src/app/send/tests.rs`, and `crates/synara-core/tests/messaging_core_send_queue.rs`.

A new status field on the shared timeline JSON must be optional with a serde default so existing decoders keep working. `synara-ios/Synara/Services/SharedCoreTimelineRows.swift` is not an allowed path for this bug. iOS already has its own delivery status and Retry chip.

## Non-goals

- Do not change Linux window chrome, drag regions, or title-bar behavior.
- Do not change auth retirement, session restore policy, the connection banner, logout, or mark-read.
- Do not require the session task's reconnect fix before this presentation and discard behavior is correct.
- Do not add an iOS control, and do not change `OutgoingSendService` or `SynaraMessageBubble`.
- Do not add a desktop "edit unsent" flow. iOS already edits unsent messages; desktop redact/edit of a sent message stays on the event-id menu.
- Do not log message bodies, tokens, or raw SDK errors in order to name the stripped diagnostic.
