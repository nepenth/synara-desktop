# Presence / typing residual ownership inventory

| Field         | Value                                                                                                                     |
| ------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Status        | **First presence slice merged (#458); docs-only residual audit remains open**; no product code changed                    |
| Measured tip  | `fd0dfbf464ea59351d2cca1b746ba9d3f00923e7` on `feature/matrix-rust-sdk-full-replacement`; #458 implementation base remains `c1e9c3be` |
| Scope         | Desktop user presence and room typing paths in `synara/src` and `src-tauri/src`                                           |
| Policy        | Native desktop is fail-closed; `dual_backend=false`; V-BURN remains **HOLD**                                             |
| Runtime proof | Presence live proof is **Not confirmed** after #458; typing live proof remains unclaimed; no acceptance claim is made     |

This inventory separates Matrix network ownership from the JS projection and UI
wrappers that remain after the native typing slice. It covers user presence
(`m.presence`-style availability), not MatrixRTC call membership presence.

> **Post-merge note at implementation base `c1e9c3be`; docs refresh at `fd0dfbf4`.** #458 is merged. Its first presence slice
> lands the native snapshot/subscription route, profile binding, JavaScript
> presence-owner deletion, and focused local evidence. #461 independently lands
> the room-directory slice at the implementation base; no product code changed
> between that base and this docs tip. The presence residual
> remains open because authenticated live proof and full acceptance evidence are
> **Not confirmed**; this file is not a merge-readiness or completion claim.

## Conclusion at the measured tip

- **Typing receive/send transport is native-owned.** Rust listens for
  `SyncTypingEvent`, excludes the local user, and projects joined-room typing
  users. `matrix_typing_snapshot` exposes that projection and
  `matrix_typing_set` calls the live SDK room's `typing_notice` API.
- **Typing is not JS-network-owned anymore, but it is not JS-free.** JS still
  owns the polling adapter, the Jotai projection cache, compose timing, room
  member-name resolution, and the typing presentation controls. These are
  residual UI/state owners, not a second Matrix typing backend.
- **The first native presence slice is merged.** `NativePresenceOwner` now
  consumes the managed client's global `PresenceEvent` stream, projects into
  `PresenceIndex`, and exposes `matrix_presence_snapshot`,
  `matrix_presence_subscribe`, `matrix_presence_unsubscribe`, and the
  `matrix-presence-updated` event. `UserRoomProfile` uses the native owner,
  and the former `useUserPresence` JavaScript owner is deleted.
- **Presence closure is not proven.** Focused local Rust/IPC/frontend/source
  evidence is present, but no authenticated two-client desktop proof with
  retained command/event evidence is recorded at this tip. The live proof and
  independent acceptance status are **Not confirmed**.
- **The generic stream topic is still not the product route.** `PresenceIndex`
  is reused by the live owner, while `PresenceStreamBody` and the generic
  `presence` topic remain protocol/foundation shapes rather than the profile's
  subscription path.

Do not describe the combined presence/typing area as fully native, and do not
use this audit as evidence that V-BURN is complete. V-BURN remains HOLD.

## Ownership map

| Product surface            | Native owner on this tip                                                                                                                                                                 | Residual JS owner / consumer                                                                                                                                                                                                 | Honest status                                                                                        |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Remote typing receive      | `src-tauri/src/matrix/typing/live.rs:22-67` — `NativeTypingOwner` handles `SyncTypingEvent`, keeps joined rooms, filters the local user, and writes `TypingIndex`                        | `synara/src/app/state/typingMembers.ts:78-184` polls `matrix_typing_snapshot` once per second and rebuilds a Jotai map; `RoomViewTyping`, `RoomNavItem`, and `MembersDrawer` consume that map                                | Native network projection; JS cache and UI remain                                                    |
| Local typing send/clear    | `src-tauri/src/matrix/auth/product.rs:1962-1973` — `matrix_typing_set` validates the active native session and calls `Room::typing_notice`; registered in `src-tauri/src/lib.rs:432-433` | `synara/src/app/hooks/useTypingStatusUpdater.ts:7-45` decides when to invoke native IPC, throttles sends, and schedules the local timeout; `RoomInput.tsx:250,630,687,810` drives it from compose events                     | Native network send; JS timing/orchestration remains                                                 |
| Typing display identity    | Native snapshot carries room/user IDs only                                                                                                                                               | `RoomViewTyping.tsx:21-25` resolves names through the SDK-backed `Room` member map; `useRoomTypingMembers.ts` selects the JS cache                                                                                           | UI/member-read residual; not a JS typing event owner                                                 |
| Drop-typing control        | No native command is needed to receive remote state                                                                                                                                      | `RoomViewTyping.tsx:32-41` deletes entries from the local Jotai map only; it does not call `matrix_typing_set`                                                                                                               | Explicit residual UI-only dismissal behavior; do not call it native remote-state mutation            |
| User presence read/display | `src-tauri/src/matrix/presence/live.rs:96-184` — `NativePresenceOwner` consumes the managed global `PresenceEvent` stream; `presence/product_commands.rs` exposes snapshot/subscribe/unsubscribe; `lib.rs` registers the commands | `UserRoomProfile.tsx` calls `useNativeUserPresence`; `nativePresence.ts` validates generation/user/subscription boundaries and maps unavailable to no badge; `useUserPresence.ts` is deleted | Native product route landed in #458; authenticated live proof and independent acceptance remain **Not confirmed** |

## Native typing path

The actual live typing path is command-based, not the generic stream-topic
shell:

```text
Native session start
  → NativeTypingOwner::start
  → SyncTypingEvent → TypingIndex
  → matrix_typing_snapshot
  → JS Jotai projection → RoomViewTyping / room list / members drawer

RoomInput compose
  → useTypingStatusUpdater
  → matrix_typing_set { roomId, typing }
  → Room::typing_notice(bool)
```

`NativeTypingOwner` is attached during password login, registration restore,
and persisted-session restore in `product.rs`. Its snapshot is session-
generation stamped and only reports non-empty rooms. The JS adapter adds
`Date.now()` timestamps for local rendering; those timestamps are not Matrix
event ownership.

There is also a `typing` topic in
`src-tauri/src/matrix/ipc/stream.rs` and
`synara/src/app/features/matrix-ipc/streamBody.ts`, but the source scan found
no live emitter or product subscriber for that generic route. The current
product binding is the two `matrix_typing_*` Tauri commands above.

## Presence path and remaining gap

The first native presence slice now runs through this route:

```text
Native session login/register/restore
  → NativePresenceOwner::start
  → managed matrix-sdk PresenceEvent → PresenceIndex
  → matrix_presence_snapshot
  → matrix_presence_subscribe
  → matrix-presence-updated
  → useNativeUserPresence
  → UserHero / PresenceBadge
```

The native route is fail-closed: unavailable, malformed, stale, or mismatched
native state produces no badge and does not fall back to the JavaScript SDK.
`matrix_presence_unsubscribe` is used when the profile owner is disposed.

The remaining presence gap is evidence, not a second backend: no authenticated
two-client desktop proof with retained command/event readback is recorded at
the `fd0dfbf4` docs tip for the `c1e9c3be` implementation base, and the full
lifecycle/error acceptance matrix has not been independently accepted. The
generic `presence` stream topic and
`PresenceStreamBody` remain protocol/foundation shapes and are not substitutes
for the landed profile route.

## Residual work boundary

| Remaining gate            | Required evidence before claiming closure                                                                                                                                                                                                                         |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Presence live proof       | Authenticated two-client desktop run showing snapshot, subscription, remote state change, unknown/unavailable handling, teardown, and no JS fallback through the exact native commands/event; current status **Not confirmed**                                      |
| Presence acceptance       | Rerun the focused lifecycle/error/source-absence evidence for the `c1e9c3be` implementation base, retain results at docs tip `fd0dfbf4`, and complete independent acceptance. Local tests/source guards are not live proof. |
| Typing residual cleanup   | Decide and document the local-only “Drop Typing Status” behavior; keep JS timing/presentation only where it is intentionally UI-owned, or move it behind the native typing contract. Preserve the native-only network route and no-fallback behavior |
| Typing live proof         | Authenticated two-client desktop run showing remote typing appear/clear and local compose send/clear through `matrix_typing_*`; current status **Not confirmed**                                                                                                    |

This document records the #458 first slice at implementation base `c1e9c3be`
and the honest docs refresh tip `fd0dfbf4`; it does not claim presence
acceptance, typing closure, or V-BURN completion.

## Source inspection basis

The inventory was produced from the measured tip with focused searches over
`synara/src`, `src-tauri/src`, and the Matrix replacement docs for:

```text
presence, UserEvent.Presence, CurrentlyActive, LastPresenceTs,
matrix_presence_*, matrix-presence-updated, matrix_typing_*, SyncTypingEvent,
RoomMemberEvent.Typing, sendTyping, typingMembers, PresenceIndex,
PresenceStreamBody, NativePresenceOwner, useNativeUserPresence
```

The existing unit tests for the typing index, native typing serialization,
presence owner, native presence serialization, and JS snapshot conversion are
contract/unit evidence only. They do not prove an authenticated live desktop
session.
