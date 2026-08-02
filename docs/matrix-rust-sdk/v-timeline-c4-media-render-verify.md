# V-TIMELINE.C4 — media / render parity re-verify after cutover

| Field        | Value                                                                                                                                                                                                                                               |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status       | Docs-only verification checklist — **no product code**                                                                                                                                                                                              |
| Live proof   | **Not confirmed** — #446 product-command fan-out and #448 scoreboard refresh are not C3–C5 proof; no authenticated desktop evidence is recorded for this tip                                                                                        |
| Scope        | `synara/src/app/features/room/NativeTimelinePresenter.tsx` (`NativeTimelineMedia`, `NativeTimelineRow`), `nativeTimelineView.ts` (`nativeTimelineMediaSrc`, `NativeTimelineMediaHandle`)                                                            |
| Precondition | C1 (#285) selects `NativeTimelinePresenter` in `RoomView`; C2 (#289) deletes `RoomTimeline` + dead JS timeline path; C3 (#294) stream/delta checklist exists                                                                                        |
| Policy       | [full-vertical-policy.md](full-vertical-policy.md); [cutover-operating-model.md](cutover-operating-model.md); dual_backend **false**; **never touch #39**                                                                                           |
| Related      | [v-timeline-cutover-residual.md](v-timeline-cutover-residual.md) (C4 row), [v-timeline-full-replacement-contract.md](v-timeline-full-replacement-contract.md) (media owner route), [v-timeline-c3-stream-verify.md](v-timeline-c3-stream-verify.md) |

## 1. What C4 must prove

C4 is the **live authenticated media/render parity proof** that every retained
media/sticker row renders through the native media-handle registry and the
shared `synara-media` protocol on the selected `NativeTimelinePresenter`. It is
a **verification gate**, not a new implementation. The residual map records
"unproven — no live authenticated render proof yet"; C4 closes that unclaimed
live-proof half of the row.

The media owner route (from the contract doc + `nativeTimelineView.ts` +
`NativeTimelinePresenter.tsx`) that must be proven end-to-end on the selected
path:

```text
SDK timeline event media source
  → session-scoped native handle registry
  → TimelineMediaHandle (opaque handle + safe metadata)
  → native URI/protocol resolver requests that handle
  → SDK media request/decryption/cache
  → bytes returned directly to the renderer
```

| #   | Contract point                                                                                                           | Implementation                                                                                                                                           | Proof target                                                                                          |
| --- | ------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| M1  | Rows carry only an **opaque handle + safe metadata**, never MXC URI / encryption descriptor / download URL / media bytes | `NativeTimelineMediaHandle` = `{ handleId, mimeType?, width?, height?, durationMs? }`; `TimelineViewSnapshot`/delta batches carry no media bytes         | No raw media material crosses the webview boundary; only the opaque handle is rendered                |
| M2  | Renderer forms URLs **only** via `convertFileSrc(handle, "synara-media")`                                                | `nativeTimelineMediaSrc(handle)` → `convertDesktopFileSrc(handle.handleId, 'synara-media')`                                                              | Every image/audio/video/sticker/file URL is a `synara-media` handle URL, never an `mxc://` conversion |
| M3  | Image rows render via `<img>` with bounded dimensions                                                                    | `NativeTimelineMedia` `messageType === 'image'` → `<img src={mediaSrc} … style={mediaStyle(media)}>` (max 480×480)                                       | Image decodes and displays at bounded size; alt text from body                                        |
| M4  | Audio rows render via `<audio controls>`                                                                                 | `messageType === 'audio'` → `<audio src={mediaSrc} controls …>`                                                                                          | Audio plays; duration metadata (`data-duration-ms`) present when known                                |
| M5  | Video rows render via `<video controls>` with bounded dimensions                                                         | `messageType === 'video'` → `<video src={mediaSrc} controls style={mediaStyle(media)} …>`                                                                | Video plays; bounded size; duration metadata present when known                                       |
| M6  | File rows render as a download link + MIME label                                                                         | `messageType === 'file'` → `<a href={mediaSrc} download>` + `media.mimeType` label                                                                       | File downloads through the handle; MIME label shown                                                   |
| M7  | Sticker rows render via `<img>` at bounded size                                                                          | `NativeTimelineMedia` `sticker` → `<img src={mediaSrc} alt="Sticker" style={{ maxWidth: 256, maxHeight: 256 }}>`                                         | Sticker decodes and displays; unavailable sticker shows the fallback text                             |
| M8  | Media rows are **capability-gated** and action-parity complete                                                           | `NativeTimelineRowActions` gates reply/edit/forward/redact/report/pin/later by `capabilities`; media forward uses `forwardMediaWithNativeTimelineAction` | Every retained media/sticker affordance routes through a native owner, never a JS fallback            |
| M9  | Encrypted media decrypts through the SDK, not the webview                                                                | Contract: SDK obtains/decrypts bytes; webview receives only the handle                                                                                   | Encrypted image/audio/video/file/sticker renders after SDK decryption                                 |
| M10 | Handle lifecycle is session/stream-bound and revoked on close                                                            | Contract: registry binds handle to session generation + exact stream; revokes on ordered diffs, stream close, or session drop                            | A revoked/stale handle fails closed (no render, no raw fallback), never a guessed fetch               |

## 2. Existing tests (already green on tip)

| Test file                                        | Covers                                                                                                                                                          | C4 gap                                                                                                                        |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `__tests__/nativeTimelineViewDelta.test.ts`      | Pure `applyNativeTimelineViewDelta`: metadata-only read-frontier, pagination + pin-list metadata, empty-batch rejection, pin/forward/thread/format pure helpers | **Unit-level only.** No real media handle, no `synara-media` protocol resolution, no SDK decryption, no authenticated session |
| `__tests__/nativeTimelineActions.test.ts`        | Native action owners (edit/redact/forward/report/pin/poll/call) typed readbacks + off-desktop unavailability                                                    | Action owners, not the media render path; no `convertFileSrc`/`synara-media` resolution                                       |
| `__tests__/nativeTimelineViewportPolicy.test.ts` | `shouldRestoreNativeTimelineViewport` TTL / unread / live-tail precedence                                                                                       | Pure policy; no media                                                                                                         |

**What the unit tests do NOT prove** (the C4 live gap): `synara-media` protocol
resolution of a real opaque handle, SDK media download/decryption of plaintext
**and** encrypted sources, image/audio/video/sticker/file decode + playback in
the renderer, bounded-dimension rendering, file download through the handle,
and handle revocation fail-closed — all require an authenticated desktop
session with real media events.

## 3. Operator preflight (runbook; no proof claim)

This section makes the C4 checklist executable for an operator. It does not
change the C4 status and does not claim that live proof has run. Docker and
harness readiness are prerequisites only; they are not media/render evidence.

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

Install and start the Docker runtime before attempting the C4 session:

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
record the harness as `blocked` and keep C4 `Not confirmed`. Do not treat a
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
access/refresh tokens, Matrix user/room/event IDs, message bodies, media
bytes, encryption descriptors, or absolute paths into the proof record. Always
tear down the disposable service after the attempt, including a failed attempt:

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

### 3.4 C4 action and evidence mapping

The visible labels below are the selected `NativeTimelinePresenter` controls.
The UI is useful result corroboration; it does not by itself prove opaque
handle ownership, `synara-media` resolution, SDK decryption, or absence of a
JS fallback. Paste the sanitized native/diagnostic trace for those claims.

| C4 step / contract | Operator action                                                                                   | Required observation and evidence                                                                                                                                                                                        |
| ------------------ | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1 / M1–M3          | From a second client, send a plaintext image.                                                     | The image renders through a sanitized `synara-media` handle alias, never `mxc://`; bounded dimensions are ≤480×480 and alt text comes from the body. Trace the handle-only boundary; a screenshot alone is insufficient. |
| 2 / M1, M9         | In an encrypted room, send an image.                                                              | The image renders after SDK decryption; the webview receives only the opaque handle and safe metadata. No ciphertext, encryption descriptor, raw URI, or JS media fetch appears in the trace.                            |
| 3 / M4             | Send an audio clip.                                                                               | `<audio controls>` plays through the handle; `data-duration-ms` is present when known. Record a sanitized resolver/readback trace, not media bytes.                                                                      |
| 4 / M5             | Send a video clip.                                                                                | `<video controls>` plays through the handle at bounded dimensions; `data-duration-ms` is present when known.                                                                                                             |
| 5 / M6             | Send a file and activate its download link.                                                       | Download succeeds through the handle and the MIME label is shown; no raw media URL or JS download path is used.                                                                                                          |
| 6 / M7             | Send a sticker, then exercise an unavailable/revoked sticker handle if a dev-only harness exists. | The sticker is ≤256×256; unavailable media shows `Sticker media is unavailable.` and never falls back to a raw fetch. If the harness is unavailable, record the step as `not run`; C4 remains `Not confirmed`.           |
| 7 / M8             | Forward a media row to another room.                                                              | The trace names `forwardMediaWithNativeTimelineAction`, and the target room receives the media. A JS forward is a failure.                                                                                               |
| 8 / M10            | If available, close the stream or run the dev-only stale-handle drill, then reopen the room.      | Revoked handles fail closed and recover only through a fresh native open. Do not create product code for this PR.                                                                                                        |
| 9 / M10            | Navigate away from the room.                                                                      | The exact stream closes, handles are revoked, and no late media resolution, JS fallback, or `setState` warning appears.                                                                                                  |

### 3.5 Evidence to paste

Paste one sanitized block per attempt. Keep the verdict conservative; do not
replace `Not confirmed` with `Confirmed` from screenshots, unit tests, a
generated success label, Docker readiness, or a retry that lacks the complete
route chronology.

```text
proof: V-TIMELINE.C4
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
M1 opaque handle boundary: pass | fail | not observed
M2 synara-media-only URL: pass | fail | not observed
M3 bounded image render: pass | fail | not observed
M4 audio playback: pass | fail | not observed
M5 video playback: pass | fail | not observed
M6 file download and MIME: pass | fail | not observed
M7 sticker render/fallback: pass | fail | not observed
M8 native media action parity: pass | fail | not observed
M9 SDK encrypted-media decryption: pass | fail | not observed
M10 handle revocation/close: pass | fail | not observed
authoritative readback: <sanitized second-client/homeserver result>
native media trace: <sanitized handle aliases, resolver events, and close/revoke events>
deviations: none | <exact deviation; any fallback/retry/manual correction is disqualifying>
cleanup: pass | fail
```

The Docker and harness fields prove only that the disposable test topology was
available and cleaned up; they do not prove C4 media/render parity. The native
media trace and authoritative second-client readback must cover the required
M1–M10 chronology. Use `blocked` for an environment preflight that never
reached the harness; use `Failed` only when the attempted C4 route produced a
disqualifying observation. In either case, do not mark C4 `Confirmed`.

The exported Diagnostics report is privacy-filtered corroboration. Review it
before sharing; it intentionally excludes secrets, server URLs, media bytes,
and Matrix identifiers. If the available report cannot establish a required
handle, decryption, resolver, or close/revocation fact, write `not observed`
and keep the verdict `Not confirmed`.

## 4. Suggested live proof steps (authenticated desktop, after C1/C2/C3)

Run against the sole desktop user on a real homeserver (Synapse topology per
[test-matrix-synapse-topology.md](test-matrix-synapse-topology.md)). Record each
step with a timestamp + observed state.

1. **Plaintext image.** From a second client, send an image. Assert the row
   renders via an `<img>` whose `src` is a `synara-media` handle URL (no
   `mxc://`), bounded to ≤480×480, with alt text from the body.
2. **Encrypted image.** In an encrypted room, send an image. Assert it renders
   after SDK decryption — the webview never sees ciphertext or an encryption
   descriptor, only the handle.
3. **Audio.** Send an audio clip. Assert `<audio controls>` plays; `data-duration-ms`
   present when the SDK reports duration.
4. **Video.** Send a video. Assert `<video controls>` plays at bounded size;
   `data-duration-ms` present when known.
5. **File.** Send a file. Assert the download link resolves through the handle
   and the MIME label renders; download succeeds.
6. **Sticker.** Send a sticker. Assert `<img>` renders at ≤256×256; a sticker
   with an unavailable handle shows the "Sticker media is unavailable." fallback
   (fail-closed, no raw fetch).
7. **Media forward.** Forward a media row to another room. Assert
   `forwardMediaWithNativeTimelineAction` is used (not a JS forward) and the
   target room receives the media.
8. **Handle revocation drill (optional, dev-only).** Force a revoked/stale
   handle (e.g. via a temporary harness or by closing the stream). Assert the
   row fails closed — no render, no JS fallback — and recovers on re-open.
9. **Unmount close.** Navigate away from the room. Assert the stream closes and
   no late media resolution or `setState` warnings fire.

## 5. Fail-closed rules (non-negotiable)

- **No raw media in the webview.** No MXC URI, encryption descriptor, download
  URL, media bytes, or credential may enter `TimelineViewSnapshot`, a delta
  batch, or a Tauri command payload.
- **Handle-only URLs.** The renderer forms media URLs only through
  `nativeTimelineMediaSrc` → `convertFileSrc(handle, "synara-media")`; never an
  `mxc://` conversion or a JS media fetch.
- **No JS fallback.** A missing/revoked handle or failed decryption surfaces the
  native fail-closed state — never a JS timeline media fetch, never a
  dual-backend flag.
- **Capability-gated actions.** Every retained media/sticker affordance routes
  through a native owner; no JS action path remains on the selected presenter.
- **Close on unmount.** Every opened stream is closed with its exact `streamId`;
  handles are revoked on stream close / session drop.
- **No product code in this PR.** This doc only; C4 verification is a live
  proof gate, not a code change.

## 6. Done when

- C1 (#285), C2 (#289), and C3 (#294) are merged and `NativeTimelinePresenter`
  is the sole active timeline owner.
- Steps 1–7 and 9 above pass on an authenticated desktop session; step 8 is
  demonstrated at least once (dev harness acceptable).
- Every retained media/sticker row renders via native handles / `synara-media`
  with no JS media fallback reachable on the selected path.
- The C4 row in [v-timeline-cutover-residual.md](v-timeline-cutover-residual.md)
  is updated to "verified" with the live-proof evidence recorded.

## 7. Self-eval confidence

- **High** on the media contract points (M1–M10) — read directly from
  `NativeTimelinePresenter.tsx` (`NativeTimelineMedia`), `nativeTimelineView.ts`
  (`nativeTimelineMediaSrc`, `NativeTimelineMediaHandle`), and the contract doc's
  media owner route; unit tests already cover the pure delta/action owners.
- **Medium** on live proof — steps require an authenticated session, real media
  events, and the C1/C2/C3 cutover to be merged first; this doc frames the gate,
  it does not claim the proof.
