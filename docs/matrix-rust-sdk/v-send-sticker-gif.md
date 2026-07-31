# V-SEND sticker/GIF — native composer sticker + GIF send

| Field          | Value                                                                 |
| -------------- | --------------------------------------------------------------------- |
| Status         | Candidate draft — not yet merged or runtime-proven                    |
| Queue position | Residual after V-SEND.1 / V-SEND.5                                    |
| Owner          | `matrix_send_sticker` (`m.sticker`) + GIF via `matrix_send_attachment` |
| JS fallback    | None once a native Matrix session is logged in                        |

## Scope

Replaces the composer **sticker** and **GIF picker** send paths for native
desktop sessions:

- `RoomInput` `handleStickerSelect` no longer calls `mx.sendEvent(m.sticker)`
  when a native session is live;
- `RoomInput` `handleGifSelect` no longer calls `mx.uploadContent` /
  `mx.sendMessage` when a native session is live;
- stickers send as true `m.sticker` events (MXC already on the homeserver from
  image packs — no re-upload);
- GIFs download in the WebView, then upload+send via the existing native
  attachment owner (`image/gif` bytes over IPC once);
- optional `replyTo` / `threadRoot` mirror V-SEND.5 relation rules.

Out of this slice (residuals):

- emoji/sticker **pack settings** (`RoomPacks` / `GlobalPacks` account-data and
  state writes) — not send;
- native media **display** / authenticated media download for pack previews
  (V-TIMELINE / media);
- poll-in-thread, avatar/profile/call/forward upload paths;
- live Synapse sticker/GIF proof job (unclaimed until CI green).

## Operating path

```text
EmojiBoard sticker select
  → matrix_send_sticker
  → StickerEventContent (body, mxc, optional ImageInfo, relates_to)
  → room.send(m.sticker)

GifPicker select
  → fetchGifForUpload (remote HTTPS)
  → matrix_send_attachment (image/gif bytes)
  → Room::send_attachment
```

No dual-backend selector. Native command failure does not fall through to JS
send APIs.

## Evidence

- Tauri command registration, capability allow-list, generated permissions for
  `matrix_send_sticker`.
- Injectable frontend owner-route tests prove no legacy fallback when native.
- Rust unit tests for sticker content (MXC validation, info, reply/thread).
- Direct `matrix-js-sdk` importer delta expected **zero** (inline RoomInput
  owners; pack settings retain JS). Scoped method residual: `sendEvent` /
  `uploadContent` / `sendMessage` still appear for legacy web + other
  residuals.
