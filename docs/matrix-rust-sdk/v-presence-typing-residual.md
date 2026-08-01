# Presence / typing residual ownership inventory

| Field         | Value                                                                                                                     |
| ------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Status        | **Docs-only residual audit**; no product code changed                                                                     |
| Measured tip  | `e8a00f7273cb1ee8528df4fa2c3bffc455704322` on `feature/matrix-rust-sdk-full-replacement`                                  |
| Scope         | Desktop user presence and room typing paths in `synara/src` and `src-tauri/src`                                           |
| Policy        | Native desktop is fail-closed; `dual_backend` is forbidden                                                                |
| Runtime proof | Typing live proof remains unclaimed; presence native proof is not applicable because no live native presence owner exists |

This inventory separates Matrix network ownership from the JS projection and UI
wrappers that remain after the native typing slice. It covers user presence
(`m.presence`-style availability), not MatrixRTC call membership presence.

## Conclusion at the measured tip

- **Typing receive/send transport is native-owned.** Rust listens for
  `SyncTypingEvent`, excludes the local user, and projects joined-room typing
  users. `matrix_typing_snapshot` exposes that projection and
  `matrix_typing_set` calls the live SDK room's `typing_notice` API.
- **Typing is not JS-network-owned anymore, but it is not JS-free.** JS still
  owns the polling adapter, the Jotai projection cache, compose timing, room
  member-name resolution, and the typing presentation controls. These are
  residual UI/state owners, not a second Matrix typing backend.
- **Presence remains JS-owned on desktop.** `useUserPresence` reads a
  `matrix-js-sdk` `User` and subscribes to its presence events. The desktop
  profile path has no native presence command or fail-closed native gate.
- **The Rust presence code on this tip is foundation-only.** `PresenceIndex`
  is explicitly documented as a harness with no SDK presence APIs and no
  production Tauri commands. The generic `presence` stream topic and its
  `PresenceStreamBody` only validate a possible wire shape; no live producer,
  subscription, or `matrix_presence_*` command was found.

Do not describe the combined presence/typing area as fully native, and do not
use this audit as evidence that V-BURN is complete. V-BURN remains HOLD.

## Ownership map

| Product surface            | Native owner on this tip                                                                                                                                                                 | Residual JS owner / consumer                                                                                                                                                                                                 | Honest status                                                                                        |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Remote typing receive      | `src-tauri/src/matrix/typing/live.rs:22-67` — `NativeTypingOwner` handles `SyncTypingEvent`, keeps joined rooms, filters the local user, and writes `TypingIndex`                        | `synara/src/app/state/typingMembers.ts:78-184` polls `matrix_typing_snapshot` once per second and rebuilds a Jotai map; `RoomViewTyping`, `RoomNavItem`, and `MembersDrawer` consume that map                                | Native network projection; JS cache and UI remain                                                    |
| Local typing send/clear    | `src-tauri/src/matrix/auth/product.rs:1962-1973` — `matrix_typing_set` validates the active native session and calls `Room::typing_notice`; registered in `src-tauri/src/lib.rs:432-433` | `synara/src/app/hooks/useTypingStatusUpdater.ts:7-45` decides when to invoke native IPC, throttles sends, and schedules the local timeout; `RoomInput.tsx:250,630,687,810` drives it from compose events                     | Native network send; JS timing/orchestration remains                                                 |
| Typing display identity    | Native snapshot carries room/user IDs only                                                                                                                                               | `RoomViewTyping.tsx:21-25` resolves names through the SDK-backed `Room` member map; `useRoomTypingMembers.ts` selects the JS cache                                                                                           | UI/member-read residual; not a JS typing event owner                                                 |
| Drop-typing control        | No native command is needed to receive remote state                                                                                                                                      | `RoomViewTyping.tsx:32-41` deletes entries from the local Jotai map only; it does not call `matrix_typing_set`                                                                                                               | Explicit residual UI-only dismissal behavior; do not call it native remote-state mutation            |
| User presence read/display | No live native presence owner. `src-tauri/src/matrix/presence/mod.rs:1-6` and `index.rs:1-5` state that the module is a harness with no SDK presence APIs or production commands         | `synara/src/app/hooks/useUserPresence.ts:18-47` reads `User` fields and subscribes to `UserEvent.Presence`, `CurrentlyActive`, and `LastPresenceTs`; `UserRoomProfile.tsx:63-79` supplies it to `UserHero` / `PresenceBadge` | JS SDK remains the desktop product owner; native fail-closed requirement is not met for this surface |

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

## Presence path and gap

The native side currently has only a pure index and marker:

- `src-tauri/src/matrix/presence/index.rs` stores bounded in-memory
  `PresenceSnapshot` values.
- `src-tauri/src/matrix/presence/mod.rs` says **no SDK presence APIs** and **no
  production Tauri commands**.
- `src-tauri/src/matrix/ipc/stream_body.rs:75-95` defines a typed
  `PresenceStreamBody`, and the matching TypeScript validator accepts the
  `presence` topic. These are protocol/foundation shapes, not a live Matrix
  presence subscription.
- No `matrix_presence_*` command or native presence event handler is registered
  in `src-tauri/src/lib.rs` or `src-tauri/src/matrix/auth/product.rs`.

The remaining desktop route is therefore:

```text
UserRoomProfile
  → useUserPresence(userId)
  → useMatrixClient()
  → matrix-js-sdk User presence fields/events
  → UserHero / PresenceBadge
```

Because this route is not guarded by a native availability check and has no
native replacement, it is a real desktop residual. A future presence slice
must own the live Matrix read through Tauri/`matrix-sdk`, bind the UI to a
typed native projection, and fail closed on missing or failed native state
without adding a backend selector or JS fallback.

## Residual work boundary

| Next slice               | Required evidence before claiming closure                                                                                                                                                                                                            |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Presence native vertical | UI → Tauri IPC → live `matrix-sdk` presence read/subscription, typed snapshot/delta binding, desktop fail-closed behavior, and deletion or isolation of `useUserPresence`'s JS event owner                                                           |
| Typing residual cleanup  | Decide and document the local-only “Drop Typing Status” behavior; keep JS timing/presentation only where it is intentionally UI-owned, or move it behind the native typing contract. Preserve the native-only network route and no-fallback behavior |
| Typing live proof        | Authenticated two-client desktop run showing remote typing appear/clear and local compose send/clear through `matrix_typing_*`; current docs claim **Not confirmed**                                                                                 |

This document does not implement any of those slices and makes no V-BURN
completion claim.

## Source inspection basis

The inventory was produced from the measured tip with focused searches over
`synara/src`, `src-tauri/src`, and the Matrix replacement docs for:

```text
presence, UserEvent.Presence, CurrentlyActive, LastPresenceTs,
matrix_presence_*, matrix_typing_*, SyncTypingEvent, RoomMemberEvent.Typing,
sendTyping, typingMembers, PresenceIndex, PresenceStreamBody
```

The existing unit tests for the typing index, native typing serialization,
and JS snapshot conversion are contract/unit evidence only. They do not prove
an authenticated live desktop session.
