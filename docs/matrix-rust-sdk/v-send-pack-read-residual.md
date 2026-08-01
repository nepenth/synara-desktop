# V-SEND.R-PACK-READ — sticker/emoji pack read residual inventory

| Field   | Value                                                                                                                                                                        |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status  | **DONE** — native desktop reads are fail-closed, subscription refresh and room-ID resolution are landed, and the JS read helpers plus web-only fallback branches are deleted |
| Tip SHA | `d9a1b819`                                                                                                                                                                   |
| Base    | `feature/matrix-rust-sdk-full-replacement`                                                                                                                                   |
| Policy  | [full-vertical-policy.md](full-vertical-policy.md) — physical deletion inside each owning slice                                                                              |
| Related | V-SEND sticker/GIF **#264** (native send), V-SEND residual inventory, V-SEND.R-PACK-WRITE **#292** (write owners), V-TIMELINE #240 (HOLD)                                    |

> **Scope guard.** This closeout changes only the pack-read TypeScript owners and
> migration evidence. It does not touch `product.rs`, open **#240**
> (V-TIMELINE, HOLD), or **#39** (umbrella).
> No cutover; `dual_backend` is forbidden.

---

## 1. Native pack read and finish-line deletion

Sticker **send** is fully native since **#264** (`matrix_send_sticker` →
`Room::send(m.sticker)`; `sendComposerStickerWithNativeOwner`), and GIF send
rides the native attachment owner (`image/gif` bytes over
`matrix_send_attachment`). The pack **read** surface — which packs exist, which
are enabled, and their metadata — is native through the snapshot commands:

- `matrix_get_user_image_pack`
- `matrix_get_room_image_packs`
- `matrix_get_global_image_packs`

The native subscription emits `matrix-image-packs-updated`; the hooks
re-snapshot through native IPC. `useImagePackRooms.ts` resolves room IDs from
the `roomToParents` graph, and consumers load packs by room ID through native
IPC.

This finish-line slice also deletes the remaining JS read owners:

- `custom-emoji/utils.ts` no longer contains `makeImagePacks` or any JS
  `get*ImagePack(s)` reader.
- `useImagePacks.ts` no longer imports the Matrix JS SDK, reads account data or
  state events, or registers JS fallback listeners.
- Native command absence, failure, or the native owner's non-desktop sentinel
  resolves to an empty/undefined pack result; no JS read fallback is attempted.

The matching **write** side (add/remove/enable/update packs) remains a separate
residual (V-SEND.R-PACK-WRITE, inventory **#292**) and is out of scope here.

---

## 2. Closeout table — V-SEND.R-PACK-READ

**Snapshot get, subscribe, room-ID resolution, and JS read-owner deletion are
DONE (#297, #318, #320, this slice).**

| Path                                           | Closeout                                                                                                                            | Evidence                                                                                                                        |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `synara/src/app/plugins/custom-emoji/utils.ts` | Deleted `makeImagePacks` and all JS `get*ImagePack(s)` readers; retained `packAddressEqual`, `imageUsageEqual`, and `packMetaEqual` | The file is no longer a `matrix-js-sdk` importer; write-side and pack-render equality consumers remain supported                |
| `synara/src/app/hooks/useImagePacks.ts`        | Deleted all JS account-data/state callbacks and legacy fallback branches                                                            | Native snapshot IPC and `matrix-image-packs-updated` refresh are the only read path; native failures resolve to empty/undefined |

### Write-side ownership clarification

The deleted helpers in `custom-emoji/utils.ts` were read-only owners; their
sole consumer was `useImagePacks.ts`. The write surfaces
(`GlobalPacks.tsx`, `RoomPacks.tsx`, `UserImagePack.tsx`, and
`RoomImagePack.tsx`) do not call those helpers directly. They continue to read
through the native-backed hooks (`useGlobalImagePacks`, `useRoomsImagePacks`,
`useRoomImagePacks`, and `useUserImagePack`) and write through the separate
pack-write owner (#292).

Deletion was limited to the read-helper functions, not the whole `utils.ts`
file. Its retained equality helpers remain used by the write side and
image-pack rendering.

---

## 3. Native pack-read projection

**Landed #297 (snapshot):** native fail-closed commands return global, room,
and personal pack snapshots. `nativeImagePackOwner.ts`, `nativeImagePack.ts`,
and the hooks are the TypeScript owners.

**Landed #318 (subscribe):** the session-scoped native owner observes account
data and room-state changes, emits `matrix-image-packs-updated`, and the hooks
re-snapshot through native IPC.

**Landed #320 (room IDs without `mx.getRoom`):** `useImagePackRooms.ts` returns
the current room and parent-space IDs from `roomToParents`; consumers use
native get-by-room-ID reads.

**Closed by this slice:** the JS read helpers and all `useImagePacks.ts`
account-data/state fallback code are physically deleted. The native path is
fail-closed: absence or failure of any `matrix_get_*_image_packs` command is
terminal, with no fall-through to `mx.getAccountData` or `mx.getStateEvent`.

Pack preview media bytes remain media-adjacent and out of scope for this
account-data/state-event residual; coordinate those with V-TIMELINE.

---

## 4. Non-goals / out of scope

| Item                                                     | Status                                       |
| -------------------------------------------------------- | -------------------------------------------- |
| **V-SEND.R-PACK-WRITE** (add/remove/enable/update packs) | Separate residual — not this slice           |
| Pack image/avatar **upload**                             | V-SEND.R-PACK-UPLOAD — separate              |
| GIF pack/collection management                           | **NOOP** — no product surface exists         |
| Timeline media **display**                               | **V-TIMELINE** — do not edit #240 (HOLD)     |
| Umbrella merge to `main`                                 | **#39** — needs explicit user approval       |
| Cutover / dual-backend removal                           | No cutover; `dual_backend` remains forbidden |

---

## 5. Self-eval

**Confidence: high** for this closeout at tip `4eeefa11`. The native snapshot
commands (#297), subscription (#318), and room-ID graph resolution (#320) are
landed. This slice closes the physical deletion of the read-only helper
functions and web-only hook fallback; the pack-write side (#292), media
preview display, V-BURN readiness, and #327 remain separate. The equality
helpers in `utils.ts` are retained.

## 6. Implementation close

- Rust pack-read commands and the session-scoped subscription remain unchanged.
- `custom-emoji/utils.ts` retains only shared pack equality helpers.
- `useImagePacks.ts` is native-only and fail-closed; its web-fallback
  listeners and legacy branches are deleted.
- `dual_backend` remains forbidden; no second backend or selector is added.
