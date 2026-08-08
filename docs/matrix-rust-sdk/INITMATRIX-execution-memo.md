# INITMATRIX — execution memo (native client cutover)

> **Decision required (operator).** Docs-only. This memo converts the re-scoped INMATRIX
> epic into an operator-reviewable execution contract so the cutover can start on signal
> without re-deriving. Produced after two independent read-only audits (verdicts recorded
> in the local operating ledger) and cross-verified at tip `437ef7d0e6`.

## 1. Verified current state

- Sole `matrix-js-sdk` production importer left in the repo:
  `synara/src/client/initMatrix.ts` (`createClient` + `IndexedDBStore` +
  `IndexedDBCryptoStore` + the client bootstrap). Test import files: **0**. Allowlist: **1**.
- The **native side is already built and wired on this branch**:
  - `matrix-sdk` =0.18.0 pinned in `src-tauri/Cargo.toml` (sqlite / bundled-sqlite stores);
  - `src-tauri/src/matrix/{auth,client_builder,account_data,backup,...}` builds a live
    authenticated client and starts a sync service (`start_sync_owner`);
  - `src-tauri/src/lib.rs` registers **~142 real `matrix_*` Tauri commands** (login,
    restore_session, sync_status, room_list_snapshot, timeline_*, send_*, presence, ...);
  - native owners + Tauri events already flow to the renderer.
- `ClientRoot.tsx` boot today: `matrix_restore_session` (native session restored) **then
  still `initClient(session)` + js-sdk `startClient`** — the native session and the js-sdk
  renderer client coexist. The renderer is the only non-native surface remaining.

## 2. The remaining gap (4 artifacts)

| # | Artifact | Notes |
|---|----------|-------|
| a | Renderer native **client facade** | Backs the unchanged `Awaited<ReturnType<typeof initClient>>` anchor so ~141 `mx` consumers need zero churn; re-expresses `initClient`/`startClient`/`performLogout`/`clearCacheAndReload`/`scheduleProactiveTokenRefresh` on the `matrix_*` commands. |
| b | Native **main sync-state push** | UI gating today reads the js-sdk client's `getSyncState()`; only a `sync_status` snapshot exists natively — add the PREPARED/ERROR readiness event stream. |
| c | **Token-refresh re-point** | Renderer `refreshAndPersistSession` still calls js-sdk `mx.refreshToken`; Rust already exposes `handle_refresh_tokens`. |
| d | **Room-list live delta / local-echo** | `roomList.ts` is snapshot-pull today; the js-sdk sync event path can only be dropped when a live delta source exists. |

## 3. Operator options

- **A — Approve the native cutover epic (recommended, staged).** Landing order:
  1. (a) client facade + (c) token-refresh re-point behind the unchanged type anchor;
  2. (b) native sync-state readiness stream;
  3. (d) room-list live delta.
  Each slice is independently reviewed and must pass the Quality gate + Desktop package
  gate. After (1)-(3): drop `matrix-js-sdk@42.0.0` from `synara/package.json` (+ lock),
  `npm audit --omit=dev --audit-level=high`, regen the inventory (production **0**),
  ratchet allowlist to **0**, run the V-BURN checklist. Preserves UI/UX (staged, byte-identical
  surface; native session already live so desktop behavior is exercised the whole way).
- **B — Approve a legacy-loader redefinition.** Keep a vendored/flagged js-sdk loader on
  desktop, re-define V-BURN to "no production import, dev-only dependency". Not recommended:
  keeps a live JS runtime and weakens the plan §14 acceptance.

## 4. Acceptance criteria (unchanged plan §14)

zero production **and** test imports; no `matrix-js-sdk` in package manifests; no live JS
client; full desktop feature parity suite green. `dual_backend` stays forbidden forever.
`main` / umbrella PR #39: **gated — bridge only on explicit operator authority**.

## 5. Anti-goals

No secrets/credentials in the public tree at any step; UI/UX byte-identical; every PR
opens against `feature/matrix-rust-sdk-full-replacement`, independently reviewed, CI-green
before merge; never merge `main`/#39 without explicit operator approval.
