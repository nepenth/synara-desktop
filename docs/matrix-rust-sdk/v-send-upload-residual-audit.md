# V-SEND upload call-site audit after #325 + #328

| Field | Value |
|-------|-------|
| Status | **Docs-only audit** — no product code changed |
| Measured tip | `e38bfdab` (`feature/matrix-rust-sdk-full-replacement` worktree tip) |
| Scope | Production `mx.uploadContent` / `uploadContent(mx, …)` usages and the call-widget `client.uploadContent` equivalent under `synara/src`, excluding native-owned paths from #314, #325, and #328 |
| Guard | Do not touch `main` or umbrella PR **#39** |

## Finding

The source scan still finds four `mx.uploadContent`/`uploadContent(mx, …)`
sites, plus one call-widget equivalent (`client.uploadContent`). The #325
thumbnail site and #328 call-widget site now sit behind native owners: when
native session detection confirms a logged-in desktop Matrix session, the
owner uses `matrix_upload_media`; the JS call remains only for a non-native,
logged-out, or otherwise unavailable native-session route. The shared upload
helper is still source-reachable from those web routes and from compact-upload
consumers, while compact desktop uploads themselves use `matrix_upload_media`
after #314.

## Call-site inventory

| Source | Production operation | Reachability on a native desktop session | Residual mapping |
|--------|----------------------|------------------------------------------|------------------|
| `synara/src/app/plugins/call/CallWidgetDriver.ts:324` | `uploadCallWidgetFileWithNativeOwner` receives the legacy `client.uploadContent(file)` callback for widget `uploadFile`; the same driver also owns widget media config/download | **Not reachable** when native session detection confirms a logged-in desktop Matrix session; #328 uses `matrix_upload_media` and fails closed if that native upload is unavailable. The callback remains for web, logged-out, or unavailable native-session detection. | **V-SEND.R-CALL-UPLOAD** — complete via **#328**; the source match is a non-native fallback. |
| `synara/src/app/features/room/RoomInput.tsx:911` | GIF picker legacy branch downloads the GIF, optionally encrypts it, then calls `mx.uploadContent` before `mx.sendMessage` | **Not reachable** when `sendComposerGifWithNativeOwner` sees a logged-in native session; native GIF send uses `matrix_send_attachment` | **Composer GIF send / #264 legacy-web fallback**; not a GIF-pack residual. The [GIF-pack audit](v-send-gif-pack-audit.md) found no collection-management surface. |
| `synara/src/app/features/room/msgContent.ts:39` | Video thumbnail bytes go through `uploadMediaNative`; the `mx.uploadContent` call is the fallback while building legacy attachment message content | **Not reachable** when native session detection confirms a logged-in desktop Matrix session; #325 uses `matrix_upload_media` and fails closed if that native upload is unavailable. The JS call remains only when no live/available native session exists. | **Composer thumbnail** — complete via **#325**; the source match is a non-native fallback. |
| `synara/src/app/state/upload.ts:120` | `useBindUploadAtom` calls the shared `uploadContent(mx, file, …)` wrapper | **Not used** by `UploadCardRenderer` when `nativeComposerSend` is true; compact desktop also bypasses it | Shared owner for non-native web uploads and compact consumers: **V-SEND.R-AVATAR-UPLOAD**, **R-ROOM-PROFILE**, and historical **V-SEND.R-PACK-UPLOAD** ownership; not a new residual by itself |
| `synara/src/app/utils/matrix.ts:155` | Shared `uploadContent` wrapper calls `mx.uploadContent(file, …)` | Same as `state/upload.ts` | Shared implementation for the mappings above; retain until the remaining web/legacy consumers are removed |

## Native-path boundary

The excluded #314 path is
`CompactUploadCardRenderer` → `uploadMediaNative` →
`matrix_upload_media` on desktop. It sets an upload error when native media
upload is unavailable and does not fall through to `mx.uploadContent`.
`UploadCardRenderer` is the separate composer renderer: when
`nativeComposerSend` is true it marks the file staged and skips the shared JS
upload; otherwise it starts the legacy JS upload.

The #325 thumbnail path also uses `uploadMediaNative` and only selects
`mx.uploadContent` when there is no live/available native Matrix session. The
#328 call-widget path uses `uploadCallWidgetFileWithNativeOwner`; its
`client.uploadContent` callback is likewise outside a confirmed logged-in
native session. Once native ownership is selected, `matrix_upload_media`
absence or an invalid response is terminal for both paths; it does not select
the JS fallback.

The composer send gates are explicit:

- `RoomInput` checks `nativeComposerAttachmentReady()` before staging files;
  `sendComposerAttachmentsWithNativeOwner` owns attachment bytes on a native
  session and returns `legacy` only without one.
- `sendComposerGifWithNativeOwner` follows the same native-session check and
  owns GIF bytes through `matrix_send_attachment`.
- Native command absence/failure is terminal; it does not select the JS
  upload/send branch.

## SCOREBOARD reconciliation

| SCOREBOARD entry | Audit result |
|------------------|--------------|
| **V-SEND.R-CALL-UPLOAD** | **Complete via #328** — `CallWidgetDriver.ts:324` retains `client.uploadContent` only as the web/logged-out fallback. |
| **V-SEND.R-GIF-PACK** | **NOOP** — the picker has no pack/collection-management surface. The `RoomInput` GIF upload is a native-send legacy-web fallback, not pack management; see [audit](v-send-gif-pack-audit.md). |
| **V-SEND.R-PACK-UPLOAD** | #314 is correct for compact desktop uploads. The shared JS helper remains for web/legacy reachability, but there is no native desktop fallthrough in `CompactUploadCardRenderer`. |
| Composer attachments / GIF send / thumbnail | Native owners are landed (#248/#258/#264/#325); remaining JS upload text is fallback evidence, not a new native-session residual. |

The SCOREBOARD already records **#328** for call-widget upload and **#325** for
composer thumbnails; no additional SCOREBOARD status change is required. This
audit refresh closes the post-#325/#328 call-site inventory and makes the
remaining fallback-only usages explicit.

## Verification

The production scan was limited to `synara/src` and excluded tests. Executable
matches were checked with:

```text
rg -n 'mx\.uploadContent|uploadContent\s*\(\s*mx|client\.uploadContent' synara/src
```

Comments documenting the native fail-closed boundary and the
`CompactUploadCardRenderer` implementation were not counted as production
call sites.
