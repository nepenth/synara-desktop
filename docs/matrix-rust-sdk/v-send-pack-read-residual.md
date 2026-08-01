# V-SEND.R-PACK-READ — sticker/emoji pack read residual inventory

| Field   | Value                                                                                                                                                                                                                                                                                                             |
| ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status  | **Snapshot #297 + subscribe #318 + room ids #320 DONE** — native desktop reads are fail-closed, subscription refresh is landed, and `useImagePackRooms` is pure graph resolution; **remaining:** physical delete of the JS read-helper functions in `custom-emoji/utils.ts`, gated on the non-native web fallback |
| Tip SHA | `e38bfdab68bd57e4f3110a812c5e4c5d543c1ff5`                                                                                                                                                                                                                                                                        |
| Base    | `feature/matrix-rust-sdk-full-replacement`                                                                                                                                                                                                                                                                        |
| Policy  | [full-vertical-policy.md](full-vertical-policy.md) — physical deletion inside each owning slice                                                                                                                                                                                                                   |
| Related | V-SEND sticker/GIF **#264** (native send), V-SEND residual inventory, V-SEND.R-PACK-WRITE **#292** (write owners), V-TIMELINE #240 (HOLD)                                                                                                                                                                         |

> **Scope guard.** Docs only. No product code in `product.rs` or any TS. Does
> not touch open **#240** (V-TIMELINE, HOLD), **#327** (V-BURN readiness), or
> **#39** (umbrella). No cutover; `dual_backend` is forbidden.

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

What **remains** after #318 and #320:

- **Physical delete** — the JS read-helper functions in
  `custom-emoji/utils.ts` remain only for the **non-native web** fallback. They
  are deleted when that fallback is retired; native desktop remains fail-closed
  and does not fall through to the JS read path.

The #318 native subscription is landed: `NativeImagePackOwner` emits the
`matrix-image-packs-updated` signal and `useImagePacks.ts` re-snapshots through
the native get IPC. The #320 room path is also landed:
`useImagePackRooms.ts` returns room IDs from the `roomToParents` graph only,
with no live `mx.getRoom`; consumers load packs by room ID through native IPC.

