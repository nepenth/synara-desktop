# V-ROOMS — power-level/creator READ residual packet

| Field | Value |
| --- | --- |
| Status | **Docs-only residual packet** — native power-level/creator reads are not implemented or claimed closed |
| Base tip | `457b2760` on `feature/matrix-rust-sdk-full-replacement` |
| Source | #437 read-residual content, refreshed after #405 and #438 |
| Target | Draft PR targeting `feature/matrix-rust-sdk-full-replacement`; never `main` or umbrella #39 |
| Member boundary | **#405 merged** — drawer/lobby/mention member enumeration is native; power-level/creator reads remain residual |
| Separate write | **#439 powers-BULK** owns the write slice and the serial `product.rs` lane; not this packet |
| Serial handoff | After #439, behavior-preserving `product.rs` extraction/split comes first; READ product work waits for extract or an explicit free-lane handoff |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) — native owner, live SDK, fail-closed, no `dual_backend` |

> **Scope guard.** This PR changes documentation only. It does not edit
> `src-tauri/src/matrix/auth/product.rs`, TypeScript, IPC registration, or
> product behavior. It does not implement #439 powers-BULK, target `main`, or
> touch umbrella #39. `dual_backend` is forbidden.
>
> **Serial guard.** #439 is the current product owner for the serial
> `product.rs` lane. The next serial operation is a behavior-preserving
> extraction/split under [product-lane-protocol.md](product-lane-protocol.md).
> Do not begin READ product implementation or register READ commands until the
> extraction has landed or the orchestrator explicitly marks the lane free.

## 1. Residual boundary

This packet records the remaining **READ** work at `457b2760`:

- `m.room.power_levels` is still read from the JS SDK room state and normalized
  by `usePowerLevels` / `useRoomsPowerLevels`.
- `m.room.create` is still read from the JS SDK room state to derive the room
  creator set.
- `in.synara.room.power_level_tags` is still read from JS state for named power
  tags, which makes it part of the tag/permission read dependency.
- Power tags, member power comparisons, permission gates, the Permissions
  screens, and imperative room/space checks still consume those JS-derived
  projections.

The member-enumeration boundary changed with **#405**: the room drawer,
lobby drawer, and mention autocomplete now use the existing native member
snapshot path. That closes member enumeration for those surfaces; it does not
close the power-level, creator, or power-tag state reads they still consume.

The **powers-BULK WRITE** path is explicitly separate. **#439** owns the
complete `m.room.power_levels` write and the
`in.synara.room.power_level_tags` write. This packet inventories the reads
those components use and does not replace, remove, or rename a write command.

At this tip, #439 is not claimed merged by this packet, and no READ snapshot
command is claimed present. The current dependency shape is:

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
| `synara/src/app/hooks/usePowerLevels.ts` | `useStateEvent(room, StateEvent.RoomPowerLevels)`; the shared `getStateEvent` path reads the room's current `m.room.power_levels` state event | Normalizes missing fields into `IPowerLevels`; powers, permission thresholds, member power, and context providers depend on this object |
| `synara/src/app/hooks/usePowerLevels.ts` (`useRoomsPowerLevels`) | Direct `getStateEvent(room, StateEvent.RoomPowerLevels, '')` plus JS state-event refresh | Space/lobby hierarchy checks and other multi-room imperative permission decisions use the resulting map |
| `synara/src/app/hooks/useRoomCreators.ts` | `useStateEvent(room, StateEvent.RoomCreate)` and `getStateEvent` for the imperative helper | Derives the creator set from the event sender and `additional_creators`, gated by `creatorsSupported` |
| `synara/src/app/hooks/usePowerLevelTags.ts` | `useStateEvent(room, StateEvent.PowerLevelTags)` for `in.synara.room.power_level_tags` | Reads custom names/colors/icons and generates fallback tags for powers found in `IPowerLevels` |
| `synara/src/app/hooks/useMemberPowerTag.ts` | Indirectly consumes `usePowerLevelTags`, `readPowerLevel.user`, and the creator set | Creator short-circuits to the founder tag; other member labels depend on the JS power-level and tag projections |
| `synara/src/app/hooks/useRoomPermissions.ts` | No direct room-state call; derives gates from the JS-derived power levels and creator set | Permission-sensitive UI remains JS-owned until both input projections have native owners |

`useMemberPowerTag` remains in the residual even though it does not import
`getStateEvent`: its tag hook and power-level argument are the indirect read
boundary.

### Permissions components: reads versus writes

Both `room-settings/permissions/Permissions.tsx` and
`space-settings/permissions/Permissions.tsx` create the current `powerLevels`
and `creators`, then use `useRoomPermissions` to gate the two edit surfaces.
The common components receive those values as props:

