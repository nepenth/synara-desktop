# Matrix Rust SDK replacement — live progress log

> **2026-08-09 (post V-BURN):** Deep V-CALL cut landed on `feature/matrix-rust-sdk-full-replacement`
> (`agent/v-call-removal`, #633): call plugin + call UI + widgets native module + `matrix-widget-api`
> dependency removed → **fully zero-Matrix-JS tree**. General media `matrix_media_config` /
> `matrix_media_download` preserved in the media module; native Synapse proof family now 6 jobs.

<!-- matrix-rust-program-status-link -->

> **Remote-monitor file.** Open this on GitHub on the integration branch and refresh
> to see what the orchestrator has completed and what is next.
>
> **Branch:** [`feature/matrix-rust-sdk-full-replacement`](https://github.com/nepenth/synara-desktop/tree/feature/matrix-rust-sdk-full-replacement)
>
> **This file on GitHub:**
>
> [docs/matrix-rust-sdk/PROGRESS.md](https://github.com/nepenth/synara-desktop/blob/feature/matrix-rust-sdk-full-replacement/docs/matrix-rust-sdk/PROGRESS.md)

| Field              | Value                                                                                                                                                                                                                              |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Last updated (UTC) | **2026-08-09**                                                                                                                                                                                                                     |
| Integration tip    | **`5fe7a40e`** — F0–F6c-3 landed: js-sdk fully removed (dep, devDeps, two-client harness, CI job); native two-client receipt+ordering proof (#627); V-BURN validation loop (`check-v-burn-complete.mjs`); 63 stale branches pruned |
| Active work        | **V-BURN COMPLETE** — V-BURN epic (`initMatrix.ts`, sole importer) awaiting operator decision ([operating-instructions.md](operating-instructions.md))                                                                             |
| Product runtime    | Native owns core D0 + V-CRYPTO + full replacement (members/presence/directory/join-rule/timeline/send/receipts); zero js-sdk anywhere                                                                                              |
| Execution model    | **prime-agent orchestrator + `deepseek-v4-flash-0731` sub-agents, max 2 concurrent** (locally hosted; only configured model) — [operating-instructions.md](operating-instructions.md)                                              |
| Import accounting  | Desktop production import files **0** / baseline **220** (**220** removed, **100%**); test import files **0**. Allowlist **0** (full ban).                                                                                         |
| Dual backend       | **`false`** (forbidden forever)                                                                                                                                                                                                    |
| Public repo        | **PUBLIC — no secrets ever**; placeholder-only examples ([operating-instructions.md](operating-instructions.md) §1)                                                                                                                |
| UI/UX fidelity     | **Preserve existing look and feel** — no UX/UI change when replacing a capability ([operating-instructions.md](operating-instructions.md) §3)                                                                                      |
| Operating model    | [cutover-operating-model.md](cutover-operating-model.md) · [full-vertical-policy.md](full-vertical-policy.md) · [operating-instructions.md](operating-instructions.md)                                                             |
| Burn board         | Retired; repository-local progress and scoreboard files are authoritative                                                                                                                                                          |
| Scoreboard         | [SCOREBOARD.md](SCOREBOARD.md)                                                                                                                                                                                                     |
| Residual queue     | [d0-residual-completion.md](d0-residual-completion.md) (historical ledger; tip honesty = scoreboard + handoff)                                                                                                                     |
| Umbrella → main    | [PR #39](https://github.com/nepenth/synara-desktop/pull/39) — **do not merge without explicit user approval**                                                                                                                      |

---

## Snapshot (read this first)

|                |                                                                                                                                                                                                                        |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Now**        | **V-BURN complete (tip `5fe7a40e`).** Production importers 114→**0** (100%), test importers **0**, allowlist **0** (full ban), `matrix-js-sdk` fully removed; native two-client receipt proof + validation loop landed |
| **Tip**        | `5fe7a40e`                                                                                                                                                                                                             |
| **Active PRs** | none                                                                                                                                                                                                                   |
| **Blocked**    | feature → main bridge (#39) awaits **explicit operator approval**; dual_backend forbidden                                                                                                                              |

---

## How this file is maintained

**Orchestrator / implementers must update this file** when:

1. A product or docs PR **merges** to the integration branch.
2. Active work **starts** (set “Now” + open PR link).
3. Priority or policy **changes** (user direction).

Update rules:

- Prepend new **Work log** entries (newest first).
- Keep **Snapshot** accurate (tip SHA, Now, open PRs).
- Prefer short bullets + PR numbers + one-line meaning.
- Do **not** claim phase-gate acceptance unless strict acceptance really closed.
- For every vertical, record native wiring **and** deleted JS files/import delta; “wired” is not “done.”
- Commit as `docs(matrix): progress log — …` on a PR or as part of the landing PR.
- Measure importers from **tip inventory**, never a stale residual worktree.

## Current policy — 2026-08-06 (operating)

Standing operating instructions govern every slice: **public-repo hygiene**
(never commit secrets — this tree is public), **execute only through this agent
harness with its locally hosted model** (orchestrator + bounded sub-agents, ≤2–3
concurrent, no external model APIs), and **preserve the app's existing look and
feel** (no UX/UI change when replacing a capability; a visual difference is a
slice defect fixed forward). See
[operating-instructions.md](operating-instructions.md). The 2026-08-03 pause was
a usage pause and is superseded: product work may resume through this harness.

## Current policy — 2026-08-03

HUMAN OPERATOR LIVE-PROOF is removed as a completion or merge gate for
residual-empty `matrix-js-sdk` burns on `feature/matrix-rust-sdk-full-replacement`.
For branch purposes, a claimed file may be accepted when the implementation is
on the measured tip, focused unit/CI checks pass, and no `matrix-js-sdk` import
remains in that claimed file. Any native product path that needs live Matrix
state must fail closed. Fix-forward and private Beta are accepted.

C3–C5 live desktop proof remains optional Beta feedback: **Not confirmed** means
the session has not been recorded, not that the branch is blocked. R-DEVTOOL may
start without waiting for C3–C5 live confirmation, subject to its native,
fail-closed implementation contract. V-BURN remains **HOLD**, `dual_backend`
remains **false** forever, and `main` / umbrella #39 remain out of scope.

If a product change changes importers, regenerate with
`npm run inventory:matrix-sdk-usage`; ratchet the allowlist `pathCount` and
`paths[]`; update inventory test floors for files, declarations, and buckets;
and update the P1.6 guardrail floors. This docs-only PR changes no production
importers and carries the tip inventory's 124 files / 124 paths unchanged.

---

## Work log (newest first)

## Work log (newest first)

### 2026-08-08 — INITMATRIX execution memo (decision required)

[INITMATRIX-execution-memo.md](INITMATRIX-execution-memo.md): the re-scoped epic as an
operator-reviewable contract. Verified state: native Rust client + sync + ~142 `matrix_*`
commands + IPC are already built/wired on this branch (ClientRoot does `matrix_restore_session`
then js-sdk `initClient`); sole js-sdk importer is `initMatrix.ts`. Remaining gap = renderer
facade + native sync-state push + token-refresh re-point + room-list live delta (staged plan).
Two operator options (approve native cutover epic, staged/CI-per-PR / legacy-loader
redefinition). Acceptance remands V-BURN HOLD pending decision.

## Work log (newest first)

## Work log (newest first)

### 2026-08-08 — INITMATRIX execution memo (decision required)

[INITMATRIX-execution-memo.md](INITMATRIX-execution-memo.md): the re-scoped epic as an
operator-reviewable contract. Verified state: native Rust client + sync + ~142 `matrix_*`
commands + IPC are already built/wired on this branch (ClientRoot does `matrix_restore_session`
then js-sdk `initClient`); sole js-sdk importer is `initMatrix.ts`. Remaining gap = renderer
facade + native sync-state push + token-refresh re-point + room-list live delta (staged plan).
Two operator options (approve native cutover epic, staged/CI-per-PR / legacy-loader
redefinition). Acceptance remands V-BURN HOLD pending decision.

### 2026-08-08 — tip `4365ca96` — burn 114→1 production, test importers 0

| When (UTC)        | Item                    | Result                                                                                                                                                                                                                                                                                                                                                                                                                                          | Notes                                                                                                                                                                                                                                                                                                                                                                                  |
| ----------------- | ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current           | **Integration tip**     | **`c6777880`**                                                                                                                                                                                                                                                                                                                                                                                                                                  | #560–#621 merged (F0–F6c-2a); provenance verified; importer 1; re-point 136→97; F6c-2b = D1C boot rewrite/drop                                                                                                                                                                                                                                                                         |
| current           | **Imports / allowlist** | **1 / 1**                                                                                                                                                                                                                                                                                                                                                                                                                                       | Production 114→**1** (`synara/src/client/initMatrix.ts` only); test import files 10→**0**; repo-wide total 4 (3 tooling = guardrail fixtures); ~99.5% removed                                                                                                                                                                                                                          |
| current           | **#560–#592**           | **merged**                                                                                                                                                                                                                                                                                                                                                                                                                                      | Command/room/state/settings/timeline/search/pin/sync/room-graph/lifecycle/crypto/call-members/SpaceTabs/read-receipts/ClientRoot/Notifications/ClientNonUIFeatures/useCall/useMessageSearch burns via sliced PRs                                                                                                                                                                       |
| current           | **#593/#594**           | **merged**                                                                                                                                                                                                                                                                                                                                                                                                                                      | V-CALL lane: CallWidgetDriver + CallEmbed + useCallEmbed burned                                                                                                                                                                                                                                                                                                                        |
| **F0 (this PR)**  | **merged**              | FACADE-contract executable spec                                                                                                                                                                                                                                                                                                                                                                                                                 |
| **F1 (#605)**     | **merged**              | NativeMatrixClient facade core: emitter + lifecycle/sync (matrix_sync_status/matrix_logout) + identity/profile (matrix_session_snapshot); D1C no-token enforced; 9/9 tests; importer still 1 (drop at F6)                                                                                                                                                                                                                                       | : Option A (complete native) + D1C (renderer cedes token custody) + slice-by-slice F0–F7; 63-method/373-hit client surface mapped vs 135 native commands; 1 importer → 0 target; PR titles carry the `1 → 0` route                                                                                                                                                                     | (4→1); all live client calls kept via derived `LocalMx`/readings + probed literals; matrix-widget-api remains a runtime dep (not a js-sdk removal target) |
| **F2 (#607)**     | **merged**              | Facade rooms+timeline: getRooms/getRoom (matrix_room_list_snapshot) + fetchRoomEvent (matrix_timeline_event_readback); 14/14 tests; importer still 1 (drop at F6)                                                                                                                                                                                                                                                                               |
| **F3 (#609)**     | **merged**              | Facade send/state: sendMessage (matrix_send_text), sendEvent (m.room.message; others GAP), sendStateEvent (m.room.name/topic/avatar); account-data + receipts documented GAP; 20/20 tests; importer still 1 (drop at F6)                                                                                                                                                                                                                        |
| **F4 (#611)**     | **merged**              | Facade media+profile: uploadContent (matrix_upload_media), getMediaConfig (matrix_call_media_config), downloadMedia (matrix_media_download), getProfileInfo; 26/26 tests; importer still 1                                                                                                                                                                                                                                                      |
| **F5 (#613)**     | **merged**              | Facade crypto+extended: getCryptoStatus/getCrypto (status-only, never key material), decryptEventIfNeeded no-op, matrixRTC/search/http/store/getCapabilities/getOpenIdToken GAP-safe; 33/33 tests; importer still 1                                                                                                                                                                                                                             |
| **F6a (#615)**    | **merged**              | Sync-read contract: synchronous read cache + async refresh(); sync getUserId/getDeviceId/getIdentity/getSyncState/getRooms/getRoom/getAccountData/mxcUrlToHttp; re-point 551 → 140; 745/745 modernization; importer still 1                                                                                                                                                                                                                     |
| **F6b (#617)**    | **merged**              | Facade completion: evented rooms (on/removeListener/getUsersReadUpTo), client self-ref, sendStateEvent real arity, setAccountData; re-point 140 → 136; 33/33 + 745/745; importer still 1                                                                                                                                                                                                                                                        |
| **F6c-1 (#619)**  | **merged**              | Facade completeness: redactEvent (matrix_timeline_redact), searchUserDirectory, queueToDevice+encryptToDeviceMessages (D1C no-op), delayed-event GAP stubs, getOpenIdTokenData; 37/37 tests; operator decision: **drop web fallback** (macOS/Linux native + iOS only) => js-sdk client vestigial everywhere; importer still 1 (F6c-2 = initMatrix D1C boot rewrite = the 1→0 drop)                                                              |
| **F6c-2a (#621)** | **merged**              | Facade completion batch 2: FacadeEventedRoomReading (EventedRoomReading structural match: client/on/removeListener/getUsersReadUpTo/findEventById/hasEncryptionStateEvent), ReceiptClientReading completions (setRoomReadMarkers/sendReadReceipt/getLatestTimeline), method stubs (getUser/getThreePids/getPushers/setPusher/aliases/cancelUpload/getBaseUrl); D1C: NO getOwnDeviceKeys (test-locked); re-point 136→97; 40/40; importer still 1 |
| current           | **#595**                | **merged**                                                                                                                                                                                                                                                                                                                                                                                                                                      | Test-import burn: 10 test fixtures to probed literals/local structurals (712/712 modernization unchanged); `initMatrix` MatrixError contract duck-typed (`isMatrixErrorLike`, Error & { errcode?: string }); test importers **0**                                                                                                                                                      |
| current           | **V-BURN**              | **HOLD**                                                                                                                                                                                                                                                                                                                                                                                                                                        | Sole importer is the epic: `initMatrix` = live `createClient`/IndexedDBStore/login/sync/token-refresh bootstrap (every derived client type hangs off `typeof initClient`); no native `initClient` exists. `matrix-js-sdk@42.0.0` removal = exactly one epic away (no transitive dependents). Waits on operator INITMATRIX decision (native-bootstrap epic vs sanctioned legacy-loader) |
| current           | **Gates**               | **green**                                                                                                                                                                                                                                                                                                                                                                                                                                       | tsc 0 · node --test 292/0 · guardrails PASS allowlist 1 · modernization 712/0 · prettier clean                                                                                                                                                                                                                                                                                         |
| current           | **Operators**           | **needed**                                                                                                                                                                                                                                                                                                                                                                                                                                      | INITMATRIX decision; luna provider visibility for the max-thinking epic lane (not visible to this session's model list)                                                                                                                                                                                                                                                                |

### 2026-08-06 — operating instructions — public repo + this harness + UI/UX fidelity

| When (UTC) | Item                   | Result                   | Notes                                                                                                                                                                                                                                                                                  |
| ---------- | ---------------------- | ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Operating model**    | **This harness**         | Orchestrator + sub-agents on the locally hosted model (≤2–3 concurrent); no external model APIs. Historical pause superseded; resume authorized. See [operating-instructions.md](operating-instructions.md).                                                                           |
| current    | **Public repo policy** | **Mandatory**            | Never commit secrets/tokens/keys/credentials/recovery material/private endpoints/personal data; placeholder-only public examples. Wired into README, plan §3.7, full-vertical-policy acceptance gates, SCOREBOARD, product-lane-protocol, pause-handoff.                               |
| current    | **UI/UX fidelity**     | **Mandatory**            | Replacing a capability must preserve exact look and feel — no redesign, layout/UX/copy change, or rendering-altering component swap; a visual diff is a slice defect → named residual → fix forward. Added as a full-vertical acceptance gate + plan §3.7 + operating instructions §3. |
| current    | **Go-forward (docs)**  | SCOREBOARD "Left" item 1 | Long-tail residual-empty importer burn (RoomJoinRules writer, useMessageSearch, utils/room.ts, timeline/media listeners, CallWidget media IPC, initMatrix/cryptoStoreContinuity, R-DEVTOOL). V-BURN stays HOLD.                                                                        |

### 2026-08-03 — tip `57ab9e64` — #546 land + usage pause

| When (UTC) | Item                    | Result                                                     | Notes                                                                                                                           |
| ---------- | ----------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**     | **`ed52a787`**                                             | #559 timeline links/opening (113→111)                                                                                           |
| current    | **Imports / allowlist** | **114 / 114**                                              | Baseline 220 → **106** removed (~48.2%)                                                                                         |
| current    | **#544**                | **merged**                                                 | Live-proof not residual-empty merge gate                                                                                        |
| current    | **#546**                | **merged**                                                 | −10 production importers (stack superseding #540–#545)                                                                          |
| current    | **Beta packages**       | **success**                                                | [Desktop Package Smoke](https://github.com/nepenth/synara-desktop/actions/runs/30821912637) macOS + Arch + .deb artifacts @ tip |
| current    | **Pipeline**            | **PAUSED**                                                 | Conserve weekly Grok/Codex usage; no new agent spawns                                                                           |
| current    | **Handoff**             | [pause-handoff-2026-08-03.md](pause-handoff-2026-08-03.md) | Resume checklist + residual notes                                                                                               |

### 2026-08-03 — tip `abd736b3` — remove live-proof merge gates

| When (UTC) | Item          | Result                        | Notes                                                                                                                          |
| ---------- | ------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| current    | **Policy**    | **Updated**                   | HUMAN OPERATOR LIVE-PROOF is optional Beta feedback, not a completion or merge gate for residual-empty burns.                  |
| current    | **C3–C5**     | **Tip/unit/CI path accepted** | Live desktop proof may remain **Not confirmed**; it is not a hold when the engineering evidence is on tip.                     |
| current    | **R-DEVTOOL** | **Eligible to start**         | No longer waits for C3–C5 live confirmation; native UI → Tauri IPC → live `matrix-sdk` and fail-closed rules remain mandatory. |
| current    | **Inventory** | **Unchanged**                 | No production code/importers changed; committed tip inventory remains 124 files / 124 allowlist paths.                         |
| current    | **Scope**     | **Docs only**                 | One product PR draft; `main`, #39, `dual_backend`, and V-BURN status unchanged.                                                |

### 2026-08-03 — tip `80af6ce7` — pause + long-tail burn audit

| When (UTC)    | Item                    | Result                                                     | Notes                                                                       |
| ------------- | ----------------------- | ---------------------------------------------------------- | --------------------------------------------------------------------------- |
| current       | **Integration tip**     | **`80af6ce7`**                                             | After #538 MsgType presentation residual-empty                              |
| current       | **Imports / allowlist** | **124 / 124**                                              | Baseline 220 → **96** removed (~43.6%)                                      |
| current       | **Pipeline**            | **PAUSED**                                                 | Daytime scheduler cancelled; overnight OFF; no new spawns                   |
| current       | **Wrap-up**             | **#538 merged**                                            | Final in-flight product; docs freezes #502–#512 closed as stale             |
| current       | **Handoff**             | [pause-handoff-2026-08-03.md](pause-handoff-2026-08-03.md) | Resume checklist + residual notes                                           |
| 2026-08-02→03 | **Product chain**       | **#514–#538**                                              | Members/tags/presence/directory natives + long-tail type/presentation kills |

### 2026-08-02 — tip `cf79f975` — post-#490/#496/#497 scoreboard/progress tip honesty (superseded)

| When (UTC) | Item        | Result         | Notes                                                                                 |
| ---------- | ----------- | -------------- | ------------------------------------------------------------------------------------- |
| historical | **Tip pin** | **Superseded** | Historical docs-only pin; do not treat as current tip. See 2026-08-03 snapshot above. |

### 2026-08-02 — tip `27a854d8` — directory packet tip honesty

| When (UTC) | Item                           | Result                | Notes                                                                                                                                                                                   |
| ---------- | ------------------------------ | --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**            | **Current**           | #478 is merged at `27a854d8`; the directory packet/residual retain `c1e9c3be` as the #461 product baseline and record the current docs refresh tip. `main` and #39 remain out of scope. |
| current    | **Directory vertical**         | **First slice / WIP** | #461's first slice remains the only directory product claim; native route wiring and route-scoped JS-owner deletion are landed, while full vertical closure remains open.               |
| current    | **Proof / burn / merge gates** | **Held**              | Directory live proof and acceptance remain **Not confirmed**; C3–C5 remain **Not confirmed**; V-BURN remains **HOLD / Not ready**; `dual_backend` is forbidden.                         |

### 2026-08-02 — tip `3980f0e0` — #465 and #469 docs-only refreshes landed

| When (UTC) | Item                           | Result      | Notes                                                                                                                                                                                  |
| ---------- | ------------------------------ | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**            | **Current** | #469 R-DEVTOOL landed at `8e14726a`; #465 members READ landed as squash commit `3980f0e0`. Both target `feature/matrix-rust-sdk-full-replacement`; `main` and #39 remain out of scope. |
| current    | **Scoreboard / progress**      | **Refresh** | #478 is replayed on the current target so `PROGRESS.md` and `SCOREBOARD.md` retain current-tip truth without overwriting the landed docs refreshes.                                    |
| current    | **Proof / burn / merge gates** | **Held**    | C3–C5 remain **Not confirmed**; R-DEVTOOL remains gated; V-BURN remains **HOLD**; `dual_backend` is forbidden.                                                                         |

### 2026-08-02 — tip `60141c8b` — rebase #465 onto current docs tip

| When (UTC) | Item                           | Result        | Notes                                                                                                                                                               |
| ---------- | ------------------------------ | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**            | **Draft**     | Rebased onto `60141c8b`; the product state from `c1e9c3be` is unchanged while the V-BURN and C3–C5 docs refreshes are included. `main` and #39 remain out of scope. |
| current    | **#465 rejection repairs**     | **Preserved** | Section 5 is complete; the historical `52953091` planning base is labeled as historical; custom tag and direct helper/plugin READs remain residual.                 |
| current    | **Proof / burn / merge gates** | **Held**      | C3–C5 remain **Not confirmed**; V-BURN remains **HOLD**; `dual_backend` is forbidden; keep this PR draft/unmerged.                                                  |

### 2026-08-02 — tip `c1e9c3be` — rebase #465 after merged #461

| When (UTC) | Item                           | Result                        | Notes                                                                                                                                             |
| ---------- | ------------------------------ | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**            | **This docs-only draft**      | Based at `c1e9c3be`; #461's room-directory first slice is merged. `main` and #39 remain out of scope.                                             |
| current    | **Members / power READ**       | **Residual boundary held**    | #450 native power/creator owners remain closed for migrated paths; custom tag metadata and direct helper/plugin reads remain explicitly residual. |
| current    | **#465 refresh**               | **Rebased / repair required** | Replayed the six-file docs refresh onto `c1e9c3be`; resolved the `SCOREBOARD.md` tip conflict and repaired the rejected Section 5 truncation.     |
| current    | **Proof / burn / merge gates** | **Held**                      | C3–C5 remain **Not confirmed**; V-BURN remains **HOLD**; `dual_backend` is forbidden; keep this PR draft/unmerged.                                |

### 2026-08-02 — tip `d82e043d` — refresh after #450/#458; #465 tip honesty

| When (UTC) | Item                           | Result                      | Notes                                                                                                                                                                      |
| ---------- | ------------------------------ | --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**            | **This docs-only draft**    | Based at `d82e043d`; #450 and #458 are merged. `main` and #39 remain out of scope.                                                                                         |
| current    | **Power/creator READ**         | **Merged #450**             | Native power-level and creator snapshots back the migrated hook/permission paths with fail-closed native ownership; no JS state-event fallback on native sessions.         |
| current    | **Remaining READ residual**    | **Active**                  | No native `in.synara.room.power_level_tags` snapshot; custom tag metadata plus direct `via-servers` / `utils/room.ts` power/create readers remain explicitly tracked.      |
| current    | **Presence / directory**       | **#458 merged / #461 open** | #458's presence first slice is landed but not product-complete; #461 remains a hot WIP directory vertical with proof and acceptance open.                                  |
| current    | **#465 draft tip**             | **Refresh required**        | Prior head `3d7c5f42` was based at `103a653f`; the post-#458 replay conflicts in `SCOREBOARD.md`. This branch records `d82e043d` honestly and does not claim mergeability. |
| current    | **Proof / burn / merge gates** | **Held**                    | C3–C5 remain **Not confirmed**; V-BURN remains **HOLD**; `dual_backend` is forbidden; hold merge while #461 is hot.                                                        |

### 2026-08-01 — tip `206d24f3` — presence/directory WIP after #446 extraction

| When (UTC) | Item                          | Result                   | Notes                                                                                                                                   |
| ---------- | ----------------------------- | ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**           | **This docs-only draft** | Based at `206d24f3`; #446 product-command fan-out is merged. `main` and #39 remain out of scope.                                        |
| current    | **Presence product vertical** | **WIP in parallel**      | Implementation agent is in flight; packet, product wiring, JS-owner deletion, focused proof, live proof, and acceptance remain open.    |
| current    | **Room directory vertical**   | **WIP in parallel**      | Implementation agent is in flight; packet, product wiring, JS-owner deletion, focused proof, live proof, and acceptance remain open.    |
| current    | **Serial/parallel boundary**  | **Parallel**             | #446 removes the shared `product.rs` fan-out chokepoint for these module-owned slices; no packet or extraction is a product-done claim. |
| current    | **Proof / burn gates**        | **Held**                 | C3–C5 remain **Not confirmed**; V-BURN remains **HOLD**; `dual_backend` is forbidden.                                                   |

### 2026-08-01 — tip `9fb341af` — #439/#446 merged; parallel module fan-out next

| When (UTC) | Item                        | Result                | Notes                                                                                                                                                                                                  |
| ---------- | --------------------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| current    | **Integration tip**         | **This draft**        | Docs-only refresh based at `9fb341af`; #405, #407, #438, #439, and #446 are present in the tip. This draft does not touch `product.rs` or any product code.                                            |
| current    | **Powers-bulk writes**      | **Merged #439**       | Native bulk room power-level and tag writes are present at the tip; remaining power-level/creator reads are not closed by this write slice.                                                            |
| current    | **Product command fan-out** | **Merged #446**       | The behavior-preserving `product.rs` extract/split is merged into domain `product_commands.rs` modules. This changes the ownership/layout boundary, not the completion status of downstream verticals. |
| current    | **Next module fan-out**     | **Ready in parallel** | Power-level READ, presence, room directory, and other domain slices can proceed in module-scoped lanes; do not reopen shared `product.rs` as a serial lane.                                            |
| current    | **Import accounting**       | **152**               | Committed tip inventory reports 152 production `matrix-js-sdk` import files and 152 allowlist entries; #446 does not change the TypeScript inventory.                                                  |
| current    | **Proof / burn gates**      | **Held**              | C3–C5 remain **Not confirmed**; V-BURN remains **HOLD**; `dual_backend` is forbidden and #39/main remain out of scope.                                                                                 |

### 2026-08-01 — docs base `52953091` — post-#405 residual boundary

| When (UTC) | Item                      | Result               | Notes                                                                                                                            |
| ---------- | ------------------------- | -------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration base**      | **This draft**       | Docs-only refresh based at `52953091`; product PR #405 is not merged into this base.                                             |
| current    | **Members drawer wiring** | **Post-#405 target** | Product head `cd2d57b4` wires drawer/lobby/mentions member snapshots; after merge those member reads are done on native desktop. |
| current    | **Power/creator reads**   | **Residual**         | #405 does not add native power-level or creator snapshots; those remain the next members-read slice.                             |
| current    | **Policy**                | **Held**             | `dual_backend=false`; native paths remain fail-closed; #39 remains gated and `main` is untouched.                                |

### 2026-08-01 — base tip `8330c56b` — #405/#407 acceptance status

| When (UTC) | Item                      | Result                      | Notes                                                                                                                                                                                 |
| ---------- | ------------------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**       | **This PR**                 | Tip `8330c56b` on `feature/matrix-rust-sdk-full-replacement`; the docs-only base includes the #410 V-BURN-blocker update.                                                             |
| current    | **Members drawer wiring** | **#405 `ACCEPT`**           | CI may re-run; the drawer/lobby/mentions product wiring is not merged at this tip.                                                                                                    |
| current    | **CallWidget media IPC**  | **#407 `ACCEPT_WITH_NITS`** | Full-green proof is recorded at [`cd07f4fc`](https://github.com/nepenth/synara-desktop/commit/cd07f4fc), behind this tip; parent merge pending and product wiring is not merged here. |
| current    | **Import accounting**     | **152**                     | Production import files and allowlist remain **152**; these in-flight PRs do not change the accounting at this docs-only tip.                                                         |
| current    | **Policy**                | **Held**                    | `dual_backend=false`; V-BURN remains **HOLD**; #39 remains gated and `main` is untouched.                                                                                             |

### 2026-08-01 — tip `1c9653b2` — docs after #397/#398

| When (UTC) | Item                        | Result        | Notes                                                                                                                           |
| ---------- | --------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**         | **This PR**   | Tip `1c9653b2` on `feature/matrix-rust-sdk-full-replacement`; #397 tip honesty and #398 importer taxonomy alignment are merged. |
| current    | **Import accounting**       | **152**       | Production import files and allowlist remain **152**; taxonomy measurement remains anchored to the #395 product tip.            |
| current    | **CallWidget media IPC**    | **In flight** | Native `getMediaConfig`/`downloadFile` implementation follows packet #396; this PR is docs-only and edits no `product.rs`.      |
| current    | **Members-drawer residual** | **Next**      | Room/MembersDrawer/Lobby/UserMentionAutocomplete member reads plus power-level/creator reads remain residual after #395.        |

### 2026-08-01 — tip `22f1f06d` — members-read first slice #395

| When (UTC) | Item                         | Result                                 | Notes                                                                                                                                                                                 |
| ---------- | ---------------------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**          | **This PR**                            | Tip `22f1f06d` on `feature/matrix-rust-sdk-full-replacement`. Imports/allowlist **152** (ratcheted 153→152).                                                                          |
| current    | **Members-read first slice** | **Merged #395**                        | `matrix_room_members_snapshot` IPC + Members settings native fail-closed; drawer/lobby/mentions + power-level/creator reads residual.                                                 |
| current    | **Next**                     | **powers-bulk / CallWidget media IPC** | members-read residual (Room/MembersDrawer/Lobby/UserMentionAutocomplete + power reads); powers-bulk writes (packet #388); CallWidget media IPC (packet #396). V-BURN HOLD; #39 gated. |

### 2026-08-01 — tip `96015ccd` — docs honesty after #375/#386/#387

| When (UTC) | Item                      | Result           | Notes                                                                                                                |
| ---------- | ------------------------- | ---------------- | -------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**       | **This PR**      | Tip `96015ccd` on `feature/matrix-rust-sdk-full-replacement`. Imports/allowlist **153**.                             |
| current    | **Moderation writes**     | **Merged #375**  | Native invite/kick/ban/unban/setPowerLevel write vertical landed; marked done on scoreboard.                         |
| current    | **Tip honesty**           | **Merged #386**  | Scoreboard honesty after moderation writes #375.                                                                     |
| current    | **CallWidget media scan** | **Merged #387**  | Docs scan of CallWidget media download reuse landed; product `getMediaConfig`/`downloadFile` IPC remains residual.   |
| current    | **Next**                  | **members-read** | members-read product (#385 inventory); powers-bulk; CallWidget media IPC; C3–C5 live proofs. V-BURN HOLD; #39 gated. |

### 2026-08-01 — tip `a53f14fa` — room create #372

| When (UTC) | Item                | Result                                        | Notes                                                                 |
| ---------- | ------------------- | --------------------------------------------- | --------------------------------------------------------------------- |
| current    | **Integration tip** | **This PR**                                   | Tip `a53f14fa` after #372 create vertical.                            |
| current    | **Room create**     | **#372**                                      | `matrix_room_create` + CreateRoom/Space/Chat + `/create` fail-closed. |
| current    | **Lifecycle**       | **Leave #364/#371 · Join #369 · Create #372** | Next: members/power; C3–C5 live; V-BURN HOLD.                         |
| current    | **Imports**         | **153**                                       | Allowlist matched on product PR inventory.                            |

### 2026-08-01 — tip `ab3997fa` after #369 room join

| When (UTC) | Item                   | Result                  | Notes                                                                                                                        |
| ---------- | ---------------------- | ----------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**    | **This PR**             | Tip `ab3997fa`. Imports/allowlist **153**.                                                                                   |
| current    | **Room join vertical** | **Merged #369**         | `matrix_room_join` + native owner; RoomCard/Intro/lobby/tombstone/`/join` fail-closed; **155→154**. Create residual remains. |
| current    | **Next**               | **createRoom vertical** | CreateChat/CreateRoom/CreateSpace/`/create`; `/leave` command residual uses `mx.leave`. V-BURN HOLD; #39 gated.              |

### 2026-08-01 — tip `5f202231` after #365/#363/#362/#364

| When (UTC) | Item                                  | Result          | Notes                                                                                             |
| ---------- | ------------------------------------- | --------------- | ------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**                   | **This PR**     | Tip `5f202231` on `feature/matrix-rust-sdk-full-replacement`. Imports/allowlist **153**.          |
| current    | **V-SEND.R-PACK-READ finish-line #1** | **Merged #365** | Delete JS image pack read helpers + web fallback; equality helpers retained.                      |
| current    | **Composer GIF/upload fallbacks**     | **Merged #363** | RoomInput GIF + msgContent thumbnail fail-closed native-only; no dual_backend.                    |
| current    | **CallWidget residual**               | **Merged #362** | `getKnownRooms` native room-list snapshot; media config/download fail closed.                     |
| current    | **Room leave vertical**               | **Merged #364** | `matrix_room_leave` + LeaveRoom/LeaveSpace native owner; leave prompts drop js-sdk (**157→155**). |
| current    | **Scoreboard honesty**                | **This PR**     | Reorder finish-line: leave done; next join/create. V-BURN HOLD; #39 gated.                        |

### 2026-08-01 — tip `e4cf1a5f` — P1.6 allowlist/live-import gap

| When (UTC) | Item                                     | Result                        | Notes                                                                                                                                                                                                                                        |
| ---------- | ---------------------------------------- | ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**                      | **This PR**                   | Tip `e4cf1a5f` on `feature/matrix-rust-sdk-full-replacement`; no product code or residual state changed.                                                                                                                                     |
| current    | **Allowlist/live-import reconciliation** | **NOOP — docs-only evidence** | The committed P1.6 allowlist has `pathCount: 163`; the generated desktop-runtime inventory has `productionImportFiles: 159`. The four-path difference is a set difference, not a deletion delta.                                             |
| current    | **Allowlist policy**                     | **Preserved at 163**          | All four paths below remain present and tracked at this tip, so there is no proof supporting allowlist ratcheting. New production importers remain fail-closed; `dual_backend` stays forbidden, V-BURN/#327 stays held, and #39 stays gated. |

The allowlisted entries with no live direct `matrix-js-sdk` importer are:

- `synara/src/app/pages/client/inbox/Invites.tsx`
- `synara/src/app/state/room-list/inviteList.ts`
- `synara/src/app/utils/later.ts`
- `synara/src/app/utils/roomNotes.ts`

The comparison uses the inventory’s `productionImportFiles` field, not its broader
production `fileCount`: the latter also records two production-only networking
findings (`synara/src/app/cs-api.ts` and `synara/src/sw.ts`) that are not SDK
importers and are outside the allowlist set difference.

### 2026-08-01 — tip `13d5f6cf` after #349 (prior #347)

| When (UTC) | Item                     | Result                                         | Notes                                                                                                                                                                                                                                                                             |
| ---------- | ------------------------ | ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**      | **This PR**                                    | Tip `13d5f6cf` on `feature/matrix-rust-sdk-full-replacement`; #349 CallWidget residual inventory is merged after #347 at `9eb4689b`. Imports **159**; allowlist **163**.                                                                                                          |
| current    | **V-SEND.R-CALL-UPLOAD** | **Merged #349 — docs-only residual inventory** | #349 records the upload owner as native-first and fail-closed from #328, with `getMediaConfig`, `downloadFile`, and `getKnownRooms` retained as documented widget-adjacent JS residuals. No product code, `dual_backend`, or V-BURN state changed; #327 remains **HOLD forever**. |
| prior      | **PROGRESS**             | **Merged #347**                                | Tip `9eb4689b` after #341–#343; C3–C5 live proofs remained **Not confirmed** and R-DEVTOOL remained gated.                                                                                                                                                                        |

### 2026-08-01 — tip `3a71f482` after #341–#343

| When (UTC) | Item                    | Result                      | Notes                                                                                                                                                                                                                        |
| ---------- | ----------------------- | --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip**     | **This PR**                 | Tip `3a71f482`. #341 DevTool implementation gate, #342 narrowed residual queue, and #343 upload audit reconciliation are merged on `feature/matrix-rust-sdk-full-replacement`. Imports **159**; allowlist **163**.           |
| current    | **V-SEND.R-DEVTOOL**    | **Merged #341 — gate only** | Start only after V-TIMELINE.C3–C5 live proofs are confirmed. Native implementation must be UI → Tauri IPC → live `matrix-sdk`, fail-closed on missing/failed native state, and never add a backend selector or dual backend. |
| current    | **Residual queue**      | **Narrowed #342**           | Pack-read JS helper deletion remains V-BURN-gated; C3–C5 live proofs remain **Not confirmed**; R-DEVTOOL remains gated. #327 V-BURN stays **HOLD forever this cycle** and is not started.                                    |
| current    | **V-SEND upload audit** | **Reconciled #343**         | The audit records native-first, fail-closed upload owners and fallback-only JS reachability; it opens no new native-session residual. Audit measured tip remains `4d1240e3`; this progress log is current at `3a71f482`.     |

### 2026-08-01 — tip honesty after #331 GIF-PACK NOOP (#332)

| When (UTC) | Item                | Result      | Notes                                                                                                                                                     |
| ---------- | ------------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Integration tip** | **This PR** | Tip `27dd03f6`. #331 GIF-PACK NOOP, #325 thumbnails, and #328 call-upload are landed; #320 pack room ids leaves production imports at **159**. #39 gated. |

### 2026-08-01 — V-SEND.R-GIF-PACK residual check

| When (UTC) | Item                  | Result               | Notes                                                                                                                                                                                                                                                                 |
| ---------- | --------------------- | -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.R-GIF-PACK** | **NOOP (docs-only)** | `gifProvider.ts:3-22,175-239` and `GifPicker.tsx:38-60,148-171` provide provider search/selection/download only; selected GIF send is native via `nativeSendGifOwner.ts:17-52` and `RoomInput.tsx:899-923`. No GIF pack/collection surface exists. #39 remains gated. |

### 2026-08-01 — scoreboard after #320 pack room ids + #328 call-upload

| Field   | Value                  |
| ------- | ---------------------- | ----------- | ------------------------------------------------------------------------------------------------- |
| current | **Scoreboard honesty** | **This PR** | Tip `324c40a4`. Imports **159**. #320 room ids + #328 R-CALL-UPLOAD landed. #325 open. #39 gated. |

### 2026-08-01 — scoreboard after #318 pack-read subscribe

| When (UTC) | Item           | Result      | Notes                                                                                                     |
| ---------- | -------------- | ----------- | --------------------------------------------------------------------------------------------------------- |
| current    | **Scoreboard** | **This PR** | Honesty after #318 pack-read subscribe. Tip `95ad2656`. JS utils / useImagePackRooms residual. #39 gated. |

### 2026-08-01 — V-SEND.R-PACK-READ subscribe

| When (UTC) | Item                   | Result          | Notes                                                                                                                                                                            |
| ---------- | ---------------------- | --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.R-PACK-READ** | **Merged #318** | Session `NativeImagePackOwner` listens for ponies account-data/state; emits `matrix-image-packs-updated`; hooks re-snapshot via existing get IPC (fail-closed). No dual_backend. |

### 2026-08-01 — scoreboard after #314 PACK-UPLOAD

| When (UTC) | Item           | Result      | Notes                                                                                                                                                        |
| ---------- | -------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| current    | **Scoreboard** | **This PR** | Honesty after #314 PACK-UPLOAD. Tip `25bfa150`. CompactUploadCardRenderer reuses `matrix_upload_media` fail-closed; pack-read subscribe residual. #39 gated. |

### 2026-08-01 — V-SEND.R-PACK-UPLOAD

| When (UTC) | Item                     | Result          | Notes                                                                                                                                                                             |
| ---------- | ------------------------ | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.R-PACK-UPLOAD** | **Merged #314** | Desktop compact media upload (pack images/avatars + RoomProfile/PowersEditor) fail-closed via `matrix_upload_media`; never falls through to `mx.uploadContent` on native session. |

### 2026-08-01 — scoreboard after R-ROOM-PROFILE #313

| When (UTC) | Item           | Result      | Notes                                                                                                                                                                                                                 |
| ---------- | -------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Scoreboard** | **This PR** | Honesty after R-ROOM-PROFILE #313. Tip `4ce64909`. Room name/topic/avatar native fail-closed; pack-write personal #306 + global #309 + room #310 landed. **PACK-UPLOAD** residual remains (open **#314**). #39 gated. |

### 2026-08-01 — V-SEND.R-ROOM-PROFILE room profile

| When (UTC) | Item                      | Result          | Notes                                                                                                                                                                                                                                   |
| ---------- | ------------------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.R-ROOM-PROFILE** | **Merged #313** | Native `matrix_set_room_name`/`matrix_set_room_topic`/`matrix_set_room_avatar` + RoomProfile.tsx fail-closed on desktop; JS `sendStateEvent` only for non-native web. Room-avatar media upload residual remains (PACK-UPLOAD-adjacent). |

### 2026-08-01 — scoreboard after #310 room pack-write

| When (UTC) | Item           | Result      | Notes                                                                                                                                    |
| ---------- | -------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Scoreboard** | **This PR** | Honesty after #310 room pack-write. Tip `980231f7`. Personal #306 + global #309 + room #310 landed; **PACK-UPLOAD** residual. #39 gated. |

### 2026-08-01 — V-SEND.R-PACK-WRITE room packs

| When (UTC) | Item                    | Result          | Notes                                                                                                                                         |
| ---------- | ----------------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.R-PACK-WRITE** | **Merged #310** | Native `matrix_set_room_image_pack` (`im.ponies.room_emotes`) + RoomPacks/RoomImagePack fail-closed on desktop; PACK-UPLOAD residual remains. |

### 2026-08-01 — scoreboard after #309 global pack-write

| When (UTC) | Item           | Result      | Notes                                                                                                                           |
| ---------- | -------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Scoreboard** | **This PR** | Honesty after #309 global pack-write. Tip `de13b048`. Personal #306 + global #309 landed; room/PACK-UPLOAD residual. #39 gated. |

### 2026-08-01 — V-SEND.R-PACK-WRITE global packs

| When (UTC) | Item                    | Result          | Notes                                                                                                                                        |
| ---------- | ----------------------- | --------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.R-PACK-WRITE** | **Merged #309** | Native `matrix_set_global_image_packs` (`im.ponies.emote_rooms`) + GlobalPacks.tsx fail-closed on desktop; room pack write residual remains. |

### 2026-08-01 — scoreboard after #306 pack-write personal

| When (UTC) | Item           | Result      | Notes                                                                                                |
| ---------- | -------------- | ----------- | ---------------------------------------------------------------------------------------------------- |
| current    | **Scoreboard** | **This PR** | Honesty after #306 personal pack-write. Tip `b21578e9`. Global/room/PACK-UPLOAD residual. #39 gated. |

### 2026-08-01 — V-SEND.R-PACK-WRITE personal pack

| When (UTC) | Item                    | Result          | Notes                                                                                                         |
| ---------- | ----------------------- | --------------- | ------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.R-PACK-WRITE** | **Merged #306** | Native `matrix_set_user_image_pack` + UserImagePack.tsx fail-closed; global/room pack write residual remains. |

### 2026-08-01 — scoreboard after #303 avatar

| When (UTC) | Item           | Result      | Notes                                                                                                                                     |
| ---------- | -------------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Scoreboard** | **This PR** | Honesty after #303 V-SEND.R-AVATAR-UPLOAD user profile. Tip `c42b93c5`. R-ROOM-PROFILE residual. Next: pack-write / subscribe. #39 gated. |

### 2026-08-01 — V-SEND.R-AVATAR-UPLOAD user profile writes

| When (UTC) | Item                       | Result      | Notes                                                                                                                                                      |
| ---------- | -------------------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.R-AVATAR-UPLOAD** | **This PR** | Native `matrix_upload_media` + `matrix_set_own_avatar` + `matrix_set_own_display_name`; Profile.tsx fail-closed on desktop; room profile residual remains. |

### 2026-08-01 — V-SEND.R-PACK-READ residual truth-up after #297

| When (UTC) | Item                   | Result      | Notes                                                                                                                                                                                                                                                                                                                         |
| ---------- | ---------------------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.R-PACK-READ** | **This PR** | Docs-only residual truth-up after #297 snapshot. Snapshot get DONE; remaining: `matrix_subscribe_image_packs` live push, physical delete of read-only `custom-emoji/utils.ts` helpers (write side #292 unaffected), JS `useImagePackRooms` room resolution. See [v-send-pack-read-residual.md](v-send-pack-read-residual.md). |

### 2026-08-01 — scoreboard after #297 pack-read

| When (UTC) | Item           | Result      | Notes                                                                                                                                                          |
| ---------- | -------------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Scoreboard** | **This PR** | Honesty after #297 pack-read snapshot + #298 residual truth + #296 forward. Tip `83adef68`. Imports **163**. Next: pack-write / subscribe / avatar. #39 gated. |

### 2026-08-01 — V-SEND.R-PACK-READ implement

| When (UTC) | Item                   | Result          | Notes                                                                                                                           |
| ---------- | ---------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.R-PACK-READ** | **Merged #297** | Native `matrix_get_*_image_packs` + TS owner + `useImagePacks` fail-closed. Subscribe residual remains. dual_backend forbidden. |

### 2026-08-01 — V-SEND.R-FORWARD residual close

| When (UTC) | Item                 | Result      | Notes                                                                                                                                                                  |
| ---------- | -------------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.R-FORWARD** | **This PR** | Delete legacy `MessageForwardItem` + `utils/forward.ts` after C1/C2; native presenter forward sole product path; allowlist **168→167**; prod import files **164→163**. |

### 2026-08-01 — scoreboard after C1+C2

| When (UTC) | Item           | Result      | Notes                                                                                                                               |
| ---------- | -------------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Scoreboard** | **This PR** | Honesty after #285 C1 + #289 C2. Tip `8995add1`. Production import files **164**. Next: pack-read implement / R-FORWARD. #39 gated. |

### 2026-08-01 — V-TIMELINE.C2 delete RoomTimeline

| When (UTC) | Item              | Result          | Notes                                                                                                                            |
| ---------- | ----------------- | --------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-TIMELINE.C2** | **Merged #289** | Delete `RoomTimeline.tsx`/`RoomTimeline.css.ts` after C1 sole owner; allowlist 169→168; imports 165→164. dual_backend forbidden. |

### 2026-08-01 — scoreboard after #290–#292 inventories

| When (UTC) | Item           | Result          | Notes                                                                                                                                                           |
| ---------- | -------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Scoreboard** | **Merged #293** | Docs-only SCOREBOARD/PROGRESS honesty after inventories + #283/#294 tip. Production import files **165**. In flight: #289 C2 (C1 #285 merged). #39 still gated. |

### 2026-08-01 — V-TIMELINE.C3 stream verify checklist

| When (UTC) | Item              | Result          | Notes                                                                                                                                           |
| ---------- | ----------------- | --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-TIMELINE.C3** | **Merged #294** | Docs-only re-verify checklist for native stream/delta after C1/C2: S1–S7. See [v-timeline-c3-stream-verify.md](v-timeline-c3-stream-verify.md). |

### 2026-08-01 — V-TIMELINE.C4/C5 verify checklists

| When (UTC) | Item                 | Result                   | Notes                                                                                                                                                                                                                                                                                                 |
| ---------- | -------------------- | ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-TIMELINE.C4/C5** | **Docs-only checklists** | C4 media/render parity + C5 pins/notes/jump live-proof checklists mirroring C3. No product code; live proof still unclaimed. See [v-timeline-c4-media-render-verify.md](v-timeline-c4-media-render-verify.md) and [v-timeline-c5-pins-notes-jump-verify.md](v-timeline-c5-pins-notes-jump-verify.md). |

### 2026-08-01 — V-SEND.R-AVATAR-UPLOAD residual inventory

| When (UTC) | Item                       | Result          | Notes                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ---------- | -------------------------- | --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.R-AVATAR-UPLOAD** | **#291 merged** | Docs-only inventory of user + room avatar **upload** residual (grouped with R-ROOM-PROFILE): `Profile.tsx` (`mx.setAvatarUrl`/`mx.setDisplayName`), `RoomProfile.tsx` (`mx.sendStateEvent` m.room.avatar/name/topic), shared `state/upload.ts` + `utils/matrix.ts` `uploadContent` → `mx.uploadContent`. No native avatar IPC; native media upload only inside `matrix_send_attachment`. See [v-send-avatar-residual.md](v-send-avatar-residual.md). |

### 2026-08-01 — V-TIMELINE.C1 presenter cutover

| When (UTC) | Item              | Result          | Notes                                                                                                             |
| ---------- | ----------------- | --------------- | ----------------------------------------------------------------------------------------------------------------- |
| current    | **V-TIMELINE.C1** | **Merged #285** | `RoomView` mounts `NativeTimelinePresenter` only (sole owner). C2 deletes `RoomTimeline`. dual_backend forbidden. |

### 2026-08-01 — V-SEND.R-PACK-WRITE residual inventory

| When (UTC) | Item                    | Result          | Notes                                                                                                                                                                                              |
| ---------- | ----------------------- | --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.R-PACK-WRITE** | **Merged #292** | Docs-only inventory of sticker/emoji pack **write** residual (send native #264; read #287): pack settings write + PACK-UPLOAD. See [v-send-pack-write-residual.md](v-send-pack-write-residual.md). |

### 2026-08-01 — V-SEND.R-FORWARD residual inventory

| When (UTC) | Item                 | Result          | Notes                                                                                                                                                                                                                                                                                                                                |
| ---------- | -------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| current    | **V-SEND.R-FORWARD** | **#290 merged** | Docs-only inventory of message **forward** residual: send is already native (`matrix_timeline_forward_text`/`matrix_timeline_forward_media`); residual is the legacy `MessageForwardItem` dialog in `Message.tsx` + `utils/forward.ts` read helpers on live JS client. See [v-send-forward-residual.md](v-send-forward-residual.md). |

### 2026-08-01 — progress honesty scoreboard

| When (UTC) | Item                    | Result                                                                                | Notes                                                       |
| ---------- | ----------------------- | ------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| current    | **Integration tip**     | `310f4487`                                                                            | After #291 avatar + #290 forward inventory; #288 scoreboard |
| current    | **Auth residual**       | **loginUtil DONE #279**; UIA multi-stage non-retention #280; discovery #276           | Desktop password fail-closed native-only                    |
| current    | **V-TIMELINE**          | Contract **#240** merged; cutover map **#286**; C1 **#285** + C2 **#289** in flight   | C3–C5 residual after C1/C2 land                             |
| current    | **V-SEND residual**     | Poll-thread #282 DONE; edit #283 in flight; pack-read #287; forward #290; avatar #291 | Pack-read implement + avatar upload implement still open    |
| current    | **CI**                  | Parallel Validate Rust∥Node **#284**                                                  | Path scopes; quality gates preserved                        |
| current    | **Import files (prod)** | **165** under synara/src                                                              | Down from plan 220                                          |

### 2026-07-31 — V-SEND.R-PACK-READ residual inventory

| When (UTC) | Item                   | Result      | Notes                                                                                                                                                                                                                                                                                                                 |
| ---------- | ---------------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.R-PACK-READ** | **This PR** | Docs-only inventory of sticker/emoji pack **read** residual (send is native #264): `custom-emoji/utils.ts`, `useImagePacks.ts`, `useImagePackRooms.ts` + consumers read `PoniesEmoteRooms`/`PoniesRoomEmotes`/`PoniesUserEmotes` on live JS client. See [v-send-pack-read-residual.md](v-send-pack-read-residual.md). |

### 2026-07-31 — V-TIMELINE cutover approved + CI parallel Validate

| When (UTC) | Item                          | Result                                    | Notes                                                                                                                                            |
| ---------- | ----------------------------- | ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| current    | **V-TIMELINE cutover policy** | **Approved**                              | User: full steam js-sdk replacement; select presenter + delete RoomTimeline allowed; break/fix-forward OK. #39 still gated.                      |
| current    | **CI Validate**               | **This PR**                               | Split Validate into parallel `validate-rust` + `validate-frontend` (same gates; less wall-clock). Path scopes for rust-only / frontend-only PRs. |
| current    | **Active product**            | #279 loginUtil, #283 edit, #240 tip-merge | Serial `product.rs` residuals preferred after edit.                                                                                              |

### 2026-07-31 — tip after #282 V-SEND.R-POLL-THREAD

| When (UTC) | Item                            | Result                                                                        | Notes                                                                                                                                                                                                                                                                                                                                                                                                            |
| ---------- | ------------------------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Password loginUtil residual** | **Active this PR** [#279](https://github.com/nepenth/synara-desktop/pull/279) | Desktop password login is **only** `matrix_login_password`; delete js `createClient`/`loginRequest` fallback from `loginUtil.ts`; SDK-neutralize `PasswordLoginForm`/`loginUtil`; allowlist **171→169**; production import files **169→167**. Tip base `f4addd6f` after #280. [#240](https://github.com/nepenth/synara-desktop/pull/240) HOLD. See [v-auth-password-loginutil.md](v-auth-password-loginutil.md). |
| current    | **V-AUTH.3b login UIA stages**  | **DONE #280** at `f4addd6f`                                                   | Product non-retention; no invented `matrix_uia_*`. See [v-auth-3b-uia-login-stage.md](v-auth-3b-uia-login-stage.md).                                                                                                                                                                                                                                                                                             |
| current    | **V-SEND.R-POLL-THREAD**        | **DONE #282** at `42ef9127`                                                   | Native poll `threadRoot`/`replyTo`.                                                                                                                                                                                                                                                                                                                                                                              |
| current    | **V-AUTH.3 loginFlows**         | **DONE #276** at `4d33227f`                                                   | Native discovery; allowlist **175→171**. Stage residual closed as non-retention in #280.                                                                                                                                                                                                                                                                                                                         |
| current    | **V-TIMELINE**                  | **HOLD** [#240](https://github.com/nepenth/synara-desktop/pull/240)           | No cutover.                                                                                                                                                                                                                                                                                                                                                                                                      |

### 2026-07-31 — tip after #276 V-AUTH.3

| When (UTC) | Item                    | Result                                                              | Notes                                                                                                                                                                                                                                                              |
| ---------- | ----------------------- | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| prior      | **V-AUTH.3 loginFlows** | **DONE #276** at `4d33227f`                                         | Native `matrix_login_flows` + `HttpLoginFlowTransport`; AuthFlowsLoader live `createClient`/`loginFlows()` deleted; allowlist **175→171**; production import files **172→169**. Residual: UIA stage execution for login; password `loginUtil` non-native fallback. |
| prior      | **V-TIMELINE**          | **HOLD** [#240](https://github.com/nepenth/synara-desktop/pull/240) | No cutover.                                                                                                                                                                                                                                                        |
| prior      | **Next free slot**      | **password loginUtil / UIA stage**                                  | Per residual execution order after discovery.                                                                                                                                                                                                                      |

### 2026-07-31 — tip after #274 docs

| When (UTC) | Item                   | Result                                                                | Notes                                                                  |
| ---------- | ---------------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| current    | **Docs progress**      | **Merged** [#274](https://github.com/nepenth/synara-desktop/pull/274) | Tip `48991e77` tracking after #266; free slot **V-AUTH.3**; #240 HOLD. |
| current    | **V-AUTH.4b register** | **DONE #266** at `bc9aa283`                                           | Native registration; allowlist **191→175**.                            |

### 2026-07-31 — tip after #266 V-AUTH.4b

| When (UTC) | Item                    | Result                                                              | Notes                                                                                                                                                                                                                                                                                                                  |
| ---------- | ----------------------- | ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-AUTH.3 loginFlows** | **Active this PR**                                                  | Native `matrix_login_flows` + `HttpLoginFlowTransport`; `AuthFlowsLoader` deletes live `createClient`/`loginFlows()`; SDK-neutral login DTOs; allowlist **175→171**; production import files **172→169**. Residual: UIA stage execution for login (follow-on). See [v-auth-3-login-flows.md](v-auth-3-login-flows.md). |
| current    | Integration tip base    | `48991e77`                                                          | After [#274](https://github.com/nepenth/synara-desktop/pull/274) docs; product tip after #266 was lagging in older progress rows.                                                                                                                                                                                      |
| current    | **V-TIMELINE**          | **HOLD** [#240](https://github.com/nepenth/synara-desktop/pull/240) | No cutover.                                                                                                                                                                                                                                                                                                            |

### 2026-07-31 — tip after #266 V-AUTH.4b / #274 docs

| When (UTC) | Item                   | Result                                                              | Notes                                                                                                                                                                                                                                     |
| ---------- | ---------------------- | ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| prior      | **V-AUTH.4b register** | **DONE #266**                                                       | Native `matrix_register` / `matrix_register_flows` / `matrix_register_request_email_token`; allowlist **191→175**; production import files **172**. Docs tip-fix [#274](https://github.com/nepenth/synara-desktop/pull/274) → `48991e77`. |
| prior      | **V-AUTH.3**           | Was free-slot residual                                              | inventory [#273](https://github.com/nepenth/synara-desktop/pull/273); now active implementation.                                                                                                                                          |
| prior      | **V-TIMELINE**         | **HOLD** [#240](https://github.com/nepenth/synara-desktop/pull/240) | No cutover.                                                                                                                                                                                                                               |

### 2026-07-31 — tip after #273 V-AUTH.3 inventory

| When (UTC) | Item                   | Result                                                                    | Notes                                                                                                                       |
| ---------- | ---------------------- | ------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| prior      | **V-AUTH.3 inventory** | **Merged** [#273](https://github.com/nepenth/synara-desktop/pull/273)     | Integration tip was `04e63444`; docs-only UIA/login-flow inventory + slice plan. Implementation residual remains free slot. |
| prior      | **V-AUTH.4b register** | **Was active** [#266](https://github.com/nepenth/synara-desktop/pull/266) | Tip-merged through `04e63444`; now **DONE** at `bc9aa283`.                                                                  |
| prior      | **V-TIMELINE**         | **HOLD** [#240](https://github.com/nepenth/synara-desktop/pull/240)       | No cutover.                                                                                                                 |

### 2026-07-31 — tip after #268 V-ROOMS.2c

| When (UTC) | Item                   | Result                                                                | Notes                                                                                                                                                                                                      |
| ---------- | ---------------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| prior      | **V-ROOMS.2c**         | **Merged** [#268](https://github.com/nepenth/synara-desktop/pull/268) | Integration tip was `ac6ae435`; native space children snapshot/set/remove/reparent; JS SpaceChild writers deleted; inventory **flat 184/197**; live proof unclaimed. #267 partial CLOSED in favor of #268. |
| prior      | **V-AUTH.4b register** | **Active** [#266](https://github.com/nepenth/synara-desktop/pull/266) | Still open; body keeps **V-AUTH.3** loginFlows residual.                                                                                                                                                   |
| prior      | **V-TIMELINE**         | **HOLD** [#240](https://github.com/nepenth/synara-desktop/pull/240)   | No cutover.                                                                                                                                                                                                |
| prior      | **Next free slot**     | **V-AUTH.3**                                                          | loginFlows / AuthFlowsLoader still js.                                                                                                                                                                     |

### 2026-07-31 — V-ROOMS.2c after #264/#270

| When (UTC) | Item                   | Result                                                                    | Notes                                                                                                 |
| ---------- | ---------------------- | ------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| prior      | **V-ROOMS.2c**         | **Was active** [#268](https://github.com/nepenth/synara-desktop/pull/268) | Local space graph + mutations; tip-merged after sticker/GIF + docs #270; now merged at `ac6ae435`.    |
| prior      | **V-SEND sticker/GIF** | **Merged** [#264](https://github.com/nepenth/synara-desktop/pull/264)     | Tip product `706bf608`; docs [#270](https://github.com/nepenth/synara-desktop/pull/270) → `00fb7788`. |
| prior      | **Next free slot**     | **V-AUTH.3**                                                              | loginFlows / AuthFlowsLoader still js.                                                                |

### 2026-07-31 — active residuals after #265

| When (UTC) | Item                   | Result                                                                                                                                                         | Notes                                                               |
| ---------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| current    | **Docs progress**      | **Merged** [#265](https://github.com/nepenth/synara-desktop/pull/265)                                                                                          | Tip `a677a80b832cbb79879da1b3dab1b9dd257df4d2` tracking after #263. |
| current    | **V-ROOMS.2c**         | **Active drafts** [#268](https://github.com/nepenth/synara-desktop/pull/268) full / [#267](https://github.com/nepenth/synara-desktop/pull/267) writers-partial | Prefer #268 full vertical; close #267 when #268 lands.              |
| current    | **V-AUTH.4b register** | **Active draft** [#266](https://github.com/nepenth/synara-desktop/pull/266)                                                                                    | CI: guardrail restore_session fix in flight.                        |
| current    | **V-SEND sticker/GIF** | **Active draft** [#264](https://github.com/nepenth/synara-desktop/pull/264)                                                                                    | rustfmt fix pushed.                                                 |
| current    | **V-TIMELINE**         | **HOLD** [#240](https://github.com/nepenth/synara-desktop/pull/240)                                                                                            | No cutover.                                                         |

### 2026-07-31 — tip after #263

| When (UTC) | Item                         | Result                                                                | Notes                                                                                                                                                                                                             |
| ---------- | ---------------------------- | --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-AUTH.4a password reset** | **Merged** [#263](https://github.com/nepenth/synara-desktop/pull/263) | Integration `5ae9da2f2cdc9fc9767f65f8e2a4cf48e5f13653`; native email-token + UIAA password change; JS owners deleted; inventory **187→184** / allowlist **194→191**. **V-AUTH.4b** registration remains residual. |
| current    | **V-AUTH.2 token login**     | **Merged** [#262](https://github.com/nepenth/synara-desktop/pull/262) | Product non-retention close.                                                                                                                                                                                      |
| current    | **V-TIMELINE**               | **HOLD** [#240](https://github.com/nepenth/synara-desktop/pull/240)   | Tip-merge OK; no presenter cutover / RoomTimeline deletion.                                                                                                                                                       |

### 2026-07-31 — tip after #262

| When (UTC) | Item                         | Result                                                                 | Notes                                                                          |
| ---------- | ---------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| current    | **V-AUTH.2 token login**     | **Merged** [#262](https://github.com/nepenth/synara-desktop/pull/262)  | Integration `56d1544e5473764f9aaed64e98e074c15aa3b105`; product non-retention. |
| current    | **V-AUTH.4a password reset** | **This PR** [#263](https://github.com/nepenth/synara-desktop/pull/263) | Native password reset; 4b residual.                                            |

### 2026-07-31 — tip after #258

| When (UTC) | Item                     | Result                                                                | Notes                                                                                                                     |
| ---------- | ------------------------ | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| current    | **V-SEND.5 threads**     | **Merged** [#258](https://github.com/nepenth/synara-desktop/pull/258) | Integration `5c6e6e87eb5520e1a5953f06a03d9c4b26fbb7bf`; native composer thread send; Synapse thread-send proof Confirmed. |
| current    | **Docs remove handoffs** | **Merged** [#261](https://github.com/nepenth/synara-desktop/pull/261) | Public handoff docs removed.                                                                                              |

### 2026-07-31 — tip after #261

| When (UTC) | Item                     | Result                                                                 | Notes                                                                                                                                         |
| ---------- | ------------------------ | ---------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Docs remove handoffs** | **Merged** [#261](https://github.com/nepenth/synara-desktop/pull/261)  | Integration `d080156e30b28901959853e46b766deb56185619`; public SESSION-HANDOFF/CONTINUATION/implementation-handoff/orchestrator-loop removed. |
| current    | **V-SEND.5 threads**     | **This PR** [#258](https://github.com/nepenth/synara-desktop/pull/258) | Native composer thread send; tip-merged after #261.                                                                                           |
| current    | **V-ROOMS.2b hierarchy** | **Merged** [#254](https://github.com/nepenth/synara-desktop/pull/254)  | Integration `9c0b51e`; native lobby hierarchy summaries.                                                                                      |

### 2026-07-31 — tip sync at `9c0b51e` (after #254)

| When (UTC) | Item                          | Result                                                                                                                                           | Notes                                                                                                                                               |
| ---------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| current    | **Tip after #254**            | **Merged** [#254](https://github.com/nepenth/synara-desktop/pull/254)                                                                            | Integration `9c0b51e4d0706e7ad6cafe82172e3c9e8406fcea` — V-ROOMS.2b native hierarchy summaries.                                                     |
| current    | **Tip after #260**            | **Merged** [#260](https://github.com/nepenth/synara-desktop/pull/260)                                                                            | Integration `8accf77c548ec21c3c08ed457c878f6656bd0778` — gitignore local AI/agent operator tooling.                                                 |
| current    | **Docs handoff hygiene**      | **Merged** [#256](https://github.com/nepenth/synara-desktop/pull/256) then removed in [#261](https://github.com/nepenth/synara-desktop/pull/261) | #256 briefly added Grok session handoff after #253; #261 removes public agent/session handoffs; program index is README + residual + PROGRESS only. |
| current    | **V-SEND.4 rich messages**    | **Merged** [#253](https://github.com/nepenth/synara-desktop/pull/253)                                                                            | Integration `b558344ee0f998ffb21edad13c6cb6806bd2d010`; native emote/notice/HTML/mentions/reply via `matrix_send_text`; not in flight.              |
| current    | **V-ROOMS.5r m.direct users** | **Merged** [#252](https://github.com/nepenth/synara-desktop/pull/252)                                                                            | Integration `9579ea4462cfce5b6974ff046c547d090866fc98`; native `userIds` owns `mDirectUsersAtom`; importers **187→187**.                            |
| current    | **V-SEND.3 polls**            | **Merged** [#250](https://github.com/nepenth/synara-desktop/pull/250)                                                                            | Integration `88ed14308227b2eec2bed4fc33d97cfa0a2270f3`; reviewed head `761d2ef`; Synapse native poll proof Confirmed.                               |
| current    | **V-ROOMS.2b hierarchy**      | **Merged** [#254](https://github.com/nepenth/synara-desktop/pull/254)                                                                            | Integration `9c0b51e4d0706e7ad6cafe82172e3c9e8406fcea`; native lobby hierarchy summaries; JS getRoomHierarchy/IHierarchyRoom deleted.               |
| current    | **V-SEND.5 threads**          | **Active draft** [#258](https://github.com/nepenth/synara-desktop/pull/258)                                                                      | Tip-merged after #254; re-run CI then undraft/merge when green.                                                                                     |
| current    | **V-AUTH.2 token login**      | **Active draft** [#262](https://github.com/nepenth/synara-desktop/pull/262)                                                                      | Product non-retention close (not re-home).                                                                                                          |
| current    | **V-AUTH.4a password reset**  | **Active draft** [#263](https://github.com/nepenth/synara-desktop/pull/263)                                                                      | Native password reset; **V-AUTH.4b** registration remains residual.                                                                                 |
| current    | **V-TIMELINE boundary**       | **HOLD** [#240](https://github.com/nepenth/synara-desktop/pull/240)                                                                              | Implement/tip-merge OK; **do not claim cutover**; presenter unselected; no `RoomTimeline` deletion.                                                 |
| current    | **Docs remove handoffs**      | **This PR** [#261](https://github.com/nepenth/synara-desktop/pull/261)                                                                           | Public hygiene + progress/residual tip honesty.                                                                                                     |
| current    | **D0.6 / L1 foundations**     | **HOLD**                                                                                                                                         | #221 plateau; parked L1 PRs; umbrella #39.                                                                                                          |

### 2026-07-31 — public hygiene

| When (UTC) | Item                              | Result      | Notes                                                                                                                      |
| ---------- | --------------------------------- | ----------- | -------------------------------------------------------------------------------------------------------------------------- |
| —          | Remove agent/session handoff docs | **This PR** | SESSION-HANDOFF, CONTINUATION, implementation-handoff, orchestrator-loop, r0.2-e1-handoff removed; program index README.md |

### 2026-07-31 (UTC) — tip audit / residual close (historical)

| When (UTC) | Item                                | Result                                                                | Notes                                                                                                                                                                                                                                           |
| ---------- | ----------------------------------- | --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| earlier    | **V-SEND.4 rich composer messages** | **Merged** [#253](https://github.com/nepenth/synara-desktop/pull/253) | Superseded in-flight claim; see tip-sync row above.                                                                                                                                                                                             |
| earlier    | **V-SEND.3 polls**                  | **Merged** [#250](https://github.com/nepenth/synara-desktop/pull/250) | Integration merge before #253; reviewed head `761d2ef`; Synapse native poll proof Confirmed.                                                                                                                                                    |
| earlier    | **V-ROOMS.5w m.direct writers**     | **Merged** [#251](https://github.com/nepenth/synara-desktop/pull/251) | Integration `0fb0fe425ae932e27445b8054f3a14d628e5a869`; candidate `e4e2639` required CI green. Native add/remove own DM writers; JS helpers deleted; importers **187→187**. See [v-rooms-5w-mdirect-writers.md](v-rooms-5w-mdirect-writers.md). |
| earlier    | **V-ROOMS.5 m.direct read**         | **Merged** [#249](https://github.com/nepenth/synara-desktop/pull/249) | Integration `d17ab2c0d72b129189a80d03bd0c1b56d6c230d6`; candidate `708aef7`. Production **189→187**, repository-wide **202→200**.                                                                                                               |
| earlier    | **Docs/tracking audit**             | **Merged** [#243](https://github.com/nepenth/synara-desktop/pull/243) | Tracking rewritten onto tip after #251; handoff docs later removed in #261.                                                                                                                                                                     |
| earlier    | **V-TIMELINE boundary**             | **HOLD** [#240](https://github.com/nepenth/synara-desktop/pull/240)   | Incomplete contract; presenter unselected; no cutover.                                                                                                                                                                                          |
| earlier    | **D0.6 / L1 foundations**           | **HOLD**                                                              | #221 plateau; parked L1 PRs #109/#193/#196/#198/#199/#201/#203/#204/#207/#208/#209; umbrella #39.                                                                                                                                               |

### 2026-07-30 (UTC) — active replacement queue

    | When (UTC) | Item                          | Result                                                                                  | Notes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | | ---------- | ----------------------------- | --------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | | current    | **V-ROOMS.2a space parents**  | **Draft**                                                                               | Native `matrix_space_parents_snapshot` owns `roomToParentsAtom`; JS binder deleted; candidate production **190→189**,
    repository-wide **203→202**; allowlist **197→196**. Lobby remains V-ROOMS.2b. Live proof unclaimed. See [v-rooms-2a-space-parents.md](v-rooms-2a-space-parents.md).                                                                                                                                                                                                                                                                                                                 | | current    | **V-ROOMS.4 typing**          | **Merged** [#246](https://github.com/nepenth/synara-desktop/pull/246)                   | Integration `151948c8c2329ee6f0b37b8757607b3ac8bb44e7`; candidate `c4df9ed` required CI green. Native typing snapshot/set; JS binder/`sendTyping` deleted; production **192→190**,
    repository-wide **205→203**; live typing proof Not confirmed. See [v-rooms-4-typing.md](v-rooms-4-typing.md).                                                                                                                                                                                                                                                                                                                 | | current    | **V-ROOMS.3 unread badges**   | **Merged** [#245](https://github.com/nepenth/synara-desktop/pull/245)                   | Integration `efc90d59e6009f45589ce42a29a6f7ebafcf7624`; candidate `a81e026` required CI green. Native `matrix_room_list_snapshot` owns unread badges; JS roomList/roomToUnread binders deleted; production **194→192**,
    repository-wide **208→205**; allowlist **201→199**. Live badge proof Not confirmed (not a reopen blocker). See [v-rooms-3-unread-badges.md](v-rooms-3-unread-badges.md).                                                                                                                                                                                                                                                                                                                 | | current    | **V-AUTH.1 SSO removal**      | **Merged** [#238](https://github.com/nepenth/synara-desktop/pull/238)                   | Integration `08a185e`; required CI green. Desktop SSO/browser callback/token-completion and native SSO UIAA ownership are deleted without a replacement route; production importers 201→197 and repository-wide 215→211.                                                                                                                                                                                                                                                                                                                                                                                   | | current    | **V-ROOMS.1 native invites**  | **Merged** [#241](https://github.com/nepenth/synara-desktop/pull/241)                   | Integration `2c48fd45a08200a6e3491f100912f086e8458b3b`; candidate `7ac2c48` passed required scope,
    Synapse,
    desktop/runtime,
    and quality CI. Native invite snapshot/classification/actions/avatar ownership and JS-owner deletion measure production importers 197→194 and repository-wide 211→208.                                                                                                                                                                                                                                                                                                        | | current    | **V-SEND.2 native reactions** | **Draft** [#239](https://github.com/nepenth/synara-desktop/pull/239)                    | Second-rebased on current integration at `d26db4c`; native reaction candidate keeps whole importers at 197→197 while direct JS owner candidates decrease: `sendEvent` 8→6,
    `redactEvent` 5→3,
    `getUnfilteredTimelineSet` 8→6. Native redaction verifies the selected target/key annotation before a room redaction. Required CI is green; live runtime proof and integration ordering remain unclaimed.                                                                                                                                                                                                    | | current    | **V-TIMELINE boundary**       | **Draft; fresh CI required** [#240](https://github.com/nepenth/synara-desktop/pull/240) | `5e0c2a5` rebases on integrated V-ROOMS and adds the stream/session-bound opaque media owner,
    sole-protocol extension,
    explicit stream close,
    and native image/file/audio/video/sticker rendering route. Focused Rust/TypeScript/lint checks passed locally; a fresh full CI run is required. At pre-media `7e6a4d2`,
    run `30553357363` attempt 1 passed Synapse/iOS but exceeded the host-sensitive 119-row audit RSS cap; attempt 2 instead failed six React-hook lint errors. Neither older outcome is a green claim for `5e0c2a5`; active selection,
    JS deletion,
    and runtime proof remain incomplete. | | current    | **V-CRYPTO.7 devices/trust**  | **Merged** [#236](https://github.com/nepenth/synara-desktop/pull/236)                   | Integration `528a510`; reviewed,
    green product/test head `192be46`; native list/trust/rename/delete/UIAA owns the device page; JS owners deleted; inventory 218/273→212/265; live proof unclaimed.                                                                                                                                                                                                                                                                                                                                                                                                         | | earlier    | **V-CRYPTO.6 UTD recovery**   | **Merged** [#235](https://github.com/nepenth/synara-desktop/pull/235)                   | Integration tip `05e3f64`; native late-key readback and JS retry/listener deletion landed.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | | ~14:20     | **V-CRYPTO.4 secret storage** | **Merged** [#234](https://github.com/nepenth/synara-desktop/pull/234)                   | Tip `c2a002d`; legacy secret-storage owner deleted; direct imports 219/276 → 218/275.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | | ~13:13     | **V-CRYPTO.3 key backup**     | **Merged** [#233](https://github.com/nepenth/synara-desktop/pull/233)                   | Tip `38f0994`; legacy UI/listeners/progress/auto-restore deleted; native owner retained; direct imports 222/279 → 219/276.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | | ~12:16     | **V-CRYPTO.2 cross-signing**  | **Merged** [#231](https://github.com/nepenth/synara-desktop/pull/231)                   | Tip `0ba87f3`; legacy setup/status/reset owner and fallback deleted; native owner retained; direct imports 223/280 → 222/279.                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | | earlier    | **V-CRYPTO.1 verification**   | **Merged** [#230](https://github.com/nepenth/synara-desktop/pull/230)                   | Tip `5c68b19`; legacy owner/inbox/hooks/helpers and JS-only test deleted; native owner retained; direct imports 232/292 → 223/280.                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | | ~02:03     | **V-CRYPTO.5 closure**        | **Merged** [#227](https://github.com/nepenth/synara-desktop/pull/227)                   | Tip `146952a`; Rust owns room-key export/import,
    legacy owner/helper deleted,
    exact-head gates green.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | | ~02:03     | **Execution pause**           | **Paused**                                                                              | Clean between-slices handoff; no active implementation PR; V-CRYPTO.1-D is next.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | | ~01:39     | **Execution model**           | **Historical**                                                                          | This was a temporary Codex `gpt-5.6-terra` high setting and is superseded by the current snapshot's `gpt-5.6-sol` medium setting.                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | | ~01:25     | **V-CRYPTO.5 closure**        | **This PR**                                                                             | Single Rust IPC owner; legacy WebView/keyfile crypto deleted; retry and incomplete-export cleanup made race-safe; reviewed-SHA gates pending.                                                                                                                                                                                                                                                                                                                                                                                                                                                              | | ~01:01     | **Plan alignment #228**       | **Merged**                                                                              | Tip `fd7c934`; per-vertical deletion metrics and Codex orchestration made binding.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | | ~00:10     | **Execution model**           | **Historical**                                                                          | This superseded entry recorded an earlier model choice; the current execution model is defined in the live snapshot above.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | | ~00:10     | **Physical deletion policy**  | **Clarified**                                                                           | Superseded JS implementation/imports are deleted in each owning vertical. V-BURN becomes final convergence/dependency removal.                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | | ~00:10     | **V-CRYPTO status audit**     | **Corrected**                                                                           | #223–#226 are product-wired but deletion-open; #227 must remove its legacy path before closure.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |,
    | When (UTC) | Item                          | Result                                                                | Notes                                                                                                                                                                                                                                                                                                                                                                                                                                                     | | ---------- | ----------------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | | current    | **V-ROOMS.4 typing**          | **Merged** [#246](https://github.com/nepenth/synara-desktop/pull/246) | Integration `151948c8c2329ee6f0b37b8757607b3ac8bb44e7`; native `matrix_typing_snapshot` / `matrix_typing_set` own receive/send; JS typingMembers binder and `sendTyping` deleted; production **192→190**,
    repository-wide **205→203**; allowlist **199→197**. Live proof unclaimed. See [v-rooms-4-typing.md](v-rooms-4-typing.md).                                                        | | current    | **V-ROOMS.3 unread badges**   | **Merged** [#245](https://github.com/nepenth/synara-desktop/pull/245) | Integration `efc90d59e6009f45589ce42a29a6f7ebafcf7624`; candidate `a81e026` required CI green. Native `matrix_room_list_snapshot` owns unread badges; JS roomList/roomToUnread binders deleted; production **194→192**,
    repository-wide **208→205**; allowlist **201→199**. Live badge proof Not confirmed (not a reopen blocker). See [v-rooms-3-unread-badges.md](v-rooms-3-unread-badges.md). | | current    | **V-AUTH.1 SSO removal**      | **Merged** [#238](https://github.com/nepenth/synara-desktop/pull/238) | Integration `08a185e`; required CI green. Desktop SSO/browser callback/token-completion and native SSO UIAA ownership are deleted without a replacement route; production importers 201→197 and repository-wide 215→211.                                                                                                                                                                                                  | | current    | **V-ROOMS.1 native invites**  | **Merged** [#241](https://github.com/nepenth/synara-desktop/pull/241) | Integration `2c48fd45a08200a6e3491f100912f086e8458b3b`; candidate `7ac2c48` passed required scope,
    and quality CI. Native invite snapshot/classification/actions/avatar ownership and JS-owner deletion measure production importers 197→194 and repository-wide 211→208.                                                                                                                       | | current    | **V-SEND.2 native reactions** | **Draft** [#239](https://github.com/nepenth/synara-desktop/pull/239)  | Second-rebased on current integration at `d26db4c`; native reaction candidate keeps whole importers at 197→197 while direct JS owner candidates decrease: `sendEvent` 8→6,
    `getUnfilteredTimelineSet` 8→6. Native redaction verifies the selected target/key annotation before a room redaction. Required CI is green; live runtime proof and integration ordering remain unclaimed.                   | | current    | **V-TIMELINE boundary**       | **Draft** [#240](https://github.com/nepenth/synara-desktop/pull/240)  | Rebased on integration `151948c` (V-ROOMS.4). Native snapshot/delta/open/pagination/read,
    media,
    viewport,
    live frontier,
    typed reply/rich-send/edit/redact/forward/report/pin,
    rich/state projection,
    and composer reply-draft ownership exist. Presenter still unselected; no `RoomTimeline` deletion. Remaining: reactions (#239),
    media forward,
    poll/call actions,
    selection/JS deletion,
    live authenticated proof. | | current    | **V-CRYPTO.7 devices/trust**  | **Merged** [#236](https://github.com/nepenth/synara-desktop/pull/236) | Integration `528a510`; reviewed,
    green product/test head `192be46`; native list/trust/rename/delete/UIAA owns the device page; JS owners deleted; inventory 218/273→212/265; live proof unclaimed.                                                                                                                                                                                                                        | | earlier    | **V-CRYPTO.6 UTD recovery**   | **Merged** [#235](https://github.com/nepenth/synara-desktop/pull/235) | Integration tip `05e3f64`; native late-key readback and JS retry/listener deletion landed.                                                                                                                                                                                                                                                                                                                                | | ~14:20     | **V-CRYPTO.4 secret storage** | **Merged** [#234](https://github.com/nepenth/synara-desktop/pull/234) | Tip `c2a002d`; legacy secret-storage owner deleted; direct imports 219/276 → 218/275.                                                                                                                                                                                                                                                                                                                                     | | ~13:13     | **V-CRYPTO.3 key backup**     | **Merged** [#233](https://github.com/nepenth/synara-desktop/pull/233) | Tip `38f0994`; legacy UI/listeners/progress/auto-restore deleted; native owner retained; direct imports 222/279 → 219/276.                                                                                                                                                                                                                                                                                                | | ~12:16     | **V-CRYPTO.2 cross-signing**  | **Merged** [#231](https://github.com/nepenth/synara-desktop/pull/231) | Tip `0ba87f3`; legacy setup/status/reset owner and fallback deleted; native owner retained; direct imports 223/280 → 222/279.                                                                                                                                                                                                                                                                                             | | earlier    | **V-CRYPTO.1 verification**   | **Merged** [#230](https://github.com/nepenth/synara-desktop/pull/230) | Tip `5c68b19`; legacy owner/inbox/hooks/helpers and JS-only test deleted; native owner retained; direct imports 232/292 → 223/280.                                                                                                                                                                                                                                                                                        | | ~02:03     | **V-CRYPTO.5 closure**        | **Merged** [#227](https://github.com/nepenth/synara-desktop/pull/227) | Tip `146952a`; Rust owns room-key export/import,
    exact-head gates green.                                                                                                                                                                                                                                                                                                                     | | ~02:03     | **Execution pause**           | **Paused**                                                            | Clean between-slices handoff; no active implementation PR; V-CRYPTO.1-D is next.                                                                                                                                                                                                                                                                                                                                          | | ~01:39     | **Execution model**           | **Historical**                                                        | This was a temporary Codex `gpt-5.6-terra` high setting and is superseded by the current snapshot's `gpt-5.6-sol` medium setting.                                                                                                                                                                                                                                                                                         | | ~01:25     | **V-CRYPTO.5 closure**        | **This PR**                                                           | Single Rust IPC owner; legacy WebView/keyfile crypto deleted; retry and incomplete-export cleanup made race-safe; reviewed-SHA gates pending.                                                                                                                                                                                                                                                                             | | ~01:01     | **Plan alignment #228**       | **Merged**                                                            | Tip `fd7c934`; per-vertical deletion metrics and Codex orchestration made binding.                                                                                                                                                                                                                                                                                                                                        | | ~00:10     | **Execution model**           | **Historical**                                                        | This superseded entry recorded an earlier model choice; the current execution model is defined in the live snapshot above.                                                                                                                                                                                                                                                                                                | | ~00:10     | **Physical deletion policy**  | **Clarified**                                                         | Superseded JS implementation/imports are deleted in each owning vertical. V-BURN becomes final convergence/dependency removal.                                                                                                                                                                                                                                                                                            | | ~00:10     | **V-CRYPTO status audit**     | **Corrected**                                                         | #223–#226 are product-wired but deletion-open; #227 must remove its legacy path before closure.                                                                                                                                                                                                                                                                                                                           |,
    composer reply-draft,
    and media/sticker forward ownership exist. Presenter still unselected; no `RoomTimeline` deletion. Remaining: reactions (#239),
    media/sticker forward,
    poll vote,
    and RTC call-decline ownership exist. Presenter still unselected; no `RoomTimeline` deletion. Remaining: reactions (#239),

### 2026-07-28 (V-CRYPTO product wiring)

| When (UTC) | Item                             | Result                                                                | Notes                                                                      |
| ---------- | -------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| ~21:09     | **V-CRYPTO.5 room-key transfer** | **Draft** [#227](https://github.com/nepenth/synara-desktop/pull/227)  | Native export/import wired; CI green; physical JS deletion still required. |
| ~20:57     | **V-CRYPTO.4 secret storage**    | **Merged** [#226](https://github.com/nepenth/synara-desktop/pull/226) | tip `4b4d921`; product wired, deletion residual V-CRYPTO.4-D open.         |
| ~20:21     | **V-CRYPTO.3 key backup**        | **Merged** [#225](https://github.com/nepenth/synara-desktop/pull/225) | tip `0f01ea3`; product wired, deletion residual V-CRYPTO.3-D open.         |
| ~19:49     | **V-CRYPTO.2 cross-signing**     | **Merged** [#224](https://github.com/nepenth/synara-desktop/pull/224) | tip `bde3c5a`; product wired, deletion residual V-CRYPTO.2-D open.         |
| ~19:17     | **V-CRYPTO.1 verification**      | **Merged** [#223](https://github.com/nepenth/synara-desktop/pull/223) | tip `70d7167`; product wired, deletion residual V-CRYPTO.1-D open.         |

### 2026-07-28 (policy)

| When (UTC) | Item                     | Result      | Notes                                                                    |
| ---------- | ------------------------ | ----------- | ------------------------------------------------------------------------ |
| —          | **Full vertical policy** | **This PR** | Complete replacement only; residual completion queue; hold #221 plateau. |

### 2026-07-28 (ledger)

| When (UTC) | Item                            | Result      | Notes                                                                                                           |
| ---------- | ------------------------------- | ----------- | --------------------------------------------------------------------------------------------------------------- |
| —          | **program-status truth resync** | **This PR** | Historical resync snapshot: **74/112** landed. Current integration states are generated in `program-status.md`. |

### 2026-07-28

| When (UTC) | Item                                   | Result                                                                 | Notes                                                                |
| ---------- | -------------------------------------- | ---------------------------------------------------------------------- | -------------------------------------------------------------------- |
| ~14:05     | **P7.5** save/share                    | **Merged** [#200](https://github.com/nepenth/synara-desktop/pull/200)  | tip `54e0419`.                                                       |
| ~14:05     | **PROGRESS.md** after #200             | **This PR**                                                            | Next #207.                                                           |
| ~13:30     | **P7.4** attachment send               | **Merged** [#192](https://github.com/nepenth/synara-desktop/pull/192)  | tip `b9b86f2`.                                                       |
| ~13:30     | **PROGRESS.md** after #192             | **This PR**                                                            | Next #200/#207.                                                      |
| ~13:00     | **P6.10** room directory               | **Merged** [#190](https://github.com/nepenth/synara-desktop/pull/190)  | tip `8d544fa`.                                                       |
| ~13:00     | **PROGRESS.md** after #190             | **This PR**                                                            | Next #192.                                                           |
| ~12:25     | **P7.3** media cache                   | **Merged** [#188](https://github.com/nepenth/synara-desktop/pull/188)  | tip `21101da`.                                                       |
| ~12:25     | **PROGRESS.md** after #188             | **This PR**                                                            | Next #190.                                                           |
| ~12:00     | **P6.7** account-data                  | **Merged** [#163](https://github.com/nepenth/synara-desktop/pull/163)  | tip `088b37e`.                                                       |
| ~12:00     | **PROGRESS.md** after #163+#187        | **This PR**                                                            | Next #188/#190.                                                      |
| ~11:45     | **P7.2** media download                | **Merged** [#187](https://github.com/nepenth/synara-desktop/pull/187)  | tip was `db97d15`.                                                   |
| ~11:05     | **P5.9** raw-content                   | **Merged** [#162](https://github.com/nepenth/synara-desktop/pull/162)  | tip `671d590`.                                                       |
| ~11:05     | **PROGRESS.md** after #162             | **This PR**                                                            | Next #163.                                                           |
| ~10:50     | **P9.2** push-rules                    | **PR open** [#196](https://github.com/nepenth/synara-desktop/pull/196) |                                                                      |
| ~10:55     | **P5.11** timeline filter              | **PR open** [#201](https://github.com/nepenth/synara-desktop/pull/201) |                                                                      |
| ~10:15     | **P5.5** unread                        | **Merged** [#161](https://github.com/nepenth/synara-desktop/pull/161)  | tip `5db58a5`.                                                       |
| ~10:20     | **PROGRESS.md** after #161             | **This PR**                                                            | Next #162.                                                           |
| ~10:10     | **P8.9** crypto bootstrap              | **PR open** [#193](https://github.com/nepenth/synara-desktop/pull/193) | Coordinator; local 6/6.                                              |
| ~09:50     | **P7.4** attachment send               | **PR open** [#192](https://github.com/nepenth/synara-desktop/pull/192) | AttachmentSendQueue.                                                 |
| ~09:35     | **P6.9** room-ops                      | **Merged** [#185](https://github.com/nepenth/synara-desktop/pull/185)  | tip `f7981ea`.                                                       |
| ~09:35     | **PROGRESS.md** after #185             | **This PR**                                                            | Next #161.                                                           |
| ~09:10     | **P6.10** room directory               | **PR open** [#190](https://github.com/nepenth/synara-desktop/pull/190) | RoomDirectorySession.                                                |
| ~08:45     | **P5.7** polls                         | **Merged** [#160](https://github.com/nepenth/synara-desktop/pull/160)  | tip `d71e8c6`.                                                       |
| ~08:45     | **PROGRESS.md** after #160             | **This PR**                                                            | Next #161–#163 + #185.                                               |
| ~08:40     | **P7.3** media cache                   | **PR open** [#188](https://github.com/nepenth/synara-desktop/pull/188) | MediaCacheIndex; local 6/6.                                          |
| ~08:25     | **P7.2** media download                | **PR open** [#187](https://github.com/nepenth/synara-desktop/pull/187) | DownloadQueue.                                                       |
| ~08:00     | **P6.5** room profile                  | **Merged** [#183](https://github.com/nepenth/synara-desktop/pull/183)  | tip `242ee57`.                                                       |
| ~08:00     | **P6.9** room ops                      | **PR open** [#185](https://github.com/nepenth/synara-desktop/pull/185) | RoomOpsQueue; local 7/7.                                             |
| ~08:00     | **PROGRESS.md** after #183             | **This PR**                                                            | Mid-stack tip-merge after #183.                                      |
| ~07:30     | **PROGRESS.md** after #182             | **This PR**                                                            | Tip-merge mid-stack; open #183 P6.5.                                 |
| ~07:21     | **PROGRESS.md** after #165+#166        | **Merged** [#182](https://github.com/nepenth/synara-desktop/pull/182)  | tip `f8fabae`.                                                       |
| ~07:25     | **P6.5** room profile                  | **PR open** [#183](https://github.com/nepenth/synara-desktop/pull/183) | RoomProfileIndex; local 8/8.                                         |
| ~07:20     | **P8.7** UTD recovery                  | **Merged** [#166](https://github.com/nepenth/synara-desktop/pull/166)  | tip `821177b`.                                                       |
| ~07:00     | **P8.6** room-keys                     | **Merged** [#165](https://github.com/nepenth/synara-desktop/pull/165)  | tip `8d5cbd8`.                                                       |
| ~07:20     | **PROGRESS.md** after #165+#166        | **This PR**                                                            | Next mid-stack #160–#163.                                            |
| ~06:35     | **P4.7** presence                      | **Merged** [#169](https://github.com/nepenth/synara-desktop/pull/169)  | tip `fd87180`.                                                       |
| ~06:35     | **P4.7** presence stream               | **Merged** [#169](https://github.com/nepenth/synara-desktop/pull/169)  | PresenceIndex; tip `fd87180`.                                        |
| ~06:05     | **P8.8** crypto-store                  | **Merged** [#168](https://github.com/nepenth/synara-desktop/pull/168)  | tip `13dcb55`.                                                       |
| ~06:35     | **PROGRESS.md** after #168+#169        | **This PR**                                                            | Next #165/#166.                                                      |
| ~05:40     | **P6.6** user profile                  | **Merged** [#173](https://github.com/nepenth/synara-desktop/pull/173)  | tip `edd6121`.                                                       |
| ~05:40     | **P6.6** user profile / ignore         | **Merged** [#173](https://github.com/nepenth/synara-desktop/pull/173)  | UserProfileIndex; tip `edd6121`.                                     |
| ~05:40     | **PROGRESS.md** after #173             | **This PR**                                                            | Next #168 crypto-store.                                              |
| ~05:15     | **PROGRESS.md** after #171             | **Merged** [#177](https://github.com/nepenth/synara-desktop/pull/177)  | tip was `fc8dcaa`.                                                   |
| ~05:05     | **P3.4** UIA                           | **Merged** [#171](https://github.com/nepenth/synara-desktop/pull/171)  | tip `3d1c46e`.                                                       |
| ~05:05     | **P3.4** UIA multi-stage               | **Merged** [#171](https://github.com/nepenth/synara-desktop/pull/171)  | Combined with SSO under auth; tip `3d1c46e`.                         |
| ~05:05     | **PROGRESS.md** after #171             | **This PR**                                                            | Next #173 profile + crypto stack.                                    |
| ~04:31     | **P3.3** SSO                           | **Merged** [#170](https://github.com/nepenth/synara-desktop/pull/170)  | tip `7d95461`.                                                       |
| ~04:32     | **PROGRESS.md** after #170             | **Merged** [#176](https://github.com/nepenth/synara-desktop/pull/176)  |                                                                      |
| ~04:30     | **P3.3** SSO / OAuth callback          | **Merged** [#170](https://github.com/nepenth/synara-desktop/pull/170)  | SsoCallbackFlow; tip `7d95461`.                                      |
| ~04:30     | **PROGRESS.md** after #170             | **This PR**                                                            | Next #171 UIA.                                                       |
| ~04:05     | **P5.10** UTD                          | **Merged** [#157](https://github.com/nepenth/synara-desktop/pull/157)  | tip `0c24e4b`.                                                       |
| ~04:11     | **PROGRESS.md** after #157             | **Merged** [#175](https://github.com/nepenth/synara-desktop/pull/175)  |                                                                      |
| ~04:05     | **P5.10** UTD / decrypt updates        | **Merged** [#157](https://github.com/nepenth/synara-desktop/pull/157)  | UtdIndex; tip `0c24e4b`.                                             |
| ~04:05     | **PROGRESS.md** after #157             | **This PR**                                                            | Next #170/#171 auth + crypto stack.                                  |
| ~03:45     | **PROGRESS.md** after #151             | **Merged** [#174](https://github.com/nepenth/synara-desktop/pull/174)  | Tip was `6332042`.                                                   |
| ~03:40     | **P3.7** legacy transition             | **Merged** [#151](https://github.com/nepenth/synara-desktop/pull/151)  | Clean-break; tip `0698147`.                                          |
| ~03:40     | **P3.7** legacy-session transition     | **Merged** [#151](https://github.com/nepenth/synara-desktop/pull/151)  | Clean-break; no JS/token continuity; tip `0698147`.                  |
| ~03:40     | **PROGRESS.md** after #151             | **This PR**                                                            | Next #157 UTD.                                                       |
| ~03:25     | **P6.6** user profile / ignore         | **PR open** [#173](https://github.com/nepenth/synara-desktop/pull/173) | UserProfileIndex; local 6/6.                                         |
| ~03:21     | **PROGRESS.md** after #154             | **Merged** [#172](https://github.com/nepenth/synara-desktop/pull/172)  | Tip was `e6caca9`.                                                   |
| ~03:15     | **P5.4** timeline focus / context      | **Merged** [#154](https://github.com/nepenth/synara-desktop/pull/154)  | TimelineFocus Live/Unread/Focused; tip `5380471`.                    |
| ~03:15     | **PROGRESS.md** after #154             | **This PR**                                                            | Next #151; note #157 conflict.                                       |
| ~02:55     | **P3.4** UIA multi-stage               | **PR open** [#171](https://github.com/nepenth/synara-desktop/pull/171) | UiaSession; local auth 49/49.                                        |
| ~02:44     | **PROGRESS.md**                        | **Merged** [#167](https://github.com/nepenth/synara-desktop/pull/167)  | Tip was `d9009ca` before #154.                                       |
| ~02:42     | **P3.3** SSO / OAuth callback          | **PR open** [#170](https://github.com/nepenth/synara-desktop/pull/170) | SsoCallbackFlow; no tokens/codes; local auth 48/48.                  |
| ~02:35     | **P3.8** remote logout + recovery copy | **Merged** [#155](https://github.com/nepenth/synara-desktop/pull/155)  | RemoteLogoutFlow + RecoveryCopyKey; tip `0e6399d`.                   |
| ~02:35     | **PROGRESS.md** after #155             | **This PR**                                                            | Next #151 legacy; tip-merged open stack.                             |
| ~02:30     | **P4.7** presence stream index         | **PR open** [#169](https://github.com/nepenth/synara-desktop/pull/169) | PresenceIndex; local 8/8; clippy+guardrails.                         |
| ~02:30     | **PROGRESS.md** refresh                | **This PR**                                                            | Open #168/#169; CI queue triage (cancel non-priority package smoke). |
| ~02:24     | **P8.8** crypto-store continuity       | **PR open** [#168](https://github.com/nepenth/synara-desktop/pull/168) | Never auto-wipe; no keys.                                            |
| ~02:15     | **PROGRESS.md** after #150             | **This PR**                                                            | Tip `c3c630e`; next #151 legacy.                                     |
| ~02:13     | **P8.5** key backup / recovery         | **Merged** [#150](https://github.com/nepenth/synara-desktop/pull/150)  | BackupRecoveryFlow; no recovery keys; tip `c3c630e`.                 |
| ~02:15     | **P8.7** UTD recovery coordinator      | **PR open** [#166](https://github.com/nepenth/synara-desktop/pull/166) | Room-level retry/history recovery.                                   |
| ~01:58     | **P8.6** room-key transfer             | **PR open** [#165](https://github.com/nepenth/synara-desktop/pull/165) | Export/import flow; no key material.                                 |
| ~00:55     | **PROGRESS.md** after #145             | **This PR**                                                            | Tip `5799d16`; open stack tip-merged; next #147.                     |
| ~00:54     | **P8.2** device list / trust           | **Merged** [#145](https://github.com/nepenth/synara-desktop/pull/145)  | DeviceIndex; no keys; tip `5799d16`.                                 |
| ~00:54     | **P5.10** UTD / decrypt updates        | **PR open** [#157](https://github.com/nepenth/synara-desktop/pull/157) | UtdIndex; no session keys / bodies.                                  |
| ~00:38     | **P3.8** remote logout + recovery copy | **PR open** [#155](https://github.com/nepenth/synara-desktop/pull/155) | RemoteLogoutFlow + RecoveryCopyKey.                                  |
| ~00:32     | **P5.4** timeline focus / context      | **PR open** [#154](https://github.com/nepenth/synara-desktop/pull/154) | TimelineFocus Live/Unread/Focused.                                   |
| ~00:29     | **PROGRESS.md** after #143             | **Merged** [#153](https://github.com/nepenth/synara-desktop/pull/153)  | After P9.1 widgets.                                                  |

### 2026-07-27

| When (UTC) | Item                                                | Result                                                                                                                             | Notes                                                                                                                                                |
| ---------- | --------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| ~00:13     | **P9.1** widget / Element Call registry             | **Merged** [#143](https://github.com/nepenth/synara-desktop/pull/143)                                                              | WidgetRegistry; no token URLs; tip `1e42d27`.                                                                                                        |
| ~00:15     | **PROGRESS.md** after #143                          | **This PR**                                                                                                                        | Next devices/verification/backup/legacy tip-merged.                                                                                                  |
| ~23:36     | **PROGRESS.md** after #149                          | **This PR**                                                                                                                        | Tip `f71be28`; open #143/#145/#147/#150/#151 tip-merged.                                                                                             |
| ~23:34     | **P8.4** cross-signing / identity                   | **Merged** [#149](https://github.com/nepenth/synara-desktop/pull/149)                                                              | CrossSigningStore presence + IdentityTrust; tip `f71be28`.                                                                                           |
| ~23:27     | **P3.7** legacy transition coordinator              | **PR open** [#151](https://github.com/nepenth/synara-desktop/pull/151)                                                             | Clean-break; no JS client / token continuity.                                                                                                        |
| ~23:15     | **P8.5** backup / recovery flow                     | **PR open** [#150](https://github.com/nepenth/synara-desktop/pull/150)                                                             | BackupRecoveryFlow; no recovery keys stored.                                                                                                         |
| ~23:05     | **PROGRESS.md** after #141                          | **Merged** [#148](https://github.com/nepenth/synara-desktop/pull/148)                                                              | After P6.8 search.                                                                                                                                   |
| ~23:00     | **P6.8** search session foundation                  | **Merged** [#141](https://github.com/nepenth/synara-desktop/pull/141)                                                              | SearchSession request-id stale protection; tip `d6ef679`. Quality + package gate green.                                                              |
| ~22:53     | **P8.2** clippy fix + tip-merge                     | **Pushed** on [#145](https://github.com/nepenth/synara-desktop/pull/145)                                                           | `bool_assert_comparison` → `assert!`; tip after #141.                                                                                                |
| ~22:45     | **P8.3** verification inbox / SAS display           | **PR open** [#147](https://github.com/nepenth/synara-desktop/pull/147)                                                             | VerificationInbox; no secrets; local 7/7.                                                                                                            |
| ~22:34     | **P4.8** route / deep-link resolution               | **Merged** [#139](https://github.com/nepenth/synara-desktop/pull/139)                                                              | resolve_path/build_path; tip `a74fb78`.                                                                                                              |
| ~22:18     | **P8.2** device list / trust projection             | **PR open** [#145](https://github.com/nepenth/synara-desktop/pull/145)                                                             | DeviceIndex; no keys; local 6/6.                                                                                                                     |
| ~22:09     | **P8.1** security status projection                 | **Merged** [#137](https://github.com/nepenth/synara-desktop/pull/137)                                                              | SecurityStatusStore no keys/secrets; tip `f461a00`.                                                                                                  |
| ~21:50     | **P9.1** widget session registry                    | **PR open** [#143](https://github.com/nepenth/synara-desktop/pull/143)                                                             | WidgetRegistry forbids token URLs; local 7/7.                                                                                                        |
| ~21:42     | **P7.1** notification candidate index               | **Merged** [#135](https://github.com/nepenth/synara-desktop/pull/135)                                                              | NotificationIndex suppress/dedup/cap; tip `848dc14`.                                                                                                 |
| ~21:27     | **P6.8** search session foundation                  | **PR open** [#141](https://github.com/nepenth/synara-desktop/pull/141)                                                             | SearchSession request-id stale protection; local 7/7.                                                                                                |
| ~21:17     | **P4.6** member / power-level index                 | **Merged** [#133](https://github.com/nepenth/synara-desktop/pull/133)                                                              | MemberIndex power-ordered; tip `22ec745`.                                                                                                            |
| ~20:58     | **P4.8** route / deep-link resolution               | **PR open** [#139](https://github.com/nepenth/synara-desktop/pull/139)                                                             | resolve_path/build_path; local 7/7.                                                                                                                  |
| ~20:50     | **P5.8** thread list index foundation               | **Merged** [#131](https://github.com/nepenth/synara-desktop/pull/131)                                                              | ThreadIndex activity order + cap; tip `27f870e`.                                                                                                     |
| ~20:30     | **P8.1** security status projection                 | **PR open** [#137](https://github.com/nepenth/synara-desktop/pull/137)                                                             | SecurityStatusStore; no keys/secrets; local 5/5.                                                                                                     |
| ~20:24     | **P5.6** relations index foundation                 | **Merged** [#129](https://github.com/nepenth/synara-desktop/pull/129)                                                              | RelationIndex reactions/replaces/refs/threads; tip `116ed3d`.                                                                                        |
| ~20:06     | **P7.1** notification candidate index               | **PR open** [#135](https://github.com/nepenth/synara-desktop/pull/135)                                                             | NotificationIndex suppress/dedup/cap; local 7/7.                                                                                                     |
| ~19:58     | **P6.4** media upload queue foundation              | **Merged** [#128](https://github.com/nepenth/synara-desktop/pull/128)                                                              | UploadQueue metadata-only; tip `f44bc5c`. Partial send/receipts/typing/media foundations landed.                                                     |
| ~19:42     | **P4.6** member / power-level index                 | **PR open** [#133](https://github.com/nepenth/synara-desktop/pull/133)                                                             | MemberIndex; local 6/6.                                                                                                                              |
| ~19:33     | **P6.3** typing index foundation                    | **Merged** [#127](https://github.com/nepenth/synara-desktop/pull/127)                                                              | TypingIndex + cap 32; tip `ef8bf60`. Quality + package gate green.                                                                                   |
| ~19:15     | **P5.8** thread list index foundation               | **PR open** [#131](https://github.com/nepenth/synara-desktop/pull/131)                                                             | ThreadIndex over ThreadSummary; local 6/6.                                                                                                           |
| ~19:06     | **P6.2** receipt index foundation                   | **Merged** [#125](https://github.com/nepenth/synara-desktop/pull/125)                                                              | ReceiptIndex over DTO Receipt; tip `ef75e3e`. Quality + package gate green.                                                                          |
| ~18:52     | **P5.6** relations index foundation                 | **PR open** [#129](https://github.com/nepenth/synara-desktop/pull/129)                                                             | RelationIndex annotations/replace/reference/thread; local 8/8.                                                                                       |
| ~18:39     | **P6.1** outbound send queue foundation             | **Merged** [#124](https://github.com/nepenth/synara-desktop/pull/124)                                                              | SendQueue + LocalEchoState; tip `c6cbc2c`. Quality ✅; package gate after Arch Docker Hub flake re-run.                                              |
| ~18:35     | Tip-merge P6.2/P6.3/P6.4 onto tip                   | **Pushed**                                                                                                                         | #125/#127/#128 had green CI on pre-#124 tip; re-tip after #124. Local receipts 7/7 + send 8/8.                                                       |
| ~17:58     | **P5.3** timeline pagination foundation             | **Merged** [#122](https://github.com/nepenth/synara-desktop/pull/122)                                                              | TimelinePagination state machine; tip `ed5b3c3`.                                                                                                     |
| ~16:00     | **P4.2** room-list snapshot/delta                   | **Merged** [#115](https://github.com/nepenth/synara-desktop/pull/115)                                                              | Pure projection + ordered ops; tip `c2cdc0b`. iOS/Synapse skipped.                                                                                   |
| ~15:56     | **P6.1** outbound send queue foundation             | **PR open** [#124](https://github.com/nepenth/synara-desktop/pull/124)                                                             | LocalEchoState queue; no Room::send; local 8/8. Disk pressure: cleaned cargo target (-34GB).                                                         |
| ~15:40     | **P4.1** sync readiness + reconnect                 | **Merged** [#114](https://github.com/nepenth/synara-desktop/pull/114)                                                              | `matrix/sync/`: readiness, reconnect table, SyncServiceOwner, guardrail confine. Tip `f9bfe0d`. Full CI green (iOS skipped via path filters).        |
| ~15:38     | **P5.3** timeline pagination foundation             | **PR open** [#122](https://github.com/nepenth/synara-desktop/pull/122)                                                             | Pure `TimelinePagination` state machine; local timeline 23/23. CI deferred until stack advances.                                                     |
| ~15:11     | **P5.2** timeline snapshot/diff projection          | **PR open** [#121](https://github.com/nepenth/synara-desktop/pull/121)                                                             | `TimelineProjection` + ordered ops; local 16/16 then extended by P5.3.                                                                               |
| ~15:40     | **#115** tip-merged after #114                      | **Pushed**                                                                                                                         | Local matrix 270/270 + clippy + guardrails; CI re-run for P4.2 merge.                                                                                |
| ~14:47     | P4.3+ clippy `needless_borrow`                      | **Fixed** on [#116](https://github.com/nepenth/synara-desktop/pull/116)–[#119](https://github.com/nepenth/synara-desktop/pull/119) | Lint Rust failed at `room_list/tests.rs` scope test; dropped extra `&` on `find().unwrap()`. Branches rebased/merged onto tip.                       |
| ~14:46     | Tip update-branch #114/#115/#109                    | **Kicked**                                                                                                                         | After #111 merge so product PRs not BEHIND; #109 should get path-filtered skip of iOS.                                                               |
| ~14:45     | **PROGRESS.md** live work log                       | **Merged** [#111](https://github.com/nepenth/synara-desktop/pull/111)                                                              | Docs-only CI: heavy jobs skipped, Quality gate green. Tip `b3397db`.                                                                                 |
| ~14:42     | CI path filters for heavy jobs                      | **Merged** [#113](https://github.com/nepenth/synara-desktop/pull/113)                                                              | Job-level scopes; quality-gate accepts success\|skipped. Prior tip `168ca2b`.                                                                        |
| ~14:30     | **P5.1** timeline registry foundation               | **PR open** [#119](https://github.com/nepenth/synara-desktop/pull/119)                                                             | TimelineRegistry lifecycle; local 8/8.                                                                                                               |
| ~14:25     | **P4.5** space hierarchy foundation                 | **PR open** [#118](https://github.com/nepenth/synara-desktop/pull/118)                                                             | SpaceHierarchy + filter/cycle; local 6/6.                                                                                                            |
| ~14:21     | **P4.4** favorite/low-priority/folder/recent        | **PR open** [#117](https://github.com/nepenth/synara-desktop/pull/117)                                                             | DTO tag fields + sorts; local room_list 15/15.                                                                                                       |
| ~14:08     | **#114** rebased on tip after #112                  | **Pushed** `d0ab3e5`                                                                                                               | Combined lifecycle restore + sync guardrail zones; matrix tests 270/270. #115/#113 also tip-merged.                                                  |
| ~14:07     | **P3.6** session restore                            | **Merged** [#112](https://github.com/nepenth/synara-desktop/pull/112)                                                              | Vault → identity bind → `restore_session`. Tip `69f1087`. Full CI green (iOS ~23m).                                                                  |
| ~14:00     | **P4.2** room-list snapshot/delta                   | **PR open** [#115](https://github.com/nepenth/synara-desktop/pull/115)                                                             | Pure `RoomListProjection` + delta ops + sequence gap resync. Local 10/10; stacks on #114.                                                            |
| ~13:53     | **P4.1** sync readiness foundation                  | **PR open** [#114](https://github.com/nepenth/synara-desktop/pull/114)                                                             | `matrix/sync/`: readiness map, reconnect table, SyncServiceOwner, guardrail confine `SyncService::builder`. Local 12/12 + clippy + guardrails green. |
| ~13:52     | CI path-filter policy checker fix                   | **Pushed** `09bd360` on [#113](https://github.com/nepenth/synara-desktop/pull/113)                                                 | First CI run failed `check:quality-gates` (expected needs lacked `changes` + skipped). Checker now matches path-filtered Quality gate.               |
| ~13:48     | MiniMax tooling #109                                | **CI fail**                                                                                                                        | iOS job cancelled (~45m hang); Quality gate failed. Not product path — deprioritize; merge after #113 if still wanted.                               |
| ~13:45     | CI path filters for heavy jobs                      | **PR open** [#113](https://github.com/nepenth/synara-desktop/pull/113)                                                             | Docs-only skip full suite; src-tauri skips iOS/Synapse.                                                                                              |
| ~13:40     | **P3.6** rustfmt CI fix                             | **Pushed** `78c61ea` on [#112](https://github.com/nepenth/synara-desktop/pull/112)                                                 | `cargo fmt --check` failed on test wrapping; local tests 5/5 + lifecycle 36/36 + guardrails PASS. CI re-run.                                         |
| ~13:29     | **P3.6** session restore foundation                 | **PR open** [#112](https://github.com/nepenth/synara-desktop/pull/112)                                                             | Vault → identity bind → `restore_session` under lifecycle only.                                                                                      |
| ~13:30     | **PROGRESS.md** live work log introduced            | **PR open** [#111](https://github.com/nepenth/synara-desktop/pull/111)                                                             | Remote-monitor file for orchestrator updates.                                                                                                        |
| ~13:23     | **P3.5** session secret / refresh-token persistence | **Merged** [#110](https://github.com/nepenth/synara-desktop/pull/110)                                                              | Host keyring vault + `persist_session_after_login`. Tip `8b7d39e`.                                                                                   |
| ~12:57     | Cutover **operating model** docs                    | **Merged** [#108](https://github.com/nepenth/synara-desktop/pull/108)                                                              | Canonical capability slices + atomic sole-owner cutover.                                                                                             |
| ~12:36     | **P3.2** password/token login + device naming       | **Merged** [#107](https://github.com/nepenth/synara-desktop/pull/107)                                                              | Harness login under `matrix/auth/`; D-NEW-DEVICE names; guardrail allowlist.                                                                         |
| earlier    | **R0.2-E1** traceability tooling                    | **Merged** [#82](https://github.com/nepenth/synara-desktop/pull/82)                                                                | Governance tooling; not product cutover.                                                                                                             |
| earlier    | R0.3–R0.8 Critical/High remediations                | **Merged** #86–#104 band                                                                                                           | Wipe, keyring, privacy, IPC, live adapters, formal residual reports.                                                                                 |
| policy     | Product-first + clean-break                         | **User-approved**                                                                                                                  | Re-login/wipe OK; no dual-backend; no elaborate JS→Rust session migration.                                                                           |
| tooling    | Local MiniMax (Spark) for bulk draft/review         | Config + open PR [#109](https://github.com/nepenth/synara-desktop/pull/109)                                                        | Free-token parallel text worker; Grok remains implementer.                                                                                           |

### Earlier foundation (condensed)

| Band                                                                   | State                                 |
| ---------------------------------------------------------------------- | ------------------------------------- |
| Phase 0 planning artifacts P0.1–P0.7                                   | Landed (strict gate **open**)         |
| Phase 1 IPC/DTO/guardrails P1.1–P1.6                                   | Landed (strict gate **open**)         |
| Phase 2 supervisor/store/builder/tasks/diagnostics/lifecycle P2.1–P2.6 | Landed harness (strict gate **open**) |
| P3.1 discovery + login-flow list                                       | Landed                                |

---

## Roadmap strip (capability order)

|   # | Slice                                              | Status                                                               |
| --: | -------------------------------------------------- | -------------------------------------------------------------------- |
|   1 | Discovery / login-flow list (P3.1)                 | **Done** (artifact)                                                  |
|   2 | Password/token login + device name (P3.2)          | **Done** (merged)                                                    |
|   3 | Session secret persist / refresh structure (P3.5)  | **Done** (merged)                                                    |
|   4 | Session restore after restart (P3.6)               | **Done** (merged #112)                                               |
|   5 | Sync readiness / reconnect (P4.1)                  | **Done** (merged #114)                                               |
|   6 | Room list snapshot/delta (P4.2)                    | **Done** (merged #115)                                               |
|   7 | Membership / unread / invites (P4.3)               | **Done** (merged #116)                                               |
|   8 | Favorite / low-priority / recent (P4.4)            | **Done** (merged #117)                                               |
|   9 | Space hierarchy (P4.5)                             | **Done** (merged #118)                                               |
|  10 | Timeline registry (P5.1)                           | **Done** (merged #119)                                               |
|  11 | Timeline diffs (P5.2)                              | **Done** (merged #121)                                               |
|  12 | Timeline pagination (P5.3)                         | **Done** (merged #122)                                               |
|  13 | Send queue / local echo (P6.1)                     | **Done** (merged #124)                                               |
|  14 | Receipt index (P6.2)                               | **Done** (merged #125)                                               |
|  15 | Typing index (P6.3)                                | **Done** (merged #127)                                               |
|  16 | Media upload queue (P6.4)                          | **Done** (merged #128)                                               |
|  17 | Relations / reactions (P5.6)                       | **Done** (merged #129)                                               |
|  18 | Thread list / summaries (P5.8)                     | **Done** (merged #131)                                               |
|  19 | Member / power-level index (P4.6)                  | **Done** (merged #133)                                               |
|  20 | Notification candidates (P7.1)                     | **Done** (merged #135)                                               |
|  21 | Security status projection (P8.1)                  | **Done** (merged #137)                                               |
|  22 | Route / deep-link resolution (P4.8)                | **Done** (merged #139)                                               |
|  23 | Search session (P6.8)                              | **Done** (merged #141)                                               |
|  24 | Cross-signing / identity (P8.4)                    | **Done** (merged #149)                                               |
|  25 | Widget / Element Call registry (P9.1)              | **Done** (merged #143)                                               |
|  26 | Device list / trust (P8.2)                         | **In PR** [#145](https://github.com/nepenth/synara-desktop/pull/145) |
|  27 | Verification inbox / SAS display (P8.3)            | **In PR** [#147](https://github.com/nepenth/synara-desktop/pull/147) |
|  28 | Backup / recovery flow (P8.5)                      | **In PR** [#150](https://github.com/nepenth/synara-desktop/pull/150) |
|  29 | Legacy transition coordinator (P3.7)               | **In PR** [#151](https://github.com/nepenth/synara-desktop/pull/151) |
|  30 | Remaining crypto / UTD / store continuity          | Not started                                                          |
|  31 | Atomic sole-owner cutover + js-sdk burn-down (P11) | Not started                                                          |
|  32 | Merge to `main` (#39)                              | Needs **explicit user approval**                                     |

---

## Links for phone / remote refresh

| What                       | URL                                                                                                                            |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| **This progress log**      | https://github.com/nepenth/synara-desktop/blob/feature/matrix-rust-sdk-full-replacement/docs/matrix-rust-sdk/PROGRESS.md       |
| Integration branch commits | https://github.com/nepenth/synara-desktop/commits/feature/matrix-rust-sdk-full-replacement                                     |
| Open PRs into integration  | https://github.com/nepenth/synara-desktop/pulls?q=is%3Apr+is%3Aopen+base%3Afeature%2Fmatrix-rust-sdk-full-replacement          |
| Machine status ledger      | https://github.com/nepenth/synara-desktop/blob/feature/matrix-rust-sdk-full-replacement/docs/matrix-rust-sdk/program-status.md |
| Umbrella PR (do not merge) | https://github.com/nepenth/synara-desktop/pull/39                                                                              |
