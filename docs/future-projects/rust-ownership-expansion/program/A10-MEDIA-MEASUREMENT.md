# A10 Media Boundary Measurement

Status: deterministic bounded-transport harness complete; executable product-owner
and live device/network evidence open.

This record measures the existing ADR 0005 operating path. It does not activate
`MediaCacheIndex`, add a retry owner, project filesystem paths or raw `mxc://`
identifiers, or move bytes onto `Core::command`.

## Operating path

1. A Core-owned timeline row exposes an opaque media handle.
2. The client requests that handle through the dedicated native byte channel.
3. Core resolves the retained Matrix SDK `MediaSource`, uses the authenticated
   Matrix media endpoint, enforces the channel cap while streaming, and owns
   decryption when the source is encrypted.
4. The platform receives bounded bytes and owns display or file handoff.

The iOS timeline channel remains capped at 32 MiB. The desktop custom protocol
remains independently capped at 64 MiB and returns `Cache-Control: no-store`.
Those are deliberate product-channel policies, not competing Matrix owners.

## Deterministic measurements

The `app::media::bounded` loopback harness executes the real Matrix SDK HTTP
client and the production `download_media_bounded` transport primitive. It
bounds elapsed wall time around that primitive and proves:

- the authenticated client-v1 route is used and carries the restored session's
  bearer authorization;
- declared and chunked responses fail closed when they exceed the exact cap;
- only a client-v1 `404` or `405` takes the documented legacy media-v3
  compatibility route, while a server error is not retried;
- two identical calls to the bounded transport perform two network fetches,
  confirming that this primitive itself has no cache; and
- aborting the Rust task while a chunked response is incomplete cancels its
  waiter. Prompt transport/socket teardown is **Not confirmed**: the current
  HTTP stack did not expose a bounded server-observable close signal.

The deterministic suite also constructs encrypted attachments with the pinned
SDK crypto implementation and enters the production decrypt-and-plaintext-cap
step. It proves an in-cap attachment returns the exact plaintext and a
plaintext result one byte above the cap fails with `TooLarge`. This closes the
previously unexecuted decrypt branch; it does not measure peak process memory or
prove Swift-to-Rust in-flight cancellation.

The post-review exact working tree reran the complete Rust workspace on
2026-09-03 with this branch enabled and passed. This is deterministic branch
coverage only; the real-device RSS, Swift/UniFFI in-flight cancellation, and
cache-decision gates below remain open.

Source-boundary guards separately pin the 32 MiB iOS/Core cap, the 64 MiB
desktop-protocol cap, the desktop no-store response, and the absence of the
currently unused `MediaCacheIndex` identifier or an inline generic `retry(`
call from their inspected owner bodies. Those guards are drift detectors, not
executable proof that every caller above the transport is uncached or
unretried.

On iOS, `TimelineMediaFetch` signposts now report only duration, byte count,
and a numeric success/failure/cancelled outcome. They intentionally contain no
handle, MXC URI, room/event identifier, filename, token, or message content.
The wrapper rejects cancellation before entering Core and after the generated
UniFFI call returns.

## Findings and limits

- No deterministic result justifies enabling the unused cache harness. Remote
  latency, radio cost, repeated user access, and process memory on real devices
  remain the evidence needed for a cache product decision.
- The deterministic network harness enters at `download_media_bounded`, not at
  `SharedCore.timeline_media_bytes` or the Tauri custom protocol. Therefore it
  does not claim that every product-owner layer is uncached or unretried. A
  disposable executable owner-route proof and the live gate below remain open.
- The pinned UniFFI 0.28.3 Swift async bridge frees a Rust future when its call
  returns but does not propagate Swift structured-concurrency cancellation into
  an already-running Rust future. The Rust operation itself is cancellation-safe
  when its owner drops it; the current iOS wrapper can only avoid entry or
  discard a result after return. Changing that bridge is separate product and
  binding work, not a reason to hide the limitation with a retry or cache.
- The cap tests bound retained response bytes, but they are not a process-RSS
  measurement. A real-device Instruments run must account for SDK buffering,
  Rust allocation, and the UniFFI `Data` handoff before any cap or cache policy
  changes.
- The loopback elapsed-time assertion is a liveness guard, not a remote-network
  performance claim.
- The cancellation harness proves caller-task termination, not TCP teardown.
  Proving prompt transport cancellation requires an observable contract at the
  HTTP-owner boundary rather than assuming socket lifetime from a dropped
  future.

## Live evidence gate

Keep A10 open until an opt-in real-device run records, for representative plain
and encrypted media below and near the 32 MiB cap:

- first and repeated fetch duration;
- peak process memory during fetch, decrypt, and Swift `Data` handoff;
- cancellation timing before dispatch and during an in-flight transfer;
- failure outcome for oversize, truncated, corrupt, and offline responses; and
- equivalent desktop measurements below and near its distinct 64 MiB cap.

The resulting report must contain aggregate numeric measurements and static
outcome codes only. It must not contain media bytes, keys, tokens, raw MXC
identifiers, opaque handles, local paths, room/event IDs, or message content.

## Decision

Retain the current opaque-handle/dedicated-channel architecture. Keep
`MediaCacheIndex` unwired, keep automatic media retry absent, and do not change
the channel caps based on loopback evidence. Revisit cache eligibility or
thumbnail metadata only after the live gate demonstrates a concrete user or
resource problem and a separately reviewed product plan defines idempotency,
cancellation, quota, eviction, logout/wipe, and disk-pressure behavior.
