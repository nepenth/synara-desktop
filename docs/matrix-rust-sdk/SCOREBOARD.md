# Matrix Rust full-replacement — scoreboard

| Field                                                  | Value                                                    |
| ------------------------------------------------------ | -------------------------------------------------------- |
| Updated                                                | 2026-08-01                                               |
| Tip                                                    | `4eeefa11` on `feature/matrix-rust-sdk-full-replacement` |
| Production `matrix-js-sdk` import files (`synara/src`) | **159** (plan baseline was **220**)                      |
| Dual backend                                           | **false** (forbidden)                                    |
| Umbrella #39                                           | **Do not merge** without explicit user approval          |

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

| Area                            | Evidence                                                                                                                                                                                               |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Crypto vertical                 | V-CRYPTO done earlier                                                                                                                                                                                  |
| Core send                       | text/attachment/reaction/poll/thread/sticker/GIF native                                                                                                                                                |
| Rooms                           | hierarchy + space writers                                                                                                                                                                              |
| Auth                            | SSO removed; token non-retention; register; password reset; login-flow discovery; UIA multi-stage non-retention; **loginUtil fail-closed #279**                                                        |
| Poll-in-thread                  | #282                                                                                                                                                                                                   |
| Timeline contract               | #240 native DTO/stream/actions + presenter code                                                                                                                                                        |
| Cutover policy                  | Approved; residual map #286; pack-read inventory #287                                                                                                                                                  |
| Send residual inventories       | forward #290; avatar #291; pack-write #292 (docs only)                                                                                                                                                 |
| V-SEND.R-EDIT                   | **#283** native `m.replace` merged                                                                                                                                                                     |
| V-TIMELINE.C3 checklist         | **#294** docs-only stream verify checklist                                                                                                                                                             |
| V-TIMELINE.C4 checklist         | **docs-only** media/render verify checklist ([v-timeline-c4-media-render-verify.md](v-timeline-c4-media-render-verify.md))                                                                             |
| V-TIMELINE.C5 checklist         | **docs-only** pins/notes/jump verify checklist ([v-timeline-c5-pins-notes-jump-verify.md](v-timeline-c5-pins-notes-jump-verify.md))                                                                    |
| V-TIMELINE.C1                   | **#285** NativeTimelinePresenter owns RoomView                                                                                                                                                         |
| V-TIMELINE.C2                   | **#289** delete `RoomTimeline` (imports **165→164**, allowlist **169→168**)                                                                                                                            |
| V-SEND.R-FORWARD                | **#296** legacy MessageForwardItem + forward.ts deleted                                                                                                                                                |
| V-TIMELINE residual truth       | **#298** C1/C2 done notes                                                                                                                                                                              |
| V-SEND.R-PACK-READ              | **#297** snapshot; **#318** subscribe; **#320** room ids without `mx.getRoom` (imports **163→159**, allowlist **167→163**); read-only JS `get*` helper deletion is now unblocked as finish-line item 1 |
| V-SEND.R-AVATAR-UPLOAD          | **#303** native `matrix_upload_media` + set own avatar/display name; Profile.tsx fail-closed                                                                                                           |
| V-SEND.R-PACK-WRITE personal    | **#306** `matrix_set_user_image_pack` + UserImagePack fail-closed                                                                                                                                      |
| V-SEND.R-PACK-WRITE global      | **#309** `matrix_set_global_image_packs` + GlobalPacks fail-closed                                                                                                                                     |
| V-SEND.R-PACK-WRITE room        | **#310** `matrix_set_room_image_pack` + RoomPacks/RoomImagePack fail-closed                                                                                                                            |
| V-SEND.R-ROOM-PROFILE           | **#313** `matrix_set_room_name`/`matrix_set_room_topic`/`matrix_set_room_avatar` + RoomProfile.tsx fail-closed                                                                                         |
| V-SEND.R-PACK-UPLOAD            | **#314** reuses `matrix_upload_media` via CompactUploadCardRenderer fail-closed (pack + compact image uploads)                                                                                         |
| V-SEND.R-GIF-PACK               | **NOOP** — `GifPicker` exposes provider search/selection only; selected GIF send is native via #264 and no GIF pack/collection owner exists                                                            |
| V-SEND.R-CALL-UPLOAD            | **#328** native upload; [CallWidgetDriver residual](v-send-call-widget-residual.md)                                                                                                                    |
| Composer thumbnail (msgContent) | **#325** video thumbnails via `uploadMediaNative` / `matrix_upload_media` fail-closed                                                                                                                  |
| V-SEND.R-DEVTOOL                | [Docs-only inventory](v-send-devtool-inventory.md) · [implementation gate](v-send-devtool-inventory.md#implementation-gate): JS client remains; start only after C3–C5 live proofs; low priority       |
| CI                              | Parallel Validate #284                                                                                                                                                                                 |

## In flight

| PR     | What                                                                                                                                                                                                                                                                                                                                                         |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| (docs) | **User-authorized finish-line order (2026-08-01):** execute [Left](#left-finish-line-order) in order **1→9**. This is an active queue, not a product-idle status; item 1 is unblocked now. Every product slice remains UI → Tauri IPC → live `matrix-sdk`, fail-closed on native failure, with the superseded JS owner physically deleted in the same slice. |

## Left (finish-line order)

1. **V-SEND.R-PACK-READ — delete the `get*` helpers now.** This is
   **unblocked**: Synara does not ship a separate browser product, so the
   read-only JS `get*` helpers and web-only fallback/listener path can be
   physically deleted. Retain `packAddressEqual`, `imageUsageEqual`, and
   `packMetaEqual`. See the [pack-read residual](v-send-pack-read-residual.md)
   and [importer taxonomy](v-burn-importer-taxonomy.md).
2. **Room leave/join/create lifecycle — native full vertical.** Re-home the
   UI through Tauri IPC to the live `matrix-sdk`, fail closed on native command
   absence/failure, and delete the superseded JS owner in the same slice. See
   [P6.9 room ops](p6.9-room-ops.md) and [P2.6 lifecycle](p2.6-destructive-lifecycle.md).
3. **Members/power — native full vertical.** Re-home member and power-level
   reads/writes through the native route, with physical JS-owner deletion and
   fail-closed desktop behavior. See [P4.6 members](p4.6-members.md) and
   [P4.3 membership](p4.3-membership-unread.md).
4. **V-TIMELINE.C3–C5 — live proofs.** All three remain **Not confirmed**;
   use the operator index: [C3](v-timeline-c3-stream-verify.md),
   [C4](v-timeline-c4-media-render-verify.md), and
   [C5](v-timeline-c5-pins-notes-jump-verify.md).
5. **V-SEND.R-DEVTOOL — native full vertical.** The inventory remains
   docs-only; start only after C3–C5 live proofs confirm. Follow the
   [implementation gate](v-send-devtool-inventory.md#implementation-gate).
6. **CallWidget residual — native widget vertical.** The upload owner is
   closed by #328, but `getMediaConfig`, `downloadFile`, and `getKnownRooms`
   remain documented JS residuals. See the [CallWidget residual inventory](v-send-call-widget-residual.md).
7. **V-BURN.1 — no live `createClient`/`startClient` on desktop.** Final
   convergence proof only; this is **not complete**. See the [V-BURN
   completion gates](d0-residual-completion.md) and [blocker snapshot](v-burn-readiness-snapshot.md).
8. **V-BURN.2 — zero production importers.** Final repository-wide importer
   audit remains **left**; current production importers are **159**. See the
   [importer taxonomy](v-burn-importer-taxonomy.md) and [blocker snapshot](v-burn-readiness-snapshot.md).
9. **V-BURN.3 — drop npm and obsolete JS bootstrap/store code.** Remove the
   `matrix-js-sdk` package, lockfile entries, startup/store/service-worker
   residue, and migration allowlist only after V-BURN.1/.2 are proven. See the
   [V-BURN completion gates](d0-residual-completion.md) and [blocker snapshot](v-burn-readiness-snapshot.md).

**V-BURN remains HOLD / Not ready and is not complete.** `dual_backend` remains
**forbidden**; [#39](https://github.com/nepenth/synara-desktop/pull/39) remains
gated and must not be merged to `main` without explicit user approval.
