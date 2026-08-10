# V-ROOMS — power-level/creator/tag READ residual packet

| Field | Value |
| --- | --- |
| Status | **Active residual after merged #450** — native power-level/creator reads are closed for the migrated native owners; custom power-level-tag reads and direct helper/plugin readers remain |
| Base tip | `60141c8b` on `feature/matrix-rust-sdk-full-replacement` |
| Source | #437 read-residual content, refreshed after merged #405, #439, #446, #450, #458, and #461 |
| Target | Draft PR #465 refresh targeting `feature/matrix-rust-sdk-full-replacement`; never `main` or umbrella #39 |
| Member boundary | **#405 merged** — drawer/lobby/mention member enumeration is native; #450 closes the native power-level/creator owner paths |
| Separate write | **#439 powers-BULK is merged** — its native writes are separate from this READ packet |
| Product lane | **#450 merged** at `103a653f` on `matrix-rust/v-power-levels-read-impl`; the remaining READ work is custom tags plus explicitly tracked direct consumers; #458/#461 are unrelated first-slice work |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) — native owner, live SDK, fail-closed; `dual_backend=false`, V-BURN **HOLD** |

> **Scope guard.** This PR changes documentation only. It does not edit
> `src-tauri/src/matrix/auth/product.rs`, TypeScript, IPC registration, or
> product behavior. It does not implement or alter merged #439 powers-BULK,
> target `main`, or touch umbrella #39. `dual_backend=false` is preserved.
>
> **Lane guard.** The behavior-preserving #446 `product.rs` extract/split is
> merged at `9fb341af`, and #450 is merged at `103a653f`. The post-#458/#461
> product state is from `c1e9c3be`, and the current docs tip is `60141c8b`;
> #458's presence first slice and #461's room-directory
> first slice are unrelated and do not close this READ residual. The prior
> rejected #465 head `28e2418d` was based at `c1e9c3be` and is now rebased onto
> `60141c8b`. This packet records the native power/creator
> closure without claiming a native power-level-tags read or closing the
> remaining direct helper/plugin readers. Do not touch `main` or umbrella #39.

## 1. Residual boundary

This packet records the remaining **READ** work after merged **#450** at
`103a653f`, refreshed on current docs/integration tip `60141c8b` after #458/#461:

- `m.room.power_levels` and `m.room.create` now have native snapshots and
  fail-closed native owners in `usePowerLevels`, `useRoomsPowerLevels`, and
  `useRoomCreators`; their JS state-event paths remain only for an explicit
  non-native/web route.
- `in.synara.room.power_level_tags` still has no native read snapshot. Native
  sessions deliberately skip the JS state-event read and use generated/default
  tags, so persisted custom names, colors, and icons remain residual.
- `via-servers.ts` and `utils/room.ts` still contain direct power/create state
  reads used by via-server selection and perfect-parent navigation. These are
  separate direct consumers and are not closed by #450.

The member-enumeration boundary changed with **#405**: the room drawer,
lobby drawer, and mention autocomplete now use the existing native member
snapshot path. **#450** then closes the power-level and creator reads consumed
by the migrated native hooks and permission paths; it does not close the
power-level-tag state read or the explicitly listed direct consumers.

The **powers-BULK WRITE** path is explicitly separate and landed in merged
**#439** at this tip. It owns the complete `m.room.power_levels` write and the
`in.synara.room.power_level_tags` write. This packet inventories the reads
those components use and does not replace, remove, or rename a write command.

At this integration tip, #439, #446, #450, and #458 are merged. The power-level and
creator READ snapshots are present; the tag snapshot and direct helper/plugin
readers remain open. The current dependency shape is:

```text
m.room.power_levels ──> nativeRoomPowerLevelsOwner ──> native power/permission paths
m.room.create ────────> nativeRoomCreatorsOwner ─────> native creator paths
in.synara.room.power_level_tags ──> JS web read / native generated-tag fallback
                                     └─> useMemberPowerTag / Permissions displays
direct getStateEvent reads ─────────> via-servers / guessPerfectParent (residual)
```

## 2. JS state-read inventory

### Core hooks and their exact read mechanisms

