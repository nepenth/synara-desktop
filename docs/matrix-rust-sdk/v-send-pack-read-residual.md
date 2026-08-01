# V-SEND.R-PACK-READ — sticker/emoji pack read residual inventory

| Field | Value |
|-------|-------|
| Status | **Inventory (docs only)** — no product code in this PR |
| Tip SHA | `76f10138a629b3aaef2c0f37bf1ccdbaf793c892` (merge #286 V-TIMELINE cutover residual map) |
| Base | `feature/matrix-rust-sdk-full-replacement` |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) — physical deletion inside each owning slice |
| Related | V-SEND sticker/GIF **#264** (native send), V-SEND residual inventory, V-TIMELINE #240 (HOLD) |

> **Scope guard.** Docs only. No product code in `product.rs` or any TS. Does
> not touch open **#240** (V-TIMELINE, HOLD) or **#39** (umbrella). No cutover.

---

## 1. Sticker send native (#264) vs pack read residual

Sticker **send** is fully native since **#264** (`matrix_send_sticker` →
`Room::send(m.sticker)`; `sendComposerStickerWithNativeOwner`), and GIF send
rides the native attachment owner (`image/gif` bytes over
`matrix_send_attachment`). What remains on the live `matrix-js-sdk` client is
the **read** side of the emoji/sticker **pack** surface: which packs exist,
which are enabled, and their metadata. The frontend still reads the
`PoniesEmoteRooms` account-data event and `PoniesRoomEmotes` / `PoniesUserEmotes`
state/account-data events through `mx.getAccountData` / `mx.getStateEvent`
wrappers and subscribes to them via `useAccountDataCallback` /
`useStateEventCallback` on the live client. There is **no** native pack-read
projection or subscription, so the composer's sticker/emoji picker and the
settings surfaces still depend on the JS client for pack discovery. This
inventory scopes that read residual as **V-SEND.R-PACK-READ**; the matching
**write** side (add/remove/enable/update packs) is a separate residual
(V-SEND.R-PACK-WRITE) and is explicitly out of scope here.

---

## 2. Residual table — V-SEND.R-PACK-READ

| Path | Role | Gap | ID |
|------|------|-----|----|
| `synara/src/app/plugins/custom-emoji/utils.ts` | `getGlobalImagePacks` / `getRoomImagePacks` / `getUserImagePack` read `PoniesEmoteRooms` / `PoniesRoomEmotes` / `PoniesUserEmotes` via `getAccountData` / `getStateEvent` (from `utils/room`) | No native pack-read projection; reads live `matrix-js-sdk` account-data/state | **V-SEND.R-PACK-READ** |
| `synara/src/app/hooks/useImagePacks.ts` | `useUserImagePack` / `useGlobalImagePacks` / `useRoomImagePack(s)` / `useRelevantImagePacks` subscribe via `useAccountDataCallback` / `useStateEventCallback` on `mx` | No native pack subscription; JS event listeners on live client | **V-SEND.R-PACK-READ** |
| `synara/src/app/hooks/useImagePackRooms.ts` | `useImagePackRooms` resolves candidate pack rooms from `mx.getRoom` + `getAllParents` | No native room→pack-room resolution; JS `mx.getRoom` | **V-SEND.R-PACK-READ** |
| `synara/src/app/components/emoji-board/EmojiBoard.tsx` | Renders pack previews via `useRelevantImagePacks` + media URL resolution | Pack preview display depends on JS pack-read + media URL resolution | **V-SEND.R-PACK-READ** (display) |
| `synara/src/app/components/editor/autocomplete/EmoticonAutocomplete.tsx` | Emoticon autocomplete via `useRelevantImagePacks` | No native pack-read for autocomplete suggestions | **V-SEND.R-PACK-READ** |
| `synara/src/app/components/image-pack-view/UserImagePack.tsx` | Reads personal pack via `useUserImagePack` | No native `PoniesUserEmotes` read | **V-SEND.R-PACK-READ** |
| `synara/src/app/features/settings/emojis-stickers/UserPack.tsx` | Reads personal pack via `useUserImagePack` | No native `PoniesUserEmotes` read | **V-SEND.R-PACK-READ** |
| `synara/src/app/features/settings/emojis-stickers/GlobalPacks.tsx` | Reads global packs via `useGlobalImagePacks` | No native `PoniesEmoteRooms` read | **V-SEND.R-PACK-READ** |
| `synara/src/app/features/common-settings/emojis-stickers/RoomPacks.tsx` | Reads room packs via `useRoomImagePacks` | No native `PoniesRoomEmotes` read | **V-SEND.R-PACK-READ** |

