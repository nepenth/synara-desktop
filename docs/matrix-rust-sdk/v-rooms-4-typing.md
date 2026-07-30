# V-ROOMS.4 — native typing indicators

| Field  | Value                                                                  |
| ------ | ---------------------------------------------------------------------- |
| Status | Implementation candidate; live authenticated runtime proof unclaimed   |
| Owner  | Managed Rust typing index + `typing_notice` send                       |
| Queue  | `V-ROOMS.4`                                                            |
| Policy | Complete native replacement of typing receive/send; no JS SDK fallback |

## Retained product contract

Room compose sends typing notices while the local user is typing, and remote
typers appear in `RoomViewTyping`. `hideActivity` clears the local projection
and suppresses refresh. Display-name resolution may still use the residual JS
`Room` member map until a members vertical owns profiles.

## Operating path

```text
Desktop session logged in
  → NativeTypingOwner (SyncTypingEvent → TypingIndex)
  → matrix_typing_snapshot (poll)
  → roomIdToTypingMembersAtom RESET
  → RoomViewTyping

RoomInput compose
  → matrix_typing_set { roomId, typing }
  → Room::typing_notice(bool)
```

Disqualifying deviations: binding `RoomMemberEvent.Typing` or calling
`MatrixClient.sendTyping` for product typing; dual-backend fallback when native
session is available.

## Deletion

- Removed `matrix-js-sdk` imports and `RoomMemberEvent.Typing` binder from
  `synara/src/app/state/typingMembers.ts`.
- Removed `MatrixClient.sendTyping` ownership from
  `synara/src/app/hooks/useTypingStatusUpdater.ts`.
- Dropped both paths from `p1.6-js-sdk-import-allowlist.json`.

## Inventory

From integration tip after V-ROOMS.3 (`efc90d5`, production **192** /
repository-wide **205**):

- desktop-runtime production import files **192 → 190**
- desktop-runtime test import files **10 → 10**
- repository-wide import files **205 → 203**
- allowlist **199 → 197**

## Evidence

- `cargo test --locked matrix::typing`
- typing projection unit test + modernization suite subset
- `npm run check:matrix-rust-guardrails`
- Regenerated `desktop-sdk-usage.{json,md}`

Runtime proof remains **Not confirmed** until an authenticated disposable
session shows remote typing appear/clear and local compose send/clear without
JS typing listeners.
