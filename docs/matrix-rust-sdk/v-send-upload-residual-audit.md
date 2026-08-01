# V-SEND upload call-site audit after #314

| Field | Value |
|-------|-------|
| Status | **Docs-only audit** — no product code changed |
| Measured tip | `8940f6ea` (`feature/matrix-rust-sdk-full-replacement` worktree tip) |
| Scope | Production `mx.uploadContent` / `uploadContent(mx, …)` usages under `synara/src`, excluding the `CompactUploadCardRenderer` native path from #314 |
| Guard | Do not touch `main` or umbrella PR **#39** |

## Finding

The source scan finds four `mx.uploadContent`/`uploadContent(mx, …)` call
sites, plus one call-widget equivalent (`client.uploadContent`). They do not
all represent native-session residuals. The composer attachment and GIF
owners are native-first and fail closed on a desktop native Matrix session;
their JS uploads remain only for the non-native web/legacy route. The shared
upload helper is still source-reachable from those web routes and from
compact-upload consumers, while compact desktop uploads themselves use
`matrix_upload_media` after #314.

## Call-site inventory

| Source | Production operation | Reachability on a native desktop session | Residual mapping |
|--------|----------------------|------------------------------------------|------------------|
| `synara/src/app/plugins/call/CallWidgetDriver.ts:318` | `client.uploadContent(file)` from the widget `uploadFile` implementation; the same driver also owns widget media config/download | **Reachable** — no native widget media owner or fail-closed gate | **V-SEND.R-CALL-UPLOAD** (active; correctly left open on the SCOREBOARD) |
| `synara/src/app/features/room/RoomInput.tsx:911` | GIF picker legacy branch downloads the GIF, optionally encrypts it, then calls `mx.uploadContent` before `mx.sendMessage` | **Not reachable** when `sendComposerGifWithNativeOwner` sees a logged-in native session; native GIF send uses `matrix_send_attachment` | **Composer GIF send / #264 legacy-web fallback**; not a new `R-GIF-PACK` upload residual. `R-GIF-PACK` remains the separate picker pack/collection-management residual. |
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
| **V-SEND.R-CALL-UPLOAD** | Still an active production residual at `CallWidgetDriver.ts:318`. |
| **R-GIF-PACK** | Still an active picker pack/collection-management residual. The `RoomInput` GIF upload is a native-send legacy-web fallback, not pack management. |
| **V-SEND.R-PACK-UPLOAD** | #314 is correct for compact desktop uploads. The shared JS helper remains for web/legacy reachability, but there is no native desktop fallthrough in `CompactUploadCardRenderer`. |
| Composer attachments / GIF send | Native owners are already landed (#248/#258/#264); remaining JS upload text is fallback evidence, not a new native-session residual. |

No SCOREBOARD status change is required. This audit closes the call-site
inventory gap and makes the remaining active IDs and fallback-only usages
explicit.

## Verification

The production scan was limited to `synara/src` and excluded tests. Executable
matches were checked with:

```text
rg -n 'mx\.uploadContent|uploadContent\s*\(\s*mx' synara/src
```

Comments documenting the native fail-closed boundary and the
`CompactUploadCardRenderer` implementation were not counted as production
call sites.