| Component | Read sites | Separate write site, not this packet |
| --- | --- | --- |
| `synara/src/app/features/common-settings/permissions/PermissionGroups.tsx` | `usePowerLevelTags(room, powerLevels)` and `getPermissionPower(powerLevels, ...)`; reads come from the JS-derived props/tag state | `mx.sendStateEvent(..., StateEvent.RoomPowerLevels, ...)`; complete bulk policy write belongs to **#439 powers-BULK** |
| `synara/src/app/features/common-settings/permissions/PowersEditor.tsx` | `getUsedPowers(powerLevels)` and `usePowerLevelTags(room, powerLevels)`; tag editing starts from the JS tag projection | `mx.sendStateEvent(..., StateEvent.PowerLevelTags, ...)`; custom-tag write belongs to **#439 powers-BULK** |
| `synara/src/app/features/common-settings/permissions/Powers.tsx` | `usePowerLevelTags` and `useRoomCreators` render named levels and founders; `getPermissionPower` supports the permission peek | Read-only presentation; its data source still needs native projections |

The `sendStateEvent` calls above are recorded only to keep the read/write
boundary explicit. This docs packet does not replace or remove them.

### Direct power/creator `getStateEvent` sites outside the hooks

The shared helper is `synara/src/app/utils/room.ts`, and the reactive wrapper
is `synara/src/app/hooks/useStateEvent.ts`. The additional direct sites in the
power/creator operating path include:

| Path | State read | Why it matters |
| --- | --- | --- |
| `synara/src/app/features/room-nav/RoomNavItem.tsx` | Direct `m.room.power_levels`, plus `getRoomCreatorsForRoomId` for `getRoomPermissionsAPI` | Call-start permission gate; cannot remain a native-desktop JS bypass |
| `synara/src/app/plugins/via-servers.ts` | Direct `m.room.create` and `m.room.power_levels` | Selects the highest-power user used for server routing |
| `synara/src/app/utils/room.ts` | `getAllVersionsRoomCreator` reads `m.room.create`; `guessPerfectParent` reads `m.room.power_levels` and creator users | Parent selection uses creator and elevated-power candidates |

A full search also finds unrelated `getStateEvent` uses for topics, aliases,
tombstones, room type, parent state, and other state events. Those are not
silently included in this packet; they need their own residuals or an
explicitly scoped follow-up.

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

This is the current fan-out, not permission to widen the product slice. The
future implementation owner must use a complete source search and prove that
every native-session consumer receives the same validated projections.

## 4. Proposed native READ IPC

These are proposals for a future product slice. They are not implemented at
`457b2760`, and this packet does not authorize edits to `product.rs`.

Product implementation is serial-gated: #439 owns the current `product.rs`
lane, extraction/split follows #439, and READ command registration waits for
that extraction or an explicit free-lane handoff.

### `matrix_room_power_levels_snapshot`

Request:

```ts
{ roomId: string }
```

The result should identify the room and native session generation and return
the complete current `m.room.power_levels` content, including the fields
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

The frontend owner must validate the fixed event type/state key, room ID,
session generation, finite numeric power values, map shapes, and required
response fields before exposing the projection to `usePowerLevels` or
`useRoomsPowerLevels`.

### `matrix_room_creators_snapshot`

Request:

```ts
{ roomId: string }
```

The result should return the creator set used by `getRoomCreators`, preserving
the current semantics: the `m.room.create` event sender plus valid
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
`matrix_room_power_level_tags_snapshot`. It is a read proposal only; the tag
write remains the separate **#439 powers-BULK** product path.

### Shared owner rules

- A native desktop session selects the native snapshot owner explicitly.
- Missing, unavailable, stale-generation, failed, or malformed IPC is
  terminal for that native route; it never falls through to JS
  `getStateEvent`/`useStateEvent`.
- The owner returns a typed unavailable/error state rather than fabricating an
  empty creator set or default power policy on IPC failure. Matrix defaults are
  applied only after valid native `m.room.power_levels` content is received,
  matching current normalization semantics.
- No generic `eventType`/`stateKey` escape hatch is needed; command contracts
  stay fixed and auditable.
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
   their **#439 powers-BULK writes remain a separate implementation slice**.
5. Native failure is visible and terminal, and no `dual_backend` selector or
   retry-to-JS path is introduced.
6. Only after the native route is proven are superseded JS power/creator read
   owners, imports, listeners, and tests physically removed from that route.
   Unrelated state-event helpers are not deleted as collateral.

Product implementation for this READ residual is additionally gated by the
serial handoff above: #439, then behavior-preserving `product.rs` extraction,
then an explicit free-lane decision if the extract does not itself release the
lane.

## 6. Done-when for this documentation packet

- The packet is based at `457b2760` and targets
  `feature/matrix-rust-sdk-full-replacement`.
- It incorporates #437's read inventory while recording #405 as merged and
  distinguishing closed member enumeration from the remaining power-level/
  creator reads.
- It names #438's product-lane protocol and makes the #439 → extract → READ
  serial order explicit.
- It names proposed READ IPC only and makes no `product.rs` or product-code
  change.
- It explicitly leaves **#439 powers-BULK WRITE** separate and does not claim it
  merged at this base.
- It keeps `main`, #39, and `dual_backend` forbidden.
