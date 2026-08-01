# V-SEND.R-PACK-WRITE — sticker/emoji pack write residual inventory

| Field | Value |
|-------|-------|
| Status | **Inventory (docs only)** — no product code in this PR |
| Tip SHA | `310f4487` (merge #291 avatar inventory; after #290 forward + #287 pack-read inventories) |
| Base | `feature/matrix-rust-sdk-full-replacement` |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) — physical deletion inside each owning slice |
| Related | V-SEND.R-PACK-READ **#287** (landed), V-SEND sticker/GIF **#264** (native send), V-SEND.R-PACK-UPLOAD (same surface, covered here), V-SEND residual inventory |

> **Scope guard.** Docs only. No product code in `product.rs` or any TS. Does
> not touch open **#283** (edit), **#285** (C1), **#289** (C2), or **#39** (umbrella). No cutover.

---

## 1. Pack write vs pack read vs pack upload

Sticker/GIF **send** is fully native since **#264** (`matrix_send_sticker` /
`matrix_send_attachment`), and the pack **read** projection was inventoried in
**#287** (V-SEND.R-PACK-READ). What remains on the live `matrix-js-sdk` client is
the **write** side of the emoji/sticker **pack** surface: creating/removing/
enabling/updating packs, and uploading pack images/avatars. The frontend still
writes through the live client via `mx.setAccountData` (global `PoniesEmoteRooms`
and personal `PoniesUserEmotes` account-data) and `mx.sendStateEvent`
(`PoniesRoomEmotes` room state), and uploads pack media via `mx.uploadContent`.
There is **no** native pack-write command or native media upload for packs, so
the settings surfaces and the pack editor still depend on the JS client for every
write. This inventory scopes that write residual as **V-SEND.R-PACK-WRITE** and
the pack image/avatar upload as **V-SEND.R-PACK-UPLOAD** (same editor surface,
covered together here).

---

## 2. Residual table — V-SEND.R-PACK-WRITE (+ PACK-UPLOAD)

> **Status after #310:** personal (#306), global (#309), and room (#310) pack **writes** are native fail-closed. Rows below for those paths are historical inventory. **Active residual:** **PACK-UPLOAD** (`state/upload.ts` / `utils/matrix.ts` `uploadContent` for pack media).

| Path | Role | Gap | ID |
|------|------|-----|----|
| `synara/src/app/features/settings/emojis-stickers/GlobalPacks.tsx` | `applyChanges` writes `mx.setAccountData(PoniesEmoteRooms, updatedContent)` to **add/remove/enable** global pack references in `EmoteRoomsContent.rooms` (selected/removed `PackAddress`es) | No native global-pack write; JS `setAccountData` on live client | **V-SEND.R-PACK-WRITE** |
| `synara/src/app/features/common-settings/emojis-stickers/RoomPacks.tsx` | `CreatePackTile` writes `mx.sendStateEvent(roomId, PoniesRoomEmotes, content, stateKey)` to **create** a room pack; `applyChanges` writes `mx.sendStateEvent(roomId, PoniesRoomEmotes, {}, stateKey)` to **delete** room packs | No native room-pack create/delete; JS `sendStateEvent` on live client | **V-SEND.R-PACK-WRITE** |
| `synara/src/app/components/image-pack-view/UserImagePack.tsx` | `handleUpdate` writes `mx.setAccountData(PoniesUserEmotes, packContent)` to **update** the personal pack | No native personal-pack write; JS `setAccountData` on live client | **V-SEND.R-PACK-WRITE** |
| `synara/src/app/components/image-pack-view/RoomImagePack.tsx` | `handleUpdate` writes `mx.sendStateEvent(address.roomId, PoniesRoomEmotes, packContent, address.stateKey)` to **update** a room pack | No native room-pack update; JS `sendStateEvent` on live client | **V-SEND.R-PACK-WRITE** |
| `synara/src/app/components/image-pack-view/ImagePackContent.tsx` | Orchestrates the pack editor: builds `PackContent` (pack meta + images) from uploaded/edited/deleted images and calls `onUpdate` (the `setAccountData`/`sendStateEvent` above) | No native pack-content assembly; editor state + write on live client | **V-SEND.R-PACK-WRITE** |
| `synara/src/app/components/image-pack-view/ImageTile.tsx` | `ImageTileUpload` / `ImageTileEdit` stage pack image files and edits (shortcode/body/usage) into the editor | No native pack-image edit/delete staging | **V-SEND.R-PACK-WRITE** (editor) |
| `synara/src/app/components/image-pack-view/PackMeta.tsx` | `ImagePackProfileEdit` stages pack avatar upload + name/attribution edits into the editor | No native pack-meta/avatar edit staging | **V-SEND.R-PACK-WRITE** (editor) |
| `synara/src/app/state/upload.ts` | `createUploadAtom` / `useBindUploadAtom` drive `uploadContent(mx, file, …)` → `mx.uploadContent` for pack image/avatar upload | No native pack media upload; JS `uploadContent` on live client | **V-SEND.R-PACK-UPLOAD** |
| `synara/src/app/utils/matrix.ts` | `uploadContent` (line ~149) calls `mx.uploadContent(file, …)`; `getImageInfo` (line ~59) builds `IImageInfo` for uploaded pack images | No native media upload for packs; JS `mx.uploadContent` | **V-SEND.R-PACK-UPLOAD** |

