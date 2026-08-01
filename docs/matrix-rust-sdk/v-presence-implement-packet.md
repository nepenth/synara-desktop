# V-PRESENCE — native user-presence implementation packet

| Field      | Value                                                                                                                      |
| ---------- | -------------------------------------------------------------------------------------------------------------------------- |
| Status     | **Implementation packet** — this PR is docs-only; it does not claim the presence vertical is implemented                   |
| Residual   | **V-PRESENCE.USER**                                                                                                        |
| Source     | [v-presence-typing-residual.md](v-presence-typing-residual.md) from #384                                                   |
| Base       | `feature/matrix-rust-sdk-full-replacement` at `e8a00f7273cb1ee8528df4fa2c3bffc455704322`                                   |
| PR shape   | Focused **draft** PR targeting `feature/matrix-rust-sdk-full-replacement`                                                  |
| Scope      | Desktop user presence (`m.presence`) in the user-room profile; typing and MatrixRTC call membership are separate residuals |
| Policy     | [full-vertical-policy.md](full-vertical-policy.md): one UI → Tauri IPC → live `matrix-sdk` owner, with JS-owner deletion   |
| Guard      | Never `main`, umbrella PR **#39**, or V-BURN; **#407 owns `product.rs`**; `dual_backend` is forbidden; native failure is fail-closed |
| Current PR | Only this packet is allowed to change; `src-tauri/src/matrix/auth/product.rs` is prohibited                                |

The source of truth for the residual is the #384 inventory. This packet turns
the user-presence portion into a bounded implementation contract. It does not
implement product code, update generated status, or claim V-BURN completion.

## 1. Objective and completion bar

Replace the desktop profile's live user-presence owner with one native route:

```text
UserRoomProfile → typed native presence IPC → managed live matrix-sdk sync → UserHero / PresenceBadge
```

The completed slice must preserve the current profile behavior for:

- `online`, `unavailable`, and `offline` display states;
- the optional status message shown by the presence tooltip;
- the optional last-active timestamp and currently-active flag used by the
  profile presentation; and
- live updates when the remote user's Matrix presence changes.

The native desktop session is the only supported Matrix route for this
capability. A missing, stale, malformed, or failed native result must produce
an explicit unavailable state and no presence badge; it must never be treated
as `offline`, served from the JavaScript SDK, or silently retained as stale
presence.

This packet covers user presence only. It does not close room typing, the
typing polling/projection residual, or MatrixRTC call-membership presence.

## 2. Scope and prerequisites

### In scope for the future presence vertical

- a Rust live presence owner under `src-tauri/src/matrix/presence/`, using the
  managed authenticated `matrix-sdk` client and the existing session-generation
  model;
- Synara-owned presence DTOs and the exact commands/events frozen in Section 3;
- Tauri command and desktop capability registration for those exact routes;
- an SDK-neutral frontend owner used by `UserRoomProfile` and the existing
  `UserHero` / `PresenceBadge` presentation;
- focused Rust, DTO/IPC, frontend owner, source-absence, and live two-client
  proof; and
- physical removal of the user-presence JavaScript SDK owner in the same
  product vertical.

### Out of scope

- `src-tauri/src/matrix/auth/product.rs` in this packet or in this docs PR;
  if the future vertical cannot reach the managed session without changing
  that file, stop and split an independently approved auth/session-boundary
  task rather than editing it here;
- `synara/src/app/state/typingMembers.ts`,
  `synara/src/app/hooks/useTypingStatusUpdater.ts`, and the room typing UI;
- MatrixRTC call membership, `Room::has_active_room_call`, widgets, or call
  join/leave/key-session behavior;
- setting the local user's presence or adding a new presence-write product
  surface;
- the generic `presence` stream topic as a substitute for a live product
  owner; and
- any change to `main`, PR #39, V-BURN status, or unrelated Matrix verticals.

Before a future implementation starts, the writer must verify:

1. `HEAD` equals the implementation packet's recorded base SHA and the PR
   target is `feature/matrix-rust-sdk-full-replacement`;
2. the managed native session is authenticated, has a live sync path, and has
   a supported session-generation-bound capability boundary for presence;
