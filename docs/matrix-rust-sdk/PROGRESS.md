# Matrix Rust SDK replacement — live progress log

> **Remote-monitor file.** Open this on GitHub on the integration branch and refresh
> to see what the orchestrator has completed and what is next.
>
> **Branch:** [`feature/matrix-rust-sdk-full-replacement`](https://github.com/nepenth/synara-desktop/tree/feature/matrix-rust-sdk-full-replacement)  
> **This file on GitHub:**
> [docs/matrix-rust-sdk/PROGRESS.md](https://github.com/nepenth/synara-desktop/blob/feature/matrix-rust-sdk-full-replacement/docs/matrix-rust-sdk/PROGRESS.md)

| Field | Value |
| --- | --- |
| Last updated (UTC) | **2026-07-27 ~14:42** |
| Integration tip | `168ca2b` — Merge #113 CI path-filter heavy jobs |
| Product runtime | Still **`matrix-js-sdk` only** until atomic sole-owner cutover |
| Dual backend | **`false`** (forbidden forever) |
| Operating model | [cutover-operating-model.md](cutover-operating-model.md) |
| Machine ledger | [program-status.md](program-status.md) (generated; do not hand-edit) |
| Short continuation | [CONTINUATION.md](CONTINUATION.md) |
| Full handoff | [implementation-handoff.md](implementation-handoff.md) |
| Umbrella → main | [PR #39](https://github.com/nepenth/synara-desktop/pull/39) — **do not merge without explicit user approval** |

---

## Snapshot (read this first)

| | |
| --- | --- |
| **Now** | **#113 CI path filters merged** (`168ca2b`). **#114** P4.1 branch updated onto tip (CI re-run). Next: merge #114 when green, then #115–#119. |
| **Inventory** | ~23/112 original task artifacts when program-status is synced (P3.1–P3.2 + P3.5–P3.6 landed; see ledger). |
| **Phase gates** | **0 / 15** strict gates closed (honest). |
| **Open PRs → integration** | [#114](https://github.com/nepenth/synara-desktop/pull/114) P4.1; [#115](https://github.com/nepenth/synara-desktop/pull/115) P4.2; [#116](https://github.com/nepenth/synara-desktop/pull/116) P4.3; [#111](https://github.com/nepenth/synara-desktop/pull/111) this log; [#109](https://github.com/nepenth/synara-desktop/pull/109) MiniMax (deprioritize). |
| **Blocked on** | Rebase/resolve #114 vs tip; CI green for product PRs. Required: Quality gate + Desktop package gate. |
| **Dogfood path** | Login ✅ → persist ✅ → restore ✅ (**#112**) → **sync readiness (#114)** → **room list (#115)** → timeline. |

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
- Commit as `docs(matrix): progress log — …` on a PR or as part of the landing PR.

---

## Work log (newest first)

### 2026-07-27

| When (UTC) | Item | Result | Notes |
| --- | --- | --- | --- |
| ~14:42 | CI path filters for heavy jobs | **Merged** [#113](https://github.com/nepenth/synara-desktop/pull/113) | Job-level scopes; quality-gate accepts success\|skipped. Tip `168ca2b`. |
| ~14:30 | **P5.1** timeline registry foundation | **PR open** [#119](https://github.com/nepenth/synara-desktop/pull/119) | TimelineRegistry lifecycle; local 8/8. |
| ~14:25 | **P4.5** space hierarchy foundation | **PR open** [#118](https://github.com/nepenth/synara-desktop/pull/118) | SpaceHierarchy + filter/cycle; local 6/6. |
| ~14:21 | **P4.4** favorite/low-priority/folder/recent | **PR open** [#117](https://github.com/nepenth/synara-desktop/pull/117) | DTO tag fields + sorts; local room_list 15/15. |
| ~14:08 | **#114** rebased on tip after #112 | **Pushed** `d0ab3e5` | Combined lifecycle restore + sync guardrail zones; matrix tests 270/270. #115/#113 also tip-merged. |
| ~14:07 | **P3.6** session restore | **Merged** [#112](https://github.com/nepenth/synara-desktop/pull/112) | Vault → identity bind → `restore_session`. Tip `69f1087`. Full CI green (iOS ~23m). |
| ~14:00 | **P4.2** room-list snapshot/delta | **PR open** [#115](https://github.com/nepenth/synara-desktop/pull/115) | Pure `RoomListProjection` + delta ops + sequence gap resync. Local 10/10; stacks on #114. |
| ~13:53 | **P4.1** sync readiness foundation | **PR open** [#114](https://github.com/nepenth/synara-desktop/pull/114) | `matrix/sync/`: readiness map, reconnect table, SyncServiceOwner, guardrail confine `SyncService::builder`. Local 12/12 + clippy + guardrails green. |
| ~13:52 | CI path-filter policy checker fix | **Pushed** `09bd360` on [#113](https://github.com/nepenth/synara-desktop/pull/113) | First CI run failed `check:quality-gates` (expected needs lacked `changes` + skipped). Checker now matches path-filtered Quality gate. |
| ~13:48 | MiniMax tooling #109 | **CI fail** | iOS job cancelled (~45m hang); Quality gate failed. Not product path — deprioritize; merge after #113 if still wanted. |
| ~13:45 | CI path filters for heavy jobs | **PR open** [#113](https://github.com/nepenth/synara-desktop/pull/113) | Docs-only skip full suite; src-tauri skips iOS/Synapse. |
| ~13:40 | **P3.6** rustfmt CI fix | **Pushed** `78c61ea` on [#112](https://github.com/nepenth/synara-desktop/pull/112) | `cargo fmt --check` failed on test wrapping; local tests 5/5 + lifecycle 36/36 + guardrails PASS. CI re-run. |
| ~13:29 | **P3.6** session restore foundation | **PR open** [#112](https://github.com/nepenth/synara-desktop/pull/112) | Vault → identity bind → `restore_session` under lifecycle only. |
| ~13:30 | **PROGRESS.md** live work log introduced | **PR open** [#111](https://github.com/nepenth/synara-desktop/pull/111) | Remote-monitor file for orchestrator updates. |
| ~13:23 | **P3.5** session secret / refresh-token persistence | **Merged** [#110](https://github.com/nepenth/synara-desktop/pull/110) | Host keyring vault + `persist_session_after_login`. Tip `8b7d39e`. |
| ~12:57 | Cutover **operating model** docs | **Merged** [#108](https://github.com/nepenth/synara-desktop/pull/108) | Canonical capability slices + atomic sole-owner cutover. |
| ~12:36 | **P3.2** password/token login + device naming | **Merged** [#107](https://github.com/nepenth/synara-desktop/pull/107) | Harness login under `matrix/auth/`; D-NEW-DEVICE names; guardrail allowlist. |
| earlier | **R0.2-E1** traceability tooling | **Merged** [#82](https://github.com/nepenth/synara-desktop/pull/82) | Governance tooling; not product cutover. |
| earlier | R0.3–R0.8 Critical/High remediations | **Merged** #86–#104 band | Wipe, keyring, privacy, IPC, live adapters, formal residual reports. |
| policy | Product-first + clean-break | **User-approved** | Re-login/wipe OK; no dual-backend; no elaborate JS→Rust session migration. |
| tooling | Local MiniMax (Spark) for bulk draft/review | Config + open PR [#109](https://github.com/nepenth/synara-desktop/pull/109) | Free-token parallel text worker; Grok remains implementer. |

### Earlier foundation (condensed)

| Band | State |
| --- | --- |
| Phase 0 planning artifacts P0.1–P0.7 | Landed (strict gate **open**) |
| Phase 1 IPC/DTO/guardrails P1.1–P1.6 | Landed (strict gate **open**) |
| Phase 2 supervisor/store/builder/tasks/diagnostics/lifecycle P2.1–P2.6 | Landed harness (strict gate **open**) |
| P3.1 discovery + login-flow list | Landed |

---

## Roadmap strip (capability order)

| # | Slice | Status |
| ---: | --- | --- |
| 1 | Discovery / login-flow list (P3.1) | **Done** (artifact) |
| 2 | Password/token login + device name (P3.2) | **Done** (merged) |
| 3 | Session secret persist / refresh structure (P3.5) | **Done** (merged) |
| 4 | Session restore after restart (P3.6) | **Done** (merged #112) |
| 5 | Sync readiness / reconnect (P4.1) | **In PR** [#114](https://github.com/nepenth/synara-desktop/pull/114) |
| 6 | Room list snapshot/delta (P4.2) | **In PR** [#115](https://github.com/nepenth/synara-desktop/pull/115) |
| 7 | Membership / unread / invites (P4.3) | **In PR** [#116](https://github.com/nepenth/synara-desktop/pull/116) |
| 8 | Favorite / low-priority / recent (P4.4) | **In PR** [#117](https://github.com/nepenth/synara-desktop/pull/117) |
| 9 | Space hierarchy (P4.5) | **In PR** [#118](https://github.com/nepenth/synara-desktop/pull/118) |
| 10 | Timeline registry (P5.1) | **In PR** [#119](https://github.com/nepenth/synara-desktop/pull/119) |
| 11 | Timeline diffs (P5.2+) | Not started |
| 10 | Crypto / verification / recovery | Not started |
| 11 | Atomic sole-owner cutover + js-sdk burn-down (P11) | Not started |
| 12 | Merge to `main` (#39) | Needs **explicit user approval** |


---

## Links for phone / remote refresh

| What | URL |
| --- | --- |
| **This progress log** | https://github.com/nepenth/synara-desktop/blob/feature/matrix-rust-sdk-full-replacement/docs/matrix-rust-sdk/PROGRESS.md |
| Integration branch commits | https://github.com/nepenth/synara-desktop/commits/feature/matrix-rust-sdk-full-replacement |
| Open PRs into integration | https://github.com/nepenth/synara-desktop/pulls?q=is%3Apr+is%3Aopen+base%3Afeature%2Fmatrix-rust-sdk-full-replacement |
| Machine status ledger | https://github.com/nepenth/synara-desktop/blob/feature/matrix-rust-sdk-full-replacement/docs/matrix-rust-sdk/program-status.md |
| Umbrella PR (do not merge) | https://github.com/nepenth/synara-desktop/pull/39 |