**Note:** the pack **read** helpers (`custom-emoji/utils.ts` read functions,
`useImagePacks.ts`, `useImagePackRooms.ts`) and the read consumers
(`EmojiBoard.tsx`, `EmoticonAutocomplete.tsx`, `UserPack.tsx`, `GlobalPacks.tsx`
read, `RoomPacks.tsx` read) are **V-SEND.R-PACK-READ** (#287) — not this slice.
The `usePermissionItems.ts` entries for `PoniesRoomEmotes` (room/space settings)
are read-only power-level permission labels, not writes. The write residual here
is the `setAccountData` / `sendStateEvent` / `uploadContent` surface and the
editor that drives it.

---

## 3. Proposed slice — native pack-write + pack-upload commands

When this residual is claimed, the native slice should expose write commands over
IPC and delete the JS write/upload owners. Proposed IPC names (fail-closed):

- `matrix_set_global_image_packs` — replace the `PoniesEmoteRooms` account-data
  content (add/remove/enable global pack references) via native account-data set.
- `matrix_set_room_image_pack` — create/update/delete a `PoniesRoomEmotes` state
  pack for a room (empty `{}` content deletes; mirrors `sendStateEvent`).
- `matrix_set_user_image_pack` — replace the personal `PoniesUserEmotes`
  account-data pack content.
- `matrix_upload_pack_media` — upload pack image/avatar bytes and return the MXC
  (native media upload; coordinate with V-SEND media upload / V-TIMELINE).

**Deletion list** (physical deletion per [full-vertical-policy.md](full-vertical-policy.md)):
the `setAccountData` / `sendStateEvent` write paths in `GlobalPacks.tsx`,
`RoomPacks.tsx`, `UserImagePack.tsx`, `RoomImagePack.tsx`; the pack editor
orchestration in `ImagePackContent.tsx` (and its `ImageTile.tsx` / `PackMeta.tsx`
staging); and the pack upload path in `state/upload.ts` / `utils/matrix.ts`
(`uploadContent` for pack media). Keep the read side (#287) and the native send
(#264) intact. Verify no other consumers of `createUploadAtom` / `uploadContent`
remain for pack media before deletion (the composer attachment upload is a
separate V-SEND surface).

**Fail-closed:** on a native logged-in session, absence/failure of any
`matrix_set_*_image_pack*` / `matrix_upload_pack_media` command is terminal — the
settings/editor must not fall through to `mx.setAccountData` /
`mx.sendStateEvent` / `mx.uploadContent`. Legacy JS write paths remain only for
non-native web sessions.

---

## 4. Non-goals / out of scope

| Item | Status |
|------|--------|
| **V-SEND.R-PACK-READ** (pack discovery/subscription) | **#287** — landed; not this slice |
| Sticker/GIF **send** | **#264** — native; not a residual |
| Composer **attachment** upload/send | V-SEND attachments — separate surface (shares `state/upload.ts` / `utils/matrix.ts` `uploadContent`; coordinate deletion) |
| Timeline media **display** (GIF playback, pack preview media bytes, authenticated media download) | **V-TIMELINE** — do not edit #285/#289/#240 |
| Open product PRs | **#283** (edit), **#285** (C1), **#289** (C2) — do not touch |
| Umbrella merge to `main` | **#39** — needs explicit user approval |
| Cutover / dual-backend removal | #240 HOLD; no cutover |

---

## 5. Confidence

**Confidence: high** for the inventory. I traced the pack **write** surface from
the four write owners (`GlobalPacks.tsx` → `setAccountData(PoniesEmoteRooms)`,
`RoomPacks.tsx` → `sendStateEvent(PoniesRoomEmotes)`, `UserImagePack.tsx` →
`setAccountData(PoniesUserEmotes)`, `RoomImagePack.tsx` →
`sendStateEvent(PoniesRoomEmotes)`) through the shared editor
(`ImagePackContent.tsx` + `ImageTile.tsx` + `PackMeta.tsx`) and the upload path
(`state/upload.ts` → `utils/matrix.ts` `uploadContent` → `mx.uploadContent`), and
confirmed all write on the live `matrix-js-sdk` client. The read side (#287) and
the `usePermissionItems.ts` power-level labels are read-only and out of scope.
Possible missed files: any other consumer of `createUploadAtom` / `uploadContent`
for pack media, or a barrel re-export in the emoji/pack trees — verify during
implementation with a full `grep -rn "setAccountData\|sendStateEvent\|uploadContent"`
over `custom-emoji`, `image-pack-view`, and the emoji/sticker settings dirs.


## Implementation (pack-write slices)

Landed:
- `matrix_set_user_image_pack` (native `im.ponies.user_emotes` write) — **#306**
- `UserImagePack.tsx` fail-closed via `setUserImagePackNative` — **#306**
- `matrix_set_global_image_packs` (native `im.ponies.emote_rooms` write) — **#309**
- `GlobalPacks.tsx` fail-closed via `setGlobalImagePacksNative` — **#309**

- `matrix_set_room_image_pack` (native `im.ponies.room_emotes` create/update/delete; empty `{}` deletes) — **#310**
- `RoomPacks.tsx` / `RoomImagePack.tsx` fail-closed via `setRoomImagePackNative` — **#310**

Remaining:
- Pack image/avatar upload (**PACK-UPLOAD**) — still via `mx.uploadContent` in `state/upload.ts` / `utils/matrix.ts`; may reuse `matrix_upload_media` from #303. Open **#314**.
