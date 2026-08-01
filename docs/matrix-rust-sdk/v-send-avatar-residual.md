# V-SEND.R-AVATAR-UPLOAD — user + room avatar upload residual inventory

| Field | Value |
|-------|-------|
| Status | **Inventory (docs only)** — no product code in this PR |
| Tip SHA | after #290 forward inventory on `feature/matrix-rust-sdk-full-replacement` |
| Base | `feature/matrix-rust-sdk-full-replacement` |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) — physical deletion inside each owning slice |
| Related | V-SEND.1 attachment send **#248** (native), P6.4 media-upload queue, P6.5 room-profile, P6.6 user-profile, V-SEND.R-PACK-UPLOAD (separate) |

> **Scope guard.** Docs only. No product code in `product.rs` or any TS. Does
> not touch open **#283** (V-SEND.R-EDIT), **#285/#289** (V-TIMELINE cutover),
> or **#39** (umbrella). No cutover, no dual-backend.

---

## 1. What is residual

Avatar **upload** (user profile avatar and room avatar) still runs on the live
`matrix-js-sdk` client. The flow is: pick a local image file → `state/upload.ts`
(`createUploadAtom` / `useBindUploadAtom`) → `utils/matrix.ts` `uploadContent`
→ `mx.uploadContent` (JS SDK media upload) → then `mx.setAvatarUrl` (user) or
`mx.sendStateEvent(m.room.avatar)` (room). There is **no** native avatar-upload
or avatar-set IPC. Native media upload exists only inside the attachment send
path (`matrix_send_attachment` → `room.send_attachment`, which performs the SDK
`Media::upload` internally); the P6.4 `UploadQueue` is metadata-only (no file
bytes, no SDK upload). So the avatar surface is a genuine residual.

The closely-related **room profile** surface (`RoomProfile.tsx`) edits room
avatar + name + topic via `mx.sendStateEvent` on the live client. Per the
scoreboard this is grouped with avatar upload as **R-ROOM-PROFILE**. Both are
small and share the same owner surface (settings), so they are inventoried in
one PR but kept as distinct residual IDs.

---

## 2. Residual table

| Path | Role | Gap | ID |
|------|------|-----|----|
| `synara/src/app/features/settings/account/Profile.tsx` | `ProfileAvatar` uploads local image via `createUploadAtom` → `uploadContent` → `mx.uploadContent`, then `mx.setAvatarUrl(mxc)`; remove via `mx.setAvatarUrl('')`. `ProfileDisplayName` uses `mx.setDisplayName`. Gated by `useCapabilities` (`m.set_avatar_url` / `m.set_displayname` from JS `Capabilities`) | No native avatar upload or `set_avatar_url` / `set_display_name` IPC; JS SDK media upload + profile write | **V-SEND.R-AVATAR-UPLOAD** |
| `synara/src/app/features/common-settings/general/RoomProfile.tsx` | `RoomProfileEdit` uploads room avatar via `createUploadAtom` → `uploadContent` → `mx.uploadContent`, then `mx.sendStateEvent(m.room.avatar)`; also `m.room.name` / `m.room.topic` via `mx.sendStateEvent` | No native room-avatar upload or `m.room.avatar` / `m.room.name` / `m.room.topic` state write | **R-ROOM-PROFILE** |
| `synara/src/app/state/upload.ts` | `createUploadAtom` / `useBindUploadAtom` / `uploadContent` — shared JS media-upload atom used by avatar, room-avatar, image-pack avatar, and composer | JS `mx.uploadContent` / `mx.cancelUpload`; no native upload atom | **V-SEND.R-AVATAR-UPLOAD** (shared; also PACK-UPLOAD) |
| `synara/src/app/utils/matrix.ts` | `uploadContent` / `getImageInfo` / `encryptFile` — JS SDK media upload + image metadata helpers | JS `mx.uploadContent`; no native equivalent | **V-SEND.R-AVATAR-UPLOAD** (shared) |
| `synara/src/app/hooks/useCapabilities.ts` | `useCapabilities` reads `Capabilities` from `matrix-js-sdk` to gate avatar/display-name buttons | JS `Capabilities`; no native capability projection | **V-SEND.R-AVATAR-UPLOAD** (gate) |
| `synara/src/app/components/upload-card/CompactUploadCardRenderer.tsx` | Renders the in-flight upload card for avatar/room-avatar | JS upload atom binding | **V-SEND.R-AVATAR-UPLOAD** (display) |