3. the pinned SDK evidence in Section 7 still matches the dependency actually
   compiled by the desktop crate; and
4. no JavaScript Matrix client is started as a presence fallback or second
   owner.

A failed prerequisite blocks implementation. It must not be worked around by
adding a fallback, weakening the native gate, or modifying `product.rs`.

## 3. Frozen native IPC contract

The future implementation must use these exact names. No aliases, implicit
fallback commands, or generic untyped payloads may be added.

| Exact command                 | Request              | Required result                                                                                                                |
| ----------------------------- | -------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `matrix_presence_snapshot`    | `{ userId }`         | `PresenceSnapshotResult` containing `sessionGeneration`, `userId`, and either a typed snapshot or an explicit `unknown` result |
| `matrix_presence_subscribe`   | `{ userId }`         | `PresenceSubscription` containing an opaque `subscriptionId`, the bound `userId`, and `sessionGeneration`                      |
| `matrix_presence_unsubscribe` | `{ subscriptionId }` | Idempotent release for the current generation, or a safe unavailable error when the generation/client is no longer live        |

The native side emits this exact event for an active subscription:

```text
matrix-presence-updated
```

Every event carries `subscriptionId`, `userId`, `sessionGeneration`, and one
of the following typed outcomes:

```text
ready(snapshot)
unknown
unavailable(diagnosticId)
```

`unavailable` is an error outcome, not a `PresenceState::Offline` value. The
frontend must discard events for another subscription, user, or generation
and must move to the unavailable state rather than guessing or merging them.

### Frozen snapshot DTO

The wire shape is owned by Synara and uses camelCase at the IPC boundary:

```text
PresenceSnapshot {
  userId: string,
  state: "unknown" | "offline" | "online" | "unavailable",
  currentlyActive: boolean,
  lastActiveTs?: integer,
  statusMsg?: string
}
```

Rules:

- `userId` must be a valid fully-qualified Matrix user ID and must equal the
  request/subscription user ID;
- `state: "unknown"` means the native client has no presence record for that
  user; it is not an offline observation;
- `lastActiveTs` is an optional non-negative millisecond timestamp. The
  implementation must preserve the current `User.getLastActiveTs()` meaning;
- `statusMsg` is optional plain text capped at 256 Unicode scalar values;
- no SDK `User`, `PresenceEvent`, `MatrixClient`, Ruma object, token, or SDK
  error crosses IPC; and
- unknown fields, invalid IDs, negative/non-finite timestamps, oversized
  status text, mismatched generation, and mismatched user IDs are rejected.

The existing `PresenceIndex` and `PresenceStreamBody` are reusable validation
foundations only. Their existence is not a live owner and does not authorize
the future implementation to emit the generic `presence` stream topic.

### Native authority and lifecycle

- The Rust owner subscribes to the managed client's global
  `matrix_sdk::ruma::events::presence::PresenceEvent` stream through the
  client's event-handler API. Presence is global; do not attach it to a room
  or infer it from MatrixRTC membership.
- The handler projects only the sender, presence state, currently-active
  value, last-active value, and bounded status text into `PresenceIndex`.
- The owner is stamped with the active session generation. Logout, account
  switch, failed restore, or client teardown retires the index and invalidates
  all subscriptions.
- Snapshot reads and update events must come from the same live native owner.
  A successful snapshot followed by a failed subscription is unavailable, not
  a license to keep using the snapshot indefinitely.
- The command owner must use the existing privacy-safe Matrix IPC error
  categories and diagnostic IDs. Error strings contain no presence status text,
  tokens, keys, event payloads, or arbitrary user content.

## 4. Frontend ownership and physical deletion

The future vertical must rewire the profile without retaining a JavaScript
Matrix presence owner.