| Path | Current read path | Consumers / behavior left residual |
| --- | --- | --- |
| `synara/src/app/hooks/usePowerLevels.ts` | Native session invokes `matrix_room_power_levels_snapshot`; the JS `useStateEvent` path is disabled there and retained only for non-native/web | Native power thresholds, member power, and context providers are closed and fail-closed; legacy web behavior remains explicit |
| `synara/src/app/hooks/usePowerLevels.ts` (`useRoomsPowerLevels`) | Native session invokes the same snapshot per room; JS `getStateEvent` and refresh listener are non-native only | Space/lobby hierarchy checks use the native map; unavailable native values remain terminal/unavailable rather than JS fallback |
| `synara/src/app/hooks/useRoomCreators.ts` | Native session invokes `matrix_room_creators_snapshot`; JS state reads are disabled there and retained only for non-native/web | Native creator set, including supported `additional_creators`, is closed for the hook and multi-room owner paths |
| `synara/src/app/hooks/usePowerLevelTags.ts` | Native sessions disable `useStateEvent(room, StateEvent.PowerLevelTags)` and synthesize defaults; web retains the JS read | Persisted custom names/colors/icons have no native projection and remain the active tag-read residual |
| `synara/src/app/hooks/useMemberPowerTag.ts` | Consumes native power/creator projections plus the native generated/default tag map | Member power calculation is native; custom tag metadata remains residual |
| `synara/src/app/hooks/useRoomPermissions.ts` | Derives gates from native power/creator projections on native sessions and returns deny-all while unavailable | Native permission-gate ownership is closed for the migrated route; non-native/web uses the legacy projections |

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
| `synara/src/app/features/common-settings/permissions/PermissionGroups.tsx` | `getPermissionPower` now receives native power levels on native sessions; tag labels still use generated/default tags there | `mx.sendStateEvent(..., StateEvent.RoomPowerLevels, ...)`; complete bulk policy write landed in **#439 powers-BULK** |
| `synara/src/app/features/common-settings/permissions/PowersEditor.tsx` | `getUsedPowers(powerLevels)` is native on native sessions, but `usePowerLevelTags` has no native custom-tag read | `mx.sendStateEvent(..., StateEvent.PowerLevelTags, ...)`; custom-tag write landed in **#439 powers-BULK** |
| `synara/src/app/features/common-settings/permissions/Powers.tsx` | `usePowerLevelTags` and `useRoomCreators` render native creator/power data; custom tag metadata remains unavailable on native sessions | Read-only presentation; native power/creator inputs are closed, tag projection remains residual |

The `sendStateEvent` calls above are recorded only to keep the read/write
boundary explicit. This docs packet does not replace or remove them.

### Direct power/creator `getStateEvent` sites outside the hooks

The shared helper is `synara/src/app/utils/room.ts`, and the reactive wrapper
is `synara/src/app/hooks/useStateEvent.ts`. The additional direct sites in the
power/creator operating path include:

| Path | State read | Why it matters |
| --- | --- | --- |
| `synara/src/app/features/room-nav/RoomNavItem.tsx` | `usePowerLevels`, `useRoomCreators`, and `useRoomPermissions` after #450 | Native call-start permission gate is closed/fail-closed; no direct JS power/create read remains in this component |
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

The core permission/member fan-out is now native on native sessions after #450.
This is still not permission to delete unrelated helpers or widen the product
slice: the tag projection and the direct `via-servers` / `guessPerfectParent`
consumers need an explicitly owned follow-up.

## 4. Proposed native READ IPC

The power-level and creator snapshots below are implemented by merged **#450**
at `103a653f`; this docs packet is refreshed on `60141c8b` and does not
authorize any product-code edits. The
tag snapshot remains a proposal.

The #446 extract and #450 product READ implementation are merged. This packet
records the landed contracts and the remaining tag/direct-reader residuals.

### Landed: `matrix_room_power_levels_snapshot`

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

### Landed: `matrix_room_creators_snapshot`

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

### Remaining proposal: `matrix_room_power_level_tags_snapshot`

The tag read is a distinct state event and is still required to migrate
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
write landed in the separate **#439 powers-BULK** product path.

### Shared owner rules

- A native desktop session selects the native power-level and creator snapshot
  owners explicitly.
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
  explicitly supported; it is not a fallback for a native session. Native tag
  consumers currently use generated/default tags rather than a JS fallback;
  this is a documented semantic gap until a native tag snapshot lands.

## 5. Migration and deletion boundary

The remaining residual may close only when it can prove:

1. The landed #450 contracts continue to back `usePowerLevels`,
   `useRoomsPowerLevels`, and `useRoomCreators` on native desktop, with
   fail-closed native permission handling.
2. A native tag snapshot backs `usePowerLevelTags` and `useMemberPowerTag`,
   preserving custom names, colors, and icons without a JS state-event read.
3. The direct sites in `via-servers` and `guessPerfectParent` either have
   native owners or are explicitly excluded in a separately approved surface
   inventory; they cannot be overlooked because they do not render the Members
   list.
4. `PermissionGroups` and `PowersEditor` read from the native power/tag
   projections while
   their **#439 powers-BULK writes remain a separate landed WRITE slice**; the
   landed writes do not close this READ residual.
5. Native failure is visible and terminal, and no `dual_backend` selector or
   retry-to-JS path is introduced.
6. Only after each remaining native route is proven are superseded JS tag/direct
   read owners, imports, listeners, and tests physically removed from that
   route. Unrelated state-event helpers are not deleted as collateral.

**#450 is merged at `103a653f`;** it closes the native power-level/creator
snapshot and permission-owner portion of this packet. The tag snapshot and
direct helper/plugin readers remain open, so this residual remains active; no
native power-level-tag read or direct-reader closure is claimed at `60141c8b`.

## 6. Done-when for this documentation packet

- The packet is based at docs tip `60141c8b` after #458/#461 and targets
  `feature/matrix-rust-sdk-full-replacement`.
- It incorporates #437's read inventory while recording #405 and #450 as
  merged, distinguishing closed native power/creator owners from the remaining
  tag and direct-reader residuals.
- It records merged #439's separate WRITE completion, merged #446's extract,
  and merged #450's native power/creator READ implementation.
- It names the remaining tag READ IPC proposal and makes no `product.rs` or
  product-code change.
- It explicitly leaves merged **#439 powers-BULK WRITE** separate and does not
  claim tag/direct-reader completion at this base.
- It keeps `main` and #39 gated, records `dual_backend=false`, and leaves
  V-BURN on **HOLD**.
