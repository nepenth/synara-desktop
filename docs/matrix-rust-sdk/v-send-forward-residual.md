# V-SEND.R-FORWARD — message forward residual inventory

| Field | Value |
|-------|-------|
| Status | **Implement PR** — legacy MessageForwardItem + `utils/forward.ts` deleted after C1/C2; native presenter forward remains sole product path |
| Tip SHA | `95a6a71b` (merge #288 scoreboard honesty; after #287 pack-read inventory) |
| Base | `feature/matrix-rust-sdk-full-replacement` |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) — physical deletion inside each owning slice |
| Related | V-SEND residual inventory, V-TIMELINE cutover #285/#289 (approved; C1/C2 in flight), V-SEND.R-PACK-READ #287 |

> **Scope guard.** Docs only. No product code in `product.rs` or any TS. Does
> not touch open **#285/#289** (timeline cutover; do not block this docs PR) or **#39** (umbrella). No cutover.

---

## 1. Where forward lives today

Forward **send** is already fully native. The Rust slice owns two IPC commands
in `src-tauri/src/matrix/auth/product.rs`:

- `matrix_timeline_forward_text` (line 2183) — loads the source event from the
  **native** timeline via `room.load_or_fetch_event`, formats a forwarded plain
  body (`format_forwarded_plain_body` in `timeline/actions.rs`), and sends with
  `send_message_to_room`.
- `matrix_timeline_forward_media` (line 2229) — loads the source event natively,
  rewrites the media body with the sender label (`format_forwarded_media_body`),
  and sends via `target_room.send(...)` (handles `m.image`/`m.file`/`m.audio`/
  `m.video`/`m.sticker`).

Both are registered in `src-tauri/src/lib.rs` (lines 437/444) and
`src-tauri/build.rs` (lines 103/104). The frontend invokes them through
`synara/src/app/features/room/nativeTimelineAction.ts`
(`forwardTextWithNativeTimelineAction` / `forwardMediaWithNativeTimelineAction`)
which call the owners in `nativeTimelineActions.ts` (`matrix_timeline_forward_text`
/ `matrix_timeline_forward_media`).

**The residual is the legacy JS forward dialog**, not the send. Two forward
dialogs exist:

1. **`Message.tsx` → `MessageForwardItem`** (legacy, JS): builds the forward
   preview with `getForwardableEventContent` / `getForwardableEventContents`
   (from `utils/forward.ts`, which read event content off the live
   `matrix-js-sdk` `MatrixEvent`), resolves target rooms with `mx.getRoom`,
   filters with `getRoomForwardTargets` / `canSendRoomMessage`, and reads
   `mx.getUserId()`. The actual send still goes through the native
   `forwardMediaWithNativeTimelineAction` / `forwardTextWithNativeTimelineAction`
   (see `handleForwardConfirmed`, Message.tsx ~491–527). So the JS dialog is a
   **read/preview + target-selection** residual; the write is native.
2. **`NativeTimelinePresenter.tsx`** (native): already builds forward targets
   from the native room list (`roomList.rooms` → `filterNativeForwardTargets`)
   and sends through the same native actions. This is the native forward dialog.

So the remaining JS surface for V-SEND.R-FORWARD is the `MessageForwardItem`
dialog and its `utils/forward.ts` read helpers.

---

## 2. Residual table — V-SEND.R-FORWARD

| Path | Role | Gap | ID |
|------|------|-----|----|
| `synara/src/app/features/room/message/Message.tsx` | `MessageForwardItem` (`handleForward` / `handleForwardConfirmed`): forward preview + target picker + encrypted-room confirm | Dialog reads event content and resolves/filters targets on the live `matrix-js-sdk` client (`mx.getRoom`, `mx.getUserId`, `getForwardableEventContent(s)`, `getRoomForwardTargets`, `canSendRoomMessage`); send itself is native | **V-SEND.R-FORWARD** |
| `synara/src/app/utils/forward.ts` | `getForwardableEventContent` / `getForwardableEventContents` / `makeForwardedContent` / `makeForwardQuoteContent` / `stripForwardUnsafeFields` / `escapeHtml` | Build forward preview content from JS `MatrixEvent` (reads `event.getContent()`, `event.getRelation()`, `event.getSender()`, `event.getTs()`) | **V-SEND.R-FORWARD** (read/preview) |
| `synara/src/app/utils/forward.ts` | `getRoomForwardTargets` / `canSendRoomMessage` | Target filtering / power-level check on JS `Room` + `getRoomCurrentState` | **V-SEND.R-FORWARD** (target selection) |