**Note:** pack **preview** display is media-adjacent (authenticated media
download / V-TIMELINE). The read residuals above are the account-data/state-event
owners; the actual media bytes for previews belong to the media vertical
(V-TIMELINE / #240 HOLD).

---

## 3. Proposed slice — native pack-read projection

When this residual is claimed, the native slice should expose a read-only pack
projection over IPC and delete the JS read/subscription owners. Proposed IPC
names (read-only, fail-closed):

- `matrix_get_global_image_packs` — return enabled global packs from
  `PoniesEmoteRooms` account-data.
- `matrix_get_room_image_packs` — return `PoniesRoomEmotes` state packs for a
  room (and optionally its parent spaces).
- `matrix_get_user_image_pack` — return the personal `PoniesUserEmotes` pack.
- `matrix_subscribe_image_packs` — push pack account-data/state changes to the
  frontend (replaces `useAccountDataCallback` / `useStateEventCallback`).
- `matrix_get_image_pack_media` — resolve pack image/avatar MXC to a usable
  media URL/bytes (media-adjacent; coordinate with V-TIMELINE).

**Deletion list** (physical deletion per [full-vertical-policy.md](full-vertical-policy.md)):
`custom-emoji/utils.ts` pack-read functions, `useImagePacks.ts`,
`useImagePackRooms.ts`, and the `useRelevantImagePacks`/`useGlobalImagePacks`/
`useRoomImagePacks`/`useUserImagePack` consumers' JS read paths in
`EmojiBoard.tsx`, `EmoticonAutocomplete.tsx`, `UserImagePack.tsx`, `UserPack.tsx`,
`GlobalPacks.tsx`, `RoomPacks.tsx`.

**Fail-closed:** on a native logged-in session, absence/failure of any
`matrix_get_*_image_packs` command is terminal — the picker/autocomplete must
not fall through to `mx.getAccountData` / `mx.getStateEvent`. Legacy JS read
paths remain only for non-native web sessions.

---

## 4. Non-goals / out of scope

| Item | Status |
|------|--------|
| **V-SEND.R-PACK-WRITE** (add/remove/enable/update packs: `RoomPacks.tsx`, `GlobalPacks.tsx`, `UserPack.tsx`, `RoomImagePack.tsx`, `UserImagePack.tsx` writes) | Separate residual — not this slice |
| Pack image/avatar **upload** (`ImageTile.tsx` / `PackMeta.tsx` → `state/upload.ts` → `mx.uploadContent`) | V-SEND.R-PACK-UPLOAD — separate |
| GIF pack/collection management | V-SEND.R-GIF-PACK — separate |
| Timeline media **display** (GIF playback, pack preview media bytes, authenticated media download) | **V-TIMELINE** — do not edit #240 (HOLD) |
| Umbrella merge to `main` | **#39** — needs explicit user approval |
| Cutover / dual-backend removal | #240 HOLD; no cutover |

---

## 5. Self-eval

**Confidence: high** for the inventory. I traced the pack-read surface from the
read helpers (`custom-emoji/utils.ts`) through the subscription hooks
(`useImagePacks.ts`, `useImagePackRooms.ts`) to every consumer
(`EmojiBoard.tsx`, `EmoticonAutocomplete.tsx`, `UserImagePack.tsx`, `UserPack.tsx`,
`GlobalPacks.tsx`, `RoomPacks.tsx`) and confirmed all read on the live
`matrix-js-sdk` client via `getAccountData` / `getStateEvent` /
`useAccountDataCallback` / `useStateEventCallback`. Sticker/GIF **send** is
native (#264); only the pack **read** projection is residual. Possible missed
files: any pack-read helper re-exported behind a barrel in the emoji/pack trees
— verify during implementation with a full `grep -rn "matrix-js-sdk"` over
`custom-emoji`, `image-pack-view`, and the emoji/sticker settings dirs.
