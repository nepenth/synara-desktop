# V-TIMELINE.C3 — stream/delta re-verify after cutover

| Field        | Value                                                                                                                                                            |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status       | Docs-only verification checklist — **no product code**                                                                                                           |
| Live proof   | **Not confirmed** — #446 product-command fan-out and #448 scoreboard refresh are not C3–C5 proof; no authenticated desktop evidence is recorded for this tip     |
| Scope        | `synara/src/app/features/room/nativeTimelineView.ts` (`useNativeTimelineView`, `applyNativeTimelineViewDelta`), `NativeTimelinePresenter.tsx` stream consumption |
| Precondition | C1 (#285) selects `NativeTimelinePresenter` in `RoomView`; C2 (#289) deletes `RoomTimeline` + dead JS timeline path                                              |
| Policy       | [full-vertical-policy.md](full-vertical-policy.md); [cutover-operating-model.md](cutover-operating-model.md); dual_backend **false**; **never touch #39**        |
| Related      | [v-timeline-cutover-residual.md](v-timeline-cutover-residual.md) (C3 row), [v-timeline-full-replacement-contract.md](v-timeline-full-replacement-contract.md)    |

## 1. What C3 must prove

C3 is the **live authenticated viewport proof** that the native stream/delta
binding — already implemented on the unselected presenter — stays correct once
`NativeTimelinePresenter` is the sole active timeline owner. It is a
**re-verify gate**, not a new implementation. The residual map records "no gap
found at tip"; C3 closes the unclaimed live-proof half of that row.

The binding contract (from `nativeTimelineView.ts` + contract doc) that must be
proven end-to-end on the selected path:

| #   | Contract point                                                                                                          | Implementation                                                                                                                         | Proof target                                                         |
| --- | ----------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| S1  | Register the `matrix-timeline-view-updated` listener **before** `matrix_timeline_open`                                  | `listen(...)` runs before `invokeDesktopWithAvailability('matrix_timeline_open', …)` in `open()`                                       | No delta is missed between subscribe and open readback               |
| S2  | Keep only the **exact** `streamId` returned by the open readback                                                        | `streamIdRef.current = readback.streamId`; batches with a different `streamId` are dropped                                             | No cross-stream / stale-stream rows leak in                          |
| S3  | Reject **revision gaps** and **malformed ops** instead of repairing via JS                                              | `applyNativeTimelineViewDelta` returns `undefined` on schema/session/room/revision mismatch or invalid op; hook sets `status: 'error'` | A gap or bad op fails closed (error state), never a guessed render   |
| S4  | Abort with the session timeline registry on unmount / close                                                             | cleanup calls `matrix_timeline_close` with the exact `streamId`; `disposed` guards async callbacks                                     | No orphaned stream, no late setState after unmount                   |
| S5  | Buffer early batches and replay them against the open readback                                                          | `earlyBatchesRef` collects pre-open batches; replayed after `streamId` is known                                                        | No lost live rows during the open race                               |
| S6  | Metadata-only batches (readState / pagination / pinnedEventIds) project live frontier, pagination, and pin-list signals | `applyNativeTimelineViewDelta` accepts metadata-only batches; empty+no-metadata rejected                                               | Live read-frontier / pin / pagination updates render without row ops |
| S7  | Jump-to-latest is stream-addressed: closes prior stream, returns fresh live-bottom readback                             | `jumpLatest()` invokes `matrix_timeline_jump_latest`, swaps `streamIdRef`/snapshot                                                     | Live-bottom re-open after jump; old stream closed                    |

## 2. Existing tests (already green on tip)

| Test file                                        | Covers                                                                                                                                                          | C3 gap                                                                                                                  |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `__tests__/nativeTimelineViewDelta.test.ts`      | Pure `applyNativeTimelineViewDelta`: metadata-only read-frontier, pagination + pin-list metadata, empty-batch rejection, pin/forward/thread/format pure helpers | **Unit-level only.** No live stream, no open/close lifecycle, no real `matrix_timeline_*` IPC, no authenticated session |
| `__tests__/nativeTimelineViewportPolicy.test.ts` | `shouldRestoreNativeTimelineViewport` TTL / unread / live-tail precedence                                                                                       | Pure policy; no stream                                                                                                  |
| `__tests__/nativeTimelineActions.test.ts`        | Native action owners (edit/redact/forward/report/pin/poll/call) typed readbacks + off-desktop unavailability                                                    | Action owners, not the view stream                                                                                      |

**What the unit tests do NOT prove** (the C3 live gap): listener-before-open
ordering, exact-streamId retention against a real registry, revision-gap
fail-closed under live load, early-batch replay, unmount close/abort, and
metadata-only live projection — all require an authenticated desktop session.

## 3. Operator preflight (runbook; no proof claim)

This section makes the C3 checklist executable for an operator. It does not
change the C3 status and does not claim that live proof has run. Use the same
preflight for C4/C5; their short UI mapping is at the end of this section.

### 3.1 Policy, tip, and toolchain gate

Run from the repository root before opening the desktop:

```sh
git status --short --branch
git branch --show-current
git rev-parse HEAD
git merge-base --is-ancestor 095dadb9a6c129a56cedd3e5d4346df4d3d702d4 HEAD
node --version
npm --version
npm exec --yes --package=prettier@2.8.1 -- prettier --version
```

Continue only when all of the following are true:

- the proof is on `feature/matrix-rust-sdk-full-replacement` or a docs branch
  whose checked-out history includes current feature tip
  `095dadb9a6c129a56cedd3e5d4346df4d3d702d4`, never `main` or PR #39. The
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

Install and start the Docker runtime before attempting the C3 session:

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
record the harness as `blocked` and keep C3 `Not confirmed`. Do not treat a
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

### 3.4 UI action to checklist mapping

The visible labels below are the selected `NativeTimelinePresenter` controls.
The UI is useful result corroboration; it does not by itself prove listener
ordering, exact stream identity, contiguous revisions, or absence of a JS
fallback. Paste the sanitized native/diagnostic trace for those claims.

| C3 step / contract | Operator action                                                                                                                                                 | Required observation and evidence                                                                                                                                                                                              |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1 / S1–S2          | Log in, select the prepared room, and wait for `Opening native timeline…` to become rows.                                                                       | `ready` state, a live-bottom or explicitly selected position, and a stable open/readback record. Paste the timestamped `room-timeline.open`/render evidence if present; do not infer listener-before-open from the screenshot. |
| 2 / S1, S2, S5     | Have the second client send one uniquely labelled text event while the desktop is at the live bottom.                                                           | The new row appears once without a manual refresh. Paste the second-client authoritative readback plus the native trace showing the same stream and the next revision. A row screenshot alone does not prove delta delivery.   |
| 3 / S3             | Use the second client to edit, react to, and redact a prepared event, or use the row’s `React`, `Edit`, and `Redact` buttons when their capabilities are shown. | The row changes in place or is removed, and the trace shows contiguous revisions. Any native error state, guessed repair, or JS timeline request is a failure.                                                                 |
| 4 / S3, S6         | Click `Load older messages` until backward pagination is exhausted, then click `Load newer messages` as needed to return to live.                               | `backward: exhausted` and accepted pagination readbacks with monotonic revisions. Paste button actions, timestamps, and the sanitized pagination trace.                                                                        |
| 5 / S6             | Click `Mark read`/`Mark unread`; use a row’s `Pin`/`Unpin` action if the capability is available.                                                               | Read-frontier and pin metadata update without a row-op requirement. Paste the UI result and metadata-only trace; missing trace keeps S6 `Not confirmed`.                                                                       |
| 6 / S7             | Scroll away from the live tail, then click `Jump to latest`.                                                                                                    | The button is shown off-tail, the view returns to live bottom, and the trace proves the old stream closed and the new exact stream was adopted.                                                                                |
| 7 / S3             | Only with an existing dev-only harness, inject one stale-stream or malformed/revision-gap batch; then reopen the room.                                          | The presenter shows native error state and never fetches a JS timeline. If the harness is unavailable, record the step as not run; C3 remains `Not confirmed`. Do not create product code for this PR.                         |
| 8 / S4             | Navigate to another room from the room list, then return or close the desktop.                                                                                  | The exact stream is closed and no late-update warning or fallback appears. Paste the close/unmount trace and any diagnostic report.                                                                                            |

For C4, reuse the same launch and evidence rules: the UI actions are sending
plaintext/encrypted image, audio, video, file, and sticker events from the
second client; using the rendered image/audio/video controls or file download;
using the row’s `Forward`; and navigating away for close. The expected evidence
is in C4 steps 1–7 and 9: `synara-media` handle URLs, bounded rendering,
decryption/playback/download, native forward, and close. C4 remains
`Not confirmed` unless its own checklist is fully run.

For C5, the UI actions are the row’s `Pin`, `Unpin`, and `Save for later`
buttons; the room-header `Pinned Messages` and `Personal Notes` controls; and
the timeline `Jump to latest` button. Map them to C5 steps 1–9 and paste the
native readbacks for `matrix_timeline_pin`/`unpin`, `matrix_later_*`,
`matrix_room_notes_*`, and the stream swap. C5 also remains `Not confirmed`
unless its own checklist is fully run.

### 3.5 Evidence to paste

Paste one sanitized block per attempt. Keep the verdict conservative; do not
replace `Not confirmed` with `Confirmed` from screenshots, unit tests, a
generated success label, or a retry that lacks the complete route chronology.

```text
proof: V-TIMELINE.C3
verdict: Not confirmed | Failed | Confirmed
base: feature/matrix-rust-sdk-full-replacement
base-tip: 095dadb9a6c129a56cedd3e5d4346df4d3d702d4
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
S1 listener-before-open: pass | fail | not observed
S2 exact stream: pass | fail | not observed
S3 fail-closed revision/schema/room checks: pass | fail | not run
S4 close on unmount: pass | fail | not observed
S5 early-batch replay: pass | fail | not run
S6 read/pagination/pin metadata: pass | fail | not observed
S7 jump-to-latest stream swap: pass | fail | not observed
authoritative readback: <sanitized second-client/homeserver result>
native trace: <sanitized ordered events, revisions, and stream aliases>
deviations: none | <exact deviation; any fallback/retry/manual correction is disqualifying>
cleanup: pass | fail
```

The Docker and harness fields prove only that the disposable test topology was
available and cleaned up; they do not prove the native timeline route. The
native trace and authoritative second-client readback must still cover the
required S1–S7 chronology. Use `blocked` for an environment preflight that
never reached the harness; use `Failed` only when the attempted C3 route
produced a disqualifying observation. In either case, do not mark C3
`Confirmed`.

The exported Diagnostics report is privacy-filtered corroboration. Review it
before sharing; it intentionally excludes secrets, server URLs, and Matrix
identifiers. If the available report cannot establish a required ordering or
exact stream/revision fact, write `not observed` and keep the verdict
`Not confirmed`.

## 4. Suggested live proof steps (authenticated desktop, after C1/C2)

Run against the sole desktop user on a real homeserver (Synapse topology per
[test-matrix-synapse-topology.md](test-matrix-synapse-topology.md)). Record each
step with a timestamp + observed state.

1. **Open live-bottom.** Log in, open a room with existing history. Assert
   `status: 'ready'`, `selectedPosition.kind === 'live_bottom'`, snapshot rows
   render, and `streamId` is stable across re-renders.
2. **Live append.** From a second client, send a text message. Assert the row
   appears via a delta `append`/`push_back` (not a full re-fetch) and `revision`
   increments by exactly 1.
3. **Live edit / reaction / redact.** Send an edit, a reaction, and a redact
   from the second client. Assert `set`/`remove` ops apply in place and the
   `revision` chain stays contiguous.
4. **Pagination.** Scroll backward until `backward: 'exhausted'`; scroll forward
   back to live. Assert `matrix_timeline_paginate` readbacks are accepted and
   `revision` stays monotonic.
5. **Read-frontier / pin metadata.** Mark read/unread and pin/unpin from the
   menu. Assert metadata-only batches project `readState` / `pinnedEventIds`
   without row ops and without a revision gap.
6. **Jump-to-latest.** Scroll up, then jump. Assert a fresh live-bottom readback
   with a **new** `streamId` and the prior stream closed (no further deltas on
   the old id).
7. **Fail-closed drill (optional, dev-only).** Force a revision gap or a
   malformed op (e.g. via a temporary harness or a stale streamId). Assert the
   hook enters `status: 'error'` and does **not** fall back to any JS timeline
   fetch. Restore by re-opening the room.
8. **Unmount close.** Navigate away from the room. Assert `matrix_timeline_close`
   fires with the exact `streamId` and no late `setState` warnings.

## 5. Fail-closed rules (non-negotiable)

- **No JS fallback.** A stream gap, malformed op, or lost sync must surface the
  native error state — never a JS timeline fetch, never a dual-backend flag.
- **Exact stream only.** Any batch whose `streamId` differs from the open
  readback is dropped; the hook never adopts an inferred stream.
- **Monotonic revision.** A batch with `revision !== snapshot.revision + 1` is
  rejected; the hook errors rather than guessing.
- **Schema/session/room lock.** A batch with a different `schemaVersion`,
  `sessionGeneration`, or `roomId` is rejected.
- **Close on unmount.** Every opened stream is closed with its exact `streamId`;
  `disposed` guards all async callbacks.
- **No product code in this PR.** This doc only; C3 verification is a live
  proof gate, not a code change.

## 6. Done when

- C1 (#285) and C2 (#289) are merged and `NativeTimelinePresenter` is the sole
  active timeline owner.
- Steps 1–6 and 8 above pass on an authenticated desktop session; step 7 is
  demonstrated at least once (dev harness acceptable).
- No JS timeline fallback is reachable on the selected path; `RoomTimeline` is
  deleted.
- The C3 row in [v-timeline-cutover-residual.md](v-timeline-cutover-residual.md)
  is updated to "verified" with the live-proof evidence recorded.

## 7. Self-eval confidence

- **High** on the binding contract points (S1–S7) — read directly from
  `nativeTimelineView.ts` and the contract doc; unit tests already cover the
  pure delta reducer.
- **Medium** on live proof — steps require an authenticated session and the
  C1/C2 cutover to be merged first; this doc frames the gate, it does not claim
  the proof.
