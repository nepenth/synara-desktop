# V-ROOMS.MEMBERS-READ — native member list / power-level/tag read residual inventory

| Field   | Value                                                                                                                                                                       |
| ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status  | **Active post-#405/#439/#446/#450/#458/#461 READ residual** — #395/#405 close native member enumeration; #439 closes the separate bulk WRITE; #450 closes native power/creator owners, while custom tags and direct readers remain residual; #458/#461 do not alter this boundary |
| Tip SHA | `fd0dfbf4` (current docs/integration tip; post-#458/#461 product state with #450 at `103a653f`, #446 at `9fb341af`, #405 at `176fc7c5`, and #439 at `f92a33f9`) |
| Base    | `feature/matrix-rust-sdk-full-replacement`                                                                                                                                  |
| Policy  | [full-vertical-policy.md](full-vertical-policy.md) — physical deletion inside each owning slice                                                                             |
| Follow-up | **Custom power-level tags + direct readers** — #450 owns native power/creator snapshots and migrated permission paths; `in.synara.room.power_level_tags`, `via-servers.ts`, and `utils/room.ts` direct reads remain explicitly open |
| Related | **#375, #405, #439, #450, #458, and #461 merged** (moderation, member-enumeration, bulk power WRITE, power/creator READ, and unrelated presence/directory first-slice verticals), P4.6 members index, [p4.6-members.md](p4.6-members.md), [p4.3-membership-unread.md](p4.3-membership-unread.md) |

> **Scope guard.** Docs only. No product code in any Rust command module or any
> TS. The moderation-write vertical **#375 is already merged**; this inventory does not
> change it. It also does not touch **#39** (umbrella) or any timeline/send
> slice. This docs-only packet is refreshed at docs tip `fd0dfbf4`; the product
> state from `c1e9c3be` remains the basis for the merged #458/#461 slices. #405 and #439
> are already merged, #446's behavior-preserving product-command fan-out is
> present at `9fb341af`, and **#450 is merged at `103a653f`** and owns
> native power-level/creator snapshots for the migrated native paths. #458's
> presence first slice and #461's room-directory first slice are also in the
> base but do not change this residual. The accepted **#482** refresh was
> docs-only and did not change product ownership; this refresh records the
> current docs tip honestly. It does not
> claim custom power-level-tag READ or direct helper/plugin READ completion.
> No cutover; `dual_backend=false` and V-BURN remains **HOLD**.

---

## Progress (first slice / #395)

Landed on the product branch at **#395**:

- IPC `matrix_room_members_snapshot` in `src-tauri/src/matrix/members/product_commands.rs`, backed by live `matrix-sdk` `Room::members`, plus Tauri ACL/build registration
- Fail-closed `nativeRoomMembersOwner` and the `useRoomMembers(..., nativeSession)` path for **Members.tsx** settings
- Native DTO-aware member filtering/sorting/rendering for that first slice

Landed on the product branch in merged **#405** at `176fc7c5`:

- `MembersDrawer` and `UserMentionAutocomplete` select the native member snapshot
  with `nativeSession`; the drawer remains fail-closed on native desktop
- `Room.tsx` and `Lobby.tsx` no longer own a legacy member-list read
- DTO-aware drawer and mention boundaries plus source-wiring coverage landed

Landed on the product branch in merged **#439** at `f92a33f9`:

- Native bulk `m.room.power_levels` and `in.synara.room.power_level_tags`
  writes are owned by the native write slice
- The WRITE completion is separate from the READ ownership recorded below

The behavior-preserving **#446** product-command fan-out is merged at `9fb341af`.
It changes the Rust ownership/layout boundary. **#450** then lands at
`103a653f` with native power-level and creator READ snapshots plus migrated
native hook/permission ownership. The current docs/integration tip `fd0dfbf4`
also contains the unrelated #458 presence first slice and #461 room-directory
first slice.

Landed on the product branch in merged **#450** at `103a653f`:

- `matrix_room_power_levels_snapshot` and `matrix_room_creators_snapshot` are
  registered live-SDK commands with validated native owners
- `usePowerLevels`, `useRoomsPowerLevels`, and `useRoomCreators` select those
  owners on native sessions; native permission paths fail closed while loading
  or unavailable and do not fall through to JS state reads
- `RoomNavItem`, `Lobby`, and the native room/space permission fan-out consume
  the native projections

Still residual on desktop native:

- `usePowerLevelTags` has no native `in.synara.room.power_level_tags` snapshot;
  native sessions use generated/default tags, so persisted custom tag metadata
  is not native-read
- `via-servers.ts` and `utils/room.ts` retain direct `m.room.create` /
  `m.room.power_levels` reads for via-server selection and perfect-parent
  navigation
- `useRoomMembers` retains its JS `getMembers` / `loadMembersIfNeeded` and
  event-listener path for the explicit non-native/web route and its legacy
  two-argument overload; native list surfaces select the three-argument owner

This is a scoped first slice with `dual_backend=false`: native session does not
fall through to JS for any wired native member-list surface. The JS member
branch is retained only for the explicit non-native route, and the native list
path is not dual-backend.

### Post-#405 disposition

Merged **#405** at `176fc7c5` wires the native member snapshot into
`MembersDrawer` and `UserMentionAutocomplete`, removes the room/lobby call-site
ownership, and adds DTO-boundary coverage. Merged **#439** adds the separate
bulk power/tag WRITE owner. Merged **#450** closes native power/creator reads
for the migrated hook and permission paths; the rows below preserve the
remaining tag/direct-reader boundary.

| Surface | Current tip (`fd0dfbf4`) | Residual after merged #405/#439/#446/#450; #458/#461 are unrelated |
| ------- | ----------------------------------------------- | ------------------------------------ |
| Members settings list | Native via #395; fail-closed | None for the native member snapshot |
| Room/lobby people drawer | Native member snapshot plus native power/creator projections; fail-closed | Custom power-level tags and any direct helper/plugin read used by the route |
| Mention autocomplete | Native snapshot; fail-closed | None for member enumeration |
| Power tags, permission gates, and creator short-circuits | Native power/creator projections; native permission gates fail-closed; tags use generated/default values on native | **Custom power-level-tag READ; direct helper/plugin readers** |

This is the current post-#405/#439/#446/#450/#458/#461 truth: drawer/lobby/mention member
reads are closed, bulk power/tag writes are landed, and #450 closes the native
power/creator owners. Custom tag metadata and the explicitly listed direct
state readers remain open. A native desktop session must never use the legacy
member or power/creator path as a fallback, and `dual_backend=false` remains
the explicit policy state.

## 1. What this residual covers

PR **#375** (merged) landed the native room moderation **write** vertical:
`matrix_room_invite`, `matrix_room_kick`, `matrix_room_ban`, `matrix_room_unban`,
`matrix_room_set_power_level`, wired into `InviteUserPrompt`, `UserModeration`,
`PowerChip`, and `useCommands`. #375 leaves two things residual:

- **Member list _read_** — enumerating the room's members for the member list /
  people drawer. #395 owns the settings list, and merged #405 closes the native
  drawer/lobby/mention member enumeration on native desktop.
- **Power-level / creator / tag _read_** — #450 closes the native
  `m.room.power_levels` and `m.room.create` snapshots used by the migrated
  power, sort, creator, and permission paths. The custom
  `in.synara.room.power_level_tags` read and direct helper/plugin reads remain
  residual.

This inventory scopes that **read** residual as **V-ROOMS.MEMBERS-READ**. The
write side (invite/kick/ban/unban/setPowerLevel) is **#375** — not this slice.

The IPC schema defines a `members` stream topic and a `RoomMember` DTO on both
sides (Rust `dto/member.rs`, TS `matrix-dto/member.ts`, `streamBody.ts` case
`'members'`). **#395 now also has a live native producer/command:**
`matrix_room_members_snapshot` lives in `members/product_commands.rs` after
#446's behavior-preserving command fan-out, and
`nativeRoomMembersOwner` invokes and validates it. On the native session path
wired in `Members.tsx`, `MembersDrawer`, and mention autocomplete, the list no
longer falls through to the JS client. The legacy
`useRoomMembers` path still reads
`mx.getRoom(roomId).getMembers()` / `room.loadMembersIfNeeded()` and listens for
`RoomMemberEvent.Membership` / `RoomMemberEvent.PowerLevel` when
`nativeSession` is not selected. The native list surfaces all select the
explicit native owner; unrelated JS member helpers remain outside this
surface.

---

## 2. Residual table — V-ROOMS.MEMBERS-READ

| Path                                                                        | Role                                                                                                                                                                                          | Gap                                                                                                                                                 | ID                                          |
| --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| `synara/src/app/hooks/useRoomMembers.ts`                                    | Explicit native-session branch via `nativeRoomMembersOwner`; legacy branch uses `mx.getRoom(roomId).getMembers()`, `room.loadMembersIfNeeded()`, and membership/power listeners               | #395/#405 own the native desktop member surfaces; retain the JS branch only for an explicit non-native/web route, never as a native fallback     | **Boundary; no native fallback**            |
| `synara/src/app/features/common-settings/members/Members.tsx`               | Full member-list page; passes `nativeSession` to `useRoomMembers`, then uses native power/creator hooks, power/tag hooks, filter/sort, and virtualized `MemberTile` | Member list and native power/creator inputs are fail-closed on native desktop; custom tag metadata is not native-read, and web/non-native sessions retain the legacy member path | **V-ROOMS.MEMBERS-READ** (tags/direct) |
| `synara/src/app/features/room/Room.tsx`                                     | Owns the room power-level context; after merged #405 the people drawer itself owns the native member snapshot | Member drawer and `usePowerLevels(room)` native ownership are closed; custom tags remain a separate read residual | **V-ROOMS.MEMBERS-READ** (tags) |
| `synara/src/app/features/room/MembersDrawer.tsx`                            | Native member snapshot after merged #405; native `usePowerLevelsContext`, `useGetMemberPowerLevel`, `useGetMemberPowerTag`, `useRoomCreators`, filter/sort, `MemberItem` | Member enumeration and power/creator inputs are closed; custom power-level tags remain residual | **V-ROOMS.MEMBERS-READ** (tags) |
| `synara/src/app/features/lobby/Lobby.tsx`                                   | Space hierarchy power reads via native `usePowerLevels(space)`, `useRoomsPowerLevels`, `useRoomCreators`, and `getRoomPermissionsAPI`; drawer member snapshot moves to #405-owned `MembersDrawer` | Member enumeration and native power/creator permission reads are closed; direct helper/plugin reads remain separately tracked | **V-ROOMS.MEMBERS-READ** (direct readers) |
| `synara/src/app/components/editor/autocomplete/UserMentionAutocomplete.tsx` | Native member snapshot after merged #405; DTO-aware filtering/search for mention autocomplete                                                                                              | Member enumeration is closed by #405 and fail-closed on native desktop                                                                         | **Closed by #405**                          |
| `synara/src/app/hooks/usePowerLevels.ts`                                    | Native session uses `matrix_room_power_levels_snapshot`; JS `getStateEvent`/listeners remain only for explicit non-native/web behavior; `readPowerLevel`, `useGetMemberPowerLevel`, `getPermissionPower` derive native values | Native power-level read is closed and fail-closed on native desktop | **Closed by #450** |
| `synara/src/app/hooks/usePowerLevelTags.ts`                                 | Native session disables the JS `m.room.power_level_tags` state read and derives generated/default labels from used powers; web retains the JS read | No native custom-tag snapshot; persisted names/colors/icons remain residual | **V-ROOMS.MEMBERS-READ** (tags) |
| `synara/src/app/hooks/useMemberPowerTag.ts`                                 | `useGetMemberPowerTag` / `useFlattenPowerTagMembers` consume native power/creator inputs and the generated/default tag map | Power calculation is native; custom tag metadata remains residual | **V-ROOMS.MEMBERS-READ** (tags) |
| `synara/src/app/hooks/useRoomCreators.ts`                                   | `useRoomCreators` / `useRoomsCreators` use `matrix_room_creators_snapshot` on native sessions; legacy `getStateEvent(RoomCreate)` remains only for non-native/web behavior | Native hook and multi-room creator reads are closed; unrelated direct utility/plugin readers remain separate | **Closed by #450** |
| `synara/src/app/hooks/useMemberFilter.ts`                                   | `useMembershipFilter` filters the shared member-list type by `membership` (joined/invited/left/kicked/banned)                                                                                 | #395/#405 support both native DTOs and legacy JS members; no separate native member-read gap remains in this filter                                  | **Closed for native member projection**      |
| `synara/src/app/hooks/useMemberSort.ts`                                     | `useMemberSort` / `useMemberPowerSort` sort the shared member-list type by name / join ts / native power | Name/join and power sorting are native-DTO compatible on native desktop | **Closed by #450** |
| `synara/src/app/hooks/useMemberPowerCompare.ts`                             | `useMemberPowerCompare` compares two users' power (creator short-circuit + native `readPowerLevel.user`) | Native power/creator comparison is closed on native desktop | **Closed by #450** |
| `synara/src/app/hooks/useRoomPermissions.ts`                                | `getRoomPermissionsAPI` / `useRoomPermissions` gate permission-sensitive UI from native creators + power levels, denying while native input is unavailable | Native permission gate is closed/fail-closed; non-native/web retains its legacy route | **Closed by #450** |
| `synara/src/app/components/member-tile/MemberTile.tsx`                      | Renders a member row (name, username, avatar) from the shared JS/native member-list type                                                                                                      | #395 DTO rendering remains valid; the #405 drawer uses its DTO-aware `MemberItem` boundary                                                           | **Closed for native member list**            |
| `synara/src/app/utils/room.ts`                                              | `getAllVersionsRoomCreator` reads `m.room.create`; `guessPerfectParent` reads `m.room.power_levels` and creator users; shared member display/avatar/search helpers also live here | Direct creator/power reads remain residual outside the #450 hook owners; unrelated member helper callers outside drawer/mentions are not closed by #405 | **V-ROOMS.MEMBERS-READ** (direct readers) |
| `synara/src/app/plugins/via-servers.ts`                                     | Direct `m.room.create` and `m.room.power_levels` reads plus JS member enumeration for via-server selection | Direct native-session reader is not migrated by #450 | **V-ROOMS.MEMBERS-READ** (direct readers) |
| `synara/src/app/utils/matrix.ts`                                            | `getOldestMember`, DM-peer member helpers (`room.getMember(userId)`, `room.getMembers()`)                                                                                                     | JS member reads on live client                                                                                                                      | **V-ROOMS.MEMBERS-READ**                    |

**Note:** `useRoomTypingMembers` / `state/typingMembers.ts` are **native** (typing
stream) and out of scope. The `applyPermissionPower` / `getPermissionPower`
write-side helpers in `usePowerLevels.ts` are used by the **PowersEditor bulk PL
rewrite**, which is the separate **#439 WRITE** slice and is landed at this
tip; it is not this read slice. The read-side `readPowerLevel` /
`getPermissionPower` use the native projection on native desktop; custom tags
and direct helper/plugin readers remain residual.

---

## 3. Remaining slice — native power-level tags and direct READ consumers

### Member wiring boundary after #405/#446

`matrix_room_members_snapshot` is no longer proposed: it is live in
`src-tauri/src/matrix/members/product_commands.rs` after #446's command
fan-out, registered in the Tauri ACL/build surfaces, and owned on the TS side
by `nativeRoomMembersOwner`. The owner validates the room/session-shaped DTO
and treats unavailable or malformed IPC as terminal. `Members.tsx`,
`MembersDrawer`, and `UserMentionAutocomplete` select this owner with
`nativeSession`, while `Room.tsx` and `Lobby.tsx` no longer own the member
list. None of these native paths may fall through to
`mx.getRoom().getMembers()`.

### Native read command status

Merged **#450** exposes the power-level and creator reads below over IPC and
removes their JS read owners from the native desktop route. The power-level-tag
read remains unimplemented:

- **Landed `matrix_room_power_levels_snapshot`** — returns the current
  `m.room.power_levels` content and feeds native power tags / permission gating /
  power sort without a JS `getStateEvent` read.
- **Landed `matrix_room_creators_snapshot`** — returns the room creator set from
  `m.room.create` plus supported `additional_creators` for creator power-tag /
  permission short-circuit.
- **Remaining `matrix_room_power_level_tags_snapshot`** — return the
  `in.synara.room.power_level_tags` content used for named power tags. This may
  be folded into the power-level snapshot, but the native READ contract must
  own the tag state rather than leave a hidden JS state-event read.

### Consumer and deletion boundary

The remaining tag/direct consumers must be migrated or explicitly scoped before
this residual can close:

- Add a native tag projection for `usePowerLevelTags.ts` and preserve custom
  names, colors, and icons through `useMemberPowerTag.ts`, `PermissionGroups`,
  `Powers`, and `PowersEditor`. Keep the PowersEditor write path scoped to its
  separate powers-bulk slice.
- Give `via-servers.ts`, `getAllVersionsRoomCreator`, and `guessPerfectParent`
  native owners or record an explicitly approved non-native/direct-reader
  boundary; do not treat #450's hook migration as closure for those helpers.
- Keep the explicit non-native web route only if that route remains supported;
  its JS member branch and unrelated member display/profile helpers are not a
  native desktop fallback and are not claimed closed by #405. Do not delete
  shared UI/helper files merely because the drawer and mention data sources
  changed.

Verify that no native desktop consumer still uses the two-argument
`useRoomMembers` path, that the native power/creator hooks remain on their
validated owners, and that no native path silently reopens a JS state-event
fallback.

**Fail-closed:** #395 already makes absence, failure, or malformed output from
`matrix_room_members_snapshot` terminal whenever `nativeSession` is selected
for `Members.tsx`, `MembersDrawer`, or `UserMentionAutocomplete`; those paths
never fall through to `getMembers()`. The people-drawer/lobby/mention member
enumeration is therefore closed by merged #405, and #450 closes the native
power-level/creator hook paths. Failure of
`matrix_room_members_snapshot`, `matrix_room_power_levels_snapshot`,
`matrix_room_creators_snapshot`, or any future
`matrix_room_power_level_tags_snapshot` must be terminal on native desktop. A native
session must never select a JS fallback, and `dual_backend=false` remains
required.

---

## 4. Non-goals / out of scope

| Item                                                                 | Status                                                                              |
| -------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| **Room moderation writes** (invite/kick/ban/unban/setPowerLevel)     | **#375 merged**; not this read slice                                                |
| **PowersEditor bulk PL rewrite** (write-side `applyPermissionPower`) | **#439 merged**; separate WRITE slice, not this read residual |
| **Typing members read**                                              | Native (typing stream) — not a residual                                             |
| **Room membership / unread**                                         | [p4.3-membership-unread.md](p4.3-membership-unread.md) — separate                   |
| **P4.6 member index**                                                | [p4.6-members.md](p4.6-members.md) — pure index over DTOs; no SDK member APIs       |
| Product input                                                        | **#405 merged** at `176fc7c5`; drawer/lobby/mentions member wiring is landed. **#439** bulk powers/tag WRITE is merged at `f92a33f9`; **#446** command fan-out is merged at `9fb341af`; **#450** power/creator READ is merged at `103a653f`; **#458** presence first slice and **#461** room-directory first slice are merged by `c1e9c3be` but are unrelated. Custom tag/direct READ remains open. |
| Product changes in this PR                                           | None; this packet is docs-only, refreshed at `fd0dfbf4`, and does not edit product command modules |
| Umbrella merge to `main`                                             | **#39** — needs explicit user approval                                              |
| Cutover / dual-backend removal                                       | #240 HOLD; no cutover                                                               |

---

## 5. Confidence

**Confidence: high** for the inventory. I traced the member-list **read** from
the two primary consumers (`Members.tsx`, `MembersDrawer.tsx`), the room owner
(`Room.tsx`), the lobby (`Lobby.tsx`), and mention autocomplete back through
`useRoomMembers`. At current docs/integration tip `fd0dfbf4`, I confirmed the live native producer in
`members/product_commands.rs` after #446, the fail-closed
`nativeRoomMembersOwner`, and the `nativeSession` wiring for `Members.tsx`,
`MembersDrawer`, and mention autocomplete; merged #405 is recorded at
`176fc7c5`. I also verified #450's native power/creator owners and traced the
remaining tag hook plus direct `via-servers` / `utils/room.ts` readers. Possible
missed files: any other consumer of the tag or direct state reads outside the
listed paths (e.g. a barrel re-export or another permission surface) — verify
with a full `rg` over `synara/src`. #405/#450 do not close unrelated profile,
notification, call, or DM member lookups.

## Done-when

- **Already true from #395:** `matrix_room_members_snapshot` returns the room
  member list as `RoomMember[]` (matching the `members` stream-topic body
  shape), and `Members.tsx` selects it through a fail-closed native owner.
- **At current tip after merged #405:** `MembersDrawer.tsx` and
  `UserMentionAutocomplete.tsx` select the same native member owner on native
  desktop; their member rows/search boundaries accept the native DTO, and
  `Room.tsx`/`Lobby.tsx` no longer own a legacy member read.
- `matrix_room_power_levels_snapshot` and `matrix_room_creators_snapshot` are
  landed and back the native power/creator paths; the remaining tag projection
  returns custom power-level tag metadata without a JS state-event fallback.
- `Members.tsx`, `MembersDrawer.tsx`, `Lobby.tsx`, and mention member reads use
  native snapshots and fail closed on native desktop; the explicit non-native
  web route is the only remaining JS member route if web support is retained.
- Native desktop no longer uses the JS power-level/creator reads in the #450
  hook/permission paths; custom tag and direct helper/plugin routes are either
  native-owned or explicitly scoped, and unrelated member helper routes are not
  deleted merely because #405 landed. Preserve the separate PowersEditor write
  slice.
- The future READ implementation must update production `matrix-js-sdk` import
  accounting only when native READ ownership is actually landed; this docs
  packet makes no such reduction claim.
- `dual_backend=false` remains explicit throughout; this residual and its
  follow-up use owner selection, not a dual-backend mode. V-BURN remains HOLD.
