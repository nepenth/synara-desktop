# V-ROOMS — native member list / power-level read residual

| Field   | Value                                                                                                    |
| ------- | -------------------------------------------------------------------------------------------------------- |
| Status  | **Residual inventory — docs only; not implemented or closed**                                            |
| Tip SHA | `3d76402f`                                                                                               |
| Base    | `feature/matrix-rust-sdk-full-replacement` only                                                          |
| Related | P4.6 member index; P4.3 membership; #375 member moderation writes                                        |
| Policy  | [full-vertical-policy.md](full-vertical-policy.md) — native full vertical and physical JS-owner deletion |

> **Scope guard.** This records the read half to execute after the #375 write
> owners land. It does not claim that write vertical is present at this tip.
> This PR changes no product code, does not touch `product.rs`, does not add a
> `dual_backend`, and does not claim V-BURN completion.

## Finding

This is not a NOOP. At `3d76402f`, the member list and power-level reads still
belong to the live `matrix-js-sdk` object graph. Rust has the P4.6 DTO/index
harness and a schema-level `members` stream topic, but there is no production
Tauri snapshot owner or UI binding for either capability.

## Residual table

| Surface                                                                                | Current JS owner at the tip                                                                                                                                                                                         | Native-read consequence / disposition                                                                                                                                                                                                                           |
| -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `synara/src/app/hooks/useRoomMembers.ts`                                               | `mx.getRoom(roomId)`; `room.getMembers()`; `room.loadMembersIfNeeded()`; `RoomMemberEvent.Membership` and `RoomMemberEvent.PowerLevel` listeners                                                                    | Replace the hook with a native room-scoped snapshot/subscription. Delete the hook after all consumers move; no JS listener or load fallback may remain on desktop.                                                                                              |
| `synara/src/app/features/common-settings/members/Members.tsx`                          | Consumes `useRoomMembers`; uses `room.getJoinedMemberCount()` for the loading guard/header; sorts SDK `RoomMember` objects; reads `usePowerLevels`, `useRoomCreators`, power tags, and SDK-only `MemberTile` fields | Rebind to a Synara-owned native member DTO. The native response must provide an authoritative joined count, membership, display/avatar handles, and per-user power. Fail closed when the native read is unavailable.                                            |
| `synara/src/app/features/room/Room.tsx` + `MembersDrawer.tsx`                          | `Room` supplies `useRoomMembers` and `usePowerLevels` to the drawer; the drawer repeats `getJoinedMemberCount()`, power sorting, creator tags, `member.getMxcAvatarUrl()`, and room-based display-name lookup       | Migrate the drawer in the same member-read slice. Do not leave the room drawer on a JS-derived list while the settings list is native.                                                                                                                          |
| `synara/src/app/features/lobby/Lobby.tsx`                                              | Uses `useRoomMembers(mx, space.roomId)` for the people drawer; uses `usePowerLevels`/`useRoomsPowerLevels`; permission checks obtain creators through `getRoomCreatorsForRoomId(mx, ...)`                           | The people drawer and all power-level inputs used by lobby permissions need native snapshots. Its `mx.getRoom(...).getJoinRule()` read at line 98 and `mx.getRoom(item.roomId)` read at line 288 are space/join-rule graph work, not this member-read residual. |
| `synara/src/app/components/editor/autocomplete/UserMentionAutocomplete.tsx`            | Direct `useRoomMembers`; uses SDK `RoomMember`, `getMemberDisplayName`, and `getMxcAvatarUrl()`                                                                                                                     | Move mention candidates to the native member DTO or explicitly name this as a separate member-list consumer before deleting `useRoomMembers`. No desktop fallback to `room.getMembers()`.                                                                       |
| `synara/src/app/hooks/usePowerLevels.ts`                                               | `usePowerLevels` calls `useStateEvent(room, m.room.power_levels)`; `useRoomsPowerLevels` calls `getStateEvent` and listens through `useStateEventCallback`                                                          | Replace only the read hooks/context binding. Keep pure `IPowerLevels`, defaults, `readPowerLevel`, and permission calculations as SDK-neutral logic over native DTOs.                                                                                           |
| `synara/src/app/hooks/usePowerLevelTags.ts`                                            | Reads `in.synara.room.power_level_tags` through `useStateEvent`                                                                                                                                                     | Include tags in the native power-level read contract, or provide a separately named native tag snapshot. Otherwise member chips and the permissions UI retain a JS state-event read.                                                                            |
| `synara/src/app/hooks/useRoomCreators.ts`                                              | Reads `m.room.create` through `useStateEvent`; `getRoomCreatorsForRoomId` resolves `mx.getRoom(roomId)` and `getStateEvent`                                                                                         | Return creator IDs in the native power-level/room-permissions DTO for this vertical. The shared helper has other product consumers and must not be deleted globally until those consumers are migrated.                                                         |
| `src-tauri/src/matrix/members/{mod,index}.rs` and `src-tauri/src/matrix/dto/member.rs` | P4.6 `MemberIndex` is a pure DTO harness; it does not own a live SDK/Tauri read                                                                                                                                     | Useful foundation only. It is not evidence of a product vertical: no production command, live `matrix-sdk` mapping, subscription, or UI owner exists at this tip.                                                                                               |

