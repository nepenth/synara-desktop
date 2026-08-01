# V-SEND.R-PACK-READ — sticker/emoji pack read residual inventory

| Field | Value |
|-------|-------|
| Status | **Snapshot #297 + subscribe #318 DONE** — live signal re-snapshot landed; **residual:** physical delete of JS read helpers (web fallback) + `useImagePackRooms` JS room resolution |
| Tip SHA | `95ad2656` (after #318 pack-read subscribe) |
| Base | `feature/matrix-rust-sdk-full-replacement` |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) — physical deletion inside each owning slice |
| Related | V-SEND sticker/GIF **#264** (native send), V-SEND residual inventory, V-SEND.R-PACK-WRITE **#292** (write owners), V-TIMELINE #240 (HOLD) |

> **Scope guard.** Docs only. No product code in `product.rs` or any TS. Does
> not touch open **#240** (V-TIMELINE, HOLD) or **#39** (umbrella). No cutover.

---

## 1. Sticker send native (#264) vs pack read residual

Sticker **send** is fully native since **#264** (`matrix_send_sticker` →
`Room::send(m.sticker)`; `sendComposerStickerWithNativeOwner`), and GIF send
rides the native attachment owner (`image/gif` bytes over
`matrix_send_attachment`). The pack **read** surface — which packs exist, which
are enabled, and their metadata — was inventoried here and its **snapshot get**
is now native since **#297**: `matrix_get_user_image_pack` /
`matrix_get_room_image_packs` / `matrix_get_global_image_packs` IPC plus the
`nativeImagePackOwner` / `nativeImagePack` TS owners, and the `useImagePacks.ts`
hooks now read through the native path and **fail-closed** on a native desktop
session (no fall-through to `mx.getAccountData` / `mx.getStateEvent`).

What **remains** on the live `matrix-js-sdk` client after #297 + #318:

- **Room→pack-room resolution** — `useImagePackRooms.ts` still resolves candidate
  pack rooms from `mx.getRoom` + `getAllParents` on the live client.
- **Physical delete** — the JS read helpers in `custom-emoji/utils.ts` and the
  JS read/subscription paths are still present (kept for the non-native web
  fallback); they are deleted once the native read path is complete.

