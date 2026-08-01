# V-SEND upload call-site audit after #325/#328/#331

| Field | Value |
|-------|-------|
| Status | **Docs-only audit** — no product code changed |
| Measured tip | \`e38bfdab\` on \`feature/matrix-rust-sdk-full-replacement\` |
| Scope | Production `mx.uploadContent` / `uploadContent(mx, …)` usages under `synara/src`, excluding the `CompactUploadCardRenderer` native path from #314 |
| Guard | Do not touch `main` or umbrella PR **#39** |

## Finding

The source scan finds four `mx.uploadContent`/`uploadContent(mx, …)` call
sites, plus one call-widget equivalent (`client.uploadContent`). They do not
all represent native-session residuals. The composer attachment, GIF, video
thumbnail, and call-widget upload owners are native-first and fail closed on a
desktop native Matrix session; their JS uploads remain only for the
non-native web/legacy route. The shared upload helper is still source-
reachable from those web routes and from compact-upload consumers, while
compact desktop uploads themselves use `matrix_upload_media` after #314.

## Call-site inventory

| Source | Production operation | Reachability on a native desktop session | Residual mapping |
|--------|----------------------|------------------------------------------|------------------|
| `synara/src/app/plugins/call/CallWidgetDriver.ts:318` | `client.uploadContent(file)` is the legacy callback passed to `uploadCallWidgetFileWithNativeOwner`; the same driver also owns widget media config/download | **Not reachable** on a native desktop session; #328 routes binary uploads through `matrix_upload_media` and fails closed on native failure | **V-SEND.R-CALL-UPLOAD** — closed by **#328** |
| `synara/src/app/features/room/RoomInput.tsx:911` | GIF picker legacy branch downloads the GIF, optionally encrypts it, then calls `mx.uploadContent` before `mx.sendMessage` | **Not reachable** when `sendComposerGifWithNativeOwner` sees a logged-in native session; native GIF send uses `matrix_send_attachment` | **Composer GIF send / #264 legacy-web fallback**; not a GIF-pack residual. The [GIF-pack audit](v-send-gif-pack-audit.md) found no collection-management surface. |
| `synara/src/app/features/room/msgContent.ts:35` | Video thumbnail generation uploads the thumbnail through `mx.uploadContent`; invoked while building legacy attachment message content | **Not reachable** after native attachment ownership succeeds; native attachment bytes and encryption are sent through `matrix_send_attachment` | **Composer attachment / V-SEND.1 + V-SEND.5 legacy-web fallback**; no new SCOREBOARD residual |
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
| **V-SEND.R-CALL-UPLOAD** | **Closed by #328**. The `CallWidgetDriver` native owner handles logged-in desktop uploads; `client.uploadContent` is retained only as the non-native/web callback. |
| **V-SEND.R-GIF-PACK** | **NOOP** — the picker has no pack/collection-management surface. The `RoomInput` GIF upload is a native-send legacy-web fallback, not pack management; see [audit](v-send-gif-pack-audit.md). |
| **V-SEND.R-PACK-UPLOAD** | #314 is correct for compact desktop uploads. The shared JS helper remains for web/legacy reachability, but there is no native desktop fallthrough in `CompactUploadCardRenderer`. |
| Composer attachments / GIF send | Native owners are already landed (#248/#258/#264); remaining JS upload text is fallback evidence, not a new native-session residual. |

The GIF-pack scoreboard entry is **NOOP**, call-upload is closed by **#328**,
and thumbnails are closed by **#325**. This audit records the remaining
fallback-only usages without opening new native-session residuals.

## Verification

The production scan was limited to `synara/src` and excluded tests. Executable
matches were checked with:

```text
rg -n 'mx\.uploadContent|uploadContent\s*\(\s*mx' synara/src
```

Comments documenting the native fail-closed boundary and the
`CompactUploadCardRenderer` implementation were not counted as production
call sites.
