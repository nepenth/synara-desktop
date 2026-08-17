# 0005 — Native media handle channel

Status: Accepted on `main` via #1001. Not program-done. Not P4 acceptance.

## Decision

Timeline rows expose an opaque `media_handle_id` (plus mime/size metadata).
iOS loads bytes through `SharedCore.timeline_media_bytes(handle_id)`, a
dedicated UniFFI `bytes` argument. Desktop live timeline media uses the
`synara-media://` protocol and `resolve_timeline_media`. The leftover
`matrix_media_download` shell command now accepts the same
`timeline-media-*` handle and resolves it through that owner. Plain
`mxc://` remains only for leftover avatar/pack paths.

Bytes must not cross `Core::command` (1 MiB envelope, 32 MiB product cap).
Do not register `matrix_send_attachment` / `matrix_upload_media` /
`matrix_media_download` on Core.

## Why

The presenter boundary already forbids `mxc://` on timeline JSON. The
native owner keeps `MediaSource` behind the handle, including encrypted
sources. Desktop no longer treats leftover `mxc://` +
`browser-encrypt-attachment` as the live timeline decrypt path. UniFFI
leftover `media_download(mxc)` stays planted fail-closed on iOS
(decision 15).

## Must not

- Put media bytes or `mxc://` on `TimelineViewRowDto`
- Download media in NSE
- Treat leftover `media_download` as the iOS live path
- Register `matrix_media_download` on `Core::command`