This inventory scopes that read residual as **V-SEND.R-PACK-READ**; the matching
**write** side (add/remove/enable/update packs) is a separate residual
(V-SEND.R-PACK-WRITE, inventory **#292**) and is explicitly out of scope here.

---

## 2. Residual table — V-SEND.R-PACK-READ (remaining after #297 + #318)

**Snapshot get is DONE (#297) and live subscribe is DONE (#318).** The table
below is the **remaining** residual after both landed. The read hooks
(`useUserImagePack`, `useGlobalImagePacks`, `useRoomImagePack(s)`,
`useRoomsImagePacks`, `useRelevantImagePacks`) now read natively and fail-closed
on desktop, and re-snapshot on the `matrix-image-packs-updated` signal; the JS
account-data/state callbacks in `useImagePacks.ts` remain only for the
non-native web fallback.

| Path | Role | Gap | ID |
|------|------|-----|----|
| `synara/src/app/plugins/custom-emoji/utils.ts` | `getGlobalImagePacks` / `getRoomImagePack(s)` / `getUserImagePack` / `makeImagePacks` read `PoniesEmoteRooms` / `PoniesRoomEmotes` / `PoniesUserEmotes` via `getAccountData` / `getStateEvent` | **Read-only helpers** — sole consumer is `useImagePacks.ts` (web fallback). **Not used by the write side** (see note below). Physically delete once web fallback is dropped | **V-SEND.R-PACK-READ** (delete) |
| `synara/src/app/hooks/useImagePackRooms.ts` | `useImagePackRooms` resolves candidate pack rooms from `mx.getRoom` + `getAllParents` | No native room→pack-room resolution; JS `mx.getRoom`; feeds `RoomInput.tsx`, `PowersEditor.tsx`, `EmojiBoard.tsx`, `EmoticonAutocomplete.tsx` | **V-SEND.R-PACK-READ** (JS consumer) |
| `synara/src/app/components/emoji-board/EmojiBoard.tsx` | Renders pack previews via `useRelevantImagePacks` (native-backed) + `imagePackRooms` from JS `useImagePackRooms` | Pack preview display still depends on JS room resolution + media URL resolution | **V-SEND.R-PACK-READ** (display) |
| `synara/src/app/components/editor/autocomplete/EmoticonAutocomplete.tsx` | Emoticon autocomplete via `useRelevantImagePacks` (native-backed) + `imagePackRooms` from JS `useImagePackRooms` | Autocomplete still depends on JS room resolution | **V-SEND.R-PACK-READ** (JS consumer) |
| `synara/src/app/hooks/useImagePacks.ts` (web fallback) | `useAccountDataCallback` / `useStateEventCallback` listeners on `mx` for non-native web sessions | Native path is fail-closed; JS listeners remain only for web. Delete once web fallback dropped | **V-SEND.R-PACK-READ** (delete) |

**Write vs read owner clarification (physical delete of `custom-emoji/utils.ts`):**
The pack-read helpers in `custom-emoji/utils.ts` are **read-only owners** — their
sole consumer is `useImagePacks.ts` (the read hooks). The **write** surfaces
(`GlobalPacks.tsx`, `RoomPacks.tsx`, `UserImagePack.tsx`, `RoomImagePack.tsx`)
do **not** call these read helpers directly; they read current pack state through
the native-backed hooks (`useGlobalImagePacks`, `useRoomsImagePacks`,
`useRoomImagePacks`, `useUserImagePack`) and write via `mx.setAccountData` /
`mx.sendStateEvent` (V-SEND.R-PACK-WRITE, #292). So deleting the read helpers
does **not** break the write side — the write surfaces keep working through the
native read hooks. The read helpers can be physically deleted once the
non-native web fallback is dropped; the write residual (#292) is a separate
slice and does not gate this deletion.

**Note:** pack **preview** display is media-adjacent (authenticated media
download / V-TIMELINE). The read residuals above are the account-data/state-event
owners; the actual media bytes for previews belong to the media vertical
(V-TIMELINE / #240 HOLD).

---

## 3. Native pack-read projection — landed vs remaining

**Landed (#297):** the read-only snapshot get commands over IPC, fail-closed:

- `matrix_get_global_image_packs` — return enabled global packs from
  `PoniesEmoteRooms` account-data. ✅ **#297**
- `matrix_get_room_image_packs` — return `PoniesRoomEmotes` state packs for a
  room (and optionally its parent spaces). ✅ **#297**
- `matrix_get_user_image_pack` — return the personal `PoniesUserEmotes` pack.
  ✅ **#297**

TS owners `nativeImagePackOwner.ts` / `nativeImagePack.ts` and the
`useImagePacks.ts` native fail-closed path are also landed (#297).

**Landed (#318, subscribe):**
- Rust: session-scoped `NativeImagePackOwner` — `add_event_handler` on
  `AnyGlobalAccountDataEvent` (PoniesUserEmotes / PoniesEmoteRooms) and
  `AnySyncStateEvent` (PoniesRoomEmotes); emits `matrix-image-packs-updated`
  (sessionGeneration only — signal, never pack content).
- TS: `useImagePacks.ts` `useNativeImagePackRefreshToken` listens for
  `matrix-image-packs-updated` and bumps a refresh token so the native snapshot
  effects re-run (fail-closed on desktop). No dual_backend.

**Remaining (this residual):**

- `useImagePackRooms.ts` — JS `mx.getRoom` + `getAllParents` room→pack-room
  resolution (feeds `RoomInput.tsx`, `PowersEditor.tsx`, `EmojiBoard.tsx`,
  `EmoticonAutocomplete.tsx`).
- `matrix_get_image_pack_media` — resolve pack image/avatar MXC to a usable
  media URL/bytes (media-adjacent; coordinate with V-TIMELINE).

**Deletion list** (physical deletion per [full-vertical-policy.md](full-vertical-policy.md),
once web fallback dropped): `custom-emoji/utils.ts` pack-read functions
(read-only owners; write side unaffected — see §2), the JS
`useAccountDataCallback` / `useStateEventCallback` fallback paths in
`useImagePacks.ts`, and `useImagePackRooms.ts` (JS `mx.getRoom` +
`getAllParents`). The `useRelevantImagePacks`/`useGlobalImagePacks`/
`useRoomImagePacks`/`useUserImagePack` consumers (`EmojiBoard.tsx`,
`EmoticonAutocomplete.tsx`, `UserImagePack.tsx`, `UserPack.tsx`, `GlobalPacks.tsx`,
`RoomPacks.tsx`) already read through the native-backed hooks; only their JS
room-resolution dependency (`useImagePackRooms`) remains.

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

**Confidence: high** for this truth-up. I re-traced the pack-read surface after
**#297** (snapshot) and **#318** (subscribe) landed: the snapshot get commands
(`matrix_get_user_image_pack` / `matrix_get_room_image_packs` /
`matrix_get_global_image_packs`), the `nativeImagePackOwner` / `nativeImagePack`
TS owners, the `useImagePacks.ts` native fail-closed path, and the live
`NativeImagePackOwner` → `matrix-image-packs-updated` re-snapshot signal are all
confirmed landed. The remaining residual is the **physical delete** of the
read-only `custom-emoji/utils.ts` helpers (sole consumer `useImagePacks.ts`;
write side #292 unaffected), the JS **room resolution** (`useImagePackRooms.ts`)
that still feeds `RoomInput.tsx`, `PowersEditor.tsx`, `EmojiBoard.tsx`,
`EmoticonAutocomplete.tsx`, and the web-fallback listeners in `useImagePacks.ts`.
Sticker/GIF **send** is native (#264). Possible
missed files: any pack-read helper re-exported behind a barrel in the emoji/pack
trees — verify during implementation with a full `grep -rn "matrix-js-sdk"` over
`custom-emoji`, `image-pack-view`, and the emoji/sticker settings dirs.


## 7. Implementation close

**Landed #297 (snapshot):**
- Rust: `matrix_get_user_image_pack` / `matrix_get_room_image_packs` / `matrix_get_global_image_packs`
- TS: `nativeImagePackOwner` + `nativeImagePack` + `useImagePacks` native path (fail-closed on desktop)

**Landed #318 (subscribe):**
- Rust: session-scoped `NativeImagePackOwner` (account-data + room state handlers)
- Signal: Tauri `matrix-image-packs-updated` (sessionGeneration only; re-snapshot via get IPC)
- TS: `useImagePacks` native refresh token listen (fail-closed desktop)

**Residual follow-on (this doc):**
- Physical delete of JS pack-read helpers (`custom-emoji/utils.ts` read-only
  owners) + `useImagePacks.ts` web-fallback listeners once non-native web path is
  retired — write side (#292) unaffected
- `useImagePackRooms.ts` JS `mx.getRoom` + `getAllParents` room→pack-room resolution
- dual_backend false; pack-write/upload out of scope