### Power-level consumers that must be accounted for

`Members.tsx` and `MembersDrawer.tsx` are the visible member-list consumers.
The same `IPowerLevels` context also feeds room/lobby permission checks, room
view permissions, call visibility, notification/search labels, and the room
and space permission screens. The implementation PR must either migrate those
consumers to the native snapshot at the same time or record their exact
vertical as a separate named residual; deleting `usePowerLevels` while leaving
any of them on JS state events is not a completed power-level read.

The following are pure and should not be deleted merely because the source
object changes: `readPowerLevel`, `getPermissionPower`,
`getRoomPermissionsAPI`, power sorting, default tag generation, and DTO
validation. Convert their inputs from SDK-derived objects to native DTOs.

## Proposed IPC contract

These names are proposed for the implementation slice; none exists as a
production command at the audited tip.

| Proposed command                     | Request       | Response required by the current UI                                                                                                                                                                                                            |
| ------------------------------------ | ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `matrix_room_members_snapshot`       | `{ roomId }`  | `{ sessionGeneration, roomId, joinedMemberCount, members }`, where each member is the existing owned `RoomMember` shape (`userId`, `displayName?`, `avatarUrl?`, `membership`, `powerLevel`, `isDirectTarget?`).                               |
| `matrix_room_power_levels_snapshot`  | `{ roomId }`  | `{ sessionGeneration, roomId, powerLevels, powerLevelTags, creatorIds }`. `powerLevels` must preserve the current defaults/users/events/action/notification semantics; `creatorIds` preserves the creator override in `getRoomPermissionsAPI`. |
| `matrix_rooms_power_levels_snapshot` | `{ roomIds }` | `{ sessionGeneration, rooms: [{ roomId, powerLevels, powerLevelTags, creatorIds }] }` for `Lobby`/`useRoomsPowerLevels` permission checks.                                                                                                     |

The native owner should use the live `matrix-sdk` session and return a
session-generation-stamped, bounded projection. A logged-in desktop session
with a missing command, invalid response, stale generation, or native read
error is terminal for that read; it must not fall through to `mx.getRoom`,
`room.getMembers`, `getStateEvent`, or JS event listeners. Non-desktop/web
behavior is outside this native desktop slice and must not be implemented as
a dual-backend selector.

The existing `members` stream topic is a possible invalidation transport, but
it currently validates only `{ members }`, is not exported as a production
command owner, and carries no room-level `IPowerLevels`/tags/creator contract.
Extend or pair it with an explicit power-level update contract before using it
as proof of a live UI path. A snapshot poll alone is not a substitute for the
required native ownership if it leaves JS listeners active.

## Deletion list for the product slice

Physical deletion belongs in the implementation PR, after the native commands
and UI binding are proven:

- Delete `useRoomMembers.ts` and its `matrix-js-sdk` member-event wiring.
- Remove the member-read dependency from `Members.tsx`, `Room.tsx`,
  `MembersDrawer.tsx`, `Lobby.tsx`, and `UserMentionAutocomplete.tsx`.
- Make `MemberTile`, drawer rows, member sorting, and member power-tag helpers
  consume the native DTO; remove SDK-only `RoomMember` types, `member.name`,
  `member.events.member`, `member.getMxcAvatarUrl()`, and room-based member
  display-name reads from this surface.
- Replace `usePowerLevels`/`useRoomsPowerLevels`/`usePowerLevelsContext` read
  binding and the `usePowerLevelTags` state-event read with native snapshot
  state. Retain the pure power/default/permission functions.
- Replace creator reads used by this surface with native `creatorIds`; remove
  `getRoomCreatorsForRoomId(mx, ...)` from the migrated lobby permission path.
- Delete any native-desktop JS fallback branch, compatibility listener, or
  `isNative ? native : JS` selector introduced during the implementation.

Do **not** delete shared `useStateEvent`, `getStateEvent`, or every
`getMemberDisplayName` call as incidental cleanup. Timeline sender labels,
profiles, call UI, and unrelated room-state consumers are separate residuals
unless explicitly included in the named implementation slice. The Lobby
join-rule reads noted above remain with the space-graph/join-rule owner.

## Acceptance evidence for the future implementation PR

- Native commands are registered and invoke the live managed `matrix-sdk`
  session; no `product.rs` change is part of this docs inventory.
- Focused owner tests cover logged-out, unavailable-command, malformed/stale
  snapshot, and successful member/power reads.
- Members settings, room drawer, lobby drawer, mention autocomplete, and all
  claimed power-level consumers render from native DTO state with no JS member
  or state-event fallback.
- A repository search shows no remaining selected-surface calls to
  `mx.getRoom(...).getMembers`, `room.getMembers`, `loadMembersIfNeeded`,
  `getStateEvent(...RoomPowerLevels)`, or `RoomMemberEvent`.
- Runtime proof is still required before calling this vertical complete. This
  document itself is only a residual inventory.
