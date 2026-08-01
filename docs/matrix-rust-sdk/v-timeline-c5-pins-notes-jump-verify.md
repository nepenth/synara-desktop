# V-TIMELINE.C5 — pins / notes / jump live proof after cutover

| Field        | Value                                                                                                                                                                                                                                                                                                                             |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status       | Docs-only verification checklist — **no product code**                                                                                                                                                                                                                                                                            |
| Scope        | `synara/src/app/features/room/NativeTimelinePresenter.tsx` (pin/unpin, jump-to-latest, Later save), `nativeTimelineAction.ts` (`pinWithNativeTimelineAction`, `unpinWithNativeTimelineAction`), `nativeLaterOwner.ts` (`upsertLaterWithNativeOwner`, `createLaterItemFromIds`), `nativeRoomNotesOwner.ts` (`matrix_room_notes_*`) |
| Precondition | C1 (#285) selects `NativeTimelinePresenter` in `RoomView`; C2 (#289) deletes `RoomTimeline` + dead JS timeline path; C3 (#294) stream/delta checklist exists; C4 media/render checklist exists                                                                                                                                    |
| Policy       | [full-vertical-policy.md](full-vertical-policy.md); [cutover-operating-model.md](cutover-operating-model.md); dual_backend **false**; **never touch #39**                                                                                                                                                                         |
| Related      | [v-timeline-cutover-residual.md](v-timeline-cutover-residual.md) (C5 row), [v-timeline-full-replacement-contract.md](v-timeline-full-replacement-contract.md), [v-timeline-c3-stream-verify.md](v-timeline-c3-stream-verify.md)                                                                                                   |

## 1. What C5 must prove

C5 is the **live authenticated proof** for pin/unpin, Later/notes, and
jump-to-latest on the selected `NativeTimelinePresenter`. It is a
**verification gate**, not a new implementation. The residual map records
"wired on the selected presenter; live authenticated proof unclaimed"; C5
closes that unclaimed live-proof half of the row.

The binding contract (from `NativeTimelinePresenter.tsx`, `nativeTimelineAction.ts`,
`nativeLaterOwner.ts`, `nativeRoomNotesOwner.ts`, and the contract doc) that must
be proven end-to-end on the selected path:

| #   | Contract point                                                                              | Implementation                                                                                                                                                           | Proof target                                                                                                         |
| --- | ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------- |
| P1  | Pin/unpin is **capability-gated** and driven by projected `pinnedEventIds`                  | `NativeTimelineRowActions` gates Pin vs Unpin via `selectNativeTimelinePinAction(Boolean(pinned))`; `pinned` from `isNativeTimelineEventPinned(pinnedEventIds, eventId)` | Pin affordance appears only when `capabilities.pin`; label flips Pin↔Unpin from the projected pin list               |
| P2  | Pin/unpin routes through native owners, never JS `sendStateEvent`                           | `pinWithNativeTimelineAction` / `unpinWithNativeTimelineAction` → `matrix_timeline_pin` / `matrix_timeline_unpin`                                                        | Pin and unpin mutate via native commands; no JS pin writer on the selected path                                      |
| P3  | Pin-list changes project live on the stream (metadata-only batch)                           | `applyNativeTimelineViewDelta` accepts `pinnedEventIds` metadata-only batches; `snapshot.pinnedEventIds` drives the "Pinned" badge                                       | A pin/unpin from another client updates the badge without a row op or revision gap                                   |
| P4  | Later save is a room-event affordance for any remote item with an id                        | `NativeTimelineRowActions` "Save for later" → `upsertLaterWithNativeOwner(createLaterItemFromIds(roomId, eventId, 'saved'))`                                             | Saving a row writes `in.synara.later` account data via `matrix_later_upsert`; no JS `setAccountData`                 |
| P5  | Later/notes read/write routes through native account-data owners                            | `nativeLaterOwner.ts` (`matrix_later_*`), `nativeRoomNotesOwner.ts` (`matrix_room_notes_*`)                                                                              | Later and room-notes snapshot/mutate commands return typed readbacks; JS `setAccountData` writers deleted            |
| P6  | Jump-to-latest is stream-addressed: closes prior stream, returns fresh live-bottom readback | `controller.jumpLatest()` → `matrix_timeline_jump_latest`; swaps `streamIdRef`/snapshot                                                                                  | Jump from a scrolled-up position returns a fresh live-bottom readback with a **new** `streamId`; prior stream closed |
| P7  | Jump affordance is position-gated                                                           | Presenter shows "Jump to latest" only when `snapshot.position.kind !== 'live_bottom'`                                                                                    | Button hidden at live bottom; shown when scrolled up / focused / restored / unread                                   |

## 2. Existing tests (already green on tip)

| Test file                                        | Covers                                                                                                                                      | C5 gap                                                                                                                |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `__tests__/nativeTimelineActions.test.ts`        | `pinWithNativeTimelineOwner` / `unpinWithNativeTimelineOwner` typed readbacks + off-desktop unavailability; `selectNativeTimelinePinAction` | **Unit-level only.** No live pin-list projection, no real `matrix_timeline_pin`/`unpin` IPC, no authenticated session |
| `__tests__/nativeTimelineViewDelta.test.ts`      | Pure `applyNativeTimelineViewDelta`: pin-list metadata-only batches, empty-batch rejection                                                  | Pure reducer; no live stream, no real pin projection                                                                  |
| `__tests__/nativeTimelineViewportPolicy.test.ts` | `shouldRestoreNativeTimelineViewport` TTL / unread / live-tail precedence                                                                   | Pure policy; no jump-to-latest stream lifecycle                                                                       |

**What the unit tests do NOT prove** (the C5 live gap): live pin/unpin
projection across clients, native pin/unpin mutation round-trip, Later save
writing `in.synara.later` account data, room-notes read/write round-trip,
jump-to-latest stream-addressed re-open with a new `streamId` and prior-stream
close, and the position-gated jump affordance — all require an authenticated
desktop session.

## 3. Operator preflight (runbook; no proof claim)

This section makes the C5 checklist executable for an operator. It does not
change the C5 status and does not claim that live proof has run. Docker and
harness readiness are prerequisites only; they are not pin, notes, Later, or
jump evidence.

### 3.1 Policy, tip, and toolchain gate

Run from the repository root before opening the desktop:

```sh
git status --short --branch
git branch --show-current
git rev-parse HEAD
git merge-base --is-ancestor c0d5ec4053511423b979d76d5586da0ed7643cf3 HEAD
node --version
npm --version
npm exec --yes --package=prettier@2.8.1 -- prettier --version
```

Continue only when all of the following are true:

- the proof is on `feature/matrix-rust-sdk-full-replacement` or a docs branch
  based on feature tip
  `c0d5ec4053511423b979d76d5586da0ed7643cf3`, never `main` or PR #39. The
  `git merge-base --is-ancestor` check must pass; if it fails, stop and record
  `Not confirmed`;
- record the exact output of `git rev-parse HEAD` as the **evidence head** in
  the proof log. Do not silently substitute a different SHA or an older run;
- the Prettier command reports `2.8.1`; and
- the worktree has no unrelated changes that could affect the desktop run.

`dual_backend` is forbidden. There is no backend selector to set. Do not use
the current-JS Synapse control (`SYNARA_RUN_SYNAPSE_INTEGRATION=1` or
`synara/scripts/run-synapse-two-client-integration.mjs`) as Rust live evidence.

### 3.2 Docker harness preflight

The disposable Synapse topology is Docker-only. The checked-in
`scripts/synapse-integration.sh` is the supported lifecycle entry point; it
calls `docker compose` (Compose v2), starts the pinned Synapse/PostgreSQL
containers, and waits for them to become ready. `npm run check:synapse-harness`
checks repository invariants without Docker; it does not start Synapse and is
not a substitute for this preflight.

Install and start the Docker runtime before attempting the C5 session:

- On macOS, install and launch [Docker Desktop](https://docs.docker.com/desktop/setup/install/).
- On Linux, install [Docker Engine](https://docs.docker.com/engine/install/)
  and the [Docker Compose plugin](https://docs.docker.com/compose/install/linux/),
  then start the Docker daemon for the operator account.

Verify the client, Compose v2, and daemon from the repository root:

```sh
command -v docker
docker --version
docker compose version
docker info >/dev/null
test -x scripts/synapse-integration.sh
bash -n scripts/synapse-integration.sh
```

`docker compose version` must report Compose v2 and `docker info` must exit
successfully. If `docker` is missing, Compose v2 is missing, the daemon is not
running, or the host cannot pull the pinned images, stop before account setup;
record the harness as `blocked` and keep C5 `Not confirmed`. Do not treat a
Docker command-not-found error as live proof failure and do not substitute the
current-JS integration runner.

### 3.3 Homeserver and desktop launch

The desktop has no `VITE_*` homeserver variable. The operator selects the
homeserver in the auth screen. For a disposable local Synapse, use the
checked-in harness only and keep all generated credentials process-local. Run
the lifecycle commands in this order; `up` creates the ignored runtime state,
and `status` must show both services before the desktop is opened:

```sh
SYNARA_PORT=18008 scripts/synapse-integration.sh reset
SYNARA_PORT=18008 scripts/synapse-integration.sh up
scripts/synapse-integration.sh status
scripts/synapse-integration.sh create-user  # primary desktop account
scripts/synapse-integration.sh create-user  # second-client account
```

The first `up` may pull the pinned images. Save only sanitized command results
(`up`/`status` exit state, service state, loopback port, and cleanup state),
never raw environment files, passwords, tokens, or full Docker paths.

The runtime file retains the selected port after `up`; use the value printed by
the harness as the authoritative homeserver port if it differs from the
example. The harness owns generated files under
`integration/synapse/runtime/`; do not edit or commit them.

The supported variable surface is:

| Variable                                               | Example                  | Meaning and evidence rule                                                                                     |
| ------------------------------------------------------ | ------------------------ | ------------------------------------------------------------------------------------------------------------- |
| `SYNARA_PORT`                                          | `18008`                  | Non-secret loopback port used by the disposable Synapse harness; the desktop URL is `http://127.0.0.1:18008`. |
| `SYNARA_MATRIX_HOMESERVER_URL`                         | `http://127.0.0.1:18008` | Only for explicitly gated Rust live-test commands; it is not the desktop launch input.                        |
| `SYNARA_RUN_MATRIX_RUST_AUTH_LIVE`                     | `1`                      | Only enables the Rust auth-test gate; it does not prove the selected desktop presenter.                       |
| `SYNARA_POSTGRES_PASSWORD`, `SYNARA_UID`, `SYNARA_GID` | generated                | Written to the ignored runtime file by the harness. Never set, print, commit, or paste their values.          |

Use the primary account in the Synara desktop. Create or select a disposable
room, invite the second account through the normal Matrix UI, and authenticate
the second account in a separate Matrix client. Do not paste passwords,
access/refresh tokens, Matrix user/room/event IDs, message bodies, or absolute
paths into the proof record. Always tear down the disposable service after the
attempt, including a failed attempt:

```sh
scripts/synapse-integration.sh reset
```

If `up`, `status`, account creation, desktop launch, or the proof itself fails,
run `reset` in a finally/cleanup step. A failed or skipped reset invalidates
the attempt’s live evidence; it does not justify a retry with a different
backend or a different base tip.

Launch the desktop from the repository root:

```sh
npm run tauri dev
```

In the desktop auth screen, enter `http://127.0.0.1:18008` in the
`Homeserver` field, enter the primary account’s localpart and password, then
click `Login`. The Tauri config starts the Vite dev server on localhost:8080;
that URL is the app shell, not the Matrix homeserver. After login, open
`Settings → Diagnostics`, enable `Diagnostic Capture` and `Room State and
Positioning` before opening the proof room, then return to the room.

### 3.4 C5 action and evidence mapping

The visible labels below are the selected `NativeTimelinePresenter` controls.
The UI is useful result corroboration; it does not by itself prove native
command ownership, account-data readbacks, exact stream replacement, or
absence of a JS fallback. Paste the sanitized native/diagnostic trace for
those claims.

| C5 step / contract | Operator action                                                                             | Required observation and evidence                                                                                                                                    |
| ------------------ | ------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1 / P1–P2          | From the row menu, pin a message, then unpin it.                                            | `matrix_timeline_pin` and `matrix_timeline_unpin` return typed readbacks; the badge appears and clears; no JS `sendStateEvent` writer or fallback is observed.       |
| 2 / P3             | From a second client, pin/unpin a message.                                                  | The selected presenter updates the badge from a metadata-only `pinnedEventIds` batch with no row op or revision gap.                                                 |
| 3 / P1             | Use a room where pinning is not permitted.                                                  | The Pin affordance is absent because `capabilities.pin` is false; do not accept a hidden-by-CSS or failed-command result as the capability gate.                     |
| 4 / P4–P5          | From a row menu, choose `Save for later`, then open the Later panel.                        | `matrix_later_upsert` writes `in.synara.later` through the native owner with the room/event identity; the Later item appears; no JS `setAccountData` writer is used. |
| 5 / P5             | Create, update, and delete a room note and a todo.                                          | `matrix_room_notes_*` snapshot/mutate commands return typed readbacks and the panel reflects each change; no JS account-data writer is observed.                     |
| 6 / P6–P7          | Scroll up or open a focused/unread position, then click `Jump to latest`.                   | The affordance is present off-tail, the old stream closes, and a fresh live-bottom readback adopts a new exact `streamId`; no old-stream delta is accepted.          |
| 7 / P7             | At live bottom, then after scrolling up, inspect the jump affordance.                       | The button is hidden at live bottom and shown when the position is not `live_bottom`; record the position/readback, not only the screenshot.                         |
| 8 / P6             | If available, inject a stale stream or revision-gap metadata batch with a dev-only harness. | The native hook errors or rejects the batch and never repairs via JS. If the harness is unavailable, record the step as `not run`; C5 remains `Not confirmed`.       |
| 9 / P6             | Navigate away from the room.                                                                | `matrix_timeline_close` fires with the exact `streamId`, async callbacks are disposed, and no late `setState` warning or fallback appears.                           |

### 3.5 Evidence to paste

Paste one sanitized block per attempt. Keep the verdict conservative; do not
replace `Not confirmed` with `Confirmed` from screenshots, unit tests, a
generated success label, Docker readiness, or a retry that lacks the complete
route chronology.

```text
proof: V-TIMELINE.C5
verdict: Not confirmed | Failed | Confirmed
base: feature/matrix-rust-sdk-full-replacement
base-tip: c0d5ec4053511423b979d76d5586da0ed7643cf3
head: <exact output of git rev-parse HEAD>
operator: <name or team alias>
platform: <macOS/Linux + desktop build or dev run>
docker: <client version, or missing>
docker-compose: <Compose v2 version, or missing>
docker-daemon: pass | fail | not run
harness-script: pass | fail
harness-up: pass | fail | blocked | not run
harness-status: pass | fail | not run
harness-reset: pass | fail | not run
harness-port: <loopback port only>
homeserver: loopback:<port>
room: <redacted room alias>
primary/second client: <redacted aliases and device labels>
launch: pass | fail
diagnostics: <export filename or approved internal artifact; no absolute path>
P1 capability-gated pin state: pass | fail | not observed
P2 native pin/unpin owner: pass | fail | not observed
P3 live pin-list projection: pass | fail | not observed
P4 Later native upsert: pass | fail | not observed
P5 native Later/notes readbacks: pass | fail | not observed
P6 exact jump stream replacement: pass | fail | not observed
P7 position-gated jump affordance: pass | fail | not observed
authoritative readback: <sanitized second-client/homeserver result>
native trace: <sanitized command order, revisions, stream aliases, and readbacks>
deviations: none | <exact deviation; any fallback/retry/manual correction is disqualifying>
cleanup: pass | fail
```

The Docker and harness fields prove only that the disposable test topology was
available and cleaned up; they do not prove C5 pin, notes, Later, or jump
parity. The native trace and authoritative second-client readback must cover
the required P1–P7 chronology. Use `blocked` for an environment preflight that
never reached the harness; use `Failed` only when the attempted C5 route
produced a disqualifying observation. In either case, do not mark C5
`Confirmed`.

The exported Diagnostics report is privacy-filtered corroboration. Review it
before sharing; it intentionally excludes secrets, server URLs, and Matrix
identifiers. If the available report cannot establish a required native
readback, exact stream swap, capability gate, or no-fallback fact, write
`not observed` and keep the verdict `Not confirmed`.

## 4. Suggested live proof steps (authenticated desktop, after C1/C2/C3/C4)

Run against the sole desktop user on a real homeserver (Synapse topology per
[test-matrix-synapse-topology.md](test-matrix-synapse-topology.md)). Record each
step with a timestamp + observed state.

1. **Pin.** From the row menu, pin a message. Assert `matrix_timeline_pin` fires
   (no JS `sendStateEvent`), the "Pinned" badge appears, and the pin persists
   across a room re-open.
2. **Unpin.** Unpin the same message. Assert `matrix_timeline_unpin` fires, the
   badge clears, and the pin list updates.
3. **Live pin projection.** From a second client, pin/unpin a message. Assert the
   badge updates via a metadata-only `pinnedEventIds` batch (no row op, no
   revision gap) on the selected presenter.
4. **Pin capability gate.** In a room where pinning is not permitted, assert the
   Pin affordance is absent (gated by `capabilities.pin`).
5. **Later save.** From the row menu, "Save for later". Assert
   `matrix_later_upsert` writes `in.synara.later` account data with the
   `{roomId, eventId}` id; the item appears in the Later panel.
6. **Room notes.** Create/update/delete a room note and a todo. Assert
   `matrix_room_notes_*` snapshot/mutate commands return typed readbacks and the
   panel reflects the change; no JS `setAccountData` writer is used.
7. **Jump-to-latest.** Scroll up (or open focused/unread), then jump. Assert a
   fresh live-bottom readback with a **new** `streamId` and the prior stream
   closed (no further deltas on the old id).
8. **Jump affordance gate.** At live bottom, assert the "Jump to latest" button
   is hidden; after scrolling up / focused open, assert it appears.
9. **Unmount close.** Navigate away from the room. Assert `matrix_timeline_close`
   fires with the exact `streamId` and no late `setState` warnings.

## 5. Fail-closed rules (non-negotiable)

- **No JS fallback.** Pin/unpin, Later/notes, and jump-to-latest must route
  through native owners — never a JS `sendStateEvent`, `setAccountData`, or JS
  timeline fetch, never a dual-backend flag.
- **Capability-gated affordances.** Pin and jump appear only when the native
  capability/position gate allows; no un-gated affordance.
- **Exact stream only.** Jump-to-latest swaps to the exact returned `streamId`;
  the prior stream is closed and never receives further deltas.
- **Monotonic revision.** A pin-list metadata batch with a revision gap is
  rejected; the hook errors rather than guessing.
- **Close on unmount.** Every opened stream is closed with its exact `streamId`;
  `disposed` guards all async callbacks.
- **No product code in this PR.** This doc only; C5 verification is a live
  proof gate, not a code change.

## 6. Done when

- C1 (#285), C2 (#289), C3 (#294), and C4 are merged and `NativeTimelinePresenter`
  is the sole active timeline owner.
- Steps 1–9 above pass on an authenticated desktop session.
- Pin/unpin, Later/notes, and jump-to-latest all route through native owners with
  no JS fallback reachable on the selected path.
- The C5 row in [v-timeline-cutover-residual.md](v-timeline-cutover-residual.md)
  is updated to "verified" with the live-proof evidence recorded.

## 7. Self-eval confidence

- **High** on the binding contract points (P1–P7) — read directly from
  `NativeTimelinePresenter.tsx`, `nativeTimelineAction.ts`, `nativeLaterOwner.ts`,
  `nativeRoomNotesOwner.ts`, and the contract doc; unit tests already cover the
  pure pin/later/notes owners and the pin-list delta reducer.
- **Medium** on live proof — steps require an authenticated session and the
  C1/C2/C3/C4 cutover to be merged first; this doc frames the gate, it does not
  claim the proof.
