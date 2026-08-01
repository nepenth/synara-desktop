# Command palette and `useCommands` residual audit

| Field       | Value                                                                                                                                                  |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Tip audited | `ee450251` (`feature/matrix-rust-sdk-full-replacement`, after #375 moderation and #395 members snapshot)                                                   |
| Scope       | `synara/src/app/hooks/useCommands.ts`, `synara/src/app/features/room/CommandAutocomplete.tsx`, and the command-specific submit path in `RoomInput.tsx` |
| Change type | Documentation only                                                                                                                                     |
| Policy      | Native desktop is fail-closed; `dual_backend` is forbidden                                                                                             |
| V-BURN      | **HOLD / not complete**                                                                                                                                |

## Finding

`CommandAutocomplete` is the command palette for slash commands. It calls
`useCommands`, takes `Object.keys(commands)`, and renders every registered ID;
there is no desktop/native filtering. The `Command` enum has 22 public slash
IDs. Of those, six still have direct Matrix JS mutation owners in
`useCommands`, and the five message-shaping IDs retain a conditional legacy
`mx.sendMessage` branch in `RoomInput`.

#375 moved the five moderation writes (`/invite`, `/disinvite`, `/kick`,
`/ban`, and `/unban`) to the native moderation owner with no JS mutation
fallback. `/kick` and `/ban` still use the SDK `Room` member list to expand
server-name targets before calling that native owner. #395's native member
snapshot is wired to the Members settings UI, not to this command expansion
path.

In this document, “residual ID” means the public slash-command value (for
example, `/invite`), not a new issue or PR identifier.

## Remaining JS command owners

| JS owner                         | Residual IDs                                        | SDK / `mx.*` ownership on the audited tip                                                                                                                                                                                         | Native state                                                                                                                                                                                                            |
| -------------------------------- | --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `useCommands.ts:386-409`         | `/ignore`, `/unignore`                              | `mx.getIgnoredUsers` and `mx.setIgnoredUsers`                                                                                                                                                                                     | **Open.** No native ignore-list owner is used by these commands.                                                                                                                                                        |
| `useCommands.ts:411-455`         | `/myroomnick`, `/myroomavatar`                      | SDK room-member state read via `getRoomCurrentState(...).getStateEvents`, identity via `mx.getSafeUserId`, and `mx.sendStateEvent` for `m.room.member`                                                                            | **Open.** Room-local profile writes remain JS-owned.                                                                                                                                                                    |
| `useCommands.ts:472-537`         | `/delete`                                           | SDK member-list expansion for server targets (`:98-104`), `mx.timestampToEvent`, `mx.http.authedRequest`, `mx.createMessagesRequest`, and `mx.redactEvent`                                                                       | **Open.** The scan-and-redact command has no native owner here.                                                                                                                                                         |
| `useCommands.ts:539-588`         | `/acl`                                              | SDK room-state read via `getStateEvent`, then `mx.sendStateEvent` for `m.room.server_acl`                                                                                                                                         | **Open.** ACL mutation remains JS-owned.                                                                                                                                                                                |
| `RoomInput.tsx:610-632, 667-682` | `/me`, `/notice`, `/shrug`, `/tableflip`, `/unflip` | These five IDs use `RoomInput`’s special message-shaping path rather than the `useCommands` executor. `sendPlainTextWithNativeOwner` is attempted first; if it returns `legacy`, `RoomInput` calls `mx.sendMessage` (`:680-682`). | **Native-backed when a logged-in native session is live; conditional legacy branch remains.** A native send IPC failure throws and does not fall through, but a missing/non-logged-in native snapshot returns `legacy`. |

The direct `useCommands` mutation residual is therefore six IDs:

```text
/ignore /unignore
/myroomnick /myroomavatar
/delete
/acl
```

Together with the five conditional message-send IDs, 11 of the 22 palette
IDs retain a JS mutation or fallback surface that must be considered in the
replacement plan. `/kick` and `/ban` are native mutation owners, but their
server-target expansion remains a separate SDK member-read coupling.

## Native-backed command IDs and retained SDK coupling

These IDs are not remaining JS mutation owners on the audited tip, but they
still appear in the same palette and may retain SDK-backed reads or static
types:

| Owner                    | IDs                              | Tip evidence                                                                                                                                            | Status                                                                  |
| ------------------------ | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `useCommands.ts:206-233` | `/startdm`                       | Existing-DM detection uses `getDMRoomFor(mx, ...)` and `mx.getSafeUserId`; creation uses `createRoomWithNativeOwner`, then the native `m.direct` writer | **Native create owner; retained SDK read only.** No JS create fallback. |
| `useCommands.ts:235-253` | `/join`                          | Uses `joinRoomWithNativeOwner` and `matrix_room_join` through the desktop bridge                                                                        | **Native and fail-closed.**                                             |
| `useCommands.ts:255-268` | `/leave`                         | Uses `leaveRoomWithNativeOwner` and `matrix_room_leave` through the desktop bridge                                                                      | **Native and fail-closed.**                                             |
| `useCommands.ts:270-385` | `/invite`, `/disinvite`, `/kick`, `/ban`, `/unban` | Writes route through `nativeRoomModerationOwner` and `matrix_room_invite/kick/ban/unban`; `/kick` and `/ban` retain SDK `Room.getMembers()` expansion for server-name targets | **Native moderation owner; fail-closed for writes.** The member-read expansion is still JS-owned, and #395's Members snapshot is not wired into this command path. |
| `useCommands.ts:457-470` | `/converttodm`, `/converttoroom` | Writes route through `nativeMDirect`; `/converttodm` retains `mx.getSafeUserId` for user identification                                                 | **Native `m.direct` writer; retained SDK identity read only.**          |
| `useCommands.ts:590-608` | `/poll`                          | Uses `sendPollWithNativeDesktopOwner`; the legacy result raises an error instead of sending through `mx.sendEvent`                                      | **Native and fail-closed for the command path.**                        |

The palette itself is a UI/registry owner, not a Matrix mutation owner:
`CommandAutocomplete.tsx:39-49` obtains the SDK client, calls `useCommands`,
and exposes all 22 IDs through `Object.keys(commands)`. Its `Room` prop is also
typed from `matrix-js-sdk` at `:4,20-22`. Any future native command slice must
update this registry/palette boundary as part of the same UI → Tauri IPC →
native owner change; replacing only a lower-level helper would leave the JS
command ID discoverable without changing its ownership.

`useCommands.ts:1-2` itself retains direct `matrix-js-sdk` imports for the
client, room, member, request, direction, and ACL content types used by the
residual handlers and retained SDK reads.

## Residual implementation order

1. Add native owners for ignore-list writes, room-local member-profile writes,
   bulk historical redaction, and server ACL writes. Each must fail closed on a
   live native desktop session with no JS fallback.
2. Re-home server-name member expansion for `/kick`, `/ban`, and `/delete` if
   the command boundary requires native member reads; #395's Members settings
   snapshot alone does not close this residual.
3. Resolve the conditional legacy branch for the five message-shaping IDs if
   the product boundary requires native desktop to fail closed even when the
   native session snapshot is unavailable or logged out.
4. Reconcile palette visibility and descriptions in the same vertical as each
   owner migration; do not hide a residual command as a substitute for moving
   its behavior.

No source or test files were changed by this audit. This does not claim
V-BURN completion and does not alter the `#39` gate.
