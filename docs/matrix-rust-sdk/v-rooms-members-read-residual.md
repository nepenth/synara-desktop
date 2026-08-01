# V-ROOMS.MEMBERS-READ — native member list / power-level read residual inventory

| Field   | Value                                                                                                                                                                       |
| ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status  | **Post-#405 residual target** — #395 made the Members settings list native; #405 closes drawer/lobby/mentions member reads when merged, while power-level/creator reads remain residual |
| Tip SHA | `52953091` (docs base; #405 product head is `cd2d57b4`)                                                                                                                       |
| Base    | `feature/matrix-rust-sdk-full-replacement`                                                                                                                                  |
| Policy  | [full-vertical-policy.md](full-vertical-policy.md) — physical deletion inside each owning slice                                                                             |
| Follow-up | **#405 prerequisite** — product head `cd2d57b4` wires drawer/lobby/mentions member reads; this draft records the post-merge boundary, but the product commit is not in docs base `52953091` |
| Related | **#375 merged** (native room moderation **write** vertical), P4.6 members index, [p4.6-members.md](p4.6-members.md), [p4.3-membership-unread.md](p4.3-membership-unread.md) |

> **Scope guard.** Docs only. No product code in `product.rs` or any TS. The
> moderation-write vertical **#375 is already merged**; this inventory does not
> change it. It also does not touch **#39** (umbrella) or any timeline/send
> slice. This docs-only draft is based at `52953091`; it does not merge or
> modify product PR **#405**. The post-#405 disposition below is conditional on
> `cd2d57b4` landing. No cutover.

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

### Post-#405 disposition

The product head for **#405** is `cd2d57b4`. It wires the native member snapshot
into `MembersDrawer` and `UserMentionAutocomplete`, removes the room/lobby
call-site ownership, and adds DTO-boundary coverage. The docs base for this
draft remains `52953091`, so the rows below stay explicitly conditional until
#405 merges into the integration branch.

| Surface | Current docs base (`52953091`) | After #405 merges | Residual after #405 |
| ------- | ------------------------------ | ----------------- | ------------------ |
| Members settings list | Native via #395 | Done | None for the native member snapshot |
| Room/lobby people drawer | Pending #405; JS member path | Done via native snapshot, fail-closed | Power-level/creator reads |
| Mention autocomplete | Pending #405; JS member path | Done via native snapshot, fail-closed | None for member enumeration |
| Power tags, permission gates, and creator short-circuits | JS state reads | Unchanged | **Power-level/creator read residual** |

This is the intended post-#405 truth: drawer/lobby/mentions member reads close
with the product merge; power-level/creator reads do not. A native desktop
session must never use the legacy member path as a fallback, and
`dual_backend` remains forbidden.

## 1. What this residual covers

PR **#375** (merged) landed the native room moderation **write** vertical:
`matrix_room_invite`, `matrix_room_kick`, `matrix_room_ban`, `matrix_room_unban`,
`matrix_room_set_power_level`, wired into `InviteUserPrompt`, `UserModeration`,
`PowerChip`, and `useCommands`. #375 leaves two things residual:

- **Member list _read_** — enumerating the room's members for the member list /
  people drawer. #395 owns the settings list, and #405 closes the native
  drawer/lobby/mention member enumeration once it merges.
- **Power-level / creator _read_** — reading the current `m.room.power_levels`
  and `m.room.create` state to
  render power tags, sort/filter by power, and gate permission-sensitive UI.

This inventory scopes that **read** residual as **V-ROOMS.MEMBERS-READ**. The
write side (invite/kick/ban/unban/setPowerLevel) is **#375** — not this slice.

The IPC schema defines a `members` stream topic and a `RoomMember` DTO on both
sides (Rust `dto/member.rs`, TS `matrix-dto/member.ts`, `streamBody.ts` case
`'members'`). **#395 now also has a live native producer/command:**
`matrix_room_members_snapshot` lives in `product.rs`, and
`nativeRoomMembersOwner` invokes and validates it. On the native session path
wired in `Members.tsx` (and, after #405 merges, the drawer and mention paths),
the list no longer falls through to the JS client. The legacy
`useRoomMembers` path still reads
`mx.getRoom(roomId).getMembers()` / `room.loadMembersIfNeeded()` and listens for
`RoomMemberEvent.Membership` / `RoomMemberEvent.PowerLevel` when
`nativeSession` is not selected or a remaining consumer has not been migrated.

---

## 2. Residual table — V-ROOMS.MEMBERS-READ

| Path                                                                        | Role                                                                                                                                                                                          | Gap                                                                                                                                                 | ID                                          |
| --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| `synara/src/app/hooks/useRoomMembers.ts`                                    | Explicit native-session branch via `nativeRoomMembersOwner`; legacy branch uses `mx.getRoom(roomId).getMembers()`, `room.loadMembersIfNeeded()`, and membership/power listeners               | #395/#405 own the native desktop member surfaces; retain the JS branch only for an explicit non-native/web route, never as a native fallback     | **Boundary; no native fallback**            |
| `synara/src/app/features/common-settings/members/Members.tsx`               | Full member-list page; passes `nativeSession` to `useRoomMembers`, then uses power/tag/creator hooks, filter/sort, and virtualized `MemberTile`                                               | Member list is native and fail-closed on native desktop; power tags/creator reads remain JS, and web/non-native sessions use the legacy member path | **V-ROOMS.MEMBERS-READ** (power/creator)   |
| `synara/src/app/features/room/Room.tsx`                                     | Owns the room power-level context; after #405 the people drawer itself owns the native member snapshot                                                                                      | Member drawer wiring is closed by #405; `usePowerLevels(room)` remains a JS state read on native desktop                                         | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/features/room/MembersDrawer.tsx`                            | Native member snapshot after #405; `usePowerLevelsContext`, `useGetMemberPowerLevel`, `useGetMemberPowerTag`, `useRoomCreators`, filter/sort, `MemberItem`                                 | Member enumeration is closed by #405; power/tag/creator reads remain residual                                                                      | **V-ROOMS.MEMBERS-READ** (power/creator)   |
| `synara/src/app/features/lobby/Lobby.tsx`                                   | Space hierarchy power reads via `usePowerLevels(space)`, `useRoomsPowerLevels`, `getRoomPermissionsAPI`, and creator helpers; drawer member snapshot moves to #405-owned `MembersDrawer` | Member enumeration is closed by #405; per-room power-level/creator reads remain residual                                                        | **V-ROOMS.MEMBERS-READ** (power/creator)   |
| `synara/src/app/components/editor/autocomplete/UserMentionAutocomplete.tsx` | Native member snapshot after #405; DTO-aware filtering/search for mention autocomplete                                                                                                     | Member enumeration is closed by #405 and fail-closed on native desktop                                                                         | **Closed by #405**                          |
| `synara/src/app/hooks/usePowerLevels.ts`                                    | `usePowerLevels` / `useRoomsPowerLevels` read `m.room.power_levels` via `getStateEvent`; `readPowerLevel`, `useGetMemberPowerLevel`, `getPermissionPower` derive per-user/event/action powers | No native power-level read; JS `getStateEvent(RoomPowerLevels)` on live client                                                                      | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/usePowerLevelTags.ts`                                 | `usePowerLevelTags` reads `m.room.power_level_tags` state + derives tag labels from used powers                                                                                               | JS state read for power-tag labels                                                                                                                  | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/useMemberPowerTag.ts`                                 | `useGetMemberPowerTag` / `useFlattenPowerTagMembers` group members by power tag                                                                                                               | Power tag derived from JS power-level read                                                                                                          | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/useRoomCreators.ts`                                   | `useRoomCreators` / `getRoomCreatorsForRoomId` read `m.room.create` to build the creators set (creator power tag + permission short-circuit)                                                  | JS `getStateEvent(RoomCreate)` on live client                                                                                                       | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/useMemberFilter.ts`                                   | `useMembershipFilter` filters the shared member-list type by `membership` (joined/invited/left/kicked/banned)                                                                                 | #395/#405 support both native DTOs and legacy JS members; no separate native member-read gap remains in this filter                                  | **Closed for native member projection**      |
| `synara/src/app/hooks/useMemberSort.ts`                                     | `useMemberSort` / `useMemberPowerSort` sort the shared member-list type by name / join ts / power                                                                                             | Name/join sorting is native-DTO compatible; power sort still depends on the residual JS power read                                                   | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/useMemberPowerCompare.ts`                             | `useMemberPowerCompare` compares two users' power (creator short-circuit + `readPowerLevel.user`)                                                                                             | JS power-level read                                                                                                                                 | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/hooks/useRoomPermissions.ts`                                | `getRoomPermissionsAPI` / `useRoomPermissions` gate permission-sensitive UI from creators + power levels                                                                                      | JS power-level read                                                                                                                                 | **V-ROOMS.MEMBERS-READ** (power-level read) |
| `synara/src/app/components/member-tile/MemberTile.tsx`                      | Renders a member row (name, username, avatar) from the shared JS/native member-list type                                                                                                      | #395 DTO rendering remains valid; the #405 drawer uses its DTO-aware `MemberItem` boundary                                                           | **Closed for native member list**            |
| `synara/src/app/utils/room.ts`                                              | `getStateEvent` / `getStateEvents` (power-level + create reads), plus shared member display/avatar/search helpers                                                                              | Power-level/create state reads remain residual; unrelated member helper callers outside drawer/mentions are not closed by #405                         | **V-ROOMS.MEMBERS-READ** (power/creator)   |
| `synara/src/app/utils/matrix.ts`                                            | `getOldestMember`, DM-peer member helpers (`room.getMember(userId)`, `room.getMembers()`)                                                                                                     | JS member reads on live client                                                                                                                      | **V-ROOMS.MEMBERS-READ**                    |

**Note:** `useRoomTypingMembers` / `state/typingMembers.ts` are **native** (typing
stream) and out of scope. The `applyPermissionPower` / `getPermissionPower`
write-side helpers in `usePowerLevels.ts` are used by the **PowersEditor bulk PL
rewrite**, which remains a separate residual write after merged #375; it is not
this read slice. The read-side `readPowerLevel` / `getPermissionPower` are this
slice.

---

## 3. Remaining slice — native power-level/creator reads

### Member wiring boundary after #405

`matrix_room_members_snapshot` is no longer proposed: it is live in
`src-tauri/src/matrix/auth/product.rs`, registered in the Tauri ACL/build
surfaces, and owned on the TS side by `nativeRoomMembersOwner`. The owner
validates the room/session-shaped DTO and treats unavailable or malformed IPC
as terminal. `Members.tsx` selects this owner with `nativeSession`; the #405
product head applies the same explicit owner selection to `MembersDrawer` and
`UserMentionAutocomplete`, while `Room.tsx` and `Lobby.tsx` stop owning the
member list. None of these native paths may fall through to
`mx.getRoom().getMembers()`.

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

The remaining power/creator consumers must be migrated before this residual can
close:

- Replace the JS power-level/creator reads in `usePowerLevels.ts`,
  `usePowerLevelTags.ts`, `useMemberPowerTag.ts`, `useRoomCreators.ts`,
  `useRoomPermissions.ts`, and `useMemberPowerCompare.ts` with the native
  projections. Keep the PowersEditor write path scoped to its separate
  powers-bulk slice.
- Keep the explicit non-native web route only if that route remains supported;
  its JS member branch and unrelated member display/profile helpers are not a
  native desktop fallback and are not claimed closed by #405. Do not delete
  shared UI/helper files merely because the drawer and mention data sources
  changed.

Verify that no native desktop consumer still uses the two-argument
`useRoomMembers` path, and that all remaining `usePowerLevels` /
`useRoomPermissions` / creator reads have a native owner before retiring the
JS owners.

**Fail-closed:** #395 already makes absence, failure, or malformed output from
`matrix_room_members_snapshot` terminal whenever `nativeSession` is selected
for `Members.tsx`, `MembersDrawer`, or `UserMentionAutocomplete`; those paths
never fall through to `getMembers()`. The people-drawer/lobby/mention member
enumeration is therefore closed by #405 once it merges; power-level and creator
reads remain residual. Once wired, failure of
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
| Product input                                                        | **#405** product head `cd2d57b4`; drawer/lobby/mentions member wiring is the post-merge boundary; power/creator reads are not included |
| Product changes in this PR                                           | None; this draft is docs-only and based at `52953091`                              |
| Umbrella merge to `main`                                             | **#39** — needs explicit user approval                                              |
| Cutover / dual-backend removal                                       | #240 HOLD; no cutover                                                               |

---

## 5. Confidence

**Confidence: high** for the inventory. I traced the member-list **read** from
the two primary consumers (`Members.tsx`, `MembersDrawer.tsx`), the room owner
(`Room.tsx`), the lobby (`Lobby.tsx`), and mention autocomplete back through
`useRoomMembers`. At tip `22f1f06d`, I confirmed the live native producer in
`product.rs`, the fail-closed `nativeRoomMembersOwner`, and the
`nativeSession` wiring for `Members.tsx`; the #405 product head
`cd2d57b4` extends that owner selection to the drawer and mention surfaces. I
also traced the power-level read (`usePowerLevels` /
`useRoomsPowerLevels` → `getStateEvent(RoomPowerLevels)`), plus the supporting
power-tag / creator / filter / sort / permission hooks and `MemberTile` renderer.
Possible missed files: any other consumer of `usePowerLevels` /
`useRoomPermissions` / creator state reads outside the listed paths (e.g. a
barrel re-export or another permission surface) — verify during implementation
with a full `rg` over `synara/src`. #405's member-snapshot wiring does not close
unrelated profile, notification, call, or DM member lookups.

## Done-when

- **Already true from #395:** `matrix_room_members_snapshot` returns the room
  member list as `RoomMember[]` (matching the `members` stream-topic body
  shape), and `Members.tsx` selects it through a fail-closed native owner.
- **After #405 merges:** `MembersDrawer.tsx` and
  `UserMentionAutocomplete.tsx` select the same native member owner on native
  desktop; their member rows/search boundaries accept the native DTO, and
  `Room.tsx`/`Lobby.tsx` no longer own a legacy member read.
- `matrix_room_power_levels_snapshot` (+ `matrix_room_creators_snapshot` if kept
  separate) return the power-level / creator read projections.
- `Members.tsx`, `MembersDrawer.tsx`, `Lobby.tsx`, and mention member reads use
  native snapshots and fail closed on native desktop; the explicit non-native
  web route is the only remaining JS member route if web support is retained.
- Native desktop no longer uses the JS power-level/creator reads; member
  snapshot surfaces remain explicit-owner/fail-closed, and unrelated member
  helper routes are not deleted merely because #405 landed. Preserve the
  separate PowersEditor write slice.
- Production `matrix-js-sdk` import count drops accordingly; allowlist updated.
- `dual_backend` remains forbidden throughout; this first slice and its follow-up
  are explicit owner selection, not a dual-backend mode.
