# R-DEVTOOL — native developer-tools implementation packet

| Field    | Value                                                                                                                                              |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status   | **Implementation packet** — this packet is docs-only; it does not claim the vertical is implemented                                                |
| Residual | **V-SEND.R-DEVTOOL**                                                                                                                               |
| Priority | Low; user-reordered finish-line item                                                                                                               |
| Base     | `feature/matrix-rust-sdk-full-replacement` at `1c9653b25ac3dd97c91f57de8eec3f0fb9586a65`                                                           |
| PR shape | Focused **draft** PR targeting `feature/matrix-rust-sdk-full-replacement`                                                                          |
| Gate     | Start only after V-TIMELINE.C3–C5 have confirmed live proofs                                                                                       |
| Policy   | [full-vertical-policy.md](full-vertical-policy.md): complete UI → Tauri IPC → live `matrix-sdk` vertical, with JS-owner deletion in the same slice |
| Guard    | Never `main`, umbrella PR **#39**, or V-BURN/#327; `dual_backend` is forbidden                                                                     |

The source of truth for the residual is
[v-send-devtool-inventory.md](v-send-devtool-inventory.md). This packet turns
that inventory into a bounded implementation contract. It is not a V-BURN
completion claim.

## 1. Objective and completion bar

Replace the common room developer-tools surface with one native owner:

```text
Developer Tools UI → Tauri IPC → managed live matrix-sdk client → homeserver
```

The completed slice must preserve the intentionally raw developer surface:

- inspect room state, including arbitrary event types and state keys;
- inspect and write room account data;
- inspect a raw timeline event and send arbitrary timeline events;
- inspect, create, and update arbitrary state events;
- preserve the current state-event permission decision and user-visible error
  behavior without sending `MatrixError` objects over IPC; and
- refresh the UI from native readbacks and live native subscriptions.

The native desktop session is the only supported product route for this slice.
There is no JS fallback, runtime backend selector, `isNative ? rust : js`
branch, or second Matrix client. A native failure is visible as an unavailable
developer-tools surface, not as a successful or silently redirected operation.

## 2. Scope and prerequisites

### In scope

The inventory's common room feature and its SDK-bound hooks:

- `synara/src/app/features/common-settings/developer-tools/DevelopTools.tsx`
- `synara/src/app/features/common-settings/developer-tools/SendRoomEvent.tsx`
- `synara/src/app/features/common-settings/developer-tools/StateEventEditor.tsx`
- `synara/src/app/hooks/useRoomState.ts`
- `synara/src/app/hooks/useRoomAccountData.ts`
- the shared `AccountDataEditor` error type, if needed to make the retained
  editor SDK-neutral.

The Rust owner, Tauri registration/capability wiring, Synara-owned DTOs, native
frontend owner, focused tests, and authenticated live proof are all part of
the same vertical. Do not split native wiring from deletion and call the
vertical done in between.

### Out of scope

- `synara/src/app/features/settings/developer-tools/**`, which is the separate
  global account-data/access-token/session-status surface and is not in the
  inventory's residual scope;
- `useStateEvent.ts` and `useStateEventCallback.ts`, which have consumers
  outside developer tools and must not be deleted as incidental cleanup;
- unrelated room settings, timeline, crypto, media, or send residuals;
- V-BURN preparation or completion; and
- any change to `main` or PR #39.

Before implementation, the writer must verify:

1. the actual `HEAD` is the base SHA in this packet and the PR target is the
   integration branch named above;
2. V-TIMELINE.C3, C4, and C5 have accepted live proof artifacts;
3. the managed native session exposes one live `matrix-sdk` client for the
   current session; and
4. no concurrent native/JS Matrix client is started for this session.

A failed prerequisite blocks implementation. It must not be worked around by
adding a fallback or by changing the gate.

## 3. Frozen native IPC contract

These are the exact command names for this slice. Do not add aliases or rename
them during implementation. `matrix_session_snapshot` is the existing session
preflight command and is not a new R-DEVTOOL command.