This inventory scopes that read residual as **V-SEND.R-PACK-READ**; the matching
**write** side (add/remove/enable/update packs) is a separate residual
(V-SEND.R-PACK-WRITE, inventory **#292**) and is explicitly out of scope here.

---

## 2. Residual table — V-SEND.R-PACK-READ (remaining after #320)

**Snapshot get, subscribe, and room-ID resolution are DONE (#297, #318,
#320).** The table below is the **remaining** residual. The read hooks (`useUserImagePack`,
`useGlobalImagePacks`, `useRoomImagePack(s)`, `useRoomsImagePacks`,
`useRelevantImagePacks`) now read natively and fail-closed on desktop; the JS
account-data/state callbacks in `useImagePacks.ts` remain only for the
non-native web fallback.

| Path                                                                            | Role                                                                                                                                                                                           | Gap                                                                                                                                                                                                                                                                                                                                                                                                                            | ID                              |
| ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------- |
| `synara/src/app/plugins/custom-emoji/utils.ts` (read-helper **functions only**) | `getGlobalImagePacks` / `getRoomImagePack(s)` / `getUserImagePack` / `makeImagePacks` read `PoniesEmoteRooms` / `PoniesRoomEmotes` / `PoniesUserEmotes` via `getAccountData` / `getStateEvent` | **Read-only helpers** — sole consumer is `useImagePacks.ts` (web fallback). **Not used by the write side** (see note below). Delete the read-helper **functions** once web fallback is dropped. **The whole file is NOT deletable** — it also exports `packAddressEqual` (write-side `GlobalPacks.tsx`/`RoomPacks.tsx`), `imageUsageEqual` (`ImageTile.tsx`), `packMetaEqual` (`ImagePackContent.tsx`), which must be retained | **V-SEND.R-PACK-READ** (delete) |
| `synara/src/app/hooks/useImagePacks.ts` (web fallback)                          | `useAccountDataCallback` / `useStateEventCallback` listeners on `mx` for non-native web sessions                                                                                               | Native path is fail-closed; JS listeners remain only for web. Delete once the non-native web fallback is dropped (V-BURN) — subscribe #318 has landed                                                                                                                                                                                                                                                                          | **V-SEND.R-PACK-READ** (delete) |

**Write vs read owner clarification (physical delete of the read helpers in `custom-emoji/utils.ts`):**
The pack-read helpers in `custom-emoji/utils.ts` are **read-only owners** — their
sole consumer is `useImagePacks.ts` (the read hooks). The **write** surfaces
(`GlobalPacks.tsx`, `RoomPacks.tsx`, `UserImagePack.tsx`, `RoomImagePack.tsx`)
do **not** call these read helpers directly; they read current pack state through
the native-backed hooks (`useGlobalImagePacks`, `useRoomsImagePacks`,
`useRoomImagePacks`, `useUserImagePack`) and write via `mx.setAccountData` /
`mx.sendStateEvent` (V-SEND.R-PACK-WRITE, #292). So deleting the read helpers
does **not** break the write side — the write surfaces keep working through the
native read hooks.

**Deletion is of the read-helper _functions_, not the whole file.** `utils.ts`
also exports `packAddressEqual` (used by write-side `GlobalPacks.tsx` /
`RoomPacks.tsx`), `imageUsageEqual` (`ImageTile.tsx`), and `packMetaEqual`
(`ImagePackContent.tsx`) — these are **not** read helpers and must be retained
even after the web fallback is dropped. Only `getUserImagePack`,
`getRoomImagePack` (singular), `getRoomImagePacks`, `getGlobalImagePacks`, and
`makeImagePacks` are deletable.

**Gating:** the read helpers **cannot be deleted now** — `useImagePacks.ts` still
calls them in its non-native web fallback paths (`isSynaraDesktop() ? ... :
getX(mx)` and the `'legacy'` branches). Deletion is gated on **dropping the
non-native web fallback** (V-BURN), not merely on #318 landing: the web fallback
code paths in `useImagePacks.ts` remain live for non-native sessions. The write
residual (#292) is a separate slice and does not gate this deletion. This doc
does not claim V-BURN readiness; #327 remains out of scope.

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

**Landed #318 (subscribe):**

- Session-scoped `NativeImagePackOwner` observes account-data and room-state
  changes.
- Tauri emits `matrix-image-packs-updated`; TS re-snapshots through the native
  get IPC. Native desktop remains fail-closed.

**Landed #320 (room IDs without `mx.getRoom`):**

- `useImagePackRooms.ts` returns `[roomId, ...parents]` from `roomToParents`.
- `RoomInput.tsx`, `PowersEditor.tsx`, `EmojiBoard.tsx`, and
  `EmoticonAutocomplete.tsx` consume IDs; pack reads use native get-by-room-ID.

**Remaining (this residual):**

- Physical deletion of the read-helper **functions** in
  `custom-emoji/utils.ts` (`getUserImagePack`, `getRoomImagePack`,
  `getRoomImagePacks`, `getGlobalImagePacks`, `makeImagePacks`) once the
  non-native web fallback is retired. The file's `packAddressEqual`,
  `imageUsageEqual`, and `packMetaEqual` equality helpers are retained.

Pack preview media bytes remain media-adjacent and out of scope for this
account-data/state-event residual; coordinate those with V-TIMELINE.

**Fail-closed:** on a native logged-in session, absence/failure of any
`matrix_get_*_image_packs` command is terminal — the picker/autocomplete must
not fall through to `mx.getAccountData` / `mx.getStateEvent`. Legacy JS read
paths remain only for non-native web sessions.

---

## 4. Non-goals / out of scope

| Item                                                                                                                                                          | Status                                                                                                      |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| **V-SEND.R-PACK-WRITE** (add/remove/enable/update packs: `RoomPacks.tsx`, `GlobalPacks.tsx`, `UserPack.tsx`, `RoomImagePack.tsx`, `UserImagePack.tsx` writes) | Separate residual — not this slice                                                                          |
| Pack image/avatar **upload** (`ImageTile.tsx` / `PackMeta.tsx` → `state/upload.ts` → `mx.uploadContent`)                                                      | V-SEND.R-PACK-UPLOAD — separate                                                                             |
| GIF pack/collection management                                                                                                                                | **NOOP** — the [GIF-pack audit](v-send-gif-pack-audit.md) found no such product surface on the measured tip |
| Timeline media **display** (GIF playback, pack preview media bytes, authenticated media download)                                                             | **V-TIMELINE** — do not edit #240 (HOLD)                                                                    |
| Umbrella merge to `main`                                                                                                                                      | **#39** — needs explicit user approval                                                                      |
| Cutover / dual-backend removal                                                                                                                                | #240 HOLD; no cutover                                                                                       |

---

## 5. Self-eval

**Confidence: high** for this truth-up at tip
`e38bfdab68bd57e4f3110a812c5e4c5d543c1ff5`. I re-traced the pack-read surface
after **#297** landed: the snapshot get commands (`matrix_get_user_image_pack` /
`matrix_get_room_image_packs` / `matrix_get_global_image_packs`) and the
`nativeImagePackOwner` / `nativeImagePack` TS owners plus the `useImagePacks.ts`
native fail-closed path are confirmed landed. #318 subscription and #320 pure
room-ID graph resolution are also landed. The only remaining pack-read residual
is the **physical delete** of the read-only helper functions in
`custom-emoji/utils.ts`; the non-native web fallback is the deletion gate, and
the write side (#292) is unaffected. The equality helpers in that file remain.
Sticker/GIF **send** is native (#264); V-BURN is not ready and #327 is not
merged.

## 7. Implementation close

**Landed #297 (snapshot):**

- Rust: `matrix_get_user_image_pack` / `matrix_get_room_image_packs` / `matrix_get_global_image_packs`
- TS: `nativeImagePackOwner` + `nativeImagePack` + `useImagePacks` native path (fail-closed on desktop)

**Landed #318 (subscribe):**

- Rust: session-scoped `NativeImagePackOwner` (account-data + room state handlers)
- Signal: Tauri `matrix-image-packs-updated` (sessionGeneration only; re-snapshot via get IPC)
- TS: `useImagePacks` native refresh token listen (fail-closed desktop)

**Landed #320 (room IDs without `mx.getRoom`):**

- `useImagePackRooms.ts` is pure `roomToParents` graph resolution and returns
  room IDs for native get-by-room-ID reads.

**Residual follow-on (this doc):**

- Physical delete of the JS pack-read helper **functions** in
  `custom-emoji/utils.ts` (read-only owners; retain the file's
  `packAddressEqual`/`imageUsageEqual`/`packMetaEqual` equality helpers used by
  the write side + image-pack-view) + `useImagePacks.ts` web-fallback listeners
  once the non-native web path is retired (V-BURN) — write side unaffected;
  retain `packAddressEqual`/`imageUsageEqual`/`packMetaEqual`
- dual_backend remains forbidden; no second backend or selector is added
