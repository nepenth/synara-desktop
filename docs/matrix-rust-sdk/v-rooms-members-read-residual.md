# V-ROOMS.MEMBERS-READ — native member list / power-level read residual inventory

| Field   | Value                                                                                                                                                                       |
| ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status  | **Partial product (first slice)** — #395 made the Members settings list native; drawer/lobby/mentions + power-level/creator reads remain residual                           |
| Tip SHA | `22f1f06d` (merged #395 members-read first slice)                                                                                                                           |
| Base    | `feature/matrix-rust-sdk-full-replacement`                                                                                                                                  |
| Policy  | [full-vertical-policy.md](full-vertical-policy.md) — physical deletion inside each owning slice                                                                             |
| Follow-up | **#405 open/in-flight** — drawer/lobby/mentions member wiring is proposed, not merged                                                                              |
| Related | **#375 merged** (native room moderation **write** vertical), P4.6 members index, [p4.6-members.md](p4.6-members.md), [p4.3-membership-unread.md](p4.3-membership-unread.md) |

> **Scope guard.** Docs only. No product code in `product.rs` or any TS. The
> moderation-write vertical **#375 is already merged**; this inventory does not
> change it. It also does not touch **#39** (umbrella) or any timeline/send
> slice. **#405 is still open/in-flight** and is not treated as landed here. No
> cutover.

---

## Progress (first slice / #395)

Landed on the product branch at **#395**:

- IPC `matrix_room_members_snapshot` in `src-tauri/src/matrix/auth/product.rs`, backed by live `matrix-sdk` `Room::members`, plus Tauri ACL/build registration
- Fail-closed `nativeRoomMembersOwner` and the `useRoomMembers(..., nativeSession)` path for **Members.tsx** settings
- Native DTO-aware member filtering/sorting/rendering for that first slice

Still residual on desktop native:

- `Room.tsx`, `MembersDrawer`, `Lobby.tsx`, and `UserMentionAutocomplete` still use the legacy two-argument `useRoomMembers` call and therefore do not pass `nativeSession`
- `Members.tsx` and those remaining consumers still use JS power-level/creator read owners
- `useRoomMembers` retains its JS `getMembers` / `loadMembersIfNeeded` + event-listener path for non-native sessions and for existing legacy call sites until they are migrated

This is a scoped first slice, not dual_backend: native session does not fall through to JS for the wired Members settings screen; remaining call sites stay residual while the open #405 follow-up is in flight.

### Follow-up status (#405)

PR **#405 is open/in-flight** against this product branch and proposes native
member-snapshot wiring for the room/lobby drawer and mention autocomplete. It
has not merged at this tip, so those consumers remain residual in this
inventory; do not describe the proposed wiring as landed until #405 merges and
the inventory is refreshed.

## 1. What this residual covers

PR **#375** (merged) landed the native room moderation **write** vertical:
`matrix_room_invite`, `matrix_room_kick`, `matrix_room_ban`, `matrix_room_unban`,
`matrix_room_set_power_level`, wired into `InviteUserPrompt`, `UserModeration`,
`PowerChip`, and `useCommands`. #375 leaves two things residual:

- **Member list _read_** — enumerating the room's members for the member list /
  people drawer.
- **Power-level _read_** — reading the current `m.room.power_levels` state to
  render power tags, sort/filter by power, and gate permission-sensitive UI.

This inventory scopes that **read** residual as **V-ROOMS.MEMBERS-READ**. The
write side (invite/kick/ban/unban/setPowerLevel) is **#375** — not this slice.

The IPC schema defines a `members` stream topic and a `RoomMember` DTO on both
sides (Rust `dto/member.rs`, TS `matrix-dto/member.ts`, `streamBody.ts` case
`'members'`). **#395 now also has a live native producer/command:**
`matrix_room_members_snapshot` lives in `product.rs`, and
`nativeRoomMembersOwner` invokes and validates it. On the native session path
wired in `Members.tsx`, the list no longer falls through to the JS client. The
legacy `useRoomMembers` path still reads
`mx.getRoom(roomId).getMembers()` / `room.loadMembersIfNeeded()` and listens for
`RoomMemberEvent.Membership` / `RoomMemberEvent.PowerLevel` when
`nativeSession` is not selected or a remaining consumer has not been migrated.

---

## 2. Residual table — V-ROOMS.MEMBERS-READ

| Path                                                                        | Role                                                                                                                                                                                          | Gap                                                                                                                                                 | ID                                          |
| --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| `synara/src/app/hooks/useRoomMembers.ts`                                    | Explicit native-session branch via `nativeRoomMembersOwner`; legacy branch uses `mx.getRoom(roomId).getMembers()`, `room.loadMembersIfNeeded()`, and membership/power listeners               | Native snapshot is landed, but JS path remains for non-native sessions and existing legacy call sites; remaining sites must receive `nativeSession` | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/features/common-settings/members/Members.tsx`               | Full member-list page; passes `nativeSession` to `useRoomMembers`, then uses power/tag/creator hooks, filter/sort, and virtualized `MemberTile`                                               | Member list is native and fail-closed on native desktop; power tags/creator reads remain JS, and web/non-native sessions use the legacy member path | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/features/room/Room.tsx`                                     | Owns the room people-drawer inputs: `useRoomMembers(mx, room.roomId)` and `usePowerLevels(room)`                                                                                              | Still uses the default JS member path and JS power-level context on native desktop                                                                  | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/features/room/MembersDrawer.tsx`                            | People drawer (room + lobby): `usePowerLevelsContext`, `useGetMemberPowerLevel`, `useGetMemberPowerTag`, `useRoomCreators`, filter/sort, `MemberItem`                                         | Still receives JS `RoomMember` objects; member and power/tag/creator reads remain residual                                                          | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/features/lobby/Lobby.tsx`                                   | Space lobby: `useRoomMembers(mx, space.roomId)` for the drawer, `usePowerLevels(space)`, `useRoomsPowerLevels` for hierarchy permission gating                                                | Still uses the JS member path and per-room power-level reads on native desktop                                                                      | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/components/editor/autocomplete/UserMentionAutocomplete.tsx` | Mention search: `useRoomMembers(mx, roomId)` and member filtering                                                                                                                             | Still uses the default JS member path; no native-session wiring                                                                                     | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/hooks/usePowerLevels.ts`                                    | `usePowerLevels` / `useRoomsPowerLevels` read `m.room.power_levels` via `getStateEvent`; `readPowerLevel`, `useGetMemberPowerLevel`, `getPermissionPower` derive per-user/event/action powers | No native power-level read; JS `getStateEvent(RoomPowerLevels)` on live client                                                                      | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/usePowerLevelTags.ts`                                 | `usePowerLevelTags` reads `m.room.power_level_tags` state + derives tag labels from used powers                                                                                               | JS state read for power-tag labels                                                                                                                  | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/useMemberPowerTag.ts`                                 | `useGetMemberPowerTag` / `useFlattenPowerTagMembers` group members by power tag                                                                                                               | Power tag derived from JS power-level read                                                                                                          | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/useRoomCreators.ts`                                   | `useRoomCreators` / `getRoomCreatorsForRoomId` read `m.room.create` to build the creators set (creator power tag + permission short-circuit)                                                  | JS `getStateEvent(RoomCreate)` on live client                                                                                                       | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/useMemberFilter.ts`                                   | `useMembershipFilter` filters the shared member-list type by `membership` (joined/invited/left/kicked/banned)                                                                                 | #395 supports both native DTOs and legacy JS members; remaining native gap is consumer wiring, not this filter                                      | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/hooks/useMemberSort.ts`                                     | `useMemberSort` / `useMemberPowerSort` sort the shared member-list type by name / join ts / power                                                                                             | #395 supports native DTOs for the first slice; power sort still depends on the residual JS power read                                               | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/hooks/useMemberPowerCompare.ts`                             | `useMemberPowerCompare` compares two users' power (creator short-circuit + `readPowerLevel.user`)                                                                                             | JS power-level read                                                                                                                                 | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/useRoomPermissions.ts`                                | `getRoomPermissionsAPI` / `useRoomPermissions` gate permission-sensitive UI from creators + power levels                                                                                      | JS power-level read                                                                                                                                 | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/components/member-tile/MemberTile.tsx`                      | Renders a member row (name, username, avatar) from the shared JS/native member-list type                                                                                                      | #395 added native DTO rendering for the Members settings first slice; drawer/lobby/mention callers still provide legacy JS members                  | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/utils/room.ts`                                              | `getStateEvent` / `getStateEvents` (power-level + create reads), `getMemberDisplayName`, `getMemberAvatarMxc`, `getMemberSearchStr`                                                           | JS state/member reads on live client                                                                                                                | **V-ROOMS.MEMBERS-READ**                    |
| `synara/src/app/utils/matrix.ts`                                            | `getOldestMember`, DM-peer member helpers (`room.getMember(userId)`, `room.getMembers()`)                                                                                                     | JS member reads on live client                                                                                                                      | **V-ROOMS.MEMBERS-READ**                    |

**Note:** `useRoomTypingMembers` / `state/typingMembers.ts` are **native** (typing
stream) and out of scope. The `applyPermissionPower` / `getPermissionPower`
write-side helpers in `usePowerLevels.ts` are used by the **PowersEditor bulk PL
rewrite**, which remains a separate residual write after merged #375; it is not
this read slice. The read-side `readPowerLevel` / `getPermissionPower` are this
slice.

---

## 3. Remaining slice — native power-level/creator reads and residual consumers

### Already landed in #395

`matrix_room_members_snapshot` is no longer proposed: it is live in
`src-tauri/src/matrix/auth/product.rs`, registered in the Tauri ACL/build
surfaces, and owned on the TS side by `nativeRoomMembersOwner`. The owner
validates the room/session-shaped DTO and treats unavailable or malformed IPC
as terminal. `Members.tsx` selects this owner with `nativeSession`; it does not
fall through to `mx.getRoom().getMembers()`.

### Remaining native read commands

The follow-up native slice should expose these reads over IPC and remove the JS
read owners from the native desktop route:

- `matrix_room_power_levels_snapshot` — return the current `m.room.power_levels`
  content for a room (users, users_default, events, events_default, state_default,
  actions, notifications) as a typed DTO, so power tags / permission gating /
  power sort can be computed natively without the JS `getStateEvent` read.
- `matrix_room_creators_snapshot` — return the room creator set (from
  `m.room.create` + `additional_creators`) for creator power-tag / permission
  short-circuit. (Could be folded into the power-levels snapshot; listed
  separately for clarity.)

### Consumer and deletion boundary

The remaining consumers must be migrated before this residual can close:

- Pass `nativeSession` through `Room.tsx`, `MembersDrawer`, `Lobby.tsx`, and
  `UserMentionAutocomplete`; adapt those render/search boundaries to the
  native `RoomMember` DTO already supported by the first slice.
- Replace the JS power-level/creator reads in `usePowerLevels.ts`,
  `usePowerLevelTags.ts`, `useMemberPowerTag.ts`, `useRoomCreators.ts`,
  `useRoomPermissions.ts`, and `useMemberPowerCompare.ts` with the native
  projections. Keep the PowersEditor write path scoped to its separate
  powers-bulk slice.
- Retire the `matrix-js-sdk` member/state reads from the native desktop route,
  including the legacy branch of `useRoomMembers.ts` once all native desktop
  consumers are migrated, and the relevant helpers in `utils/room.ts` /
  `utils/matrix.ts` (`getMemberDisplayName`, `getMemberAvatarMxc`,
  `getMemberSearchStr`, `getOldestMember`, and DM-peer helpers). Do not delete
  the shared UI files merely because their data source changes.
- Keep the explicit non-native web route only if that route remains supported;
  it is not a second desktop backend and must not become a native/JS fallback.

Verify that no native desktop consumer still uses the two-argument
`useRoomMembers` path and that all remaining `usePowerLevels` /
`useRoomPermissions` reads have a native owner before retiring the JS owners.

**Fail-closed:** #395 already makes absence, failure, or malformed output from
`matrix_room_members_snapshot` terminal whenever `nativeSession` is selected
for `Members.tsx`; that path never falls through to `getMembers()`. The people
drawer, lobby, and mention paths are not yet native-owned, so their current JS
reads remain an explicitly tracked residual. Once wired, failure of
`matrix_room_members_snapshot`, `matrix_room_power_levels_snapshot`, or
`matrix_room_creators_snapshot` must be terminal on native desktop. A native
session must never select a JS fallback, and `dual_backend` remains forbidden.

---

## 4. Non-goals / out of scope

| Item                                                                 | Status                                                                              |
| -------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| **Room moderation writes** (invite/kick/ban/unban/setPowerLevel)     | **#375 merged**; not this read slice                                                |
| **PowersEditor bulk PL rewrite** (write-side `applyPermissionPower`) | Separate residual write; coordinate with the powers-bulk slice, not this read slice |
| **Typing members read**                                              | Native (typing stream) — not a residual                                             |
| **Room membership / unread**                                         | [p4.3-membership-unread.md](p4.3-membership-unread.md) — separate                   |
| **P4.6 member index**                                                | [p4.6-members.md](p4.6-members.md) — pure index over DTOs; no SDK member APIs       |
| Open product PRs                                                     | **#405 open/in-flight** — proposes drawer/lobby/mentions member-snapshot wiring; #375 is already merged and excluded from this row |
| Product changes in this PR                                           | None; this update is docs-only                                                      |
| Umbrella merge to `main`                                             | **#39** — needs explicit user approval                                              |
| Cutover / dual-backend removal                                       | #240 HOLD; no cutover                                                               |

---

## 5. Confidence

**Confidence: high** for the inventory. I traced the member-list **read** from
the two primary consumers (`Members.tsx`, `MembersDrawer.tsx`), the room owner
(`Room.tsx`), the lobby (`Lobby.tsx`), and mention autocomplete back through
`useRoomMembers`. At tip `22f1f06d`, I confirmed the live native producer in
`product.rs`, the fail-closed `nativeRoomMembersOwner`, and the
`nativeSession` wiring for `Members.tsx`; the remaining consumers still use the
legacy JS member path. I also traced the power-level read (`usePowerLevels` /
`useRoomsPowerLevels` → `getStateEvent(RoomPowerLevels)`), plus the supporting
power-tag / creator / filter / sort / permission hooks and `MemberTile` renderer.
Possible missed files: any other consumer of `useRoomMembers` /
`usePowerLevels` / `useRoomPermissions` / `getMemberDisplayName` outside the
listed paths (e.g. a barrel re-export or a profile surface) — verify during
implementation with a full `rg` over `synara/src`.

## Done-when

- **Already true from #395:** `matrix_room_members_snapshot` returns the room
  member list as `RoomMember[]` (matching the `members` stream-topic body
  shape), and `Members.tsx` selects it through a fail-closed native owner.
- `Room.tsx`, `MembersDrawer.tsx`, `Lobby.tsx`, and
  `UserMentionAutocomplete.tsx` select the native member owner on native
  desktop; their member rows/search boundaries accept the native DTO.
- `matrix_room_power_levels_snapshot` (+ `matrix_room_creators_snapshot` if kept
  separate) return the power-level / creator read projections.
- `Members.tsx`, `MembersDrawer.tsx`, `Lobby.tsx`, and mention member reads use
  native snapshots and fail closed on native desktop; the explicit non-native
  web route is the only remaining JS route if web support is retained.
- Native desktop no longer uses the JS power-level/creator reads or the legacy
  member/state read helpers; preserve the separate PowersEditor write slice.
- Production `matrix-js-sdk` import count drops accordingly; allowlist updated.
- `dual_backend` remains forbidden throughout; this first slice and its follow-up
  are explicit owner selection, not a dual-backend mode.
