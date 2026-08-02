# V-PRESENCE — native user-presence implementation packet

| Field      | Value                                                                                                                      |
| ---------- | -------------------------------------------------------------------------------------------------------------------------- |
| Status     | **First slice merged (#458); full V-PRESENCE.USER remains open** — this PR is docs-only; live proof and acceptance are not confirmed |
| Residual   | **V-PRESENCE.USER**                                                                                                        |
| Source     | [v-presence-typing-residual.md](v-presence-typing-residual.md) from #384                                                   |
| Base       | `feature/matrix-rust-sdk-full-replacement` at `d82e043db25e4ec786bde103c4d457a898ef664b`                                   |
| PR shape   | Focused **draft** docs PR targeting `feature/matrix-rust-sdk-full-replacement`; #458 is merged at this base                  |
| Scope      | Desktop user presence (`m.presence`) in the user-room profile; typing and MatrixRTC call membership are separate residuals |
| Policy     | [full-vertical-policy.md](full-vertical-policy.md): one UI → Tauri IPC → live `matrix-sdk` owner, with JS-owner deletion   |
| Guard      | Never `main`, umbrella PR **#39**, or V-BURN; #458 is merged; `dual_backend=false`; native failure is fail-closed; V-BURN remains **HOLD** |
| Current PR | Docs-only update to this packet and its linked residual; `src-tauri/src/matrix/auth/product.rs` and all product files are prohibited |

The source of truth for the residual is the #384 inventory. This packet now
records the first native presence slice landed by #458 and the evidence still
required before the full user-presence residual can be accepted. It does not
implement product code, update generated status, or claim V-BURN completion.

> **Post-merge note at `d82e043d`.** #458 is merged at this integration tip.
> Its first slice lands the native snapshot/subscription route, profile binding,
> JavaScript presence-owner deletion, and focused local evidence. The earlier
> rebase hold in #467 is superseded. This draft docs PR may remain draft, but it makes
> no merge-readiness or acceptance claim: authenticated live proof and full
> closure evidence remain **Not confirmed**. `main`, #39, and V-BURN remain
> out of scope.

## Current state at the merge tip

### Landed in #458

- `NativePresenceOwner` consumes the managed authenticated `matrix-sdk` global
  `PresenceEvent` stream and projects bounded Synara snapshots.
- `matrix_presence_snapshot`, `matrix_presence_subscribe`, and
  `matrix_presence_unsubscribe` are registered, with
  `matrix-presence-updated` as the native update event.
- `UserRoomProfile` uses `useNativeUserPresence`; the profile presentation is
  SDK-neutral; and the former `useUserPresence` JavaScript owner is deleted.
- Focused Rust, wire/source, frontend-owner, and source-absence tests are
  present. The desktop production importer inventory is **151** at this tip.

### Still open for V-PRESENCE.USER closure

- The authenticated two-client desktop proof in Section 6 has not been run or
  retained here; its status is **Not confirmed**.
- The complete lifecycle/error matrix and independent acceptance review must
  be rerun against `d82e043d`; local tests and source guards do not substitute
  for the product proof.
- Room typing, MatrixRTC call-membership presence, and the V-BURN gates remain
  separate residuals and are not closed by #458.

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

### Landed first-slice scope

- Rust live presence ownership under `src-tauri/src/matrix/presence/`, using
  the managed authenticated `matrix-sdk` client and session-generation model;
- the Synara-owned DTOs and exact commands/events frozen in Section 3;
- Tauri command and desktop capability registration for those exact routes;
- the SDK-neutral frontend owner used by `UserRoomProfile` and the existing
  `UserHero` / `PresenceBadge` presentation;
- focused Rust, DTO/IPC, frontend-owner, and source-absence evidence; and
- physical removal of the user-presence JavaScript SDK owner in this product
  path.

These items are present at `d82e043d` through merged PR #458. Their presence
is implementation evidence, not authenticated live-product acceptance.

### Remaining closure scope

- rerun the focused lifecycle/error/source-absence checks at this merge tip and
  retain the result against the acceptance matrix;
- record the authenticated two-client desktop proof in Section 6; and
- obtain independent review and acceptance for V-PRESENCE.USER.

### Out of scope

- `src-tauri/src/matrix/auth/product.rs` in this packet or in this docs PR;
  if follow-up closure work cannot reach the managed session without changing
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

Before any follow-up proof or acceptance run, the writer must verify:

1. `HEAD` equals `d82e043db25e4ec786bde103c4d457a898ef664b`, #458 is merged,
   and the target is `feature/matrix-rust-sdk-full-replacement`;
2. the managed native session is authenticated, has a live sync path, and has
   a supported session-generation-bound capability boundary for presence;
3. the pinned SDK evidence in Section 7 still matches the dependency actually
   compiled by the desktop crate; and
4. no JavaScript Matrix client is started as a presence fallback or second
   owner.

A failed prerequisite blocks the proof or acceptance run. It must not be
worked around by adding a fallback, weakening the native gate, or modifying
`product.rs`.

## 3. Frozen native IPC contract

The landed first slice uses these exact names. Follow-up work must preserve
them; no aliases, implicit fallback commands, or generic untyped payloads may
be added.

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
the product path to emit the generic `presence` stream topic.

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

The first slice rewires the profile without retaining a JavaScript Matrix
presence owner. The current ownership evidence is:

| Path                                                         | First-slice evidence                                                                                                                                   |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `synara/src/app/components/user-profile/UserRoomProfile.tsx` | Uses the typed native owner; binds and disposes one user subscription with profile lifecycle; unavailable state renders no badge and has no JS fallback |
| `synara/src/app/components/user-profile/UserHero.tsx`        | Receives SDK-neutral presence props and does not obtain presence from a Matrix SDK model                                                               |
| `synara/src/app/components/presence/Presence.tsx`            | Keeps labels/colors/tooltip presentation SDK-neutral and consumes the bounded DTO type                                                                  |
| `synara/src/app/hooks/useUserPresence.ts`                    | Deleted; the former `useMatrixClient`/`User`/`UserEvent` presence owner is absent                                                                        |
| `synara/src/app/features/matrix-presence/`                   | Owns native invoke/listen, DTO parsing, generation/user filtering, disposal, and unavailable-state mapping                                               |

The landed path does not leave a `useUserPresence` compatibility hook that
calls `matrix-js-sdk`, a hidden `UserEvent` listener, an
`isNative ? native : js` selector, or a legacy sentinel that lets the profile
continue with JS presence. Any future consumer discovered under this
capability must be split into an SDK-neutral helper or a separately owned
residual, and must not reintroduce a second presence backend.

The current docs PR itself must not modify any product file. In particular,
there must be no `product.rs` diff, no generated Tauri schema churn, and no
frontend implementation in this packet PR.

## 5. Focused evidence and remaining gaps

PR #458 adds focused Rust projection/index tests, native wire/source tests,
frontend owner tests, and a profile source-absence guard. These are local
implementation evidence at `d82e043d`; they do not prove an authenticated
desktop session or close this packet.

### Rust owner tests

The landed tests cover the presence index and basic native projection and
serialization. The remaining acceptance run must confirm the full native
owner matrix, including:

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

The landed contract/source tests confirm the exact command/event names,
camelCase wire fields, and privacy-safe serialization. The remaining review
must confirm:

- snapshot, subscription, update, unknown, and unavailable shapes;
- unknown/missing fields, wrong user/generation, invalid IDs, oversized
  status text, invalid timestamps, and media/secret-like fields;
- command argument/result names and exact event name
  `matrix-presence-updated`; and
- privacy-safe serialization of every failure class.

### Frontend owner and source-absence tests

The landed owner suite under
`synara/src/app/features/matrix-presence/__tests__/` uses injected
`invoke`/`listen` functions and asserts:

1. snapshot and subscription commands receive the exact user ID;
2. initial snapshot delivery and subsequent matching events update the
   profile;
3. other-user, other-subscription, and stale-generation events are not merged
   and fail closed to an unavailable result;
4. unmount/profile close unsubscribes and late events do not update state;
5. missing command, invoke rejection, malformed payload, failed subscription,
   and unavailable event produce no badge and no JS SDK call; and
6. a source guard proves the profile path contains no `matrix-js-sdk` import,
   `useMatrixClient` presence read, `UserEvent` presence listener, or direct
   `User` presence field access.

The complete lifecycle/error matrix remains an acceptance item: authenticated
session teardown, failed store reads, malformed native events, late-event
rejection, and all required generation/subscription cases must have retained
evidence at the merge tip. No local test result may be relabeled as live proof.

The source guard must be capability-scoped. It must not assert that unrelated
Matrix JS imports across the repository are already zero.

## 6. Authenticated live proof — Not confirmed at `d82e043d`

No authenticated two-client desktop proof or retained logs/screenshots are
recorded in this docs PR. Closure requires a two-client desktop proof against
the repository's [test-matrix-synapse-topology.md](test-matrix-synapse-topology.md)
topology, naming the exact native commands and event observed.

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
- [`Client::add_event_handler`](https://github.com/matrix-org/matrix-rust-sdk/blob/1c44fb66214667c6d00acaf72ab592493653708b/crates/matrix-sdk/src/client/mod.rs#L944-L979) establishes the client event-handler registration API used by the landed native owner.
- [`SyncResponse` presence projection](https://github.com/matrix-org/matrix-rust-sdk/blob/1c44fb66214667c6d00acaf72ab592493653708b/crates/matrix-sdk/src/sync.rs#L40-L60) establishes that the pinned sync path carries typed `PresenceEvent` updates; the dependency's content-field shape is API-shape evidence, not live product parity.

These sources establish SDK API shape only. They do not prove the Synara
desktop route's authenticated behavior; the remaining proof must still cover
the managed-client lifecycle, DTO boundary, and UI readback.

## 8. Ordered closure work

1. Reconfirm `d82e043db25e4ec786bde103c4d457a898ef664b`, the target branch,
   approved SDK pin, and the no-`product.rs` scope guard. Stop on any mismatch.
2. Rerun the focused unit/contract/source-absence checks and fill any missing
   lifecycle/error evidence without adding a fallback or second owner.
3. Run the authenticated two-client proof in Section 6. Retain exact
   command/event evidence and mark every required case.
4. Independently review the resulting evidence and mark V-PRESENCE.USER
   accepted only if every item in Section 9 is closed.
5. Keep the docs PR scoped to documentation; do not alter `main`, #39,
   V-BURN, `dual_backend`, unrelated verticals, or generated product files.

## 9. Acceptance statement

V-PRESENCE.USER is **not accepted at `d82e043d`**. The first slice supplies
the native route and deletes the former JavaScript presence owner, but the
residual remains open until all of the following are evidenced:

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
