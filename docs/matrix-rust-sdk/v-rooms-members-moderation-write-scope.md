# V-ROOMS members moderation **writes** — slice scope (#375)

| Field | Value |
|-------|-------|
| Status | **Wired writes** (invite/kick/ban/unban/setPowerLevel single-user) |
| Base | `feature/matrix-rust-sdk-full-replacement` |
| Policy | Full vertical for **per-user moderation write** paths only |

## In scope (this slice)

- Native IPC: `matrix_room_invite`, `matrix_room_kick`, `matrix_room_ban`, `matrix_room_unban`, `matrix_room_set_power_level`
- UI: InviteUserPrompt, UserModeration kick/ban/unban, PowerChip single PL change
- Commands: `/invite`, `/kick`, `/ban`, `/unban`, `/disinvite` via native owners with `rateLimitedActions` multi-user serialization

## Explicit out-of-scope residuals (named)

| ID | Residual | Notes |
|----|----------|-------|
| **V-ROOMS.R-MEMBERS-READ** | Member list / membership listeners still JS (`useRoomMembers`, etc.) | Follow-up full vertical |
| **V-ROOMS.R-POWERS-BULK** | PowersEditor / PermissionGroups bulk `m.room.power_levels` via `mx.sendStateEvent` | Not dual-backend for single-user PowerChip; bulk editor is separate residual |
| **V-ROOMS.R-INVITE-REASON** | matrix-sdk 0.18 `invite_user_by_id` has no reason field; reason arg accepted then discarded | Preserve UI flag until SDK exposes reason or product drops the flag |

## Invite reason honesty

Native invite accepts optional `reason` for API stability with slash-command `-r` flag, but the homeserver invite request does **not** include a reason under matrix-sdk 0.18. This is an explicit SDK gap, not a JS fallback.

