# V-BURN blocker snapshot — not a readiness claim

> Documentation-only evidence snapshot at the pinned source tip. It does not
> authorize cutover, merge, release, or a V-BURN-ready claim. Readiness remains
> **Not ready**.

| Field                   | Value                                                                            |
| ----------------------- | -------------------------------------------------------------------------------- |
| Readiness               | **Not ready**                                                                    |
| Runtime proof           | **Not confirmed**                                                                |
| Source tip              | `e4cf1a5f1d4c6127afd34f2b2f23d93400a03d45`                                       |
| Base                    | `feature/matrix-rust-sdk-full-replacement` only                                  |
| Scope                   | Docs only; no product code or cutover state change                               |
| `dual_backend`          | **Forbidden**; this snapshot does not enable or claim it                         |
| Fail-closed policy      | Native-session command absence/failure is terminal; no JS fallback is authorized |
| Prettier                | `2.8.1`                                                                          |
| Production import files | **159** under `synara/src/`                                                      |
| P1.6 allowlist entries  | **163**                                                                          |
| #327                    | **HOLD** — do not merge                                                          |
| #39 / `main`            | **Gated** — do not merge                                                         |

## Direct source blockers

The tip still constructs and starts a live JavaScript Matrix client:

- [`initMatrix.ts:206`](../../synara/src/client/initMatrix.ts#L206) calls
  `createClient(clientOptions)`.
- [`initMatrix.ts:459`](../../synara/src/client/initMatrix.ts#L459) calls
  `mx.startClient(...)`.
- [`synara/package.json:99`](../../synara/package.json#L99) retains
  `matrix-js-sdk` at `42.0.0`; the lockfile retains the dependency as well.
- The committed [`desktop-sdk-usage.md`](desktop-sdk-usage.md) report counts
  **159** production import files. The committed
  [`p1.6-js-sdk-import-allowlist.json`](p1.6-js-sdk-import-allowlist.json)
  contains **163** paths.

These facts alone prevent a zero-live-client or zero-import V-BURN conclusion.
The allowlist is inventory policy during migration; it is not evidence that the
remaining importers are acceptable at final convergence.

## Residual Left at the tip

1. **Pack read `get*` helpers — V-BURN-gated.** Delete the remaining JS read
   helpers in [`v-send-pack-read-residual.md`](v-send-pack-read-residual.md)
   only when the non-native web fallback is retired. Retain the shared pack
   comparison helpers called out by that residual. Native-session paths remain
   fail-closed.
2. **V-TIMELINE.C3–C5 — Not confirmed.** The stream/delta, media/render, and
   pins/notes/jump checklists are docs-only live-proof gates; no authenticated
   runtime proof is claimed.
3. **V-SEND.R-DEVTOOL — gated after C3–C5.** The inventory remains docs-only;
   implementation may start only after all three live timeline proofs confirm.
4. **V-BURN.1–3 final convergence — still left.** Keep #327 on HOLD and keep
   #39 gated. Do not set `active_slice=V-BURN`.

The current residual queue is summarized in [`SCOREBOARD.md`](SCOREBOARD.md).
Its tip field is intentionally not changed by this snapshot; the source
evidence above is pinned explicitly to `e4cf1a5f1d4c6127afd34f2b2f23d93400a03d45`.

## Bottom line

Native replacement progress is real, but the tip still has a live JS client,
the npm dependency, 159 production import files against a 163-entry migration
allowlist, and unconfirmed residual proofs. Therefore the honest statement is:

**V-BURN is blocked; readiness is Not ready; this file is not a readiness claim.**
