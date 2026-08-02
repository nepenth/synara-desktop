# Matrix Rust full-replacement — scoreboard

| Field                                                  | Value                                                    |
| ------------------------------------------------------ | -------------------------------------------------------- |
| Updated                                                | 2026-08-01                                               |
| Tip                                                    | `9fb341af` on `feature/matrix-rust-sdk-full-replacement`; #405, #407, #438, #439, and #446 are merged at this tip |
| Production `matrix-js-sdk` import files (`synara/src`) | **152** (plan baseline was **220**)                      |
| Product lane                                           | **#446 merged** the behavior-preserving `product.rs` extract/split; subsequent product fan-out remains separate from this docs-only refresh |
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

| Area                            | Evidence                                                                                                                                                                                         |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Crypto vertical                 | V-CRYPTO done earlier                                                                                                                                                                            |
| Core send                       | text/attachment/reaction/poll/thread/sticker/GIF native                                                                                                                                          |
| Rooms                           | hierarchy + space writers                                                                                                                                                                        |
| Auth                            | SSO removed; token non-retention; register; password reset; login-flow discovery; UIA multi-stage non-retention; **loginUtil fail-closed #279**                                                  |
| Poll-in-thread                  | #282                                                                                                                                                                                             |
| Timeline contract               | #240 native DTO/stream/actions + presenter code                                                                                                                                                  |
| Cutover policy                  | Approved; residual map #286; pack-read inventory #287                                                                                                                                            |
| Send residual inventories       | forward #290; avatar #291; pack-write #292 (docs only)                                                                                                                                           |
| V-SEND.R-EDIT                   | **#283** native `m.replace` merged                                                                                                                                                               |
| V-TIMELINE.C3 checklist         | **#294** docs-only stream verify checklist                                                                                                                                                       |
| V-TIMELINE.C4 checklist         | **docs-only** media/render verify checklist ([v-timeline-c4-media-render-verify.md](v-timeline-c4-media-render-verify.md))                                                                       |
| V-TIMELINE.C5 checklist         | **docs-only** pins/notes/jump verify checklist ([v-timeline-c5-pins-notes-jump-verify.md](v-timeline-c5-pins-notes-jump-verify.md))                                                              |
| V-TIMELINE.C1                   | **#285** NativeTimelinePresenter owns RoomView                                                                                                                                                   |
| V-TIMELINE.C2                   | **#289** delete `RoomTimeline` (imports **165→164**, allowlist **169→168**)                                                                                                                      |
| V-SEND.R-FORWARD                | **#296** legacy MessageForwardItem + forward.ts deleted                                                                                                                                          |
| V-TIMELINE residual truth       | **#298** C1/C2 done notes                                                                                                                                                                        |
| V-SEND.R-PACK-READ              | **#297** snapshot; **#318** subscribe; **#320** room ids; **#365** JS `get*` helpers + web fallbacks deleted (keep equals); imports **157→155** after #364 leave                                 |
| V-SEND.R-AVATAR-UPLOAD          | **#303** native `matrix_upload_media` + set own avatar/display name; Profile.tsx fail-closed                                                                                                     |
| V-SEND.R-PACK-WRITE personal    | **#306** `matrix_set_user_image_pack` + UserImagePack fail-closed                                                                                                                                |
| V-SEND.R-PACK-WRITE global      | **#309** `matrix_set_global_image_packs` + GlobalPacks fail-closed                                                                                                                               |
| V-SEND.R-PACK-WRITE room        | **#310** `matrix_set_room_image_pack` + RoomPacks/RoomImagePack fail-closed                                                                                                                      |
| V-SEND.R-ROOM-PROFILE           | **#313** `matrix_set_room_name`/`matrix_set_room_topic`/`matrix_set_room_avatar` + RoomProfile.tsx fail-closed                                                                                   |
| V-SEND.R-PACK-UPLOAD            | **#314** reuses `matrix_upload_media` via CompactUploadCardRenderer fail-closed (pack + compact image uploads)                                                                                   |
| V-SEND.R-GIF-PACK               | **NOOP** — `GifPicker` exposes provider search/selection only; selected GIF send is native via #264 and no GIF pack/collection owner exists                                                      |
| Room leave vertical             | **#364** `matrix_room_leave` + LeaveRoom/LeaveSpace native owner                                                                                                                                 |
| Room create vertical            | **#372** `matrix_room_create` + native owner; CreateRoom/Space/Chat + `/startdm` fail-closed (imports **154→153**)                                                                               |
| Room moderation writes          | **#375** native invite/kick/ban/unban/setPowerLevel writes; moderation write product vertical merged                                                                                             |
| Members-read member surfaces    | **#395** `matrix_room_members_snapshot` + Members settings native fail-closed; **#405 merged** wires drawer/lobby/mentions member snapshots; **#439 merged** adds native powers-bulk writes; power-level/creator reads remain residual |
| /leave command                  | **#371** useCommands Leave → leaveRoomWithNativeOwner                                                                                                                                            |
| Room join vertical              | **#369** `matrix_room_join` + native owner; all production `joinRoom` sites fail-closed (imports **155→154**)                                                                                    |
| Composer GIF/upload fallbacks   | **#363** RoomInput GIF + msgContent thumbnail JS upload fallbacks deleted                                                                                                                        |
| V-SEND.R-CALL-UPLOAD            | **#328** native upload; **#362** native known rooms; **#407 merged** native media config/download; inventoried CallWidget native desktop surfaces are closed ([residual](v-send-call-widget-residual.md)) |
| Product lane protocol            | **#438 merged** docs-only single-owner protocol for `product.rs`; **#439 merged** powers-bulk; **#446 merged** the extract/split |
| Composer thumbnail (msgContent) | **#325** video thumbnails via `uploadMediaNative` / `matrix_upload_media` fail-closed                                                                                                            |
| V-SEND.R-DEVTOOL                | [Docs-only inventory](v-send-devtool-inventory.md) · [implementation gate](v-send-devtool-inventory.md#implementation-gate): JS client remains; start only after C3–C5 live proofs; low priority |
| CI                              | Parallel Validate #284                                                                                                                                                                           |

## In flight

| PR     | What                                                                                           |
| ------ | ---------------------------------------------------------------------------------------------- |
| Post-#446 product fan-out | Separate product work; this docs-only refresh does not touch `product.rs` or claim fan-out completion. |

## Left (finish-line order)

1. **Members/power — native full vertical.** Leave/join/create closed
   (**#364/#369/#372**); `/leave` command **#371**; moderation writes
   invite/kick/ban/unban/setPowerLevel **#375 merged**; members-read first slice
   **#395** (`matrix_room_members_snapshot` + Members settings native). **#405**
   is merged at this tip and closes Room/MembersDrawer/Lobby/UserMentionAutocomplete
   member enumeration on native desktop. **#439** powers-bulk is also merged;
   power-level/creator reads remain residual. **#446** is merged, while subsequent
   product fan-out remains separate and unclaimed here. See
   [read residual inventory](v-rooms-members-read-residual.md),
   [P4.6 members](p4.6-members.md) and [P4.3 membership](p4.3-membership-unread.md).
2. **V-TIMELINE.C3–C5 — live proofs.** All three remain **Not confirmed**
   (C3 blocked without Docker Synapse harness in agent env). Operator index:
   [C3](v-timeline-c3-stream-verify.md),
   [C4](v-timeline-c4-media-render-verify.md),
   [C5](v-timeline-c5-pins-notes-jump-verify.md).
3. **V-SEND.R-DEVTOOL — native full vertical.** Inventory remains; implement
   after C3–C5 live or explicit reorder. See
   [implementation gate](v-send-devtool-inventory.md#implementation-gate).
4. **V-BURN.1 — no live `createClient`/`startClient` on desktop.** Not
   complete. See [V-BURN gates](d0-residual-completion.md) and
   [blockers](v-burn-readiness-snapshot.md).
5. **V-BURN.2 — zero production importers.** Current production importers
   **152**. See [taxonomy](v-burn-importer-taxonomy.md).
6. **V-BURN.3 — drop npm and obsolete JS bootstrap/store code.** After
   V-BURN.1/.2. See the
   [V-BURN completion gates](d0-residual-completion.md) and [blocker snapshot](v-burn-readiness-snapshot.md).

**V-BURN remains HOLD / Not ready and is not complete.** `dual_backend` remains
**forbidden**; [#39](https://github.com/nepenth/synara-desktop/pull/39) remains
gated and must not be merged to `main` without explicit user approval.
