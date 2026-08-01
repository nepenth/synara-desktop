# V-SEND residual inventory — remaining send / media-adjacent gaps after sticker/GIF #264 and threads #258

| Field | Value |
|-------|-------|
| Status | **Inventory (docs only)** — no product code in this PR |
| Tip measured | `4d33227f` (docs-pin after #275; integration `48991e77` after #274 docs / #266 V-AUTH.4b) |
| Base | `feature/matrix-rust-sdk-full-replacement` |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) — physical deletion inside each owning slice |
| Related | V-SEND.1 #248 (attach), V-SEND.2 #239 (reactions), V-SEND.3 #250 (polls), V-SEND.4 #253 (rich), V-SEND.5 #258 (threads), V-SEND sticker/GIF #264 |

> **Scope guard.** This is an **inventory** document only. It does **not**
> implement product code in `product.rs` or any TS. It does not touch open
> **#266** (V-AUTH.4b, now DONE) or **#240** (V-TIMELINE, HOLD). No cutover.
> No SESSION-HANDOFF docs are produced.

---

## 1. Tip measured

- Branch: `matrix-rust/v-send-residual-inventory-ds`
- Tip SHA: **`4d33227f`** (docs-pin after #275; integration `48991e77` after #274 docs / #266 V-AUTH.4b register DONE)
- Working tree clean at measurement time.
- Base integration branch: `feature/matrix-rust-sdk-full-replacement`.
- Import accounting at tip: production import files **172**; allowlist **191→175** (per #266).

---

## 2. What is DONE (native send owners landed)

| Capability | PR | Native owner | Status |
|------------|----|--------------|--------|
| Attachments / media upload send | **#248** (V-SEND.1) | `matrix_send_attachment` → `Room::send_attachment` + `AttachmentSendQueue`; `sendComposerAttachmentsWithNativeOwner` | **DONE** — Synapse native attachment proof Confirmed |
| Reactions | **#239** (V-SEND.2) | Native reaction send/redact commands | **Merged** — Synapse native reaction proof Confirmed |
| Polls | **#250** (V-SEND.3) | `matrix_send_poll` / `matrix_poll_respond`; `sendPollWithNativeDesktopOwner` / `respondPollWithNativeDesktopOwner` | **Merged** — Synapse native poll proof Confirmed |
| Emotes / notices / rich HTML + mentions | **#253** (V-SEND.4) | `matrix_send_text`; `sendPlainTextWithNativeOwner` | **DONE** |
| Threads (composer text/attachment) | **#258** (V-SEND.5) | `matrix_send_text` / `matrix_send_attachment` with `threadRoot` → `Relation::Thread` | **DONE** — Synapse thread-send proof Confirmed |
| Sticker send | **#264** | `matrix_send_sticker` (`m.sticker`); `sendComposerStickerWithNativeOwner` | **Merged** |
| GIF send | **#264** | GIF via `matrix_send_attachment` (`image/gif` bytes); `sendComposerGifWithNativeOwner` | **Merged** |

All composer send owners (`RoomInput`) route through native owners first and
fall back to the JS client **only** when no native Matrix session is live
(`isNativeMatrixLoggedIn` → `matrix_session_snapshot`). On a native logged-in
session, command absence/failure is terminal and never falls through to
`mx.sendMessage` / `mx.uploadContent` / `mx.sendEvent`.

---

## 3. Residual inventory — remaining send / media-adjacent gaps

Each row: **path** | **current owner** | **native gap** | **proposed residual ID**.

### 3.1 Sticker / emoji pack settings + pack preview (media-adjacent, not send)

Sticker **send** is native (#264), but the **pack management** surface (which
packs exist, which are enabled, pack metadata/avatar, pack image content) is
still fully on `matrix-js-sdk` account-data / state-event reads and writes.

| Path | Current owner | Native gap | Proposed residual ID |
|------|---------------|------------|----------------------|
| `synara/src/app/plugins/custom-emoji/utils.ts` | `getGlobalImagePacks` / `getRoomImagePacks` / `getUserImagePack` read `PoniesEmoteRooms` account-data + `PoniesRoomEmotes` state events via `mx.getAccountData` / `mx.getStateEvents` | No native pack-read projection; reads live `matrix-js-sdk` account-data/state | **V-SEND.R-PACK-READ** |
| `synara/src/app/hooks/useImagePacks.ts` | `useUserImagePack` / `useGlobalImagePacks` / `useRoomImagePack(s)` / `useRelevantImagePacks` subscribe via `useAccountDataCallback` / `useStateEventCallback` on `mx` | No native pack subscription; JS event listeners on live client | **V-SEND.R-PACK-READ** |
| `synara/src/app/features/common-settings/emojis-stickers/RoomPacks.tsx` | `mx.sendStateEvent(PoniesRoomEmotes, …)` to add/remove room packs | No native `PoniesRoomEmotes` state write | **V-SEND.R-PACK-WRITE** |
| `synara/src/app/features/settings/emojis-stickers/GlobalPacks.tsx` | `mx.setAccountData(PoniesEmoteRooms, …)` to enable/disable global packs | No native `PoniesEmoteRooms` account-data write | **V-SEND.R-PACK-WRITE** |
| `synara/src/app/features/settings/emojis-stickers/UserPack.tsx` | `mx.setAccountData(PoniesUserEmotes, …)` for personal pack | No native `PoniesUserEmotes` account-data write | **V-SEND.R-PACK-WRITE** |
| `synara/src/app/components/image-pack-view/RoomImagePack.tsx` | `mx.sendStateEvent(PoniesRoomEmotes, …)` to update pack content | No native pack-content state write | **V-SEND.R-PACK-WRITE** |
| `synara/src/app/components/image-pack-view/UserImagePack.tsx` | `mx.setAccountData(PoniesUserEmotes, …)` to update personal pack | No native pack-content account-data write | **V-SEND.R-PACK-WRITE** |
| `synara/src/app/components/image-pack-view/ImageTile.tsx` / `PackMeta.tsx` | `createUploadAtom` → `state/upload.ts` → `uploadContent` (`mx.uploadContent`) for pack image/avatar bytes | Pack image/avatar upload still JS `uploadContent` | **V-SEND.R-PACK-UPLOAD** |
| `synara/src/app/components/emoji-board/EmojiBoard.tsx` | Renders pack previews via `useRelevantImagePacks` + `resolveOptionalMatrixMediaUrl` | Pack preview display depends on JS pack-read + media URL resolution | **V-SEND.R-PACK-READ** (display) |

**Note:** pack **preview** display is media-adjacent (authenticated media
download / V-TIMELINE). The pack **read/write** residuals above are the
account-data/state-event owners; the actual media bytes for previews belong to
the media vertical (V-TIMELINE / #240 HOLD).

### 3.2 GIF — send native, display/pack residual

GIF **send** is native (#264, `image/gif` bytes over `matrix_send_attachment`).
The residual is **display** (GIF playback in timeline) and any GIF **pack**
management, both media-adjacent.

| Path | Current owner | Native gap | Proposed residual ID |
|------|---------------|------------|----------------------|
| Timeline GIF rendering | `RoomTimeline` / media renderers (V-TIMELINE scope) | GIF playback/display not on native DTO path | **V-TIMELINE** (do not edit #240) |
| GIF picker pack/collection management | `gifProvider` / picker | No native GIF collection ownership (send-only is native) | **V-SEND.R-GIF-PACK** |

### 3.3 Avatar upload (user + room) — fully residual

There is **no** native avatar/profile write command on tip (verified: no
`matrix_set_avatar` / `matrix_set_display_name` / `matrix_set_room_avatar` /
`matrix_set_room_name` / `matrix_set_room_topic` in `src-tauri/src`). The
`user_profile` module is a **read-only** P6.6 projection. Avatar upload is
fully residual on the live JS client.

| Path | Current owner | Native gap | Proposed residual ID |
|------|---------------|------------|----------------------|
| `synara/src/app/features/settings/account/Profile.tsx` | `mx.setAvatarUrl(mxc)` (upload via `CompactUploadCardRenderer` → `state/upload.ts` → `mx.uploadContent`) | No native user-avatar upload + `setAvatarUrl` write | **V-SEND.R-AVATAR-UPLOAD** |
| `synara/src/app/hooks/useUserProfile.ts` | `mx.getProfileInfo` + `UserEvent.AvatarUrl`/`DisplayName` listeners | No native profile read/subscription | **V-SEND.R-AVATAR-UPLOAD** (read half) |
| `synara/src/app/features/common-settings/general/RoomProfile.tsx` | `mx.sendStateEvent(RoomAvatar/RoomName/RoomTopic, …)` | No native room-profile state writes | **V-SEND.R-ROOM-PROFILE** |

### 3.4 Call widget upload — residual

| Path | Current owner | Native gap | Proposed residual ID |
|------|---------------|------------|----------------------|
| `synara/src/app/plugins/call/CallWidgetDriver.ts` | `client.uploadContent(file)` (widget `uploadFile`), `client.getMediaConfig()`, `downloadFile` via `mxcUrlToHttp` | No native widget media upload/download; JS `uploadContent` | **V-SEND.R-CALL-UPLOAD** |

### 3.5 Message forward — residual

| Path | Current owner | Native gap | Proposed residual ID |
|------|---------------|------------|----------------------|
| `synara/src/app/features/room/message/Message.tsx` | `handleForwardConfirmed` → `mx.sendMessage(targetRoom.roomId, content)` | No native forward/send-to-room owner; JS `sendMessage` | **V-SEND.R-FORWARD** |

### 3.6 Poll-in-thread — DONE

Poll **send** is native (#250). The native poll owner now accepts
`threadRoot`/`replyTo` (mirroring V-SEND.5): `matrix_send_poll` wires
`RelationWithoutReplacement::Thread` / `Reply` and `sendPollWithNativeDesktopOwner`
forwards the thread args from `handleSendPoll`. Poll-in-thread is supported.

| Path | Current owner | Native gap | Residual ID |
|------|---------------|------------|-------------|
| `synara/src/app/features/room/RoomInput.tsx` (`handleSendPoll`) | `sendPollWithNativeDesktopOwner({ roomId, question, answers, maxSelections, threadRoot, replyTo })` | None — thread relation wired natively | **V-SEND.R-POLL-THREAD** (resolved) |
| `synara/src/app/features/room/nativePoll.ts` / `nativePollOwner.ts` | Poll owner forwards `threadRoot`/`replyTo` to `matrix_send_poll` | None | **V-SEND.R-POLL-THREAD** (resolved) |

### 3.7 Other RoomInput send paths still on live JS client

The composer text/attachment/sticker/GIF/poll paths are native-first. The
remaining `RoomInput` JS-client send methods are **legacy-web fallbacks** (only
when no native session) plus the residuals above. The **live-client** send
methods that remain reachable on a native session are the residuals in §3.3–§3.6
(avatar, forward, call, poll-in-thread). The `mx.sendMessage`/`mx.sendEvent`/
`mx.uploadContent` calls inside `RoomInput`'s `handleSendUpload`/`handleStickerSelect`/
`handleGifSelect` legacy branches are **not** reachable on a native session
(fail-closed native-first routing).

| Path | Current owner | Native gap | Proposed residual ID |
|------|---------------|------------|----------------------|
| `synara/src/app/features/room/message/MessageEditor.tsx` | `mx.sendMessage(roomId, content)` for message **edit/replace** (`m.relates_to` `m.replace`) | Native `matrix_edit_message` (`m.replace` + `m.new_content`) owner added; JS path only when no native session | **V-SEND.R-EDIT** (in-PR) |
| `synara/src/app/features/common-settings/developer-tools/SendRoomEvent.tsx` | `mx.sendEvent` / `mx.sendStateEvent` (developer tool) | Developer-tool raw event send; not a product path | **V-SEND.R-DEVTOOL** (low priority) |

---

## 4. Residual ID summary

| ID | Capability | Owner today | Native gap |
|----|------------|-------------|------------|
| **V-SEND.R-PACK-READ** | Sticker/emoji pack read + subscription (`PoniesEmoteRooms` / `PoniesRoomEmotes` / `PoniesUserEmotes`) | `custom-emoji/utils.ts`, `useImagePacks.ts`, `EmojiBoard.tsx` | No native pack-read projection/subscription |
| **V-SEND.R-PACK-WRITE** | Sticker/emoji pack settings writes (add/remove/enable/update) | `RoomPacks.tsx`, `GlobalPacks.tsx`, `UserPack.tsx`, `RoomImagePack.tsx`, `UserImagePack.tsx` | No native `PoniesRoomEmotes` / `PoniesEmoteRooms` / `PoniesUserEmotes` writes |
| **V-SEND.R-PACK-UPLOAD** | Pack image/avatar byte upload | `ImageTile.tsx`, `PackMeta.tsx` via `state/upload.ts` → `mx.uploadContent` | No native pack-media upload |
| **V-SEND.R-GIF-PACK** | GIF picker pack/collection management (send is native) | `gifProvider` / picker | No native GIF collection ownership |
| **V-SEND.R-AVATAR-UPLOAD** | User avatar upload + profile read | `Profile.tsx`, `useUserProfile.ts` | No native `setAvatarUrl` / profile read |
| **V-SEND.R-ROOM-PROFILE** | Room avatar/name/topic writes | `RoomProfile.tsx` | No native `RoomAvatar`/`RoomName`/`RoomTopic` state writes |
| **V-SEND.R-CALL-UPLOAD** | Call widget media upload/download | `CallWidgetDriver.ts` | No native widget media upload |
| **V-SEND.R-FORWARD** | Message forward to another room | `Message.tsx` | No native forward/send-to-room owner |
| **V-SEND.R-POLL-THREAD** | Poll start/response in a thread | native poll owner | **DONE #282** — `threadRoot`/`replyTo` wired |
| **V-SEND.R-EDIT** | Message edit/replace | `MessageEditor.tsx` | Native `matrix_edit_message` owner (in-PR); JS path only when no native session |
| **V-SEND.R-DEVTOOL** | Developer-tool raw event send | `SendRoomEvent.tsx` | Developer tool; not a product path |

---

## 5. Explicit non-goals / out of scope

| Item | Status |
|------|--------|
| Timeline media **display** (GIF playback, pack preview media bytes, authenticated media download) | **V-TIMELINE** — do not edit #240 (HOLD) |
| Thread list / summary / open-thread view cutover | V-TIMELINE / P5.8 |
| Message edit/replace in thread | V-SEND.R-EDIT (separate) |
| V-AUTH.3 loginFlows implementation | Free slot after #273 inventory; not V-SEND |
| V-AUTH.4b register | **DONE #266** |
| Cutover / dual-backend removal | #240 HOLD; no cutover |
| Live Synapse proofs for the residuals above | Required per owning slice, not claimed here |

---

## 6. Self-eval

**Confidence: high** for the inventory. I traced every `RoomInput` send path
(text/attachment/sticker/GIF/poll) and confirmed each is native-first with a
fail-closed legacy branch. I enumerated the remaining send/media-adjacent
residuals: pack settings/preview (read/write/upload), GIF display/pack, avatar
upload (user + room), call-widget upload, message forward, poll-in-thread, and
message edit. I verified there is **no** native avatar/profile/room-profile
write command on tip, so those residuals are fully on the live JS client.

**Possible missed files:**
- `synara/src/app/state/upload.ts` — the shared `uploadContent` wrapper used by
  avatar/pack uploads; it is the common JS upload owner for the residuals above.
- `synara/src/app/utils/matrix.ts` (`uploadContent`) — shared JS upload helper.
- `synara/src/app/features/room/msgContent.ts` — legacy attachment content
  builders (thumbnail upload) used only on the non-native fallback path.
- Any `matrix-js-sdk` import hidden behind a barrel re-export in the
  emoji/pack/call trees — verify during implementation with a full
  `grep -rn "matrix-js-sdk" synara/src/app/plugins synara/src/app/components/image-pack-view synara/src/app/features/settings synara/src/app/features/common-settings`.

**Caveat:** the allowlist (`p1.6-js-sdk-import-allowlist.json`, 175 paths)
still lists `RoomInput.tsx`, `Message.tsx`, `MessageEditor.tsx`,
`CallWidgetDriver.ts`, `custom-emoji/*`, `useImagePacks.ts`, `RoomPacks.tsx`,
`GlobalPacks.tsx`, `SendRoomEvent.tsx`, `PollContent.tsx`, and `msgContent.ts`.
These are the send/media-adjacent files that will leave the allowlist as the
owning residuals land.