| Exact command                               | Request                                                   | Required result                                                                                                                                       |
| ------------------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `matrix_devtool_room_state_snapshot`        | `{ roomId }`                                              | Session-generation-stamped `RoomStateSnapshot` containing all current room state except `m.room.member`, matching `useRoomState`'s existing exclusion |
| `matrix_devtool_room_account_data_snapshot` | `{ roomId }`                                              | Session-generation-stamped `RoomAccountDataSnapshot` containing `{ type, content }` entries                                                           |
| `matrix_devtool_state_event_read`           | `{ roomId, eventType, stateKey }`                         | The matching raw `StateEventDto`, or an explicit absent result; includes the native permission result needed by the editor                            |
| `matrix_devtool_timeline_event_read`        | `{ roomId, eventId }`                                     | The matching raw `TimelineEventDto`, or an explicit absent result                                                                                     |
| `matrix_devtool_set_room_account_data`      | `{ roomId, eventType, content }`                          | Native write followed by a typed account-data readback for the same room and type                                                                     |
| `matrix_devtool_send_state_event`           | `{ roomId, eventType, stateKey, content }`                | Native state-event send followed by a typed event readback/ack                                                                                        |
| `matrix_devtool_send_event`                 | `{ roomId, eventType, content }`                          | Native timeline-event send followed by a typed event readback/ack                                                                                     |
| `matrix_devtool_subscribe`                  | `{ roomId, topics: ['room_state', 'room_account_data'] }` | A native `subscriptionId`; subscriptions are scoped to the room and session generation                                                                |
| `matrix_devtool_unsubscribe`                | `{ subscriptionId }`                                      | Successful resource release or a safe unavailable error; it must be idempotent for the current generation                                             |

The frontend listens to these exact native event names for the topics returned
by `matrix_devtool_subscribe`:

- `matrix-devtool-room-state-updated`
- `matrix-devtool-room-account-data-updated`

Each event carries `subscriptionId`, `roomId`, `sessionGeneration`, and the
updated typed DTO/readback. Events for another room, subscription, or session
generation are ignored and cause a resnapshot or an unavailable state; they
must never be merged into the current room by guesswork.

### DTO and authority rules

- Use Synara-owned serializable DTOs, never `Room`, `MatrixEvent`,
  `MatrixClient`, Ruma event objects, or SDK error objects in the UI boundary.
- Preserve the raw JSON content needed by the existing editors. Do not
  normalize arbitrary developer-tool content into a product-specific allowlist.
- Native Rust reads and permission checks must use the managed live
  `matrix-sdk` client. Do not implement a raw-HTTP substitute or create a
  second client.
- `matrix_devtool_send_state_event` is the only owner of custom state writes;
  `matrix_devtool_send_event` is the only owner of custom timeline writes;
  `matrix_devtool_set_room_account_data` is the only owner of room account-data
  writes.
- Reuse the repository's privacy-safe Matrix IPC error categories and
  diagnostic IDs. Errors must not contain access/refresh tokens, passwords,
  recovery material, crypto keys, ciphertext, or arbitrary event content in
  logs.
- A successful write is not complete until its native readback/ack validates
  the expected room and session generation.

### Permission and error semantics

The current `StateEventEditor` determines editability with
`useRoomPermissions(...).stateEvent(type, mx.getSafeUserId())`. The native
state-event read path must compute the equivalent result from the live native
room state and return a Synara-owned `canEdit` value. If permission state is
missing, stale, or unreadable, the UI must hide/disable editing and show the
native unavailable state.

Native Matrix failures map to the existing safe UI categories. In particular,
forbidden writes remain a visible permission error, connectivity/session
failures remain unavailable/retryable errors, and malformed JSON remains a
local editor validation error. Do not stringify an SDK error as an IPC payload
and do not convert a failed native write into a successful close.

## 4. Physical deletion list

The following JS ownership must be removed in this same vertical. “Rewire”
means the file may retain SDK-neutral presentation code, but the listed live
client owner/import/call is physically absent when the slice is accepted.

