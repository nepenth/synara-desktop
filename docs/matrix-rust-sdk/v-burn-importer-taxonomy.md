# V-BURN importer taxonomy — Matrix Rust full replacement

| Field                  | Value                                                                                                                           |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Status                 | **V-BURN REACHED (full)** — 0 importers; `matrix-js-sdk` fully removed from package.json/lock (two-client harness + CI retired) |
| Measured tip           | `0f93424e`                                                                                                                      |
| Base                   | `feature/matrix-rust-sdk-full-replacement`                                                                                      |
| Scope                  | Production `matrix-js-sdk` importers under `synara/src`                                                                         |
| Current importers      | **0**                                                                                                                           |
| P1.6 allowlist entries | **0** (empty = full ban)                                                                                                        |
| Policy                 | Full replacement; `dual_backend` forbidden; fail-closed                                                                         |
| V-BURN                 | **REACHED (full)** — importer drop 1 → 0 (#624) + js-sdk fully removed (dep, devDep, harness, CI; F6c-2c/3)                     |
| Hold                   | **V-BURN HOLD**; do not claim V-BURN-ready; **#39 remains gated**                                                               |

This is a classification snapshot, not a cutover plan or readiness claim. The
pre-burn path enumeration (150 importers across
`client-lifecycle`/`component`/`feature`/`hook`/`media-boundary`/`page`/`plugin`/
`shared-type`/`state`/`utility` buckets) is superseded by the tip inventory: all
but one of those files are now SDK-free (see `desktop-sdk-usage.md` and the git
history for the burned-path record).

## Measurement and reconciliation

Counts are taken from the generated [desktop SDK usage inventory](desktop-sdk-usage.md)
and checked against a direct source import scan at the measured tip:

- **1** production importer file under `synara/src` (`synara/src/client/initMatrix.ts`);
- **0** test importer files under `synara/src` (10 burned in #595; fixtures now
  exercise probed literals / local structural projections);
- **1** path in [`p1.6-js-sdk-import-allowlist.json`](p1.6-js-sdk-import-allowlist.json);
- the current production importer set exactly equals the allowlist.

Tooling-only fixtures under `scripts/fixtures/` reference js-sdk as guardrail
negative fixtures and are not counted here (they prove the fail-closed rule).

## Remaining importer — the V-BURN epic core

| Path                              | Bucket             | Why it remains                                                                                                                                                                                                                                                                                                    |
| --------------------------------- | ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `synara/src/client/initMatrix.ts` | `client-lifecycle` | Constructs the live JS client (`createClient`), IndexedDB stores, login/sync/start, token refresh, crypto continuity. Every derived client type in the app (`useMatrixClient`, `LocalMx` throughout) is `Awaited<ReturnType<typeof initClient>>`, so this file is the constructor the whole type graph hangs off. |

Burning it requires re-expressing that runtime on the Rust SDK via a native
`initClient` (no native client bootstrap exists yet) or an operator-sanctioned
legacy-loader that redefines V-BURN. `matrix-js-sdk@42.0.0` has no transitive
dependents; `matrix-widget-api` (a separate runtime dep for the call widget) does
not require it — so npm-dependency removal is exactly one epic away.

### V-CALL / matrix-widget-api (2026-08-09)

Operator-authorized deep cut: the V-CALL widget surface (call plugin + `matrix-widget-api`) was removed entirely, so the taxonomy below has **no remaining Matrix npm dependency of any kind** — the tree is fully zero-Matrix-JS outside the guardrail fixtures. General native media config/download owners moved under the media module (`matrix_media_config`/`matrix_media_download`).
