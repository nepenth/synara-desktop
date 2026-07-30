# V-SEND.1 — native composer attachment upload/send

| Field          | Value                                                                 |
| -------------- | --------------------------------------------------------------------- |
| Status         | Candidate — not yet merged or runtime-proven                          |
| Queue position | `V-SEND.1` after V-SEND.2 reactions merged                            |
| Owner          | Managed Rust `Room::send_attachment` + `AttachmentSendQueue`          |
| JS fallback    | None once a native Matrix session is logged in                        |

## Scope and deleted owners

This vertical replaces the composer UploadBoard attachment **upload and send**
path for ordinary file/image/video/audio attachments:

- `RoomInput` `handleSendUpload` no longer calls `mx.uploadContent` /
  `mx.sendMessage` when a native session is live;
- Upload cards stage locally (`mxc://synara.native/staged`) instead of starting
  the JS upload binder;
- encrypted rooms are encrypted by the managed SDK — JS `encryptFile` is not
  used on the native path (no dual-encrypt).

Out of this slice (residuals):

- GIF picker upload/send;
- stickers, polls, voice-message product path;
- avatar/profile/pack `CompactUploadCardRenderer` uploads;
- call-widget upload;
- message forward `sendMessage`;
- timeline media **display** (V-TIMELINE / #240).

## Operating path

```text
composer UploadBoard send
  → matrix_send_attachment
  → AttachmentSendQueue enqueue
  → matrix_sdk Room::send_attachment (upload + room message)
  → mark sent / failed
```

Bytes cross IPC once (32 MiB soft cap). Reply relations use
`AttachmentConfig::reply`. No WebView MXC assembly and no JS SDK fallback after
native ownership is selected.

## Evidence

- Tauri command registration, capability allow-list, generated permissions, and
  ACL schemas agree for `matrix_send_attachment`.
- Injectable frontend owner-route tests prove no legacy fallback when native.
- Post-#239 tip `988cdc2` inventory for this candidate: desktop runtime
  production importers **190 → 190**, repository-wide **203 → 203**. Scoped
  method candidates `uploadContent` **4 → 4** and `sendMessage` **5 → 5** remain
  because GIF/avatar/call/forward residuals still reference those APIs; the
  composer UploadBoard path no longer invokes them when a native session is
  live. Direct-import delta is honestly **zero**.
- Runtime proof: preferred disposable-Synapse image/file send via the managed
  client on the reviewed SHA. Until that is green, proof remains
  **Not confirmed**. Keep draft until Confirmed.
