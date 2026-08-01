# V-ROOMS — power-level/creator READ residual

| Field | Value |
| --- | --- |
| Status | **Docs-only read inventory** — native power-level/creator reads are not implemented or claimed closed |
| Base tip | `b0fd4241` on `feature/matrix-rust-sdk-full-replacement` |
| Target | Draft PR targeting `feature/matrix-rust-sdk-full-replacement`; never `main` or umbrella #39 |
| Member boundary | #395 member-list path is the prior slice; #405 may still be open for drawer/mentions member enumeration |
| Adjacent product | **#407 CallWidget media is merged**; it does not change this read residual |
| Separate write | **#388 powers-BULK** is a separate product-in-flight write packet; not this document |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) — native owner, live SDK, fail-closed, no `dual_backend` |

> **Scope guard.** This PR changes documentation only. It does not edit
> `src-tauri/src/matrix/auth/product.rs`, TypeScript, IPC registration, or
> product behavior. It does not merge #405, touch #407, target `main`, or
> touch umbrella #39. `dual_backend` is forbidden.

## 1. Residual boundary

This packet records the remaining **READ** work after the #395 native member
list path:

- `m.room.power_levels` is still read from the JS SDK room state and normalized
  by `usePowerLevels` / `useRoomsPowerLevels`.
- `m.room.create` is still read from the JS SDK room state to derive the room
  creator set.
- `in.synara.room.power_level_tags` is still read from JS state for named power
  tags, which makes it part of the tag/permission read dependency.
- Power tags, member power comparisons, permission gates, the Permissions
  screens, and imperative room/space checks still consume those JS-derived
  projections.

#405 may still be open at this base for the room drawer, lobby, and mention
member enumeration. Once #405 merges, that **member enumeration** boundary is
expected to close through the existing native member snapshot. It does not
close the power-level/creator state reads documented here. This document does
not claim #405 has landed.

The **powers-BULK WRITE** path is explicitly separate: packet **#388** and its
product work in flight own the `PermissionGroups` complete
`m.room.power_levels` write and the `PowersEditor` custom-tag write. This
packet only inventories the reads those components use. Do not implement or
rename a write command from this document.

The current dependency shape is:

```text
m.room.power_levels ─┐
                     ├─> usePowerLevels / useRoomsPowerLevels
m.room.create ───────┘             │
                                   ├─> useRoomPermissions / permission gates
in.synara.room.power_level_tags ───┘
        │
        ├─> useMemberPowerTag / member power sort and labels
        └─> PermissionGroups / Powers / PowersEditor displays
```

## 2. JS state-read inventory

### Core hooks and their exact read mechanisms

| Path | Current JS read | Consumers / behavior left residual |
| --- | --- | --- |
| `synara/src/app/hooks/usePowerLevels.ts:68-76` | `useStateEvent(room, StateEvent.RoomPowerLevels)`; `useStateEvent.ts:27-30` calls the shared `getStateEvent` helper, which reads `getRoomCurrentState(room).getStateEvents('m.room.power_levels', '')` | Normalizes missing fields into `IPowerLevels`; powers, permission thresholds, member power, and context providers all depend on this object |
| `synara/src/app/hooks/usePowerLevels.ts:88-121` | `useRoomsPowerLevels` directly calls `getStateEvent(room, StateEvent.RoomPowerLevels, '')` and refreshes from JS client state on a matching state event | Space/lobby hierarchy checks and other multi-room imperative permission decisions use the resulting map |
| `synara/src/app/hooks/useRoomCreators.ts:30-38` | `useStateEvent(room, StateEvent.RoomCreate)` → shared JS `getStateEvent` path | Derives the creator set from the event sender and `additional_creators`, gated by `creatorsSupported` |
| `synara/src/app/hooks/useRoomCreators.ts:41-48` | `getRoomCreatorsForRoomId` calls `mx.getRoom(roomId)` and direct `getStateEvent(room, StateEvent.RoomCreate)` | Imperative lobby, hierarchy, room-navigation, and other permission checks use this JS owner |
| `synara/src/app/hooks/usePowerLevelTags.ts:90-108` | `useStateEvent(room, StateEvent.PowerLevelTags)` → the same JS `getStateEvent` path for `in.synara.room.power_level_tags` | Reads custom names/colors/icons and generates fallback tags for every power found in `IPowerLevels` |
| `synara/src/app/hooks/useMemberPowerTag.ts:16-36` | No direct `getStateEvent` call; consumes `usePowerLevelTags`, `readPowerLevel.user`, and the creator set | Creator short-circuits to the founder tag; all other member labels depend on the JS power-level and tag projections |
| `synara/src/app/hooks/useRoomPermissions.ts:16-59` | No direct room-state call; derives `event`, `stateEvent`, `action`, and notification gates from the JS-derived power levels and creator set | Permission-sensitive UI remains JS-owned until both input projections have native owners |

