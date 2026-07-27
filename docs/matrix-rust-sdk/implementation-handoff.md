# Matrix Rust SDK Replacement — Execution Handoff

Last updated: 2026-07-27

Authoritative program plan: [`../matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md)

<!-- matrix-rust-program-status-link -->
Current delivery and strict-acceptance state:
[`program-status.md`](program-status.md) (generated from
[`program-status.json`](program-status.json)).

Traceability artifacts:

- [`feature-parity-traceability.json`](feature-parity-traceability.json)
- [`feature-parity-traceability.md`](feature-parity-traceability.md)

## Live continuation snapshot (2026-07-27)

This section is the current handoff. The dated audit and former implementation
ledger below remain historical evidence and must not override this snapshot or
the canonical status ledger.

| Field | Current value |
|---|---|
| **Integration branch / tip** | `feature/matrix-rust-sdk-full-replacement` @ `447cbdcdd43a26775db32d8d62d6929d8a5c09b9` (PR #82 / R0.2-E1 on R0.8 slice-1 + R0.7 + R0.3–R0.6) |
| **Dual-track strategy** | Land Critical/High product fixes first; R0.2-E1 unparked with memory-bound fix and merged; R0.8 residual formal reports without false gate close |
| **R0.5 / REV-001** | **Merged and accepted** via PR [#86](https://github.com/nepenth/synara-desktop/pull/86) |
| **R0.4 / REV-002/006/007** | **Merged and accepted** via PR [#87](https://github.com/nepenth/synara-desktop/pull/87) (path confinement) + PR [#94](https://github.com/nepenth/synara-desktop/pull/94) (native keyring vault + encrypted reopen evidence) |
| **R0.6 / REV-003** | **Merged and accepted** via PR [#89](https://github.com/nepenth/synara-desktop/pull/89) — privacy-safe plan/layout/wipe/SDK errors; adversarial redaction fixtures |
| **R0.3 / REV-004/005** | **Merged and accepted** via PR [#91](https://github.com/nepenth/synara-desktop/pull/91) (wire counters + stream-id authority) + PR [#92](https://github.com/nepenth/synara-desktop/pull/92) (topic→DTO bodies + secret/media reject) |
| **R0.7 live adapters** | **Slices 1–4 merged** via PR [#96](https://github.com/nepenth/synara-desktop/pull/96) (CS transports) + [#98](https://github.com/nepenth/synara-desktop/pull/98) (loopback login-types) + [#100](https://github.com/nepenth/synara-desktop/pull/100) (composed encrypted store open/ready/logout/reopen/wipe) + [#102](https://github.com/nepenth/synara-desktop/pull/102) (stale-generation after real SDK logout + wrong-key reopen privacy). Strict acceptance **open** (authenticated live sync residual — login APIs guardrail-banned until deliberate P3.2 allowlist) |
| **R0.8 formal reports** | **Slice 1 merged** via PR [#104](https://github.com/nepenth/synara-desktop/pull/104): readiness inventory + Phase 0/1/2 + P3.1 formal reports with verdict **`not_accepted`**. See [`r0.8-phase-gate-readiness-inventory.md`](r0.8-phase-gate-readiness-inventory.md). Strict acceptance **open**; **0** phase gates closed |
| **Active remediation** | R0.8 residual formal path / R0.7 auth residual / R0.2 parked; **P3.2** blocked by unaccepted R0.2 / R0.7 residual / R0.8 |
| **R0.2-E1** | **Merged** via PR [#82](https://github.com/nepenth/synara-desktop/pull/82) after deliberate memory-bound RSS fix (phase-isolated audit/v2 children + incremental 512 MiB budget). Strict R0.2 acceptance **open** — E2 + Phase 0 evidence residuals remain |
| **Next product slice** | **R0.2-E2** (normalized audit+v2 artifacts via E1 tooling) or R0.7 authenticated residual / R0.8 accepting re-issue; never false-accept phase gates |
| **Product runtime** | `matrix-js-sdk` only; Rust remains harness foundation; no dual backend or cutover |
| **Progress** | 20/112 original artifacts (~18%); R0 work does not increment this; 0/15 strict phase gates closed |
| **Main PR** | [#39](https://github.com/nepenth/synara-desktop/pull/39) remains open and must not merge without explicit user approval |

### R0.2 work landed before E1

| Delivery | Evidence | State |
|---|---|---|
| Security threat model and owned risk register | PR #79, integration commit `239c04b` | Merged; R0.2 remains open |
| Native-agent task/review governance contracts and validators | PR #80, integration commit `4ec78c7` | Merged; R0.2 remains open |
| Phase 0 test/Synapse topology and evidence manifest | PR #81, integration commit `c358502` | Merged; R0.2 remains open |
| CI full-history prerequisite for traceability validation | PR #83, integration commit `7ffd588` | Merged; prerequisite only |

R0.1 was accepted before this sequence. The entries above are corrective
deliveries within R0.2, not additional original-plan features and not authority
to accept R0.2 or Phase 0.

### E1 implementation and present blocker

PR #82 adds deterministic R0.2-E1 audit-normalization and traceability-v2
tooling in exactly 11 paths: two closed JSON schemas; one shared implementation
library; check, generate, and migrate entry points; the package script; and two
test files. The tooling binds normalized evidence to immutable Git objects,
enforces a durable append-only lifecycle/report/authorization chain, validates
source and cutover projections, detects prior v2 history, and exercises
production-shaped plus adversarial real-Git cases. It does not change a product
Matrix client, dependency, lockfile, CI workflow, or runtime backend.

Independent review rejected earlier drafts until they prevented one-shot state
authorization, truncatable report history, invalid risk/decision transitions,
deletable historical evidence, incorrect subject ancestry handling, created-
entity source loss, stale execution contracts, and inadequately bounded Git
history discovery. Benchmark isolation and historical schema/semantic checks were
also tightened. The frozen 11-path content at `8ded923` matched the locally
accepted content byte-for-byte.

Local acceptance evidence for that exact content:

| Validation | Result |
|---|---|
| Independent frozen-tree review | **ACCEPT**, no remaining blocking correctness, lifecycle-integrity, schema/runtime, CLI-safety, security, or scope finding |
| `npm run test:matrix-rust-traceability-tooling` | **86/86 PASS** |
| `node --test scripts/__tests__/*.test.mjs` | **284/284 PASS** |
| `npm run check:matrix-rust-guardrails` | **PASS**, 1,588 files checked |
| `npm run check:matrix-rust-governance` | **PASS** |
| `npm run check:quality-gates` | **PASS** |
| Prettier 2.8.1 across the exact 11 paths | **PASS** |
| `node --check` across all six production E1 scripts | **PASS** |
| `git diff --check` and exact changed-path audit | **PASS** |

CI provided two useful environment checks. The first E1 run failed because CI
used a shallow checkout while the validator correctly requires pinned historical
objects. PR #83 changed validation checkout to full history and passed all of its
required jobs. The refreshed PR #82 run then passed 283 of 284 repository script
tests. Its sole primary failure is in the test harness:
`temporaryLocalGitClone` creates a commit without setting repository-local
`user.name` and `user.email`. Developer-local Git identity masked that portability
defect. The downstream `Quality gate` failure is a consequence of the failed
desktop-validation job, not a second tooling failure. iOS simulator, disposable
Synapse, macOS bundle, Linux package, and desktop package jobs passed.

At handoff, the focused test-helper fix is intentionally not present. PR #82 is
mergeable at the Git level but blocked by CI and the plan's exact-head merge gate.

### Exact next-owner procedure

1. Fetch and verify integration remains at or after `7ffd588`, PR #82 remains
   open, and the E1 branch is clean at `8ded923` or a documented later SHA.
2. In `temporaryLocalGitClone`, configure `user.name` and `user.email` in the
   cloned repository before any fixture commit. Keep the change inside the
   existing E1 test file and do not broaden the exact 11-path scope.
3. Run the focused failing temporary-clone regression, then the 86-test E1 suite,
   the full repository script suite, guardrails, governance, quality gates,
   exact-scope Prettier, six-script syntax checks, and `git diff --check`.
4. Independently inspect the complete integration-to-head diff again. Confirm no
   implementation, schema, or assertion weakening accompanied the helper fix.
5. Commit and push the correction to PR #82. Record acceptance against the new
   exact head and wait for every required job to finish green and non-cancelled.
6. Merge #82 only into `feature/matrix-rust-sdk-full-replacement`; update the
   local integration branch and rerun the relevant smoke gates.
7. Start E2 only after that merge. E2 recovers the authoritative reviewed
   119-row source payloads and generates/commits the normalized audit and v2
   traceability artifacts. It must not reconstruct, paraphrase, or invent missing
   payloads and must use the accepted E1 tooling.
8. Do not mark R0.2 complete after E1 or E2. Continue the remaining owned
   residual/evidence closure and readiness-report work required by the plan.

Detailed validation, workflow assessment, and the exact 11-path inventory are in
[`r0.2-e1-handoff-2026-07-26.md`](r0.2-e1-handoff-2026-07-26.md).

## Authoritative continuation snapshot (2026-07-25 independent audit)

Start with [`review-2026-07-25.md`](review-2026-07-25.md). It contains the
reviewed commit range, finding-level evidence, validation results, R0.1–R0.8
remediation tasks, and the corrected continuation sequence.

| Field | Audited value |
|---|---|
| **Current status and next task** | See [`program-status.md`](program-status.md) |
| **Integration branch** | `feature/matrix-rust-sdk-full-replacement` |
| **Audited integration tip** | `edfefee499064b736985b6528896b693e5120f22` (always re-fetch and verify) |
| **Open PRs → integration at audit start** | None |
| **Open PR → `main`** | [#39](https://github.com/nepenth/synara-desktop/pull/39) — do not merge without explicit user approval |
| **Product Matrix runtime** | `matrix-js-sdk` only; no Rust production login/sync, dual backend, or cutover |
| **Landed inventory** | P0.1–P0.7, P1.1–P1.6, P2.1–P2.6, and P3.1 foundation (20/112 original task artifacts) |
| **Strict acceptance** | Phase 0 open; Phase 1 open; Phase 2 open; P3.1 open; 0/15 phase gates closed |

The former handoff's clean-worktree and no-cutover claims were accurate. Its
“Phases 0–2 complete,” “P3.1 complete,” and “next P3.2” claims are superseded.
The independent run found Rust fmt/clippy failures, TypeScript lint/Prettier
failures, a failed GitHub desktop-validation job, missing required planning/live
evidence, and critical/high lifecycle, filesystem, IPC, and privacy findings.

Continue the active task named in the canonical status ledger and follow the
main plan's native orchestrator protocol. A task may merge only on a green,
non-cancelled required CI run for the reviewed SHA.

## Superseded historical handoff snapshot (2026-07-26)

The following section is retained as a record of what the prior agent reported;
it is not current acceptance or continuation guidance.

Do not use this section to resume work. It is preserved only as a dated record
of the prior agent's claims.

| Field | Value |
|-------|--------|
| **Status** | **Paused for human handoff** — no automatic 4-minute progress loop is running |
| **Integration branch** | `feature/matrix-rust-sdk-full-replacement` |
| **Integration tip** | `3bea95a915c6b4368e81ed1aced88c86cb1fc602` (verify with `git rev-parse origin/feature/matrix-rust-sdk-full-replacement`) |
| **Tip messages** | `docs(matrix): continuation handoff snapshot…` on top of P3.1 merge/handoff (`#72` / `#73`) |
| **Open PRs → integration** | **None** after this handoff lands (all work through P3.1 + this snapshot merged) |
| **Open PR → `main`** | [#39](https://github.com/nepenth/synara-desktop/pull/39) umbrella “Plan complete Matrix Rust SDK replacement” — **do not merge without explicit user approval** |
| **Product Matrix runtime** | Still **`matrix-js-sdk` only** (no dual-backend; no frontend cutover) |
| **Next task** | **P3.2 — Password/token login and device naming** (harness only) |
| **Local validation (2026-07-26)** | `cargo test --locked matrix::` → **189 PASS**; `npm run check:matrix-rust-guardrails` → **PASS** |
| **Progress (plan line items)** | **~20 / ~112 tasks (~18%)**; Phases **0–2 complete**; Phase **3** at **P3.1/8** |
| **Working tree** | Integration tip is the source of truth; no uncommitted handoff work should remain after this snapshot lands |

### What exists on integration (Rust)

Under `src-tauri/src/matrix/`:

| Module | Plan tasks | Notes |
|--------|------------|--------|
| `ipc/` | P1.3, P1.5 | Versioned envelope, fixtures, contract tests |
| `dto/` | P1.4 | 15 domain DTO families |
| `supervisor/` | P2.1 | Lifecycle actor + generation isolation |
| `store/` | P2.2 | Per-account paths + store-key vault |
| `client_builder/` | P2.3 | Sole allowed `Client::builder` site; unauthenticated open |
| `tasks/` | P2.4 | Generation-stamped task supervision |
| `diagnostics/` | P2.5 | Privacy-filtered metrics + redaction |
| `lifecycle/` | P2.6 | Logout ≠ wipe; exact-target wipe; no auto-delete on store failure |
| `auth/` | P3.1 | Discovery + login-flow **list** only (no login execution) |
| (+ `matrix_sdk_link_smoke` in crate root) | P1.2 | Compile-only SDK type-path smoke |

Frontend mirrors: `synara/src/app/features/matrix-ipc/`, `matrix-dto/` (not wired as product runtime).

### Hard rules for the next owner

1. **Never merge to `main` without explicit user approval.**
2. **PRs target `feature/matrix-rust-sdk-full-replacement` only** until cutover approval.
3. **No dual-backend / no Matrix backend selector.**
4. **No production Matrix Tauri product commands** for app cutover until planned phases (guardrails enforce).
5. **Do not re-open FR-7.8–7.11 rows** or re-promote FR-7.9-011 (partial `GATE-7.9-011`).
6. Guardrails must stay green: `npm run check:matrix-rust-guardrails`.
7. Prefer `cargo test --locked matrix::` when disk is tight; full suite when possible.
8. Tokens/credentials must never appear in logs, IPC errors, or diagnostics.

### Resume recipe

```bash
git fetch origin
git checkout feature/matrix-rust-sdk-full-replacement
git pull origin feature/matrix-rust-sdk-full-replacement
# confirm tip matches origin (at/after 3bea95a… continuation snapshot)
npm run check:matrix-rust-guardrails
(cd src-tauri && cargo test --locked matrix::)
```

Then implement **P3.2** on a fresh branch (e.g. `matrix-rust/p3.2-password-token-login`) from integration tip. After merge, update this handoff’s session state + tip SHA.

### Orchestration state

- **4-minute session scheduler:** **cancelled / not running** (former id `019f95928db7`).
- **Daily durable host task id** `9022c2f8-9b21-411a-acf9-a36c10515f72`: historically referenced; **do not assume it is active** — re-create only if the user wants a durable check-in.
- Work proceeds via **manual orchestration** or a loop the user explicitly restarts.

---

## Historical implementation ledger

This ledger preserves the per-PR/task narrative from the prior handoff. Words
such as “accepted,” “complete,” or “next P3.2” below describe that former
assessment and are overridden by the authoritative audit section above.

This remains a complete replacement program: desktop production must move from
`matrix-js-sdk` to Matrix Rust SDK, not retain a selectable or permanent second
backend. **Harness foundation code is accepted on the integration branch**
(Phases 0–2 + P3.1). **No production cutover** has been accepted: the shipping
app still uses `matrix-js-sdk` as the sole Matrix runtime; there is no dual-backend
selector and no production Matrix login/sync path in Tauri for the product UI.

### Orchestrator goal (persistent)

**Objective:** Execute the Matrix Rust SDK replacement plan using bounded native
sub-agent implementation tasks. Independently review and validate every change;
only commit, push, open PRs, or merge accepted work. Never merge to `main`
without explicit user approval.

**Session state (2026-07-26, PAUSED — P3.1 MERGED; next P3.2):**

- **Integration tip:** `feature/matrix-rust-sdk-full-replacement` @
  `3bea95a915c6b4368e81ed1aced88c86cb1fc602`
  (Phase 0–2 complete + **P3.1 MERGED** + continuation snapshot #74; re-fetch
  and prefer `git rev-parse` if HEAD has advanced)
- **Active work:** next **P3.2 — Password/token login and device naming**
  (harness only; no dual-backend; no production Matrix Tauri cutover commands;
  JS client remains sole product runtime backend). **Pipeline paused** for
  human handoff — do not auto-advance without owner instruction.
- **P2.2:** **MERGED** into integration (PR #61). Per-account store paths +
  encryption-key vault foundation under `src-tauri/src/matrix/store/`; 14 unit
  tests; design note `p2.2-store-paths-keys.md` + `.json`.
- **P0.2 PR:** https://github.com/nepenth/synara-desktop/pull/42 — **MERGED**
  into integration (docs-only). **Never merge to `main` without explicit user
  approval.**
- **P0.2 quality audit of §7.8–7.11:** COMPLETE (source-line evidence; honest
  partial/NCE gates retained; zero shallow rows remaining in 7.8–7.11). **Do not
  re-open FR-7.8–7.11 rows or re-promote FR-7.9-011.**
- **P0.4:** **MERGED** into integration (docs-only, PR #44). Evidence:
  `swift-rust-version-provenance.md` + `.json`. Embedded Rust commit for
  components-swift `26.06.06` proven as
  `1c44fb66214667c6d00acaf72ab592493653708b` (same as desktop
  `matrix-sdk-0.18.0`). Alignment decision: **exact same git commit** (A); iOS
  pin and desktop crate pins **unchanged**.
- **P0.5:** **MERGED** into integration (docs + isolated probes only, PR #45).
  Artifacts: `toolchain-compatibility-report.md` + `.json`; coexistence probe
  `probes/tauri-matrix-sdk-compat/`. Verdict: **`pass-with-residuals`**.
  Local proofs on Rust 1.93: matrix-sdk 0.18 probe `cargo check --locked` PASS;
  production `src-tauri` `cargo check --locked` PASS (no SDK); Tauri 2.11 +
  matrix-sdk/ui 0.18 coexistence `cargo check/test --locked` PASS including
  `--target x86_64-apple-darwin`. Production `src-tauri` **not** modified; no
  production matrix-sdk deps; workflows not edited. Residuals: Linux not local;
  full universal product + notarization not re-run with SDK; permanent pin is
  **P1.1**. **Never merge to `main` without explicit user approval.**
- **P0.6:** **MERGED** into integration (docs + harness only, PR #46). Artifacts:
  `performance-baseline.md` + `.json`; aggregator
  `scripts/matrix-rust-p0.6-baseline-harness.mjs` (+ unit test). Baseline is
  current **`matrix-js-sdk` product**, not Rust SDK. Automated timeline
  row-mapping multi-iteration p50/p95 recorded on macOS arm64; live UX
  latencies / Linux live / memory-CPU-disk are **methodology-only residuals**
  (no fabricated live p50/p95). Verdict: **`pass-with-residuals`**. No production
  Matrix code changes.
- **P0.7:** **MERGED** into integration (docs only). Artifacts:
  `migration-ux-decision.md` + `.json`; optional ADR pointer
  `docs/adr/0003-matrix-rust-sdk-migration-ux.md`. Verdict:
  **`migration_ux_decided`**. Required decision IDs
  `D-LEGACY-DETECT`…`D-NO-DUAL-BACKEND` recorded; no dual-backend; no unsafe
  token/device reuse into fresh crypto store; FR-7.9-011 remains sequential
  single-active only. **No production session/migration code.**
- **P1.1:** **MERGED** into integration (PR #48). Artifacts:
  repo-root `rust-toolchain.toml` (`channel = "1.93"`, rustfmt+clippy);
  `src-tauri` `rust-version = "1.93"` (edition remains 2021); all desktop
  workflows pin `dtolnay/rust-toolchain` `toolchain: 1.93`; build-and-release
  prerequisite note. Local independent review: `cargo check/test --locked`
  PASS on rustc 1.93.1 (101 tests). Clippy `-D warnings` residual is
  pre-existing product lints (out of P1.1 scope).
- **P1.2:** **MERGED** into integration (PR #49). Exact `matrix-sdk` /
  `matrix-sdk-ui` `=0.18.0` with `default-features = false` in production
  `src-tauri`; compile-only `matrix_sdk_link_smoke` (type-path only, no Client
  session); rationale + license/security review:
  `p1.2-sdk-dependency-rationale.md` + `.json`. Independent review:
  `cargo check/test --locked` PASS (102 tests) on rustc 1.93.1; transitive
  `matrix-sdk/e2e-encryption` via `matrix-sdk-ui` documented; frontend still
  `matrix-js-sdk@42.0.0`. **No** production login/sync; **no** dual-backend.
- **P1.3:** **MERGED** into integration (PR #50). Versioned Matrix IPC
  foundation: envelope (`protocolVersion`/`sessionGeneration`/`sequence`/
  kinds), 13 control kinds, 21 error categories (§6.4), stream lifecycle +
  sequence helpers, policy constants, shared JSON fixtures, parallel Rust
  (`src-tauri/src/matrix/ipc/`) + TypeScript (`synara/.../matrix-ipc/`).
  Independent review: matrix IPC unit tests PASS; `cargo test --locked`
  matrix filter 21 ok; no `matrix_sdk` in IPC modules; no production Matrix
  Tauri commands; no dual-backend. Domain DTO bodies deferred to **P1.4**.
- **P1.4:** **MERGED** into integration (PR #52). Synara domain DTOs (15
  families: session, room summary, member, timeline item [9 kinds], relation,
  receipt, typing, upload, media, security, notification, search, space,
  thread, widget). Parallel Rust (`src-tauri/src/matrix/dto/`) + TypeScript
  (`synara/.../matrix-dto/`); shared fixtures `docs/matrix-rust-sdk/dto/`;
  design note `p1.4-domain-dtos.md` + `.json`. Independent review:
  `cargo test --locked matrix` 41 ok; TS matrix-dto 23 ok; no `matrix_sdk` in
  DTO modules; no tokens/media bytes on wire; no production Matrix Tauri
  commands; P1.3 IPC left independent. **No** production login/sync.
- **P1.5:** **MERGED** into integration (PR #54). Expanded IPC protocol
  contract tests (Rust `contract_tests.rs` + `tests.rs`; TS
  `matrixIpcContract.test.ts` + `matrixIpc.test.ts`), shared fixtures +
  `schema_catalog_v1.json`, bounds helpers, envelope payload-object guard,
  design note `p1.5-ipc-contract-tests.md` + `.json`. Independent review:
  `cargo test --locked matrix::ipc` 54 ok; TS matrix-ipc 48 ok; no
  `matrix_sdk` in IPC modules; no production Matrix Tauri commands; no
  dual-backend. **No** production login/sync.
- **P1.6:** **MERGED** into integration (PR #56). Architectural CI guardrails:
  `matrix-rust-p1.6-guardrails.mjs` (JS SDK import allowlist freeze, wire-module
  SDK bans, raw HTTP, versioned IPC) + `check-matrix-rust-sdk-guardrails.mjs`
  (dual-backend ban, no production Client under matrix/, no matrix_* Tauri
  commands); allowlist `p1.6-js-sdk-import-allowlist.json`; prohibited fixtures;
  wired into `check:matrix-boundaries` + CI. Independent review: unit tests 26
  ok; live guardrails PASS. **No** production login/sync; **no** dual-backend.
- **P2.1:** **MERGED** into integration (PR #59). Pure supervisor actor
  foundation under `src-tauri/src/matrix/supervisor/`. Design:
  `p2.1-matrix-supervisor-actor.md` + `.json`.
- **P2.3:** **MERGED** into integration (PR #63). Unauthenticated
  `matrix_sdk::Client` builder under `src-tauri/src/matrix/client_builder/`
  (sqlite + bundled-sqlite features; sole allowed `Client::builder` site;
  no login/restore/sync). Independent review: `matrix::` **118** PASS;
  guardrails PASS. Design: `p2.3-sdk-client-builder.md` + `.json`.
- **P2.4:** **MERGED** into integration (PR #65). `TaskSupervisor` under
  `src-tauri/src/matrix/tasks/` (kinds sync/listener/upload/search/generic;
  generation-stamped spawn/cancel/join; `retire_stale` / `shutdown_all`;
  bridge `follow_supervisor_generation`). Independent review: `matrix::`
  **135** PASS; guardrails PASS. Design: `p2.4-task-supervision.md` + `.json`.
- **P2.5:** **MERGED** into integration (PR #67). Privacy-filtered health model
  under `src-tauri/src/matrix/diagnostics/` (`MatrixMetrics` +
  `MatrixHealthSnapshot` + desktop projection + redaction fixtures; P2.4 task
  counters exported). Independent review: `matrix::` **147** PASS; guardrails
  PASS. Design: `p2.5-diagnostics-health.md` + `.json`.
- **P2.6:** **MERGED** into integration (PR #70). Destructive lifecycle
  under `src-tauri/src/matrix/lifecycle/` (logout ≠ wipe; exact-target wipe;
  store failures never auto-delete). Independent review: `matrix::lifecycle`
  **17** PASS; guardrails PASS. Design: `p2.6-destructive-lifecycle.md` +
  `.json`. **Phase 2 complete.**
- **P3.1:** **MERGED** into integration (PR #72). Discovery + login-flow
  service under `src-tauri/src/matrix/auth/` (mockable transports; product
  well-known 404 IGNORE fallback; Synara domain login-flow types; no login
  execution). Independent review: `matrix::auth` **24** PASS; `matrix::`
  **189** PASS; guardrails PASS. Design: `p3.1-discovery-login-flow.md` +
  `.json`. Residual: live SDK well-known / `get_login_types` adapter deferred.
- **Optional later:** §7.1–7.7 scaffold rows may still have shallow notes if a
  full-matrix depth pass is desired beyond the handoff resume scope
- **Progress loop:** **paused** (former 4-minute session scheduler cancelled;
  no automatic fire). Resume only with explicit owner instruction.
- **No production Matrix login/sync accepted yet** (P1.2–P1.6 foundation;
  P2.1–P2.6 harness lifecycle/store/builder/tasks/diagnostics/wipe;
  P3.1 discovery/login-flow **service only** — does not start a second product
  sync loop or execute login)

Phase 0 evidence accepted (complete):

- P0.1 SDK usage inventory — **merged**
- P0.3 exact Matrix Rust SDK 0.18.0 capability dossier — **merged**
- P0.2 feature-parity traceability (§7.8–7.11 quality audit) — **merged**
- P0.4 Swift/Rust version provenance — **merged**
- P0.5 Toolchain compatibility (Rust 1.93 / Tauri 2 / matrix-sdk 0.18) —
  **merged** (PR #45; `pass-with-residuals`)
- P0.6 Baseline reliability/performance evidence —
  **merged** (PR #46; `pass-with-residuals`; automated timeline mapping
  measured; live UX residuals documented)
- P0.7 Migration UX decision record —
  **merged** (`migration_ux_decided`; docs only; implementation ownership
  starts Phase 3 / P3.7)

Phase 1 progress:

- P1.1 permanent Rust 1.93 toolchain pin — **merged** (PR #48)
- P1.2 exact Matrix Rust SDK deps (`matrix-sdk` / `matrix-sdk-ui` `=0.18.0`)
  + feature rationale / license review — **merged** (PR #49); no production
  login/sync
- P1.3 versioned Matrix IPC schemas — **merged** (PR #50); contracts only;
  no production login/sync; no SDK wire types
- P1.4 Synara domain DTOs (15 families) — **merged** (PR #52); contracts
  only; no SDK object graph; no production login/sync
- P1.5 IPC protocol contract tests — **merged** (PR #54); contracts only;
  no production login/sync; no dual-backend
- P1.6 architectural CI guardrails — **merged** (PR #56); allowlist freeze +
  wire/dual-backend/Tauri bans; no production login/sync

Phase 2 progress:

- P2.1 Matrix supervisor actor — **merged** (PR #59); pure state machine +
  generation isolation; Client handle deferred to P2.3
- P2.2 Store paths and encryption keys — **merged** (PR #61); per-account paths
  + key vault foundation; live keyring Entry deferred
- P2.3 SDK client builder — **merged** (PR #63); unauthenticated open only;
  sole `Client::builder` site under `matrix/client_builder/`
- P2.4 Task supervision and cancellation — **merged** (PR #65);
  `TaskSupervisor` + generation isolation under `matrix/tasks/`
- P2.5 Diagnostics and health model — **merged** (PR #67);
  privacy-filtered metrics + desktop projection under `matrix/diagnostics/`
- P2.6 Destructive lifecycle operations — **merged** (PR #70); **Phase 2 complete**; logout + exact-target wipe +
  non-destructive store-failure recovery under `matrix/lifecycle/`

Phase 3 progress:

- P3.1 Discovery and login-flow service — **merged** (PR #72); mockable
  discovery + login-flow harness under `matrix/auth/`; product 404 IGNORE
  fallback; design `p3.1-discovery-login-flow.md` + `.json`; no login execution
- P3.2 Password/token login and device naming — **not started** (next)
- P3.3 SSO/OAuth callback lifecycle — not started
- P3.4 UIA/registration/password-reset capability — not started
- P3.5 Refresh-token persistence and rotation — not started
- P3.6 Session restore and account switching — not started
- P3.7 Legacy-session detection and transition coordinator — not started
- P3.8 Logout, remote logout, local wipe, and recovery copy — not started

**Next program work:**

1. **P3.2** password/token login and device naming (harness only; no
   dual-backend; no production Matrix Tauri cutover; tokens must not enter
   WebView storage or IPC after Rust login)
2. P3.3–P3.8 remainder of Phase 3, then Phases 4–14 per plan
3. Continue sole-owner cutover path (no dual-backend selector)
4. **Umbrella PR #39 → `main` stays open/unmerged** until user-approved release

Accepted notification findings that must be preserved:

- FR-7.8-001: current global/default push-rule behavior is implemented; its
  cutover test must use the named All Messages controls and typed Rust push-rule
  behavior, not capability IDs alone.
- FR-7.8-002: current per-room modes are implemented through
  `RoomNotificationModeSwitcher` and `useRoomsNotificationPreferences`; global
  controls cannot substitute.
- FR-7.8-003: status is `partial` under
  `GATE-7.8-003-INVITE-PREFERENCE`. Invite delivery is not a persistent,
  user-configurable invite-notification preference.
- FR-7.8-004: status `implemented`. Desktop native notification **generation**
  is owned by `ClientNonUIFeatures` (`InviteNotifications`,
  `MessageNotifications`, plus `AgentApprovalNotifications` /
  `LaterReminderNotifications` emitters) with `SystemNotification` **enablement
  only** (`showNotifications` + OS/browser permission) and platform bridge
  `normalizeSystemNotificationRequest` / `showPlatformNotification`. Push-rule
  preference UIs, badge/tray/favicon, EmailNotification pusher, SC-057 alone,
  helper/fixture-only, and raw `/_matrix/` HTTP never pass. Cutover is P9.2
  Rust-owned notification candidate stream + desktop bridge.
- FR-7.8-005: status `implemented`. Unread/badge summaries are owned by
  `roomToUnreadAtom` (Timeline/Receipt/MyMembership/MarkedUnread listeners +
  parent roll-up), `badgeSummary.summarizeNotifications` (`appBadgeCount` vs
  `inboxBadgeCount`), `RoomNavItem` `UnreadBadge` (highlight vs total), and
  `PlatformBadgeAndTrayUpdater` / `setPlatformBadgeCount`. SC-057 alone,
  push-rule preference UI, native generation path, and helper-only never pass.
  Cutover is P9.3/P4.3 Rust-owned unread/highlight + product badge/IPC DTO.
- FR-7.8-006: status `implemented`. Event resolution and deep-link routing use
  `buildDesktopNotificationRoomRoute` + `normalizeSystemNotificationRequest` /
  platform route pass-through + `navigateRoom` open (message/agent/later);
  invite notifications use distinct `getInboxInvitesPath`; inbox list Open uses
  `navigateRoom(roomId, openEventId)` with thread-root resolution;
  `timelineOpening` focuses event context. SC-032/SC-022 alone, push-rule UI,
  badge-only, generation-only, and helper-only never pass. Cutover is P9.4/P4.8
  Rust-owned event identity + Synara route DTOs.
- FR-7.8-007: status `implemented`. Focus/suppression owned by
  MessageNotifications gates (`document.hasFocus` + selected room or
  notifications inbox; SYNCING; Mute; self; showNotifications; unread delta)
  with SystemNotification/tray DND enablement; Invite has SYNCING +
  showNotifications without a focused-room gate (do not invent). Cutover P9.3
  via Rust candidate stream + product focus state; SC-057 alone never passes.
- FR-7.8-008: status `partial` under `GATE-7.8-008-ENCRYPTED-PRIVATE-MODE`.
  Message/Invite/Later OS bodies avoid event plaintext/ciphertext; AgentApproval
  may disclose `commandPreview`; `privacy:'private'` is never set by generation
  and is dropped before `desktop_notify`. Cutover must not dump decrypted content
  into OS notifications without a privacy gate.
- FR-7.8-009: status `implemented` (iOS). `SynaraPushService` +
  `MatrixRustSDKService.setPusher`/`deletePusher` (SDK-owned); resolveRoute
  including sparse event-id fallback; Settings registration UI; existing XCTest
  baseline. SC-057/SC-058 are not pusher CRUD. Desktop APNs pusher N/A.
- FR-7.9-001: status `implemented`. Ordered path: IndexedDB stores →
  `store.startup` → `initRustCrypto` → `assertCryptoStoreContinuity` → ready
  client **without** sync → product `startClient` only after crypto readiness
  (`initMatrix.ts` + `cryptoStoreContinuity.ts` + ClientRoot). Current product
  is browser IndexedDB + rust-crypto wasm; cutover is native encrypted SQLite
  under Rust. SC-061/062/083 compile-only blocked states are not runtime pass.
- FR-7.9-002: status `implemented`. Cross-signing active flag is
  `useCrossSigningActive` via `m.cross_signing.master` account-data presence
  (not JS `getCrossSigningStatus`); device `crossSigningVerified` via
  `getDeviceVerificationStatus`; Devices/UnverifiedTab/Logout gated on active;
  bootstrap/reset via `bootstrapCrossSigning`. Ceremony SAS is FR-7.9-005.
  SC-064/061/062 compile-only are not runtime pass.
- FR-7.9-003: status `implemented`. Own-account device list via
  `useDeviceList` → `mx.getDevices()`; Current vs Others split by
  `getDeviceId()` (other = other sessions of the logged-in user, not third
  parties). Devices/OtherDevices/UnverifiedTab; refresh via
  `CryptoEvent.DevicesUpdated`. SC-067 primary (compile-only blocked ≠ pass).
- FR-7.9-004: status `implemented`. Trust bit is
  `getDeviceVerificationStatus.crossSigningVerified` via `verifiedDevice` →
  VerificationStatus badges on Devices/OtherDevices/UnverifiedTab/Logout;
  refresh via DevicesUpdated. SAS ceremony is FR-7.9-005; list is FR-7.9-003.
  SC-063/064 compile-only ≠ product pass.
- FR-7.9-005: status `implemented`. SAS + request inbox:
  `verificationRequestInbox` (install before startClient) queues
  VerificationRequestReceived; ReceiveSelfDeviceVerification presents inbound;
  requestOwnUserVerification / requestDeviceVerification outbound; DeviceVerification
  SAS accept/start/verify/cancel. Trust status is 004; device list is 003.
  SC-084 + GAP-SAS compile-only ≠ product pass.
- FR-7.9-006: status `implemented`. Recovery setup via DeviceVerificationSetup
  (createRecoveryKeyFromPassphrase → bootstrapSecretStorage → resetKeyBackup);
  recovery key display/entry; BackupRestore status + restoreKeyBackup; auto-restore
  on KeyBackupDecryptionKeyCached; repair via reset re-setup. Room-key file
  import/export is FR-7.9-007. SC-065/066 compile-only ≠ product pass.
- FR-7.9-007: status `implemented` (retained UI). Settings Devices LocalBackup
  exportRoomKeysAsJson + encryptMegolmKeyFile → synara-keys.txt; import decrypt
  + importRoomKeysAsJson. Not server key backup (006). SC-061 compile-only ≠ pass.
- FR-7.9-008: status `implemented`. Automatic UTD retry via
  decryptAllTimelineEvent → attemptDecryption({isRetry:true}) on encrypted
  pagination; EncryptedContent MatrixEventEvent.Decrypted re-render; permanent
  UTD fallbacks. No dedicated Retry button.
- FR-7.9-009: status `implemented`. Key-backup state listeners in
  `useKeyBackup.ts` (KeyBackupStatus / SessionsRemaining / Failed /
  DecryptionKeyCached) drive BackupRestore Connected/Disconnected/Syncing/
  failure/trust UI. Recovery setup is 006; LocalBackup files is 007.
- FR-7.9-010: status `implemented`. Other-device multi-select delete via
  `OtherDevices` `mx.deleteMultipleDevices` + sticky Logout; 401 UIA via
  `useUIAMatrixError` → `ActionUIA` Password/SSO; success
  `refreshDeviceList`; OIDC path external `sessionEnd`. Current session uses
  `DeviceLogoutBtn` (not multi-delete). Primary Rust gaps
  GAP-DEVICE-NAMING-DELETE + GAP-UIA; SC-067 list-only secondary.
  List ownership is 003; trust 004; SAS 005; recovery UIA 006.
- FR-7.9-011: status **`partial`** under
  `GATE-7.9-011-CONCURRENT-MULTI-ACCOUNT-STORES`. Sequential single-active
  isolation only: fixed `MATRIX_LOCAL_STORE_NAMES` clear-and-replace via
  `clearMatrixStoresForIdentityChange` on fresh-login identity mismatch;
  single-slot `FALLBACK_SESSION_KEYS` / `clearSessionLocalStorage`;
  `ClientRoot` one `getActiveSession`→`initClient`. Concurrent dual clients /
  per-userId parallel stores are product non-goals (plan text “fully isolated
  multi-account stores” not met). Continuity is FR-7.9-012; logout wipe
  FR-7.1-010; crypto boot FR-7.9-001.
- FR-7.9-012: status `implemented`. Restored sessions must not wipe stores
  (`initClient` freshLogin gate); reopen fixed IndexedDB + `initRustCrypto`;
  `assertCryptoStoreContinuity` (`getCrypto`/`getOwnDeviceKeys`/
  `downloadKeysForUsers`); `stopClient` without store delete on safety fail;
  ClientRoot store-intact UI + Retry only for `server-query-incomplete`.
  Upgrades = reopen same fixed names (no separate migrator). Store-init order
  is FR-7.9-001; multi-account wipe FR-7.9-011; corruption FR-7.9-013.
  Cutover P2.2/P8.8/P13.2 SC-083+SC-061 compile-shape-only.
- FR-7.9-013: status **`partial`**. Continuity anomaly detection +
  non-destructive ClientRoot guidance; no true corruption integrity scan or
  automatic non-destructive repair.
- FR-7.10-001: status `implemented`. Room-scoped `mx.search` room_events
  search_term + filter.rooms limit 20; `next_batch`/`nextToken` infinite query
  pagination; MessageSearch/useMessageSearch. Global search is 002.
- FR-7.10-002: status `implemented`. Home/Space Message Search
  `allowGlobal` + Global chip → `rooms` undefined → `mx.search` without room filter.
  SC-071 only (not SC-072 local). Room-scoped is 001.
- FR-7.10-003: status `implemented`. Two-layer Message Search filters: server
  `filter.rooms`/`filter.senders`/`search_term`/`order_by` via
  `useMessageSearch`/`mx.search`; client type + from/to via
  `filterMessageSearchGroups` / `messageSearchFilters`; SearchFilters
  multi-room/type/sender/date UI. Global is 002; room-scoped default is 001.
- FR-7.10-004: status `implemented`. Open Chip → `navigateRoom(eventId)` →
  `getRoomTimelineOpenMode` focused-event. Not Matrix `/search` event_context
  (before/after 0). JumpToTime not on Open path.
- FR-7.10-005: status **`partial`** under
  `GATE-7.10-005-USER-DIRECTORY-SEARCH`. Public rooms: Explore
  `PublicRooms` POST `/publicRooms`. Server user-directory is widget-only
  (`CallWidgetDriver`); product Invite/CreateChat are exact-ID/local only.
- FR-7.10-006: status `implemented`. Explicit decision: message search is
  server `mx.search` only (SC-071); SC-072 experimental local **not adopted**.
  `useAsyncSearch` is list filter honesty only, not message bodies.
- FR-7.10-007: status **`partial`** under
  `GATE-7.10-007-SEARCH-ABORT-SIGNAL`. Stale isolation via React Query
  `queryKey=['search', term, order, rooms, senders]`; transport cancel
  missing (`mx.search` without optional `abortSignal`; queryFn does not
  forward RQ `signal`).

- FR-7.11-001: status `implemented`. DISPLAY via useCallMembers
  (session.memberships + MembershipsChanged) → RoomNavItem/CallView/CallStatus
  Live UIs. rust_target gap presence boolean-only (not SC-082 primary).
  Cutover residual GATE-7.11-001-FULL-MEMBERSHIP-LIST-PROJECTION.


- FR-7.11-002: status `implemented`. Element Call embed: createCallEmbed /
  CallEmbed.getWidget → `/public/element-call/index.html` + iframe +
  ClientWidgetApi postMessage + CallWidgetDriver capabilities. SC-082
  experimental-widgets (not membership display 001). Widget plumbing ≠ call parity.


- FR-7.11-003: status **`partial`** under
  `GATE-7.11-003-NATIVE-OR-PRODUCT-MEMBERSHIP-WRITE`. Join via useCallStart/
  JoinCall; leave via hangup; decline capability-only (no product Decline UI);
  member status after actions (display ownership 001). Widget-mediated write.


- FR-7.11-004: status **`partial`** under
  `GATE-7.11-004-NATIVE-MATRIXRTC-KEY-SESSION`. Widget-mediated to-device
  encrypt/queue + feedToDevice + encryption_keys capabilities; no product-
  owned native MatrixRTC key-session API.


- FR-7.11-005: status **`partial`** under
  `GATE-7.11-005-LOGOUT-WINDOW-CLOSE-HANGUP-CLEANUP`. Hangup/dispose pipeline
  present; room nav retains session; logout/window-close lack explicit hangup.


- FR-7.11-006: status **`partial`** under
  `GATE-7.11-006-CSP-ORIGIN-HARDENING`. Tauri CSP + iframe sandbox + same-origin
  EC + parentUrl; residual: no HTML CSP meta, scripts+same-origin sandbox,
  broad connect-src, no strictOriginCheck.


- FR-7.11-007: status **`partial`** under
  `GATE-7.11-007-EXPERIMENTAL-WIDGETS-RISK-ACCEPTANCE`. Plan/dossier risk language
  present (SC-082 blocked, P10.1, RISK-CALLS); no formal product acceptance artifact
  for pin 0.18.0 yet.


- FR-7.11-008: status **`not-currently-exposed`** under
  `GATE-7.11-008-DOCUMENTED-CONTINGENCY-ARTIFACT`. Plan §7.11 + P10.7 contingency
  language present; formal contingency decision artifact not delivered. Must not
  reintroduce permanent dual-backend without new user decision.



## Branch and PR contract

1. Start each task branch from the current
   `feature/matrix-rust-sdk-full-replacement` integration branch:
   `matrix-rust/<task-id>-<short-slug>`.
2. Task PRs target the integration branch only. Do not target or merge `main`.
3. Keep each PR to one bounded task with production code, tests, fixtures, and
   necessary documentation in the same reviewable change.
4. Do not mix refactors, formatting sweeps, dependency upgrades, unrelated bug
   fixes, or changes from another task.
5. Rebase/reconcile with integration only after reviewing conflict semantics and
   rerunning the affected task gate.
6. Commit messages must name the task ID. Generated lockfiles/schema changes
   belong with the change that requires them.
7. The final integration-to-`main` PR needs every Section 14 final gate, green
   checks, independent review, and explicit user approval. It is never an
   automatic merge.

## Writer-harness contract

The implementation harness may write only its explicitly allowed task scope. It
must not commit, push, rebase, switch branches, open/merge PRs, delete unrelated
files, or alter this program plan unless the task explicitly authorizes a
documentation update.

Every task prompt must supply:

- task ID, exact branch/base commit, allowed paths, and prohibited paths;
- relevant plan sections and pinned upstream evidence;
- concrete behavior, non-goals, deletion/convergence target, and failure modes;
- exact commands/tests plus live Synapse, platform, or fixture requirements;
- prohibition on `matrix-js-sdk` additions, runtime raw `/_matrix/` HTTP,
  backend selectors, dual clients, weak/fixture-only substitutions, and
  suppressed errors;
- a stop condition when typed SDK support is absent or experimental beyond an
  approved gate.

Use a fresh writer session for each task. Keep prompts narrow enough that a
reviewer can compare the entire diff to the task acceptance criteria.

## Required reviewer evidence

Before accepting a task or merging its PR, the reviewer must independently:

1. Inspect branch/base, `git status`, complete diff, and changed-file scope.
2. Check the exact pinned Matrix Rust SDK API/source—not moving upstream `main`.
3. Reproduce all stated tests and inspect that they exercise the required
   behavior rather than helpers, mocks, renamed controls, or compile shape only.
4. Audit for dual clients, raw Matrix runtime HTTP, SDK-shaped IPC leakage,
   insecure token/store/media handling, lifecycle races, and unremoved legacy
   paths.
5. Check desktop and iOS impacts where the task touches shared contracts.
6. Reject defects with an evidence-backed correction request; review the full
   resulting diff after every correction.
7. Confirm CI is green and the task's plan/documentation/fixtures remain
   synchronized.

## Remaining program sequence

1. Finish and independently accept the active remediation named in the status
   ledger; advance the ledger only after its merge gate passes.
2. Complete R0.2 to deliver the missing threat model, test/Synapse topology,
   review template, owned risk register, full traceability, and residual Phase 0
   evidence.
3. Complete R0.3–R0.6 to repair IPC, store confinement/keyring/layout,
   destructive lifecycle ordering, and diagnostic privacy.
4. Complete R0.7 live disposable-Synapse/platform evidence and the actual P3.1
   SDK adapter.
5. Complete R0.8 formal Phase 0–2 and P3.1 acceptance reports. Do not begin P3.2
   until these gates close.
6. Implement original P3.2 through Phase 11 by bounded capability task:
   authentication/sync, rooms/timelines, messaging/media,
   E2EE/verification/recovery, account data, notifications, search,
   spaces/threads, and calls/widgets.
7. Complete Phase 12 cutover and deletion: Rust is sole desktop Matrix owner;
   no `matrix-js-sdk`, JS sync/crypto/store, or product raw Matrix HTTP remains.
8. Complete Phases 13–14: reliability/performance/security/release validation,
   final deletion audit, integration review, and an explicitly approved final PR
   to `main`.

## Handoff acceptance checklist

- [ ] Task branch is based on the latest integration branch.
- [ ] PR targets integration, not `main`.
- [ ] Diff is bounded and contains no unapproved dependency/version drift.
- [ ] Required tests are present and independently reproduced.
- [ ] Capability/traceability artifacts are updated when behavior evidence
      changes.
- [ ] Reviewer findings are resolved and the complete final diff is re-reviewed.
- [ ] Required CI is green and non-cancelled on the reviewed SHA before merge.
