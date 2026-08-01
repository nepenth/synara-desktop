# V-TIMELINE.C4 — media / render parity re-verify after cutover

| Field  | Value |
| ------ | ----- |
| Status | Docs-only verification checklist — **no product code** |
| Scope  | `synara/src/app/features/room/NativeTimelinePresenter.tsx` (`NativeTimelineMedia`, `NativeTimelineRow`), `nativeTimelineView.ts` (`nativeTimelineMediaSrc`, `NativeTimelineMediaHandle`) |
| Precondition | C1 (#285) selects `NativeTimelinePresenter` in `RoomView`; C2 (#289) deletes `RoomTimeline` + dead JS timeline path; C3 (#294) stream/delta checklist exists |
| Policy | [full-vertical-policy.md](full-vertical-policy.md); [cutover-operating-model.md](cutover-operating-model.md); dual_backend **false**; **never touch #39** |
| Related | [v-timeline-cutover-residual.md](v-timeline-cutover-residual.md) (C4 row), [v-timeline-full-replacement-contract.md](v-timeline-full-replacement-contract.md) (media owner route), [v-timeline-c3-stream-verify.md](v-timeline-c3-stream-verify.md) |

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

| # | Contract point | Implementation | Proof target |
| -- | -------------- | -------------- | ------------ |
| M1 | Rows carry only an **opaque handle + safe metadata**, never MXC URI / encryption descriptor / download URL / media bytes | `NativeTimelineMediaHandle` = `{ handleId, mimeType?, width?, height?, durationMs? }`; `TimelineViewSnapshot`/delta batches carry no media bytes | No raw media material crosses the webview boundary; only the opaque handle is rendered |
| M2 | Renderer forms URLs **only** via `convertFileSrc(handle, "synara-media")` | `nativeTimelineMediaSrc(handle)` → `convertDesktopFileSrc(handle.handleId, 'synara-media')` | Every image/audio/video/sticker/file URL is a `synara-media` handle URL, never an `mxc://` conversion |
| M3 | Image rows render via `<img>` with bounded dimensions | `NativeTimelineMedia` `messageType === 'image'` → `<img src={mediaSrc} … style={mediaStyle(media)}>` (max 480×480) | Image decodes and displays at bounded size; alt text from body |
| M4 | Audio rows render via `<audio controls>` | `messageType === 'audio'` → `<audio src={mediaSrc} controls …>` | Audio plays; duration metadata (`data-duration-ms`) present when known |
| M5 | Video rows render via `<video controls>` with bounded dimensions | `messageType === 'video'` → `<video src={mediaSrc} controls style={mediaStyle(media)} …>` | Video plays; bounded size; duration metadata present when known |
| M6 | File rows render as a download link + MIME label | `messageType === 'file'` → `<a href={mediaSrc} download>` + `media.mimeType` label | File downloads through the handle; MIME label shown |
| M7 | Sticker rows render via `<img>` at bounded size | `NativeTimelineMedia` `sticker` → `<img src={mediaSrc} alt="Sticker" style={{ maxWidth: 256, maxHeight: 256 }}>` | Sticker decodes and displays; unavailable sticker shows the fallback text |
| M8 | Media rows are **capability-gated** and action-parity complete | `NativeTimelineRowActions` gates reply/edit/forward/redact/report/pin/later by `capabilities`; media forward uses `forwardMediaWithNativeTimelineAction` | Every retained media/sticker affordance routes through a native owner, never a JS fallback |
| M9 | Encrypted media decrypts through the SDK, not the webview | Contract: SDK obtains/decrypts bytes; webview receives only the handle | Encrypted image/audio/video/file/sticker renders after SDK decryption |
| M10 | Handle lifecycle is session/stream-bound and revoked on close | Contract: registry binds handle to session generation + exact stream; revokes on ordered diffs, stream close, or session drop | A revoked/stale handle fails closed (no render, no raw fallback), never a guessed fetch |

## 2. Existing tests (already green on tip)

| Test file | Covers | C4 gap |
| --------- | ------ | ------ |
| `__tests__/nativeTimelineViewDelta.test.ts` | Pure `applyNativeTimelineViewDelta`: metadata-only read-frontier, pagination + pin-list metadata, empty-batch rejection, pin/forward/thread/format pure helpers | **Unit-level only.** No real media handle, no `synara-media` protocol resolution, no SDK decryption, no authenticated session |
| `__tests__/nativeTimelineActions.test.ts` | Native action owners (edit/redact/forward/report/pin/poll/call) typed readbacks + off-desktop unavailability | Action owners, not the media render path; no `convertFileSrc`/`synara-media` resolution |
| `__tests__/nativeTimelineViewportPolicy.test.ts` | `shouldRestoreNativeTimelineViewport` TTL / unread / live-tail precedence | Pure policy; no media |

**What the unit tests do NOT prove** (the C4 live gap): `synara-media` protocol
resolution of a real opaque handle, SDK media download/decryption of plaintext
**and** encrypted sources, image/audio/video/sticker/file decode + playback in
the renderer, bounded-dimension rendering, file download through the handle,
and handle revocation fail-closed — all require an authenticated desktop
session with real media events.

## 3. Suggested live proof steps (authenticated desktop, after C1/C2/C3)

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

## 4. Fail-closed rules (non-negotiable)

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

## 5. Done when

- C1 (#285), C2 (#289), and C3 (#294) are merged and `NativeTimelinePresenter`
  is the sole active timeline owner.
- Steps 1–7 and 9 above pass on an authenticated desktop session; step 8 is
  demonstrated at least once (dev harness acceptable).
- Every retained media/sticker row renders via native handles / `synara-media`
  with no JS media fallback reachable on the selected path.
- The C4 row in [v-timeline-cutover-residual.md](v-timeline-cutover-residual.md)
  is updated to "verified" with the live-proof evidence recorded.

## 6. Self-eval confidence

- **High** on the media contract points (M1–M10) — read directly from
  `NativeTimelinePresenter.tsx` (`NativeTimelineMedia`), `nativeTimelineView.ts`
  (`nativeTimelineMediaSrc`, `NativeTimelineMediaHandle`), and the contract doc's
  media owner route; unit tests already cover the pure delta/action owners.
- **Medium** on live proof — steps require an authenticated session, real media
  events, and the C1/C2/C3 cutover to be merged first; this doc frames the gate,
  it does not claim the proof.