`useMemberPowerTag` is therefore a residual even though its file does not
import `getStateEvent`: its `usePowerLevelTags` call and `powerLevels` argument
are the indirect read boundary.

### Permissions components: reads versus writes

Both `room-settings/permissions/Permissions.tsx:19-25` and
`space-settings/permissions/Permissions.tsx:19-25` create the current
`powerLevels` and `creators`, then use `useRoomPermissions` to gate the two
edit surfaces. The common components receive those values as props:

| Component | Read sites | Separate write site, not this packet |
| --- | --- | --- |
| `synara/src/app/features/common-settings/permissions/PermissionGroups.tsx:32-113` | `usePowerLevelTags(room, powerLevels)` at `:41`; `getPermissionPower(powerLevels, ...)` at `:78` and `:108`; all reads are from the JS-derived props/tag state | `mx.sendStateEvent(..., StateEvent.RoomPowerLevels, ...)` at `:88`; complete bulk policy write belongs to **#388 powers-BULK** |
| `synara/src/app/features/common-settings/permissions/PowersEditor.tsx:291-337` | `getUsedPowers(powerLevels)` at `:296`; `usePowerLevelTags(room, powerLevels)` at `:301`; tag display/edit state starts from the JS tag projection | `mx.sendStateEvent(..., StateEvent.PowerLevelTags, ...)` at `:336`; custom-tag write belongs to **#388 powers-BULK** |
| `synara/src/app/features/common-settings/permissions/Powers.tsx:110-180` | `usePowerLevelTags` and `useRoomCreators` render named levels and founders; `getPermissionPower` is used by its permission peek | Read-only presentation; its data source still needs the native projections |

The `sendStateEvent` calls above are recorded only to keep the read/write
boundary explicit. This PR does not replace, remove, or implement them.

### Direct power/creator `getStateEvent` sites outside the hooks

The shared helper itself is `synara/src/app/utils/room.ts:34-42`, and the
reactive wrapper is `synara/src/app/hooks/useStateEvent.ts:8-31`. In the
power/creator operating path, the additional direct sites at this tip are:

| Path | State read | Why it matters |
| --- | --- | --- |
| `synara/src/app/features/room-nav/RoomNavItem.tsx:294-298` | Direct `m.room.power_levels`, plus `getRoomCreatorsForRoomId` for `getRoomPermissionsAPI` | Call-start permission gate; cannot remain a native desktop JS bypass |
| `synara/src/app/plugins/via-servers.ts:7-21` | Direct `m.room.create` and `m.room.power_levels` | Selects the highest-power user used for server routing |
| `synara/src/app/utils/room.ts:489-526` | `getAllVersionsRoomCreator` reads `m.room.create`; `guessPerfectParent` reads `m.room.power_levels` and creator users | Parent selection uses creator and elevated-power candidates |

A full `rg` also finds unrelated `getStateEvent` uses for topics, aliases,
tombstones, room type, parent state, and other state events. Those are not
silently included in this power-level/creator packet; they need their own
residuals or an explicitly scoped follow-up.

## 3. Consumer fan-out to preserve

The two core hooks are shared beyond the Members settings surface. The native
read migration must cover all native-desktop consumers, including these
families:

- room shell and member UI: `Room.tsx`, `Members.tsx`, `MembersDrawer.tsx`,
  `RoomView.tsx`, `RoomViewHeader.tsx`, `RoomInput.tsx`, `RoomPinMenu.tsx`, and
  `PowerChip.tsx`;
- room and space permissions: both `Permissions.tsx` routes,
  `PermissionGroups.tsx`, `Powers.tsx`, and `PowersEditor.tsx`;
- navigation and hierarchy: `Lobby.tsx`, `LobbyHeader.tsx`,
  `HierarchyItemMenu.tsx`, `RoomNavItem.tsx`, and `via-servers.ts`;
- other permission/tag presentations: `RoomPacks.tsx`, `RoomImagePack.tsx`,
  `SearchResultGroup.tsx`, `CallView.tsx`, `StateEventEditor.tsx`,
  `RoomUpgrade.tsx`, and related message/member renderers.

This list describes the current fan-out, not permission to widen the product
slice. The implementation owner must use a complete source search and prove
that every native-session consumer receives the same validated projections.

## 4. Proposed native READ IPC

These are proposed names and shapes for the follow-up product slice. They are
not implemented here, and this document does not authorize edits to
`product.rs`.