| Path                                                         | Required change                                                                                                                                       |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `synara/src/app/components/user-profile/UserRoomProfile.tsx` | Consume the typed native owner; bind and dispose one user subscription with room/profile lifecycle; render unavailable as no badge, never JS fallback |
| `synara/src/app/components/user-profile/UserHero.tsx`        | Keep SDK-neutral presence props/presentation; do not obtain presence from a Matrix SDK model                                                          |
| `synara/src/app/components/presence/Presence.tsx`            | Keep labels/colors/tooltip presentation SDK-neutral; consume the bounded DTO type                                                                     |
| `synara/src/app/hooks/useUserPresence.ts`                    | Delete the hook's `useMatrixClient`, `User`, and `UserEvent` ownership; retain/rehome only SDK-neutral label/type helpers if needed                   |
| `synara/src/app/features/matrix-presence/`                   | Add the native invoke/listen owner, DTO parser, generation/user filtering, disposal, and unavailable-state mapping                                    |

The future implementation must not leave a `useUserPresence` compatibility
hook that calls `matrix-js-sdk`, a hidden `UserEvent` listener, an
`isNative ? native : js` selector, or a legacy sentinel that lets the profile
continue with JS presence. If another consumer of `useUserPresence` is found,
the writer must split the SDK-neutral helper from the SDK-bound owner and
prove that the remaining consumers no longer import the Matrix SDK.

The current docs PR itself must not modify any product file. In particular,
there must be no `product.rs` diff, no generated Tauri schema churn, and no
frontend implementation in this packet PR.

## 5. Required focused tests

The future implementation PR must add or extend the smallest focused evidence
set below. This packet's docs-only PR does not claim any of these tests pass.

### Rust owner tests

Place tests with the native owner under `src-tauri/src/matrix/presence/` and
cover:

- projection of `PresenceEvent` into `PresenceSnapshot` for all supported
  states, optional fields, and a missing presence record;
- status-message cap, valid/invalid user IDs, timestamp bounds, and unknown
  state handling;
- subscription delivery for the requested user only;
- account/session-generation mismatch, logout retirement, teardown, duplicate
  unsubscribe, and late-event rejection;
- failed client read, failed subscription, malformed presence event, and
  failed serialization as unavailable errors; and
- diagnostic strings that contain neither status text nor arbitrary user data.

### DTO/IPC contract tests

Extend the focused contract suites for:

- snapshot, subscription, update, unknown, and unavailable shapes;
- unknown/missing fields, wrong user/generation, invalid IDs, oversized
  status text, invalid timestamps, and media/secret-like fields;
- command argument/result names and exact event name
  `matrix-presence-updated`; and
- privacy-safe serialization of every failure class.

### Frontend owner and source-absence tests

Add a focused owner suite under
`synara/src/app/features/matrix-presence/__tests__/` using injected
`invoke`/`listen` functions. It must assert:

1. snapshot and subscription commands receive the exact user ID;
2. initial snapshot delivery and subsequent matching events update the
   profile;
3. other-user, other-subscription, and stale-generation events are ignored
   and produce an unavailable/resnapshot result;
4. unmount/profile close unsubscribes and late events do not update state;
5. missing command, invoke rejection, malformed payload, failed subscription,
   and unavailable event produce no badge and no JS SDK call; and
6. a source guard proves the profile path contains no `matrix-js-sdk` import,
   `useMatrixClient` presence read, `UserEvent` presence listener, or direct
   `User` presence field access after the vertical lands.

The source guard must be capability-scoped. It must not assert that unrelated
Matrix JS imports across the repository are already zero.

## 6. Authenticated live proof

The future implementation must record a two-client desktop proof against the
repository's [test-matrix-synapse-topology.md](test-matrix-synapse-topology.md)
topology. The proof must name the exact native commands and event observed.

1. Sign in client A natively and open a room profile for user B. Confirm
   `matrix_presence_snapshot` returns a generation-matched typed result.
2. Confirm `matrix_presence_subscribe` returns an opaque subscription bound to
   B and the same generation; no JS `UserEvent` listener is installed.
3. Change B's presence using the approved product/SDK path from client B.
   Confirm client A receives `matrix-presence-updated` and the profile changes
   from the observed state without polling the JS client.
4. Exercise online, unavailable, offline, status-message, currently-active,
   and absent/unknown observations. Confirm `unknown` and `unavailable` do
   not render as offline.
5. Log out or switch the native session on client A. Confirm the subscription
   is invalidated, the profile shows no stale badge, and no JS fallback runs.
