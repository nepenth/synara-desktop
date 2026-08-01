# V-ROOMS members moderation **writes** — slice scope (#375)

| Field  | Value                                                              |
| ------ | ------------------------------------------------------------------ |
| Status | **Wired writes** (invite/kick/ban/unban/setPowerLevel single-user) |
| Base   | `feature/matrix-rust-sdk-full-replacement`                         |
| Policy | Full vertical for **per-user moderation write** paths only         |

## In scope (this slice)

- Native IPC: `matrix_room_invite`, `matrix_room_kick`, `matrix_room_ban`, `matrix_room_unban`, `matrix_room_set_power_level`
- UI: InviteUserPrompt, UserModeration kick/ban/unban, PowerChip single PL change
- Commands: `/invite`, `/kick`, `/ban`, `/unban`, `/disinvite` via native owners with `rateLimitedActions` multi-user serialization

## Explicit out-of-scope residuals (named)

| ID                          | Residual                                                                                    | Notes                                                                                                                                        |
| --------------------------- | ------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| **V-ROOMS.R-MEMBERS-READ**  | #395 owns the settings member snapshot; #405 is the pending native drawer/lobby/mention wiring. After #405, power-level/creator reads remain residual. | See [members-read residual inventory](v-rooms-members-read-residual.md); do not treat #405 as a power-read implementation |
| **V-ROOMS.R-POWERS-BULK**   | PowersEditor / PermissionGroups bulk `m.room.power_levels` via `mx.sendStateEvent`          | Not dual-backend for single-user PowerChip; bulk editor is separate residual                                                                 |
| **V-ROOMS.R-INVITE-REASON** | matrix-sdk 0.18 `invite_user_by_id` has no reason field; reason arg accepted then discarded | See [invite-reason residual options](v-rooms-invite-reason-residual.md); preserve UI flag until SDK exposes reason or product drops the flag |

## Invite reason honesty

Native invite accepts optional `reason` for API stability with slash-command `-r` flag, but the homeserver invite request does **not** include a reason under matrix-sdk 0.18. This is an explicit SDK gap, not a JS fallback.

The gap and closure options are tracked in [V-ROOMS.R-INVITE-REASON — invite
reason SDK gap after #375](v-rooms-invite-reason-residual.md).