**Note:** the native `NativeTimelinePresenter.tsx` forward dialog is **not** a
residual — it already uses the native room list and native send. The residual is
specifically the legacy `MessageForwardItem` path and its `utils/forward.ts`
read helpers. The `format_forwarded_*_body` / `format_forwarded_plain_body`
formatting already lives natively in Rust; the JS `makeForwardedContent` /
`makeForwardQuoteContent` are only used for the legacy dialog preview.

---

## 3. Proposed slice — native forward dialog

The native send commands already exist, so this slice is about deleting the
legacy JS dialog and its read helpers, not adding new send IPC. Two options:

- **Preferred — reuse existing native send + native room list.** Route the
  forward dialog through the native presenter's pattern: build targets from the
  native room list (`roomList.rooms` → `filterNativeForwardTargets`) and send
  with the existing `matrix_timeline_forward_text` / `matrix_timeline_forward_media`.
  No new `matrix_forward_*` command is needed — the send is already native and
  reuses `matrix_send_text`-style `send_message_to_room` / `Room::send` under the
  hood. Delete `MessageForwardItem` and the `utils/forward.ts` read helpers.
- **Alternative — new `matrix_forward_*` preview command.** If the dialog needs
  a native preview (event content + sender label) without a JS `MatrixEvent`,
  add a read-only `matrix_timeline_forward_preview` that returns the forwardable
  content from the native timeline (mirroring `load_forwardable_text` /
  `load_forwardable_media`). This is only needed if the legacy dialog's preview
  must be preserved verbatim; otherwise reuse the native presenter.

**Fail-closed:** on a native logged-in session, the forward dialog must not fall
through to `mx.getRoom` / `getForwardableEventContent` / `getRoomForwardTargets`.
If native targets or the native send are unavailable, forward is terminal (no JS
`mx.sendMessage` fallback). Legacy JS read paths remain only for non-native web
sessions.

**Deletion list** (physical deletion per [full-vertical-policy.md](full-vertical-policy.md)):
`MessageForwardItem` in `Message.tsx` (and its `handleForward` /
`handleForwardConfirmed` / encrypted-room confirm), and the `utils/forward.ts`
read helpers (`getForwardableEventContent`, `getForwardableEventContents`,
`makeForwardedContent`, `makeForwardQuoteContent`, `stripForwardUnsafeFields`,
`escapeHtml`, `getRoomForwardTargets`, `canSendRoomMessage`) — keeping only the
native presenter's forward dialog. Verify no other consumers of these helpers
remain before deletion.

---

## 4. Non-goals / out of scope

| Item | Status |
|------|--------|
| **Timeline cutover** (selecting `NativeTimelinePresenter` / deleting `RoomTimeline` / removing the dual-backend) | **V-TIMELINE #240** — HOLD; do not edit |
| **Pack residuals** (sticker/emoji pack read/write/upload) | **V-SEND.R-PACK-READ #287** (done), V-SEND.R-PACK-WRITE / PACK-UPLOAD — separate |
| Umbrella merge to `main` | **#39** — needs explicit user approval |
| Forward **send** implementation | Already native (`matrix_timeline_forward_text` / `matrix_timeline_forward_media`) — not a residual |
| Other V-SEND residuals (reactions, polls, rich messages, threads, attachments) | Separate slices |

---

## 5. Confidence

**Confidence: high** for the inventory. I traced the forward surface from the
Rust send commands (`matrix_timeline_forward_text` / `matrix_timeline_forward_media`
in `product.rs`, registered in `lib.rs` / `build.rs`) through the frontend action
wrappers (`nativeTimelineAction.ts` / `nativeTimelineActions.ts`) to both dialogs.
The native `NativeTimelinePresenter.tsx` forward dialog already uses the native
room list and native send; the residual is the legacy `MessageForwardItem` in
`Message.tsx` and its `utils/forward.ts` read helpers, which still read event
content and resolve/filter targets on the live `matrix-js-sdk` client. Possible
missed files: any other consumer of `utils/forward.ts` helpers or a barrel
re-export — verify during implementation with a full `grep -rn "utils/forward"`
and `grep -rn "getForwardableEventContent"` over `synara/src`.


## 7. Implementation close (this PR)

After V-TIMELINE C1 (#285) + C2 (#289), `RoomTimeline` no longer mounts multi-select forward.
`NativeTimelinePresenter` owns product forward (native room list + `matrix_timeline_forward_*`).

This PR deletes:
- `MessageForwardItem` (+ menu mount) from `message/Message.tsx`
- `synara/src/app/utils/forward.ts` and unit tests
- allowlist path `synara/src/app/utils/forward.ts` (**168→167**)
- production import files **164→163**

Fail-closed: no JS forward dialog residual on the selected timeline path; send was already native.
