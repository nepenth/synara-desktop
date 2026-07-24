# Matrix Rust SDK Replacement — Execution Handoff

Last updated: 2026-07-24

Authoritative program plan: [`../matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md)

Traceability artifacts:

- [`feature-parity-traceability.json`](feature-parity-traceability.json)
- [`feature-parity-traceability.md`](feature-parity-traceability.md)

## Current state

This remains a complete replacement program: desktop production must move from
`matrix-js-sdk` to Matrix Rust SDK, not retain a selectable or permanent second
backend. No production Matrix Rust SDK migration code has been accepted yet.

Phase 0 evidence accepted before this handoff:

- P0.1 SDK usage inventory is merged to the integration branch.
- P0.3 exact Matrix Rust SDK 0.18.0 capability dossier is merged to the
  integration branch.
- P0.2 traceability checkpoint commit `06d0f86` is pushed on
  `matrix-rust/p0.2-parity-traceability`. It contains the full 119-requirement
  matrix and accepted corrections for FR-7.8-001 through FR-7.8-003.

P0.2 is not complete. Resume at FR-7.8-004, then audit all remaining 7.8–7.11
requirements before declaring P0.2 accepted.

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
- FR-7.8-004 is source-reviewed but not accepted. The next writer must use the
  concrete SystemNotification and ClientNonUIFeatures native notification
  evidence already recorded in the execution transcript, then provide both JSON
  and Markdown synchronization for review.

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

1. Complete P0.2 and merge its reviewed PR into integration.
2. Complete remaining Phase 0 gates: P0.4 Swift provenance, P0.5 toolchain
   compatibility, P0.6 performance baseline, and P0.7 migration UX.
3. Build the Phase 1 foundation: Rust 1.93, exact SDK pins, versioned
   Synara-owned IPC schemas, lifecycle/store security, and test infrastructure.
4. Implement Phases 2–11 by bounded capability task: authentication/sync,
   rooms/timelines, messaging/media, E2EE/verification/recovery, account data,
   notifications, search, spaces/threads, and calls/widgets.
5. Complete Phase 12 cutover and deletion: Rust is sole desktop Matrix owner;
   no `matrix-js-sdk`, JS sync/crypto/store, or product raw Matrix HTTP remains.
6. Complete Phases 13–14: reliability/performance/security/release validation,
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
- [ ] CI is green before merge.
