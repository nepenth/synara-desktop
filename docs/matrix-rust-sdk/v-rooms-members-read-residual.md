# V-ROOMS.MEMBERS-READ — native member list / power-level read residual inventory

| Field   | Value                                                                                                                                                                             |
| ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status  | **Partial product (first slice)** — `matrix_room_members_snapshot` + Members settings UI native; drawer/lobby/mentions + power-level reads remain residual |
| Tip SHA | `3d76402f` (merge #374 scoreboard honesty; after create #372)                                                                                                                     |
| Base    | `feature/matrix-rust-sdk-full-replacement`                                                                                                                                        |
| Policy  | [full-vertical-policy.md](full-vertical-policy.md) — physical deletion inside each owning slice                                                                                   |
| Related | **#375** (native room moderation **write** vertical — open/draft), P4.6 members index, [p4.6-members.md](p4.6-members.md), [p4.3-membership-unread.md](p4.3-membership-unread.md) |

> **Scope guard.** Docs only. No product code in `product.rs` or any TS. Does
> not touch open **#375** (moderation writes — serial lock on `product.rs`),
> **#39** (umbrella), or any timeline/send slice. No cutover.

---

## Progress (first slice / #395)

Landed on the product branch:

- IPC `matrix_room_members_snapshot` (live matrix-sdk `Room::members`) + Tauri ACL/build registration
- Fail-closed `nativeRoomMembersOwner` and `useRoomMembers(..., nativeSession)` for **Members.tsx** settings
- **Still residual on desktop native:** `Room.tsx` / `MembersDrawer` / `Lobby.tsx` / `UserMentionAutocomplete` (still default JS `useRoomMembers` path), plus all power-level/creator read owners

This is a scoped first slice, not dual_backend: native session does not fall through to JS for the wired Members settings screen; remaining call sites stay residual until a follow-up PR.

## 1. What this residual covers

PR **#375** (open/draft) lands the native room moderation **write** vertical:
`matrix_room_invite`, `matrix_room_kick`, `matrix_room_ban`, `matrix_room_unban`,
`matrix_room_set_power_level`, wired into `InviteUserPrompt`, `UserModeration`,
`PowerChip`, and `useCommands`. #375 explicitly leaves two things residual:

- **Member list _read_** — enumerating the room's members for the member list /
  people drawer.
- **Power-level _read_** — reading the current `m.room.power_levels` state to
  render power tags, sort/filter by power, and gate permission-sensitive UI.

This inventory scopes that **read** residual as **V-ROOMS.MEMBERS-READ**. The
write side (invite/kick/ban/unban/setPowerLevel) is **#375** — not this slice.

The IPC schema already defines a `members` stream topic and a `RoomMember` DTO
on both sides (Rust `dto/member.rs`, TS `matrix-dto/member.ts`, `streamBody.ts`
case `'members'`), but **no live native producer/command is wired to it**. The
member list read still runs on the live `matrix-js-sdk` client via
`useRoomMembers` → `mx.getRoom(roomId).getMembers()` / `room.loadMembersIfNeeded()`
and `RoomMemberEvent.Membership` / `RoomMemberEvent.PowerLevel` listeners.

---

## 2. Residual table — V-ROOMS.MEMBERS-READ

| Path                                                          | Role                                                                                                                                                                                          | Gap                                                                                                      | ID                                          |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| `synara/src/app/hooks/useRoomMembers.ts`                      | Core member-list hook: `mx.getRoom(roomId).getMembers()`, `room.loadMembersIfNeeded()`, and `RoomMemberEvent.Membership` / `RoomMemberEvent.PowerLevel` listeners to refresh the list         | No native member snapshot; JS `getMembers`/`loadMembersIfNeeded` + client event listeners on live client | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/features/common-settings/members/Members.tsx` | Full member-list page: `useRoomMembers`, `usePowerLevels`, `useGetMemberPowerLevel`, `useGetMemberPowerTag`, `useRoomCreators`, filter/sort, virtualized `MemberTile` list                    | Renders JS `RoomMember` objects; power tags from JS power-level read                                     | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/features/room/MembersDrawer.tsx`              | People drawer (room + lobby): `usePowerLevelsContext`, `useGetMemberPowerLevel`, `useGetMemberPowerTag`, `useRoomCreators`, filter/sort, `MemberItem`                                         | Renders JS `RoomMember` objects; power tags from JS power-level read                                     | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/features/lobby/Lobby.tsx`                     | Space lobby: `useRoomMembers(mx, space.roomId)` for the drawer, `usePowerLevels(space)`, `useRoomsPowerLevels` for hierarchy permission gating                                                | Member list + per-room power-level read on JS client                                                     | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/hooks/usePowerLevels.ts`                      | `usePowerLevels` / `useRoomsPowerLevels` read `m.room.power_levels` via `getStateEvent`; `readPowerLevel`, `useGetMemberPowerLevel`, `getPermissionPower` derive per-user/event/action powers | No native power-level read; JS `getStateEvent(RoomPowerLevels)` on live client                           | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/usePowerLevelTags.ts`                   | `usePowerLevelTags` reads `m.room.power_level_tags` state + derives tag labels from used powers                                                                                               | JS state read for power-tag labels                                                                       | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/useMemberPowerTag.ts`                   | `useGetMemberPowerTag` / `useFlattenPowerTagMembers` group members by power tag                                                                                                               | Power tag derived from JS power-level read                                                               | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/useRoomCreators.ts`                     | `useRoomCreators` / `getRoomCreatorsForRoomId` read `m.room.create` to build the creators set (creator power tag + permission short-circuit)                                                  | JS `getStateEvent(RoomCreate)` on live client                                                            | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/useMemberFilter.ts`                     | `useMembershipFilter` filters `RoomMember[]` by `membership` (joined/invited/left/kicked/banned)                                                                                              | Operates on JS `RoomMember` objects                                                                      | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/hooks/useMemberSort.ts`                       | `useMemberSort` / `useMemberPowerSort` sort `RoomMember[]` by name / join ts / power                                                                                                          | Operates on JS `RoomMember` objects + JS power read                                                      | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/hooks/useMemberPowerCompare.ts`               | `useMemberPowerCompare` compares two users' power (creator short-circuit + `readPowerLevel.user`)                                                                                             | JS power-level read                                                                                      | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/useRoomPermissions.ts`                  | `getRoomPermissionsAPI` / `useRoomPermissions` gate permission-sensitive UI from creators + power levels                                                                                      | JS power-level read                                                                                      | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/components/member-tile/MemberTile.tsx`        | Renders a member row (name, username, avatar) from a JS `RoomMember`                                                                                                                          | Consumes JS `RoomMember` object                                                                          | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/utils/room.ts`                                | `getStateEvent` / `getStateEvents` (power-level + create reads), `getMemberDisplayName`, `getMemberAvatarMxc`, `getMemberSearchStr`                                                           | JS state/member reads on live client                                                                     | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/utils/matrix.ts`                              | `getOldestMember`, DM-peer member helpers (`room.getMember(userId)`, `room.getMembers()`)                                                                                                     | JS member reads on live client                                                                           | **V-ROOMS.MEMBERS-READ**                    |

**Note:** `useRoomTypingMembers` / `state/typingMembers.ts` are **native** (typing
stream) and out of scope. The `applyPermissionPower` / `getPermissionPower`
write-side helpers in `usePowerLevels.ts` are used by the **PowersEditor bulk PL
rewrite**, which #375 leaves residual as a _write_ — coordinate with #375; the
read-side `readPowerLevel` / `getPermissionPower` are this slice.

---

## 3. Proposed slice — native member-list + power-level read commands

When this residual is claimed, the native slice should expose read commands over
IPC and delete the JS read owners. Proposed IPC names (fail-closed):

- `matrix_room_members_snapshot` — return the full member list for a room as an
  array of `RoomMember` DTOs (roomId, userId, displayName, avatarUrl, membership,
  powerLevel, isDirectTarget). Reuses the existing `members` stream topic body
  shape (`{ members: RoomMember[] }`) already validated in `streamBody.ts` /
  `stream_body.rs` and the existing `RoomMember` DTO on both sides.
- `matrix_room_power_levels_snapshot` — return the current `m.room.power_levels`
  content for a room (users, users_default, events, events_default, state_default,
  actions, notifications) as a typed DTO, so power tags / permission gating /
  power sort can be computed natively without the JS `getStateEvent` read.
- `matrix_room_creators_snapshot` — return the room creator set (from
  `m.room.create` + `additional_creators`) for creator power-tag / permission
  short-circuit. (Could be folded into the power-levels snapshot; listed
  separately for clarity.)

**Deletion list** (physical deletion per [full-vertical-policy.md](full-vertical-policy.md)):
`useRoomMembers.ts` (the `mx.getRoom().getMembers()` / `loadMembersIfNeeded` /
`RoomMemberEvent` listener surface); the JS power-level read in `usePowerLevels.ts`
(`usePowerLevels` / `useRoomsPowerLevels` / `readPowerLevel` /
`useGetMemberPowerLevel` / `getPermissionPower`), `usePowerLevelTags.ts`,
`useMemberPowerTag.ts`, `useRoomCreators.ts`, `useRoomPermissions.ts`,
`useMemberPowerCompare.ts`; the member-list consumers `Members.tsx`,
`MembersDrawer.tsx`, `Lobby.tsx` (member + power-level read paths), `MemberTile.tsx`;
and the JS member/state read helpers in `utils/room.ts` / `utils/matrix.ts`
(`getMemberDisplayName`, `getMemberAvatarMxc`, `getMemberSearchStr`,
`getOldestMember`, DM-peer helpers). Keep the native typing read and the #375
moderation **write** commands intact. Verify no other consumers of
`useRoomMembers` / `usePowerLevels` / `useRoomPermissions` remain before deletion
(a full `grep -rn "useRoomMembers\|usePowerLevels\|useRoomPermissions"` over
`synara/src`).

**Fail-closed:** on a native logged-in session, absence/failure of any
`matrix_room_members_snapshot` / `matrix_room_power_levels_snapshot` /
`matrix_room_creators_snapshot` command is terminal — the member list / people
drawer / lobby must not fall through to `mx.getRoom().getMembers()` /
`getStateEvent`. Legacy JS read paths remain only for non-native web sessions.

---

## 4. Non-goals / out of scope

| Item                                                                 | Status                                                                        |
| -------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| **Room moderation writes** (invite/kick/ban/unban/setPowerLevel)     | **#375** — open/draft; not this slice (serial lock on `product.rs`)           |
| **PowersEditor bulk PL rewrite** (write-side `applyPermissionPower`) | #375 residual _write_; coordinate, not this read slice                        |
| **Typing members read**                                              | Native (typing stream) — not a residual                                       |
| **Room membership / unread**                                         | [p4.3-membership-unread.md](p4.3-membership-unread.md) — separate             |
| **P4.6 member index**                                                | [p4.6-members.md](p4.6-members.md) — pure index over DTOs; no SDK member APIs |
| Open product PRs                                                     | **#375** (moderation writes) — do not touch `product.rs`                      |
| Umbrella merge to `main`                                             | **#39** — needs explicit user approval                                        |
| Cutover / dual-backend removal                                       | #240 HOLD; no cutover                                                         |

---

## 5. Confidence

**Confidence: high** for the inventory. I traced the member-list **read** from
the two primary consumers (`Members.tsx`, `MembersDrawer.tsx`) and the lobby
(`Lobby.tsx`) back through `useRoomMembers` (`mx.getRoom().getMembers()` +
`loadMembersIfNeeded` + `RoomMemberEvent` listeners) and the power-level read
(`usePowerLevels` / `useRoomsPowerLevels` → `getStateEvent(RoomPowerLevels)`),
plus the supporting power-tag / creator / filter / sort / permission hooks and
the `MemberTile` renderer. I confirmed the `members` stream topic and `RoomMember`
DTO already exist on both sides but have **no live native producer**, so the read
still runs on the JS client. Possible missed files: any other consumer of
`useRoomMembers` / `usePowerLevels` / `useRoomPermissions` / `getMemberDisplayName`
outside the listed paths (e.g. a barrel re-export or a profile/mention surface) —
verify during implementation with a full `grep -rn` over `synara/src`.

## Done-when

- `matrix_room_members_snapshot` returns the room member list as `RoomMember[]`
  (matches the `members` stream-topic body shape).
- `matrix_room_power_levels_snapshot` (+ `matrix_room_creators_snapshot` if kept
  separate) return the power-level / creator read projections.
- `Members.tsx`, `MembersDrawer.tsx`, `Lobby.tsx` member/power read paths and
  `MemberTile.tsx` render from native snapshots, fail-closed on desktop.
- `useRoomMembers.ts`, the JS power-level read hooks, and the JS member/state
  read helpers are physically deleted (per full-vertical policy).
- Production `matrix-js-sdk` import count drops accordingly; allowlist updated.