**Note:** image-pack avatar upload (`PackMeta.tsx` → `state/upload.ts` →
`mx.uploadContent`) is **V-SEND.R-PACK-UPLOAD** (separate residual, out of
scope here) even though it shares `state/upload.ts`. The composer attachment
upload is already native (**#248**); the shared `state/upload.ts` atom is only
still used by the avatar / room-avatar / pack-avatar surfaces.

---

## 3. Proposed native slice / IPC

When claimed, the native slice should expose avatar upload + profile write over
IPC and delete the JS owners. Reuse the existing native media-upload path
(`room.send_attachment` performs SDK `Media::upload`) rather than inventing a
parallel uploader. Proposed IPC (fail-closed):

- `matrix_upload_media` — upload raw bytes via SDK `Media::upload`, return the
  `mxc://` URI. Reuses the same byte-IPC + size-guard pattern as
  `matrix_send_attachment` (`MAX_ATTACHMENT_IPC_BYTES`).
- `matrix_set_own_avatar` — `set_avatar_url(mxc)` for the logged-in user
  (empty string removes). Companion `matrix_set_display_name` for the display
  name field on the same surface.
- `matrix_set_room_avatar` — `m.room.avatar` state write for a room (empty
  removes). Companion `matrix_set_room_name` / `matrix_set_room_topic` for the
  same `RoomProfileEdit` form (R-ROOM-PROFILE).
- `matrix_get_capabilities` — native capability projection replacing JS
  `Capabilities` for the `m.set_avatar_url` / `m.set_displayname` gates.

**Deletion list** (physical deletion per [full-vertical-policy.md](full-vertical-policy.md),
named for the implement slice):
- `Profile.tsx` avatar + display-name JS owners (`mx.setAvatarUrl`,
  `mx.setDisplayName`, `mx.uploadContent` path).
- `RoomProfile.tsx` `RoomProfileEdit` JS state writes (`mx.sendStateEvent` for
  `m.room.avatar` / `m.room.name` / `m.room.topic`).
- The avatar/room-avatar consumers of `state/upload.ts` and `utils/matrix.ts`
  `uploadContent`; keep the shared atom only if still needed by PACK-UPLOAD,
  otherwise delete.

**Fail-closed:** on a native logged-in session, absence/failure of any
`matrix_upload_media` / `matrix_set_*` command is terminal — the avatar/room
profile form must not fall through to `mx.uploadContent` / `mx.setAvatarUrl` /
`mx.sendStateEvent`. Legacy JS paths remain only for non-native web sessions.

---

## 4. Non-goals / out of scope

| Item | Status |
|------|--------|
| **V-SEND.R-PACK-UPLOAD** (image-pack avatar upload via `PackMeta.tsx`) | Separate residual — not this slice |
| **V-SEND.R-FORWARD** / **V-SEND.R-CALL-UPLOAD** / **V-SEND.R-GIF-PACK** | Separate residuals |
| **V-SEND.R-EDIT** (`m.replace`) | Open **#283** — do not touch |
| **V-TIMELINE** cutover (RoomTimeline delete, presenter selection) | Open **#285/#289** — do not touch |
| Umbrella merge to `main` | **#39** — needs explicit user approval |
| Dual-backend / invented dual flag | **Forbidden forever** — no `dual_backend`, no new dual flag |
| Cutover / V-BURN | After residual owners clear; not this PR |

---

## 5. Self-eval

**Confidence: high** for the inventory. I traced the avatar-upload surface from
the settings forms (`Profile.tsx`, `RoomProfile.tsx`) through the shared upload
atom (`state/upload.ts`) to the JS SDK media upload (`utils/matrix.ts`
`uploadContent` → `mx.uploadContent`) and the profile writes (`mx.setAvatarUrl`,
`mx.sendStateEvent`). I confirmed there is **no** native avatar-upload or
profile-write IPC: native media upload exists only inside the attachment send
path (`matrix_send_attachment` → `room.send_attachment`), and the P6.4
`UploadQueue` is metadata-only. Possible missed files: any avatar/profile
surface re-exported behind a barrel in the settings tree, and the `useCapabilities`
context provider wiring — verify during implementation with a full
`grep -rn "setAvatarUrl\|setDisplayName\|sendStateEvent\|uploadContent"` over
`synara/src/app/features` and `synara/src/app/state`.


## Implementation (user profile slice)

Landed native IPC + desktop Profile.tsx wiring:
- `matrix_upload_media` / `matrix_set_own_avatar` / `matrix_set_own_display_name`
- Fail-closed on native logged-in desktop sessions
- Room profile (m.room.avatar/name/topic) remains residual **R-ROOM-PROFILE**

## Implementation (room profile slice)

Landed native IPC + desktop RoomProfile.tsx wiring:
- `matrix_set_room_name` / `matrix_set_room_topic` / `matrix_set_room_avatar` — **#313**
- `RoomProfile.tsx` fail-closed via `nativeRoomProfile` / `nativeRoomProfileOwner`; JS `sendStateEvent` only for non-native web
- Room-avatar **media upload** now covered by **#314** PACK-UPLOAD: `RoomProfile.tsx` uses `CompactUploadCardRenderer` → `uploadMediaNative` → `matrix_upload_media` fail-closed on desktop (never falls through to `mx.uploadContent` on a native session). No remaining avatar/room-avatar `mx.uploadContent` path on desktop.
