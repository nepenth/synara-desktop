# V-SEND upload call-site audit after native-only GIF and thumbnail replacement

| Field        | Value                                                                                                                                             |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status       | **Native desktop product + audit update** — GIF and video-thumbnail legacy upload branches removed                                                |
| Measured tip | `4eeefa11` on the focused branch; PR base is `feature/matrix-rust-sdk-full-replacement`                                                           |
| Scope        | Production `mx.uploadContent` / `uploadContent(mx, …)` usages under `synara/src`, excluding the `CompactUploadCardRenderer` native path from #314 |
| Guard        | Do not touch `main` or umbrella PR **#39**                                                                                                        |

## Finding

The source scan finds two `mx.uploadContent`/`uploadContent(mx, …)` call
sites, plus one call-widget equivalent (`client.uploadContent`). The GIF and
video-thumbnail callers are no longer in this inventory: their native owners
are the only composer desktop route and fail closed when native support is
unavailable. The shared upload helper remains source-reachable for unrelated
web/legacy and compact-upload consumers, while compact desktop uploads use
`matrix_upload_media` after #314.

## Call-site inventory

| Source                                                | Production operation                                                                                                                                        | Reachability on a native desktop session                                                                                                   | Residual mapping                                                                                                                                                                               |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `synara/src/app/plugins/call/CallWidgetDriver.ts:318` | `client.uploadContent(file)` is the legacy callback passed to `uploadCallWidgetFileWithNativeOwner`; the same driver also owns widget media config/download | **Not reachable** on a native desktop session; #328 routes binary uploads through `matrix_upload_media` and fails closed on native failure | **V-SEND.R-CALL-UPLOAD** — closed by **#328**                                                                                                                                                  |
| `synara/src/app/state/upload.ts:120`                  | `useBindUploadAtom` calls the shared `uploadContent(mx, file, …)` wrapper                                                                                   | **Not used** by `UploadCardRenderer` when `nativeComposerSend` is true; compact desktop also bypasses it                                   | Shared owner for non-native web uploads and compact consumers: **V-SEND.R-AVATAR-UPLOAD**, **R-ROOM-PROFILE**, and historical **V-SEND.R-PACK-UPLOAD** ownership; not a new residual by itself |
| `synara/src/app/utils/matrix.ts:155`                  | Shared `uploadContent` wrapper calls `mx.uploadContent(file, …)`                                                                                            | Same as `state/upload.ts`                                                                                                                  | Shared implementation for the mappings above; retain until the remaining web/legacy consumers are removed                                                                                      |

## Native-path boundary

The excluded #314 path is
`CompactUploadCardRenderer` → `uploadMediaNative` →
`matrix_upload_media` on desktop. It sets an upload error when native media
upload is unavailable and does not fall through to `mx.uploadContent`.
`UploadCardRenderer` is the separate composer renderer: when
`nativeComposerSend` is true it marks the file staged and skips the shared JS
upload; otherwise it starts the legacy JS upload.

The remaining composer send gates are explicit:

- `RoomInput` checks `nativeComposerAttachmentReady()` before staging files;
  `sendComposerAttachmentsWithNativeOwner` owns attachment bytes on a native
  session and returns `legacy` only without one.
- `sendComposerGifWithNativeOwner` requires a native session and owns GIF bytes
  through `matrix_send_attachment`; an unavailable session is terminal and no
  JS GIF upload/send branch remains.
- Native command absence/failure is terminal; it does not select the JS
  upload/send branch.

## SCOREBOARD reconciliation

| SCOREBOARD entry                | Audit result                                                                                                                                                                      |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **V-SEND.R-CALL-UPLOAD**        | **Closed by #328**. The `CallWidgetDriver` native owner handles logged-in desktop uploads; `client.uploadContent` is retained only as the non-native/web callback.                |
| **V-SEND.R-GIF-PACK**           | **NOOP** — the picker has no pack/collection-management surface. The selected GIF send is now native-only; this is not pack management. See [audit](v-send-gif-pack-audit.md).    |
| **V-SEND.R-PACK-UPLOAD**        | #314 is correct for compact desktop uploads. The shared JS helper remains for web/legacy reachability, but there is no native desktop fallthrough in `CompactUploadCardRenderer`. |
| Composer attachments / GIF send | Native attachment/GIF owners are the desktop route; GIF JS upload/send was deleted here, while shared attachment staging/upload remains outside this focused slice.               |
| Video thumbnail upload          | Native `matrix_upload_media` is the sole thumbnail upload owner; the `msgContent` JS `mx.uploadContent` fallback was deleted here and now fails closed.                           |

The GIF-pack scoreboard entry is **NOOP**, call-upload is closed by **#328**,
and thumbnail upload is closed by **#325** plus this fallback deletion. This
audit records only the remaining unrelated fallback-only usages.

## Verification

The production scan was limited to `synara/src` and excluded tests. Executable
matches were checked with:

```text
rg -n 'mx\.uploadContent|uploadContent\s*\(\s*mx' synara/src
```

Comments documenting the native fail-closed boundary and the
`CompactUploadCardRenderer` implementation were not counted as production
call sites. The expected remaining production matches are the shared helper,
its state consumer, and the documented non-native call-widget callback.
