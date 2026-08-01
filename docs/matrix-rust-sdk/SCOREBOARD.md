# Matrix Rust full-replacement — scoreboard

| Field | Value |
|-------|-------|
| Updated | 2026-08-01 |
| Tip | `5116c9da` on `feature/matrix-rust-sdk-full-replacement` |
| Production `matrix-js-sdk` import files (`synara/src`) | **159** (plan baseline was **220**) |
| Dual backend | **false** (forbidden) |
| Umbrella #39 | **Do not merge** without explicit user approval |

## Operator index — timeline live proofs

These are docs-only operator checklists for the selected native desktop path.
All three live proofs remain **Not confirmed** until an authenticated desktop
run passes the relevant checklist.

| Proof                         | Operator checklist                                                                 | Live proof        |
| ----------------------------- | ---------------------------------------------------------------------------------- | ----------------- |
| V-TIMELINE.C3 stream/delta    | [v-timeline-c3-stream-verify.md](v-timeline-c3-stream-verify.md)                   | **Not confirmed** |
| V-TIMELINE.C4 media/render    | [v-timeline-c4-media-render-verify.md](v-timeline-c4-media-render-verify.md)       | **Not confirmed** |
| V-TIMELINE.C5 pins/notes/jump | [v-timeline-c5-pins-notes-jump-verify.md](v-timeline-c5-pins-notes-jump-verify.md) | **Not confirmed** |

## Done (high level)

| Area | Evidence |
|------|----------|
| Crypto vertical | V-CRYPTO done earlier |
| Core send | text/attachment/reaction/poll/thread/sticker/GIF native |
| Rooms | hierarchy + space writers |
| Auth | SSO removed; token non-retention; register; password reset; login-flow discovery; UIA multi-stage non-retention; **loginUtil fail-closed #279** |
| Poll-in-thread | #282 |
| Timeline contract | #240 native DTO/stream/actions + presenter code |
| Cutover policy | Approved; residual map #286; pack-read inventory #287 |
| Send residual inventories | forward #290; avatar #291; pack-write #292 (docs only) |
| V-SEND.R-EDIT | **#283** native `m.replace` merged |
| V-TIMELINE.C3 checklist | **#294** docs-only stream verify checklist |
| V-TIMELINE.C4 checklist | **docs-only** media/render verify checklist ([v-timeline-c4-media-render-verify.md](v-timeline-c4-media-render-verify.md)) |
| V-TIMELINE.C5 checklist | **docs-only** pins/notes/jump verify checklist ([v-timeline-c5-pins-notes-jump-verify.md](v-timeline-c5-pins-notes-jump-verify.md)) |
| V-TIMELINE.C1 | **#285** NativeTimelinePresenter owns RoomView |
| V-TIMELINE.C2 | **#289** delete `RoomTimeline` (imports **165→164**, allowlist **169→168**) |
| V-SEND.R-FORWARD | **#296** legacy MessageForwardItem + forward.ts deleted |
| V-TIMELINE residual truth | **#298** C1/C2 done notes |
| V-SEND.R-PACK-READ | **#297** snapshot; **#318** subscribe; **#320** room ids without `mx.getRoom` (imports **163→159**, allowlist **167→163**); JS utils delete remains V-BURN-gated |
| V-SEND.R-AVATAR-UPLOAD | **#303** native `matrix_upload_media` + set own avatar/display name; Profile.tsx fail-closed |
| V-SEND.R-PACK-WRITE personal | **#306** `matrix_set_user_image_pack` + UserImagePack fail-closed |
| V-SEND.R-PACK-WRITE global | **#309** `matrix_set_global_image_packs` + GlobalPacks fail-closed |
| V-SEND.R-PACK-WRITE room | **#310** `matrix_set_room_image_pack` + RoomPacks/RoomImagePack fail-closed |
| V-SEND.R-ROOM-PROFILE | **#313** `matrix_set_room_name`/`matrix_set_room_topic`/`matrix_set_room_avatar` + RoomProfile.tsx fail-closed |
| V-SEND.R-PACK-UPLOAD | **#314** reuses `matrix_upload_media` via CompactUploadCardRenderer fail-closed (pack + compact image uploads) |
| V-SEND.R-GIF-PACK | **NOOP** — `GifPicker` exposes provider search/selection only; selected GIF send is native via #264 and no GIF pack/collection owner exists |
| V-SEND.R-CALL-UPLOAD | **#328** `CallWidgetDriver.uploadFile` → `uploadCallWidgetFileWithNativeOwner` / `matrix_upload_media` fail-closed |
| Composer thumbnail (msgContent) | **#325** video thumbnails via `uploadMediaNative` / `matrix_upload_media` fail-closed |
| V-SEND.R-DEVTOOL | [Docs-only inventory](v-send-devtool-inventory.md): all three developer-tools runtime files still use the JS client; implementation remains low priority |
| CI | Parallel Validate #284 |

## In flight

| PR | What |
|----|------|
| (docs) | Next: pack-read **JS utils delete** (V-BURN-gated); **C3–C5** live verify |

## Left (ordered)

### Timeline cutover
1. ~~**C1** land #285~~ ✅
2. ~~**C2** delete RoomTimeline~~ ✅ #289
3. **C3** live re-verify stream deltas (checklist #294; residual truth #298)
4. **C4** media/render parity on selected presenter (checklist [v-timeline-c4-media-render-verify.md](v-timeline-c4-media-render-verify.md))
5. **C5** pins/notes/jump live proof (checklist [v-timeline-c5-pins-notes-jump-verify.md](v-timeline-c5-pins-notes-jump-verify.md))

### Send / media residuals (inventories done; implementation remains where noted)
- ~~**V-SEND.R-FORWARD**~~ **#296**
- ~~**V-SEND.R-PACK-READ** snapshot~~ **#297**; ~~**subscribe**~~ **#318**; ~~**room ids**~~ **#320** (JS utils deletion remains gated on the non-native web fallback)
- ~~**V-SEND.R-PACK-WRITE** personal~~ **#306**; ~~**global**~~ **#309**; ~~**room**~~ **#310**; ~~**PACK-UPLOAD**~~ **#314** (reuses `matrix_upload_media` #303 via CompactUploadCardRenderer)
- ~~**V-SEND.R-AVATAR-UPLOAD** user profile~~ **#303**; ~~**R-ROOM-PROFILE**~~ **#313** (room-avatar media upload covered by #314 CompactUploadCardRenderer path)
- ~~**V-SEND.R-CALL-UPLOAD**~~ **#328** (`CallWidgetDriver.uploadFile` → `uploadCallWidgetFileWithNativeOwner`)
- ~~**V-SEND.R-GIF-PACK**~~ **NOOP** ([audit](v-send-gif-pack-audit.md)); provider search/selection only, with no pack or collection owner
- ~~**Composer thumbnail (msgContent)**~~ **#325** via `uploadMediaNative`; remaining JS upload calls are legacy-web fallbacks only ([call-site audit](v-send-upload-residual-audit.md))
- **V-SEND.R-DEVTOOL** (implementation remains low priority; [inventory complete](v-send-devtool-inventory.md))

### Convergence
- **V-BURN.1–3** zero live JS client + drop npm `matrix-js-sdk` after residual owners clear

### Not residual free-for-all
- Widgets/calls polish beyond residual IDs — full verticals after burn queue allows
