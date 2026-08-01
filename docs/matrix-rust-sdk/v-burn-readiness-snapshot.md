# V-BURN readiness snapshot — zero live `matrix-js-sdk` client

| Field                      | Value                                                                      |
| -------------------------- | -------------------------------------------------------------------------- |
| Status                     | **Not ready** — zero live `matrix-js-sdk` client is not achieved           |
| Runtime-proof verdict      | **Not confirmed** — the final zero-usage/runtime qualification has not run |
| Integration branch         | `feature/matrix-rust-sdk-full-replacement` only                            |
| Scoreboard basis           | [`SCOREBOARD.md`](SCOREBOARD.md) at tip `8940f6ea`                         |
| Local source tip at review | `cd895575` (`#324`, docs-only; no product/import delta)                    |
| Umbrella / `main`          | **Out of scope** — do not merge `#39` without explicit user approval       |
| Scope                      | Documentation only; no product code or branch policy changes               |

## Bottom line

The replacement branch is still in capability burn-down, not final V-BURN. The
current source directly constructs and starts a live JavaScript Matrix client:
[`initMatrix.ts`](../../synara/src/client/initMatrix.ts#L206) calls
`createClient()`, and [`initMatrix.ts`](../../synara/src/client/initMatrix.ts#L459)
calls `mx.startClient()`. The npm dependency is also still present at
[`synara/package.json`](../../synara/package.json#L99).

The committed import inventory reports **163 production import files** under
`synara/src/`, **10 test import files**, and **176 import files repository-wide**
across production, test, and tooling roles. The production count is down from
the plan's 220-file baseline, but it is not zero. The largest remaining model
coupling counts are `Room` in 81 files, `MatrixClient` in 43, `MatrixEvent` in
28, and `MatrixError` in 26. These counts measure import coupling; the
inventory's method/listener findings are AST candidates, not type-proven API
calls.

`Dual backend: false` in the scoreboard records the policy constraint. It does
not prove that the JavaScript client has already disappeared.

## What still blocks V-BURN

| Blocker                                            | Current evidence                                                                                                                                                                              | Required closure                                                                                                                                                                                                 |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Live JS bootstrap and cross-cutting import surface | `createClient()` / `startClient()` remain in `initMatrix.ts`; the inventory remains at **163 production import files** and lists 157 root-module production import sites plus deep imports    | Complete the owning vertical deletions and Phase 11 convergence: replace shared SDK models/hooks/components, remove JS bootstrap/sync/crypto/store ownership, and drive the production/test import count to zero |
| Pack-read room resolution and fallback deletion    | The scoreboard keeps **#320 open** for room→pack-room resolution without `mx.getRoom`; JS read-helper deletion remains gated on dropping the non-native web fallback                          | Land the native room-resolution owner, then delete the remaining JS read-helper functions and web-fallback listeners at the owning boundary                                                                      |
| Timeline cutover evidence                          | **C3, C4, and C5** remain live-verification items; the current checklists are documentation, not runtime parity proof                                                                         | Run the complete native stream, media/render, pins/notes/jump scenarios and record authoritative readback                                                                                                        |
| Remaining send/media residuals                     | The scoreboard still lists **V-SEND.R-CALL-UPLOAD / R-GIF-PACK** and **V-SEND.R-DEVTOOL**; the upload audit identifies reachable widget `client.uploadContent()` at `CallWidgetDriver.ts:318` | Give each residual its native owner and perform physical deletion; keep fallback-only composer upload code distinct from still-reachable native-session paths                                                    |
| Dependency and final-boundary removal              | `matrix-js-sdk@42.0.0` remains in the manifest; the inventory still reports direct Matrix networking findings in production, test, and tooling scopes                                         | Remove the package and lockfile entries, JS stores/bootstrap, obsolete service-worker/media ownership, and any unapproved raw runtime Matrix networking; then rerun the final audits                             |
| Final qualification and release authority          | V-BURN.1–3 are still listed as left; no zero-usage, parity, packaged desktop, or final release qualification is claimed here                                                                  | Pass the Phase 11/14 AST, package-lock, raw-network, TypeScript, Rust, Synapse, package-smoke, and manual parity gates before considering a `main` PR                                                            |

## Landed work that does not close V-BURN

The scoreboard records substantial native work as landed, including core send,
auth, rooms, timeline C1/C2, pack snapshot/subscribe, avatar/profile, pack
writes, and compact media upload. Those entries establish progress at their
owning verticals; they do not authorize a zero-client or zero-import claim
while the cross-cutting bootstrap and residual owners above remain.

In particular:

- #318 closed the native pack subscription signal, but the scoreboard still
  leaves #320 and the web-fallback/read-helper deletion work open.
- #314 closes the compact desktop upload path; it does not close the reachable
  call-widget upload owner identified by the upload audit.
- Timeline C1/C2 are landed, but C3–C5 still require live proof.

## Exact V-BURN acceptance still outstanding

The branch is not ready to claim any of these as complete:

- [ ] no production or test `matrix-js-sdk` imports;
- [ ] no JavaScript Matrix client construction or sync loop;
- [ ] no Matrix IndexedDB/crypto store bootstrap or obsolete service-worker
      Matrix ownership;
- [ ] no `matrix-js-sdk` package or crypto-WASM dependency in manifests or
      lockfiles;
- [ ] no unapproved production `/_matrix/` networking;
- [ ] Rust is the sole desktop Matrix owner with full feature-parity evidence;
- [ ] final AST/package-lock/raw-network/runtime/package-smoke and manual
      qualification passes;
- [ ] explicit user approval before any merge to `main` through #39.

Until those checks are independently evidenced, the correct readiness statement
is: **native replacement progress is real, but zero live `matrix-js-sdk` client
readiness is not confirmed and V-BURN remains blocked.**

## Evidence limits

This snapshot uses the committed [`desktop-sdk-usage.md`](desktop-sdk-usage.md)
and [`desktop-sdk-usage.json`](desktop-sdk-usage.json) artifacts plus the
current scoreboard and source readback. The inventory regeneration check was
attempted in this checkout but could not run because
`synara/node_modules/typescript` is absent; no inventory artifact was
regenerated here. The committed report is therefore cited as the evidence
snapshot, not presented as a freshly executed audit.