### `matrix_room_power_levels_snapshot`

Request:

```ts
{ roomId: string }
```

Result should identify the room and native session generation and return the
complete current `m.room.power_levels` content, including the fields currently
represented by `IPowerLevels`: `users`, `users_default`, `events`,
`events_default`, `state_default`, `invite`, `redact`, `kick`, `ban`,
`historical`, and `notifications`. Existing JSON fields must not be silently
dropped when the native DTO is normalized.

Suggested semantic result:

```ts
{
  status: 'ok',
  roomId: string,
  eventType: 'm.room.power_levels',
  stateKey: '',
  sessionGeneration: number,
  content: RoomPowerLevelsContent,
}
```

The frontend owner should validate the fixed event type/state key, room ID,
session generation, finite numeric power values, map shapes, and required
response fields before exposing the projection to `usePowerLevels` or
`useRoomsPowerLevels`.

### `matrix_room_creators_snapshot`

Request:

```ts
{ roomId: string }
```

Result should return the creator set used by `getRoomCreators`, preserving the
current semantics: the `m.room.create` event sender plus valid
`additional_creators`, and an empty set when the room version is not supported
by `creatorsSupported`.

Suggested semantic result:

```ts
{
  status: 'ok',
  roomId: string,
  eventType: 'm.room.create',
  stateKey: '',
  sessionGeneration: number,
  creators: string[],
}
```

### `matrix_room_power_level_tags_snapshot`

The tag read is a distinct state event and is required to migrate
`useMemberPowerTag`, `PermissionGroups`, `Powers`, and `PowersEditor` without
leaving a hidden JS `useStateEvent` dependency. Either include this content in
the power-level snapshot or expose this explicit command:

```ts
{
  status: 'ok',
  roomId: string,
  eventType: 'in.synara.room.power_level_tags',
  stateKey: '',
  sessionGeneration: number,
  tags: PowerLevelTags,
}
```

If kept separate, the command name should be
`matrix_room_power_level_tags_snapshot`. It is a read proposal only; the
existing tag write remains the separate **#388 powers-BULK** product path.

### Shared owner rules

- A native desktop session selects the native snapshot owner explicitly.
- Missing, unavailable, stale-generation, failed, or malformed IPC is
  terminal for that native route; it never falls through to JS
  `getStateEvent`/`useStateEvent`.
- The owner must return a typed unavailable/error state rather than fabricate
  an empty creator set or default power policy on IPC failure. Matrix defaults
  are applied only after a valid native `m.room.power_levels` content object is
  received, matching current normalization semantics.
- No generic `eventType`/`stateKey` escape hatch is needed for this slice. The
  command contracts stay fixed and auditable.
- Non-native/web behavior may retain the JS owner only where that route is
  explicitly supported; it is not a fallback for a native session.

## 5. Migration and deletion boundary

The eventual implementation may close this residual only when it can prove:

1. `usePowerLevels`, `useRoomsPowerLevels`, and `useRoomCreators` consume
   validated native snapshots on native desktop.
2. `useMemberPowerTag`, `usePowerLevelTags`, `useRoomPermissions`, and the
   Members/room/space permission consumers use those projections without a JS
   state-event fallback.
3. The direct sites in `RoomNavItem`, `via-servers`, and `guessPerfectParent`
   either have native owners or are explicitly excluded in a separately
   approved surface inventory; they cannot be overlooked because they do not
   render the Members list.
4. `PermissionGroups` and `PowersEditor` read from the native projections while
   their **#388 powers-BULK writes remain a separate implementation slice**.
5. Native failure is visible and terminal, and no `dual_backend` selector or
   retry-to-JS path is introduced.
6. Only after the native route is proven are the superseded JS power/creator
   read owners, imports, listeners, and tests physically removed from that
   route. Unrelated state-event helpers are not deleted as collateral.

## 6. Done-when for this documentation packet

- This file is based at `b0fd4241` and targets
  `feature/matrix-rust-sdk-full-replacement`.
- It inventories the requested `usePowerLevels`, `useRoomCreators`,
  `useMemberPowerTag`, `PermissionGroups`, and `PowersEditor` read paths,
  including their direct/indirect JS `getStateEvent` route.
- It distinguishes the #395/#405 member-enumeration boundary from the
  remaining power-level/creator state-read residual.
- It names proposed read IPC only and makes no `product.rs` or product-code
  change.
- It explicitly leaves **#388 powers-BULK WRITE** separate and in flight.
- It records #407 as merged, #405 as potentially still open, `main`/#39 as
  forbidden targets, and `dual_backend` as forbidden.
