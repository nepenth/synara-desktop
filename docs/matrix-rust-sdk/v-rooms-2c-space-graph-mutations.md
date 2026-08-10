# V-ROOMS.2c — local space graph + mutations/reordering

| Field | Value |
| --- | --- |
| Status | Implementation candidate; live authenticated runtime proof unclaimed |
| Owner | Managed Rust `m.space.child` graph + set/remove + restricted join reparent |
| Queue | `V-ROOMS.2c` (final slice of V-ROOMS.2 after 2a parents / 2b hierarchy) |
| Policy | Full vertical: native readback + mutation + JS owner deletion |

## Retained product contract

Lobby and space nav continue to show ordered space children with suggest flags,
support DnD reorder, set/unset suggested, remove child, add-existing attach, and
create-room parent attach. Restricted rooms dragged across spaces update
`m.room.join_rules` allow membership.

## Operating path

```text
Lobby / space hierarchy UI
  → matrix_space_children_snapshot (polled)
  → pure TS hierarchy assembly (order/suggested/via/ts)
  → mutations:
      matrix_space_child_set { parentId, childId, via, order?, suggested? }
      matrix_space_child_remove { parentId, childId }
      matrix_restricted_join_reparent { roomId, removeParentId?, addParentId }
```

Disqualifying deviations: `mx.sendStateEvent(..., m.space.child, …)`, JS
`getStateEvents(SpaceChild)` graph ownership, dual-backend fallback when desktop
session is available.

## Explicitly out of scope

- Power-level permission evaluation still reads product power-level helpers
- `mx.createRoom` itself and generic join-rules editor UI
- SynaraSpaces account-data pin list
- V-TIMELINE / RoomTimeline paths

## Deletion

- Removed JS `sendStateEvent` SpaceChild writers from Lobby, HierarchyItemMenu,
  AddExisting, and create-room parent attach.
- Removed JS `getStateEvents(SpaceChild)` graph/listener ownership from
  `useSpaceHierarchy` / `useSpaceJoinedHierarchy`.
- Removed dead `isValidChild` / `getSpaceChildren` / `getRoomToParents` helpers.

## Evidence

- `cargo test --locked matrix::spaces`
- nativeSpaceChildOwner unit tests + modernization suite
- `npm run typecheck:modernization` / guardrails as available

Runtime proof remains **Not confirmed** until an authenticated disposable
session reorders/suggests/removes space children and moves a restricted room
exclusively through the native commands.
