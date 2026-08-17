# 0005 — Native media handle channel

Status: Proposed on #1001. Not program-done. Not P4 acceptance.

## Decision

Timeline rows expose an opaque `media_handle_id` (plus mime/size metadata).
iOS loads bytes through `SharedCore.timeline_media_bytes(handle_id)`, a
dedicated UniFFI `bytes` argument. Desktop leftover `matrix_media_download`
stays a shell command that takes an `mxc://` URI.

Bytes must not cross `Core::command` (1 MiB envelope, 32 MiB product cap).
Do not register `matrix_send_attachment` / `matrix_upload_media` /
`matrix_media_download` on Core.

## Why

The presenter boundary already forbids `mxc://` on timeline JSON. The
native owner keeps `MediaSource` behind the handle. UniFFI leftover
`media_download(mxc)` stays planted fail-closed on iOS (decision 15).

## Must not

- Put media bytes or `mxc://` on `TimelineViewRowDto`
- Download media in NSE
- Treat leftover `media_download` as the iOS live path
