# V-SEND.5 — native thread send / relations

| Field          | Value                                                                 |
| -------------- | --------------------------------------------------------------------- |
| Status         | Draft — runtime proof pending CI **Synapse native thread-send proof** |
| Queue position | After V-SEND.4 rich messages                                          |
| Owner          | Managed Rust client via extended `matrix_send_text` / `matrix_send_attachment` |
| JS fallback    | None for composer text/attachment thread relations on a desktop native session |

## Scope and deleted owner

This vertical extends the existing native composer send owners so that **start
thread** and **reply in thread** produce a correct `m.relates_to` /
`rel_type: m.thread` relation without building that relation in JS for the
native happy path.

### Product paths owned natively

| Path | Operating path |
| ---- | -------------- |
| Composer text (incl. emote/notice/rich HTML) | `RoomInput` → `sendPlainTextWithNativeOwner` → `matrix_send_text` (`replyTo` + `threadRoot`) → `message_content` → `Relation::Thread(Thread::reply\|without_fallback)` → `Room::send` |
| Composer attachments | `RoomInput` → `sendComposerAttachmentsWithNativeOwner` → `matrix_send_attachment` (`replyTo` + `threadRoot`) → `EnforceThread::Threaded(ReplyWithinThread::Yes)` when `threadRoot` is set |
| Composer GIF | `RoomInput` → `sendComposerGifWithNativeOwner` → `fetchGifForUpload` → `matrix_send_attachment` (`image/gif`, `replyTo` + `threadRoot`) |

On a desktop **native logged-in** session, command absence or failure is
terminal and never falls through to `mx.sendMessage` / JS relation construction
for those two owners.

### Relation wire shape (text)

Matches the previous JS `getReplyRelation` product behavior:

```json
{
  "m.relates_to": {
    "rel_type": "m.thread",
    "event_id": "$thread_root",
    "m.in_reply_to": { "event_id": "$replied_to" },
    "is_falling_back": false
  }
}
```

- **Start thread** (UI sets draft relation `m.thread` on the root event):
  `threadRoot == replyTo == root event id`.
- **Reply in thread**: `threadRoot` is the root; `replyTo` is the specific
  event being answered.
- **Classic reply (no thread)**: `replyTo` only → `Relation::Reply` (unchanged
  from V-SEND.4).

No tokens, keys, ciphertext, or raw SDK errors cross IPC. Results remain
`{ roomId, eventId, localTxnId, status }` only.

## Superseded JS ownership

For **composer text, attachment, and GIF** on a native session, the authoritative
relation write is Rust. The legacy `getReplyRelation` / `content['m.relates_to']`
path remains **only** for:

1. Web / native-logged-out sessions (legacy owner retained), and
2. Other explicitly inventoried residual product paths.

This slice does **not** delete whole `matrix-js-sdk` importer files: the same
`RoomInput` file still uses the JS client for legacy web sessions and other
residual paths. Physical owner deletion for this capability is the
native-session fail-closed route plus IPC ownership of `m.thread` for each
native composer sender.

## Out of scope / residual (named)

| Residual | Owner / follow-up |
| -------- | ----------------- |
| GIF picker pack/collection management | **NOOP** — no such product surface exists on this tip; selected GIF send is native (#264) |
| Poll start/response in a thread | V-SEND.3 residual if product adds draft threading to polls |
| Thread list / summary SDK subscription & UI | P5.8 harness only today; timeline/list vertical |
| Thread-focused timeline / open-thread view cutover | V-TIMELINE (do not edit #240) |
| Thread receipts / MSC4306 subscriptions | Receipts / notifications verticals |
| Message edit / replace in thread | Separate residual |

## Runtime proof

Authoritative gate: required CI job **Synapse native thread-send proof**
(`live_native_thread_send_against_disposable_synapse_when_configured`), gated by
`SYNARA_RUN_MATRIX_RUST_THREAD_SEND_LIVE=1`.

The proof:

1. Registers/logs in with the managed Rust client on disposable Synapse.
2. Sends a root `m.room.message`.
3. Sends an in-thread reply via the same `message_content(..., reply_to, thread_root)`
   builder as `matrix_send_text`.
4. Fetches both events and asserts wire `m.relates_to.rel_type == m.thread`,
   correct root / `m.in_reply_to`, and non-fallback.
5. Sends a nested in-thread reply (root ≠ replied-to) and re-asserts.

Until that CI job is green on the reviewed SHA, runtime proof remains
**Not confirmed**.

## Focused local evidence

- Rust unit tests: `message_content` thread/reply shapes, invalid thread root id.
- Frontend owner-route tests: `threadRoot` forwarded on `matrix_send_text` /
  `matrix_send_attachment`; no legacy fallthrough when native logged-in.
- `cargo test` for auth product send tests; scoped Node tests for native owners.

## Permissions

Reuses existing Tauri permissions:

- `allow-matrix-send-text`
- `allow-matrix-send-attachment`

Optional `threadRoot` is an additional validated event-id argument; no new
command identifiers required.