| Path                                                                           | Delete from the path                                                                                                               | Retain or replace with                                                                                                                       |
| ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `synara/src/app/features/common-settings/developer-tools/DevelopTools.tsx`     | `useMatrixClient`, the SDK `Room` dependency, `useRoomState`, `useRoomAccountData`, and `mx.setRoomAccountData`                    | Native room identity, native snapshots/subscriptions, and the existing UI shape                                                              |
| `synara/src/app/features/common-settings/developer-tools/SendRoomEvent.tsx`    | `MatrixError` import, `useMatrixClient`, SDK `Room`, `mx.sendStateEvent`, and `mx.sendEvent`                                       | Native owner calls to the three frozen DTO/IPC write/readback routes                                                                         |
| `synara/src/app/features/common-settings/developer-tools/StateEventEditor.tsx` | `MatrixError` import, `useStateEvent` for this surface, `useMatrixClient`, SDK `Room`, `mx.getSafeUserId`, and `mx.sendStateEvent` | Native state-event readback, native `canEdit`, and native state-event write/readback                                                         |
| `synara/src/app/hooks/useRoomState.ts`                                         | The entire hook, after confirming the inventory's developer-tools import is removed and no other consumer exists                   | Native developer-tools state snapshot/subscription owner                                                                                     |
| `synara/src/app/hooks/useRoomAccountData.ts`                                   | The entire hook, after confirming no other consumer exists                                                                         | Native developer-tools account-data snapshot/subscription owner                                                                              |
| `synara/src/app/components/AccountDataEditor.tsx`                              | Its `matrix-js-sdk`-specific error type import, if still present                                                                   | An SDK-neutral `unknown`/safe-error presentation type; the shared editor may remain because the separate global settings surface consumes it |

The following are explicit negative requirements:

- Do not delete `useStateEvent.ts` or `useStateEventCallback.ts`; they have
  unrelated consumers. The common developer-tools feature must no longer
  import them for its native state read path.
- Do not keep a `Legacy*` developer-tools component, a hidden JS callback, or a
  `nativeAvailable` selector that can route a native desktop operation to the
  JS client.
- Do not delete the separate global settings developer-tools surface as part
  of this packet.
- Delete any test, fixture, type, or helper introduced solely to preserve the
  removed common developer-tools JS owner. Retain tests for SDK-neutral UI and
  native behavior.

The implementation PR must include a negative source scan over the common
developer-tools tree proving absence of `matrix-js-sdk`, `useMatrixClient`,
`mx.`, `sendEvent`, `sendStateEvent`, `setRoomAccountData`,
`useRoomState`, and `useRoomAccountData` there. The repository-wide import
count may remain nonzero because other verticals are still open; that is not a
reason to retain this residual's owner.

## 5. Required focused tests

Add and retain the following focused evidence. Test names may be expanded, but
the paths and cases below are required.

### Frontend owner tests

`synara/src/app/features/common-settings/developer-tools/__tests__/nativeDeveloperToolsOwner.test.ts`

The test must use an injected invoke/listen harness and assert exact command
names and arguments for:

1. logged-in native session preflight, room state snapshot, room account-data
   snapshot, selected state-event read, and selected timeline-event read;
2. account-data, state-event, and timeline-event writes, including the typed
   native readback/ack before the UI reports success;
3. subscription creation, room/session-generation filtering, update delivery,
   unsubscription on close/unmount, and no late update after disposal;
4. native permission `canEdit: false` hiding/disabling the state editor;
5. missing command, unavailable result, invoke rejection, missing readback,
   stale session generation, and failed subscription each becoming a visible
   unavailable/error result; and
6. every native failure making zero calls to any legacy JS writer. The owner
   must reject rather than return a `legacy` sentinel.

Add a source-absence test alongside it if the owner tests do not already cover
the negative scan, for example:

`synara/src/app/features/common-settings/developer-tools/__tests__/developerToolsSourceGuard.test.ts`

The source guard must cover the three runtime files and assert that the
SDK-bound hooks deleted by this packet no longer exist or are no longer
imported by the feature. It must not assert that unrelated repository-wide
Matrix JS imports have already reached zero.

### DTO/IPC contract tests

Extend the existing targeted contract suites:

- `synara/src/app/features/matrix-dto/__tests__/matrixDto.test.ts` for the
  state, room-account-data, event, permission, readback, and generation DTOs;
- `synara/src/app/features/matrix-ipc/__tests__/matrixIpcContract.test.ts` for
  command argument/result shapes, unknown or missing fields, stale generation,
  bounded raw JSON, and privacy-safe error serialization; and
- `src-tauri/src/matrix/ipc/contract_tests.rs` for the same wire-level
  rejection/round-trip cases where the shared IPC envelope is used.

### Rust owner tests

Place focused unit tests with the new native owner, preferably under
`src-tauri/src/matrix/developer_tools/tests.rs` (or the module's existing test
module if the implementation is kept beside another live owner). Cover:

- room/member filtering parity with `useRoomState`;
- arbitrary event type/state-key/content preservation;
- room and session-generation authority checks;
- permission calculation parity and forbidden-write mapping;
- subscription filtering and resource release;
- missing client/session, failed SDK read/write, and failed readback as
  fail-closed errors; and
- absence of secrets or raw event content in diagnostic/error strings.

Do not run a full workspace build merely to author or review this packet. The
implementation PR should run only the affected frontend tests, the affected
Rust module tests, the IPC/DTO contract tests, and the existing relevant
guardrail checks. Full cargo builds and `npm ci` are not required unless a
targeted failure makes one essential.

## 6. Authenticated live proof

After C3–C5 are accepted, record a focused two-client Synapse proof using the
repository's [test-matrix-synapse-topology.md](test-matrix-synapse-topology.md)
topology. The proof is required for acceptance and must name the exact native
commands observed.

1. Open Developer Tools for a known room. Confirm
   `matrix_session_snapshot` and the two snapshot commands return the expected
   native state/account-data readbacks.
2. Change room account data. Confirm
   `matrix_devtool_set_room_account_data` is the only write, its readback is
   reflected in the UI, and the second client observes the update.
3. Edit an allowed state event. Confirm the native permission result enables
   the editor, `matrix_devtool_send_state_event` writes it, and the second
   client observes the exact raw content.
4. Attempt a forbidden state write. Confirm the UI shows the permission error,
   no JS writer runs, and the editor does not report success.
5. Send an arbitrary timeline event and read it back by event ID. Confirm
   `matrix_devtool_send_event` and `matrix_devtool_timeline_event_read` use the
   live native client and preserve the raw event content.
6. Mutate the room from the second client. Confirm the two exact native update
   events refresh the first client's room state/account-data view without a
   JS room listener.
7. Interrupt or end the native session. Confirm a missing/failed command,
   readback, subscription, or live client disables the surface and never falls
   through to `matrix-js-sdk`.

The proof must be marked **not run**, **failed**, or **passed** with evidence;
absence of a live proof is not acceptance. This packet does not claim C3–C5 or
R-DEVTOOL live proof complete.

## 7. Ordered implementation work

1. Reconfirm the exact base SHA, branch target, C3–C5 gate, and the managed
   native session owner. Stop on any mismatch.
2. Define and test Synara-owned DTOs and the exact IPC names in Section 3.
   Register only those commands with Tauri and the desktop capability surface.
3. Implement the Rust owner against the managed live `matrix-sdk` client,
   including readbacks, room/session-generation checks, native subscriptions,
   permission calculation, safe errors, and teardown.
4. Implement the SDK-neutral frontend owner and wire `DevelopTools.tsx`,
   `SendRoomEvent.tsx`, and `StateEventEditor.tsx` to the native commands and
   subscriptions.
5. Delete the JS owners and the two developer-tools-only hooks listed in
   Section 4. Remove SDK-only error types from shared UI where required.
6. Run the focused frontend, DTO/IPC, Rust, source-absence, and guardrail tests.
   Then run the authenticated live proof and record its evidence.
7. Review the diff for accidental changes to `main`, #39, V-BURN status,
   unrelated verticals, or any `dual_backend`/fallback mechanism. Keep the PR
   draft and focused.

## 8. Acceptance statement

R-DEVTOOL may be marked complete only when all of the following are true:

- the common room developer-tools UI reaches the live Matrix service only via
  the exact native IPC contract in Section 3;
- room-state/account-data reads and subscriptions, raw state/timeline reads,
  and all three write classes have native owners and typed readbacks;
- current permission and safe error semantics are preserved;
- the Section 4 JS owners/imports/hooks and JS-only tests/types are physically
  deleted or neutralized in this slice;
- focused tests and the authenticated live proof pass;
- no native-session failure can invoke a JS writer or silently claim success;
- no tokens, keys, recovery material, ciphertext, or event content is emitted
  in logs/errors; and
- the residual ledger names no remaining R-DEVTOOL item. Any unrelated
  residual must have its own vertical ID and must not be relabeled as
  “deferred” R-DEVTOOL work.

This completion statement applies only to R-DEVTOOL. It must not be used to
claim V-BURN/#327 complete, to reopen #39, or to change the C3–C5 gate.
