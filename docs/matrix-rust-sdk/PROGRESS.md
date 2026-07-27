# Matrix Rust SDK replacement — live progress log

> **Remote-monitor file.** Open this on GitHub on the integration branch and refresh
> to see what the orchestrator has completed and what is next.
>
> **Branch:** [`feature/matrix-rust-sdk-full-replacement`](https://github.com/nepenth/synara-desktop/tree/feature/matrix-rust-sdk-full-replacement)  
> **This file on GitHub:**
> [docs/matrix-rust-sdk/PROGRESS.md](https://github.com/nepenth/synara-desktop/blob/feature/matrix-rust-sdk-full-replacement/docs/matrix-rust-sdk/PROGRESS.md)

| Field | Value |
| --- | --- |
| Last updated (UTC) | **2026-07-27 ~20:26** |
| Integration tip | `116ed3d` — Merge #129 P5.6 relations index |
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
| **Now** | **#129 P5.6 merged.** Primary next: **#131 P5.8** threads (tip-updated after #129; CI re-run). |
| **Inventory** | ~36/112 original task artifacts when program-status is synced (through P5.6 landed; see ledger). |
| **Phase gates** | **0 / 15** strict gates closed (honest). |
| **Open PRs → integration** | [#131](https://github.com/nepenth/synara-desktop/pull/131) threads, [#133](https://github.com/nepenth/synara-desktop/pull/133) members, [#135](https://github.com/nepenth/synara-desktop/pull/135) notifications; [#109](https://github.com/nepenth/synara-desktop/pull/109) MiniMax (deprioritize). |
| **Blocked on** | CI green for #131 after tip-merge with #129. Required: Quality gate + Desktop package gate. |
| **Dogfood path** | media ✅ → relations ✅ (**#129**) → **threads (#131)** → members/notifications. |

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
| ~20:24 | **P5.6** relations index foundation | **Merged** [#129](https://github.com/nepenth/synara-desktop/pull/129) | RelationIndex reactions/replaces/refs/threads; tip `116ed3d`. |
| ~20:06 | **P7.1** notification candidate index | **PR open** [#135](https://github.com/nepenth/synara-desktop/pull/135) | NotificationIndex suppress/dedup/cap; local 7/7. |
| ~19:58 | **P6.4** media upload queue foundation | **Merged** [#128](https://github.com/nepenth/synara-desktop/pull/128) | UploadQueue metadata-only; tip `f44bc5c`. Dogfood send/receipts/typing/media foundations landed. |
| ~19:42 | **P4.6** member / power-level index | **PR open** [#133](https://github.com/nepenth/synara-desktop/pull/133) | MemberIndex; local 6/6. |
| ~19:33 | **P6.3** typing index foundation | **Merged** [#127](https://github.com/nepenth/synara-desktop/pull/127) | TypingIndex + cap 32; tip `ef8bf60`. Quality + package gate green. |
| ~19:15 | **P5.8** thread list index foundation | **PR open** [#131](https://github.com/nepenth/synara-desktop/pull/131) | ThreadIndex over ThreadSummary; local 6/6. |
| ~19:06 | **P6.2** receipt index foundation | **Merged** [#125](https://github.com/nepenth/synara-desktop/pull/125) | ReceiptIndex over DTO Receipt; tip `ef75e3e`. Quality + package gate green. |
| ~18:52 | **P5.6** relations index foundation | **PR open** [#129](https://github.com/nepenth/synara-desktop/pull/129) | RelationIndex annotations/replace/reference/thread; local 8/8. |
| ~18:39 | **P6.1** outbound send queue foundation | **Merged** [#124](https://github.com/nepenth/synara-desktop/pull/124) | SendQueue + LocalEchoState; tip `c6cbc2c`. Quality ✅; package gate after Arch Docker Hub flake re-run. |
| ~18:35 | Tip-merge P6.2/P6.3/P6.4 onto tip | **Pushed** | #125/#127/#128 had green CI on pre-#124 tip; re-tip after #124. Local receipts 7/7 + send 8/8. |
| ~17:58 | **P5.3** timeline pagination foundation | **Merged** [#122](https://github.com/nepenth/synara-desktop/pull/122) | TimelinePagination state machine; tip `ed5b3c3`. |
| ~16:00 | **P4.2** room-list snapshot/delta | **Merged** [#115](https://github.com/nepenth/synara-desktop/pull/115) | Pure projection + ordered ops; tip `c2cdc0b`. iOS/Synapse skipped. |
| ~15:56 | **P6.1** outbound send queue foundation | **PR open** [#124](https://github.com/nepenth/synara-desktop/pull/124) | LocalEchoState queue; no Room::send; local 8/8. Disk pressure: cleaned cargo target (-34GB). |
| ~15:40 | **P4.1** sync readiness + reconnect | **Merged** [#114](https://github.com/nepenth/synara-desktop/pull/114) | `matrix/sync/`: readiness, reconnect table, SyncServiceOwner, guardrail confine. Tip `f9bfe0d`. Full CI green (iOS skipped via path filters). |
| ~15:38 | **P5.3** timeline pagination foundation | **PR open** [#122](https://github.com/nepenth/synara-desktop/pull/122) | Pure `TimelinePagination` state machine; local timeline 23/23. CI deferred until stack advances. |
| ~15:11 | **P5.2** timeline snapshot/diff projection | **PR open** [#121](https://github.com/nepenth/synara-desktop/pull/121) | `TimelineProjection` + ordered ops; local 16/16 then extended by P5.3. |
| ~15:40 | **#115** tip-merged after #114 | **Pushed** | Local matrix 270/270 + clippy + guardrails; CI re-run for P4.2 merge. |
| ~14:47 | P4.3+ clippy `needless_borrow` | **Fixed** on [#116](https://github.com/nepenth/synara-desktop/pull/116)–[#119](https://github.com/nepenth/synara-desktop/pull/119) | Lint Rust failed at `room_list/tests.rs` scope test; dropped extra `&` on `find().unwrap()`. Branches rebased/merged onto tip. |
| ~14:46 | Tip update-branch #114/#115/#109 | **Kicked** | After #111 merge so product PRs not BEHIND; #109 should get path-filtered skip of iOS. |
| ~14:45 | **PROGRESS.md** live work log | **Merged** [#111](https://github.com/nepenth/synara-desktop/pull/111) | Docs-only CI: heavy jobs skipped, Quality gate green. Tip `b3397db`. |
| ~14:42 | CI path filters for heavy jobs | **Merged** [#113](https://github.com/nepenth/synara-desktop/pull/113) | Job-level scopes; quality-gate accepts success\|skipped. Prior tip `168ca2b`. |
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
| 5 | Sync readiness / reconnect (P4.1) | **Done** (merged #114) |
| 6 | Room list snapshot/delta (P4.2) | **Done** (merged #115) |
| 7 | Membership / unread / invites (P4.3) | **Done** (merged #116) |
| 8 | Favorite / low-priority / recent (P4.4) | **Done** (merged #117) |
| 9 | Space hierarchy (P4.5) | **Done** (merged #118) |
| 10 | Timeline registry (P5.1) | **Done** (merged #119) |
| 11 | Timeline diffs (P5.2) | **Done** (merged #121) |
| 12 | Timeline pagination (P5.3) | **Done** (merged #122) |
| 13 | Send queue / local echo (P6.1) | **Done** (merged #124) |
| 14 | Receipt index (P6.2) | **Done** (merged #125) |
| 15 | Typing index (P6.3) | **Done** (merged #127) |
| 16 | Media upload queue (P6.4) | **Done** (merged #128) |
| 17 | Relations / reactions (P5.6) | **Done** (merged #129) |
| 18 | Thread list / summaries (P5.8) | **In PR** [#131](https://github.com/nepenth/synara-desktop/pull/131) |
| 19 | Member / power-level index (P4.6) | **In PR** [#133](https://github.com/nepenth/synara-desktop/pull/133) |
| 20 | Notification candidates (P7.1) | **In PR** [#135](https://github.com/nepenth/synara-desktop/pull/135) |
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