6. End the native sync/client path. Confirm snapshot/subscription failure is
   visible as unavailable and never becomes a successful presence observation.

The proof must be marked **not run**, **failed**, or **passed** with retained
logs/screenshots and exact command/event evidence. A fixture, compile check,
raw HTTP request, or direct SDK probe cannot substitute for this product proof.

## 7. Pinned upstream evidence

The packet relies on the approved Matrix Rust SDK pin already recorded in the
repository:

- Repository: `https://github.com/matrix-org/matrix-rust-sdk`
- Release: `matrix-sdk-0.18.0`
- Commit: `1c44fb66214667c6d00acaf72ab592493653708b`

Relevant immutable evidence:

- [`PresenceEvent` handler registration](https://github.com/matrix-org/matrix-rust-sdk/blob/1c44fb66214667c6d00acaf72ab592493653708b/crates/matrix-sdk/src/event_handler/static_events.rs#L157-L161) establishes that the pinned SDK accepts `PresenceEvent` as a sync event-handler type.
- [`Client::add_event_handler`](https://github.com/matrix-org/matrix-rust-sdk/blob/1c44fb66214667c6d00acaf72ab592493653708b/crates/matrix-sdk/src/client/mod.rs#L944-L979) establishes the client event-handler registration API used by the future native owner.
- [`SyncResponse` presence projection](https://github.com/matrix-org/matrix-rust-sdk/blob/1c44fb66214667c6d00acaf72ab592493653708b/crates/matrix-sdk/src/sync.rs#L40-L60) establishes that the pinned sync path carries typed `PresenceEvent` updates; the dependency's content-field shape is API-shape evidence, not live product parity.

These sources do not prove that the Synara desktop route is implemented. The
future writer must still prove the managed-client lifecycle, DTO boundary,
physical JS-owner deletion, and authenticated UI behavior.

## 8. Ordered implementation work

1. Reconfirm the exact base SHA, target branch, approved SDK pin, and the
   no-`product.rs` scope guard. Stop on any mismatch.
2. Confirm the existing managed-session boundary can provide a live client to
   a presence owner without a second client or a JavaScript fallback. If it
   cannot, stop and return an auth/session-boundary escalation.
3. Freeze and test the Synara-owned DTOs, exact command names, exact event
   name, bounds, generation rules, and unavailable semantics in Section 3.
4. Implement the Rust presence projection and subscription lifecycle using the
   pinned SDK presence event handler and `PresenceIndex`; do not use raw HTTP
   or the generic unowned stream topic.
5. Register the exact native commands/events and implement the SDK-neutral
   frontend owner with strict user/subscription/generation filtering.
6. Delete the JavaScript presence owner and remove any capability-local
   compatibility branch, while retaining only SDK-neutral presentation code.
7. Run focused unit/contract/source-absence tests, then the authenticated
   two-client proof. Retain exact evidence and mark every required case.
8. Review the diff for `product.rs`, `main`, #39, V-BURN, `dual_backend`, raw
   HTTP, hidden JS fallback, unrelated verticals, and generated-file churn.
   Keep the implementation PR focused and draft until independently reviewed.

## 9. Acceptance statement

V-PRESENCE.USER may be marked complete only when:

- the profile obtains user presence exclusively through the exact native
  snapshot/subscription contract;
- the live Rust owner consumes the managed `matrix-sdk` presence stream and
  projects bounded Synara DTOs;
- generation, user, subscription, unknown, unavailable, teardown, and error
  behavior are tested and fail closed;
- the JavaScript `User`/`UserEvent` presence owner is physically deleted and
  no profile path can invoke it;
- the live two-client product proof passes for state changes and teardown;
- no status text, tokens, keys, SDK objects, or raw errors cross the IPC/log
  boundary; and
- typing and MatrixRTC presence remain separately tracked rather than being
  relabeled as closed by this slice.

This completion statement applies only to V-PRESENCE.USER. It must not be used
to claim P4.7, FR-7.2-013, V-BURN/#327, or PR #39 complete without their own
remaining evidence and deletion gates.
