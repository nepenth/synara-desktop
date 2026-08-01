# V-ROOMS — PowersEditor and bulk power-level residual

| Field    | Value                                                                                                                                |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Status   | **Inventory (docs only)** — no product code or native vertical claim                                                                 |
| Base tip | `ee450251` (`feature/matrix-rust-sdk-full-replacement`; tip after #395 members-read first slice + #397 tip honesty)                  |
| Scope    | PowersEditor tag writes and bulk `m.room.power_levels` writes after the single-user set-power-level slice                            |
| Related  | #375 / adjacent commit `7eb5bc3d` (`matrix_room_set_power_level`); room-member moderation is separate                                |
| Policy   | [full-vertical-policy.md](full-vertical-policy.md) — UI → Tauri IPC → live `matrix-sdk`, physical JS-owner deletion, no dual backend |
| Serial   | **powers-bulk product NOT started.** `product.rs` serial: #395 members-read first slice **MERGED**; next product in flight may be CallWidget media IPC; powers-bulk is not in flight. Never `main`/umbrella #39; `dual_backend` forbidden |

**Implementation packet:** [v-rooms-power-levels-implement-packet.md](v-rooms-power-levels-implement-packet.md)

> **Scope guard.** This inventory is based only on tip `3d76402f`. It does not
> use `main`, #39, or claim V-BURN complete. The adjacent #375 commit is used
> only to define the single-user comparison boundary; it is not present in the
> base worktree.

## 1. Baseline and comparison boundary

On the requested base tip, there is no `matrix_room_set_power_level` command.
The current single-user UI call is still the JS SDK call in
`synara/src/app/components/user-profile/PowerChip.tsx:181-187`:

```ts
await mx.setPowerLevel(room.roomId, userId, power);
```

The adjacent #375 slice adds `matrix_room_set_power_level` over the live native
SDK and wires `PowerChip` through `setPowerLevelWithNativeOwner`. Its contract
accepts one `room_id`, one `user_id`, and one `power_level`, then calls
`Room::update_power_levels` for that user. That closes the per-user `PowerChip`
write when #375 is present; it does not close the settings editors below.

## 2. Residual write owners

| Owner / UI route                                                                                                                                                                                   | JS call site                                                                                               | Matrix surface and behavior                                                                                                                                                                                                                                              | Gap after single-user set-power-level                                                                                                                                                        |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `synara/src/app/features/common-settings/permissions/PowersEditor.tsx:291-337`, opened by `room-settings/permissions/Permissions.tsx:34-35` and `space-settings/permissions/Permissions.tsx:34-35` | `PowersEditor.tsx:336`: `mx.sendStateEvent(room.roomId, StateEvent.PowerLevelTags, content)`               | Replaces the complete `in.synara.room.power_level_tags` state content. Supports creating, editing, and deleting custom labels/colors/icons for numeric power levels.                                                                                                     | No native tag-state command or native owner. `matrix_room_set_power_level` changes a user's numeric level and cannot write this custom state event.                                          |
| `synara/src/app/features/common-settings/permissions/PermissionGroups.tsx:32-89`, mounted by the same room and space Permissions routes                                                            | `PermissionGroups.tsx:88`: `mx.sendStateEvent(room.roomId, StateEvent.RoomPowerLevels, editedPowerLevels)` | Stages multiple permission changes, applies them to a copy of the complete power-level content, and sends one full `m.room.power_levels` event. It covers user default, event/state defaults and overrides, actions, and notifications exposed by the room/space groups. | No native bulk policy command. The single-user command only targets one `users[user_id]` entry; it cannot replace the full event or the defaults/action/event/notification maps edited here. |

`Powers.tsx` is a read-only presentation and edit-entry surface; it does not
perform a write. The two `Permissions.tsx` files are route owners for both
residual editors, not additional Matrix write sites.

## 3. JS read and permission dependencies

These are not additional write calls, but they remain part of the editor
operating path and must not be mistaken for a closed native vertical:

- `synara/src/app/hooks/usePowerLevels.ts:68-73` reads `m.room.power_levels`
  through the JS `Room`/`MatrixEvent` model and supplies the bulk editor.
- `synara/src/app/hooks/usePowerLevelTags.ts:90-105` reads
  `in.synara.room.power_level_tags` through the JS state-event model and fills
  fallback tags for the editors and permission displays.
- Both Permissions routes use `useRoomPermissions` and `mx.getSafeUserId()` to
  gate `StateEvent.PowerLevelTags` and `StateEvent.RoomPowerLevels` editing.

This document inventories the residual **writes** requested here. Native
readback/projection and permission-gate ownership need to be proven in the
same full vertical or tracked as an explicitly named follow-up; they are not
silently treated as native because a write command exists.

## 4. Gap versus `matrix_room_set_power_level`

| Capability                        | `matrix_room_set_power_level`                                                      | Required native owner for this residual                                                                                                                              |
| --------------------------------- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| One user's numeric power          | `room_id` + `user_id` + `power_level`; one `Room::update_power_levels` user update | #375 / `PowerChip` only; not an editor replacement                                                                                                                   |
| Bulk room/space permission policy | Not represented by the command contract                                            | A live-SDK IPC command carrying a typed full policy or an equivalent validated bulk patch, preserving all fields that `PermissionGroups` currently retains and edits |
| Custom power-level tags           | Not represented by the command or `m.room.power_levels`                            | A live-SDK IPC command for the `in.synara.room.power_level_tags` state content, including create/edit/delete parity                                                  |
| Power-tag icon upload             | Not a power-level command                                                          | The existing compact native media-upload owner covers the upload transport; the tag-state write still belongs to this residual                                       |

Do not implement bulk permission editing as repeated
`matrix_room_set_power_level` calls. That would only update individual user
entries and would lose the semantics of defaults, event/state thresholds,
moderation actions, and notification thresholds. Do not use generic JS
`mx.sendStateEvent` as a desktop fallback for either editor.

## 5. Required completion slice

The eventual product slice must keep the complete operating path together:

1. Add validated native IPC/SDK ownership for the custom tag state and the
   complete bulk `m.room.power_levels` policy (command names are an
   implementation decision, but both contracts must be explicit).
2. Wire both Permissions routes and their common editor components through
   native owners. On a native desktop session, missing/unavailable native
   capability is terminal; there is no JS fallback.
3. Preserve current behavior: tag create/edit/delete, tag metadata and icon
   association, room and space permission groups, pending multi-change apply,
   and all retained power-level fields.
4. Physically delete the superseded JS write owners/imports and any
   JS-only tests or compatibility branches in the same slice. Verify that
   `PowersEditor.tsx` and `PermissionGroups.tsx` no longer call
   `mx.sendStateEvent` for these surfaces.
5. Separately prove native readback/projection and permission gating before
   calling the full vertical closed.

## 6. Non-goals

| Item                                 | Status                                                                         |
| ------------------------------------ | ------------------------------------------------------------------------------ |
| Single-user `PowerChip` set          | #375 comparison boundary; not this residual                                    |
| Invite/kick/ban/unban moderation     | Separate room-members moderation surface                                       |
| Generic developer `StateEventEditor` | Separate unrestricted developer-tool residual; not a product Permissions owner |
| `main` or umbrella #39               | Out of scope                                                                   |
| V-BURN completion or runtime proof   | Not claimed by this docs PR                                                    |

**Confidence: high** for the write inventory. The base-tip search finds one
`mx.setPowerLevel` call, one `PowersEditor` `sendStateEvent` call, and one
`PermissionGroups` `sendStateEvent` call. The #375 comparison command is
single-user by its Rust signature and implementation, so it cannot close the
two settings-editor writes.
